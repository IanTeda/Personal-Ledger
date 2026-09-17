//! The floating "1e" file explorer behind `:open`
//! (`docs/ux/desktop/Shell & Navigation/README.md`) -- browsing real directories to a `.pldb`
//! ledger file. `Shell` owns `Option<FileExplorer>`, `Some` only while the dialog is open,
//! mirroring `crate::palette::Palette`'s own split: pure, `gpui`-free state and unit tests
//! here, `render` the one method that touches `gpui`.
//!
//! Unlike `Dashboard`'s dummy data, the handoff's own Implementation note 11 is explicit that
//! row eligibility must be "type-driven, not name-driven... gate selectability on the actual
//! file extension read from the filesystem" -- so [`FileExplorer`] walks the real filesystem
//! (`std::fs::read_dir`), not fixture data. "Opening" the selection itself stays a stand-in
//! (`crate::nav::NavState::open_ledger`, from issue #164): real `.pldb` parsing is separate
//! future work, out of scope for this map (issue #144).

use std::{
    path::{Path, PathBuf},
    rc::Rc,
    time::SystemTime,
};

use gpui::{App, BoxShadow, SharedString, Window, div, point, prelude::*, px};
use gpui_component::Sizable;

use crate::{icon::DesktopIcon, theme::color};

/// Dialog width: the "1e" spec's own `640px`.
pub const WIDTH: gpui::Pixels = px(640.0);

/// Distance from the window's top edge: the "1e" spec's own `top: 80px`.
pub const TOP_OFFSET: gpui::Pixels = px(80.0);

/// Caps the row list at roughly the mockup's own 6 visible rows (9px vertical padding + a
/// ~13px line, per row) before it scrolls -- the mockup's own illustrative listing never shows
/// more than that, but a real directory can hold far more entries than fit the dialog.
const MAX_ROWS_HEIGHT: gpui::Pixels = px(6.0 * 31.0);

/// What kind of thing a row represents -- drives both its icon/dimming and whether a click on
/// it selects, navigates, or does nothing (README's "Row click selects it; only `.pldb` rows
/// are selectable as the open target -- folders navigate in, other extensions are inert").
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryKind {
    Folder,
    /// A real `.pldb` file -- the only kind [`FileExplorer::click_entry`] will select.
    Ledger,
    /// Any other file: visible (so the mockup's own decoys read at a glance), never selectable.
    Other,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DirEntry {
    pub name: String,
    pub path: PathBuf,
    pub kind: EntryKind,
    /// `None` for a folder -- the mockup's own SIZE column renders `—` for one.
    pub size: Option<u64>,
    pub modified: Option<SystemTime>,
}

/// Which command opened the dialog (issue #167): `:open` and `:new` share the exact same
/// browse-an-existing-`.pldb` dialog today, differing only in title/button text -- `:new`'s own
/// eventual "create a fresh `.pldb`" workflow is still fog (the user's own words: "we still
/// need to plan out the new file workflow"), not this. `Shell::confirm_explorer_open` also
/// reads this: only `Open` actually flips `NavState::ledger_open` on confirm, so picking an
/// existing file in `New` mode does nothing beyond closing the dialog -- copying an existing
/// ledger's data isn't what "new" means, even as a stand-in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExplorerMode {
    Open,
    New,
}

/// State for the floating file explorer: the directory currently being browsed, its entries,
/// and the current `.pldb` selection (if any). Re-reads the filesystem on every navigation
/// (`reload`) rather than caching -- a directory listing is cheap enough that staleness isn't
/// worth guarding against here.
pub struct FileExplorer {
    mode: ExplorerMode,
    current_path: PathBuf,
    entries: Vec<DirEntry>,
    selected: Option<PathBuf>,
    /// The row list's own scroll, unlike `Palette`'s fixed, small registry -- a real directory
    /// (the mockup's own illustrative 6 rows aside) can hold far more entries than fit the
    /// dialog's [`MAX_ROWS_HEIGHT`], discovered by browsing a real home directory (50 entries)
    /// during this ticket's own verification.
    scroll_handle: gpui::ScrollHandle,
}

impl FileExplorer {
    /// Opens browsing `start` -- callers pass `dirs::home_dir()` (falling back to the current
    /// directory), since the handoff names no default starting directory of its own.
    pub fn open_at(mode: ExplorerMode, start: PathBuf) -> Self {
        let mut explorer = Self {
            mode,
            current_path: start,
            entries: Vec::new(),
            selected: None,
            scroll_handle: gpui::ScrollHandle::new(),
        };
        explorer.reload();
        explorer
    }

    pub fn mode(&self) -> ExplorerMode {
        self.mode
    }

    pub fn current_path(&self) -> &Path {
        &self.current_path
    }

    pub fn entries(&self) -> &[DirEntry] {
        &self.entries
    }

    pub fn selected(&self) -> Option<&Path> {
        self.selected.as_deref()
    }

    /// README's "Open is disabled until a `.pldb` row is selected".
    pub fn can_open(&self) -> bool {
        self.selected.is_some()
    }

    /// Re-reads `current_path` from the real filesystem: folders first, then every other entry,
    /// both ordered case-insensitively by name -- a directory a user can actually read never
    /// depends on the mockup's own illustrative (non-alphabetical) row order. An unreadable
    /// directory (permissions, a race with deletion) yields an empty listing rather than an
    /// error -- there's nothing actionable a user could do about it from this dialog beyond
    /// navigating elsewhere.
    fn reload(&mut self) {
        self.selected = None;
        self.scroll_handle.set_offset(gpui::Point::default());
        let mut entries = Vec::new();
        if let Ok(read_dir) = std::fs::read_dir(&self.current_path) {
            for entry in read_dir.flatten() {
                let path = entry.path();
                let Ok(metadata) = entry.metadata() else {
                    continue;
                };
                let is_dir = metadata.is_dir();
                let kind = if is_dir {
                    EntryKind::Folder
                } else if is_pldb(&path) {
                    EntryKind::Ledger
                } else {
                    EntryKind::Other
                };
                entries.push(DirEntry {
                    name: entry.file_name().to_string_lossy().into_owned(),
                    path,
                    kind,
                    size: (!is_dir).then_some(metadata.len()),
                    modified: metadata.modified().ok(),
                });
            }
        }
        entries.sort_by(|a, b| {
            let a_is_folder = a.kind == EntryKind::Folder;
            let b_is_folder = b.kind == EntryKind::Folder;
            b_is_folder
                .cmp(&a_is_folder)
                .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
        });
        self.entries = entries;
    }

    /// A row click (README: "Row click selects it; only `.pldb` rows are selectable as the open
    /// target -- folders navigate in, other extensions are inert"). A `path` that no longer
    /// matches any current entry (stale after a concurrent filesystem change) is a no-op.
    pub fn click_entry(&mut self, path: &Path) {
        let Some(entry) = self.entries.iter().find(|entry| entry.path == path) else {
            return;
        };
        match entry.kind {
            EntryKind::Folder => {
                self.current_path = entry.path.clone();
                self.reload();
            }
            EntryKind::Ledger => self.selected = Some(entry.path.clone()),
            EntryKind::Other => {}
        }
    }

    /// A breadcrumb segment click (README: "Breadcrumb segments are clickable to jump up the
    /// path"): jumps straight to `path`, one of [`breadcrumb_segments`]'s own ancestor paths.
    pub fn navigate_to(&mut self, path: PathBuf) {
        self.current_path = path;
        self.reload();
    }
}

/// The literal, case-insensitive `.pldb` extension check the handoff's Implementation note 11
/// calls for ("gate selectability on the actual file extension... not name-driven") --
/// `Path::extension()`, never a filename substring match, so `old-ledger.pldb.bak` (the
/// mockup's own decoy) reads as extension `bak`, not a match.
fn is_pldb(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("pldb"))
}

/// Breadcrumb segments for `path`, oldest first: a display label paired with the real ancestor
/// path a click on it should jump to (`FileExplorer::navigate_to`). A path under the home
/// directory collapses its home prefix to a leading `~` segment, matching the mockup's own
/// `~ / documents / **ledgers**`; anything else walks every component from the filesystem root.
pub fn breadcrumb_segments(path: &Path) -> Vec<(String, PathBuf)> {
    if let Some(home) = dirs::home_dir()
        && let Ok(relative) = path.strip_prefix(&home)
    {
        let mut segments = vec![("~".to_string(), home.clone())];
        let mut ancestor = home;
        for component in relative.components() {
            ancestor.push(component);
            segments.push((
                component.as_os_str().to_string_lossy().into_owned(),
                ancestor.clone(),
            ));
        }
        return segments;
    }

    let mut segments = Vec::new();
    let mut ancestor = PathBuf::new();
    for component in path.components() {
        ancestor.push(component);
        let label = component.as_os_str().to_string_lossy().into_owned();
        if !label.is_empty() {
            segments.push((label, ancestor.clone()));
        }
    }
    segments
}

/// `bytes` as the mockup's own "184 kb" / "2.61 mb" style: whole kilobytes under 1MB, two
/// decimal megabytes at or above it. Decimal (1000-based), matching the mockup's own figures
/// more closely than binary (1024-based) would.
fn format_size(bytes: u64) -> String {
    const KB: f64 = 1_000.0;
    const MB: f64 = 1_000_000.0;
    let bytes = bytes as f64;
    if bytes < MB {
        format!("{} kb", (bytes / KB).round() as u64)
    } else {
        format!("{:.2} mb", bytes / MB)
    }
}

/// `modified` relative to `now` as the mockup's own "10 minutes ago" / "3 weeks ago" style --
/// `now` is threaded through rather than read live so this stays a pure, unit-testable
/// function; [`row`] calls it with `SystemTime::now()`.
fn format_modified(modified: SystemTime, now: SystemTime) -> String {
    let elapsed = now.duration_since(modified).unwrap_or_default().as_secs();
    if elapsed < 60 {
        return "just now".to_string();
    }
    let (value, unit) = if elapsed < 3_600 {
        (elapsed / 60, "minute")
    } else if elapsed < 86_400 {
        (elapsed / 3_600, "hour")
    } else if elapsed < 7 * 86_400 {
        (elapsed / 86_400, "day")
    } else if elapsed < 30 * 86_400 {
        (elapsed / (7 * 86_400), "week")
    } else if elapsed < 365 * 86_400 {
        (elapsed / (30 * 86_400), "month")
    } else {
        (elapsed / (365 * 86_400), "year")
    };
    let plural = if value == 1 { "" } else { "s" };
    format!("{value} {unit}{plural} ago")
}

/// A row click: the clicked entry's path and the platform's own click count (`2` for a real
/// double-click) -- `Shell::handle_explorer_entry_click` opens immediately on a double-click
/// landing on an already-selected `.pldb` row (README: "double-click a `.pldb` row opens
/// immediately"), mirroring `rail::primary::OnRowClick`'s own `Rc`-shared-closure shape.
pub type OnEntryClick = Rc<dyn Fn(PathBuf, usize, &mut Window, &mut App)>;
pub type OnBreadcrumbClick = Rc<dyn Fn(PathBuf, &mut Window, &mut App)>;
pub type OnCancel = Rc<dyn Fn(&mut Window, &mut App)>;
pub type OnOpen = Rc<dyn Fn(&mut Window, &mut App)>;

impl FileExplorer {
    /// The floating dialog itself: `position: absolute`, centred at [`TOP_OFFSET`] from the
    /// window's top edge -- the same centring technique as `Palette::render`'s own doc explains
    /// (`gpui` has no CSS-style transform to recentre a percentage position). Borrows `self`
    /// rather than consuming it, since `Shell` keeps this state past the render call that draws
    /// it.
    pub fn render(
        &self,
        on_entry_click: OnEntryClick,
        on_breadcrumb_click: OnBreadcrumbClick,
        on_cancel: OnCancel,
        on_open: OnOpen,
    ) -> gpui::AnyElement {
        div()
            .absolute()
            .top(TOP_OFFSET)
            .left(px(0.0))
            .right(px(0.0))
            .flex()
            .justify_center()
            .child(
                div()
                    .w(WIDTH)
                    .flex()
                    .flex_col()
                    .bg(color::GROUND)
                    .border(px(2.0))
                    .border_color(color::INK)
                    .shadow(vec![BoxShadow {
                        color: color::DIALOG_SHADOW.into(),
                        offset: point(px(0.0), px(16.0)),
                        blur_radius: px(48.0),
                        spread_radius: px(0.0),
                    }])
                    .child(header(self.mode))
                    .child(path_bar(
                        &self.current_path,
                        self.entries.len(),
                        on_breadcrumb_click,
                    ))
                    .child(column_header())
                    .child(
                        div()
                            .id("explorer-rows")
                            .max_h(MAX_ROWS_HEIGHT)
                            .overflow_y_scroll()
                            .track_scroll(&self.scroll_handle)
                            .children(self.entries.iter().enumerate().map(|(index, entry)| {
                                let is_last = index == self.entries.len() - 1;
                                let selected =
                                    self.selected.as_deref() == Some(entry.path.as_path());
                                row(entry, index, selected, is_last, on_entry_click.clone())
                            })),
                    )
                    .child(footer(self.mode, self.can_open(), on_cancel, on_open)),
            )
            .into_any_element()
    }
}

fn header(mode: ExplorerMode) -> impl IntoElement {
    let title = match mode {
        ExplorerMode::Open => "Open ledger file",
        // A literal copy of the Open dialog's own title, per issue #167 -- `:new`'s real
        // "create a fresh .pldb" workflow is still fog, not this.
        ExplorerMode::New => "New ledger file",
    };
    div()
        .px(px(20.0))
        .py(px(16.0))
        .border_b(px(2.0))
        .border_color(gpui::rgba(0x201e1d4d)) // rgba(32,30,29,.30)
        .font_weight(gpui::FontWeight::EXTRA_BOLD)
        .text_size(px(16.0))
        .child(title)
}

fn path_bar(
    current_path: &Path,
    entry_count: usize,
    on_breadcrumb_click: OnBreadcrumbClick,
) -> impl IntoElement {
    let segments = breadcrumb_segments(current_path);
    let last_index = segments.len().saturating_sub(1);

    div()
        .flex()
        .items_center()
        .gap(px(8.0))
        .px(px(20.0))
        .py(px(12.0))
        .border_b(px(1.0))
        .border_color(color::HAIRLINE)
        .text_size(px(12.0))
        .text_color(color::INK_SECONDARY)
        .child(
            DesktopIcon::Folder
                .icon()
                .with_size(px(13.0))
                .text_color(color::INK_SECONDARY),
        )
        .child(
            div().flex().items_center().gap(px(6.0)).children(
                segments
                    .into_iter()
                    .enumerate()
                    .map(|(index, (label, path))| {
                        breadcrumb_segment(
                            label,
                            path,
                            index == last_index,
                            on_breadcrumb_click.clone(),
                        )
                    }),
            ),
        )
        .child(div().flex_1())
        .child(div().child(format!("{entry_count} items")))
}

fn breadcrumb_segment(
    label: String,
    path: PathBuf,
    current: bool,
    on_breadcrumb_click: OnBreadcrumbClick,
) -> impl IntoElement {
    div()
        .id(SharedString::from(format!("explorer-breadcrumb-{label}")))
        .cursor_pointer()
        .when(current, |this| {
            this.font_weight(gpui::FontWeight::EXTRA_BOLD)
                .text_color(color::INK)
        })
        .on_click(move |_event, window, cx| on_breadcrumb_click(path.clone(), window, cx))
        .child(label)
}

fn column_header() -> impl IntoElement {
    div()
        .flex()
        .px(px(20.0))
        .py(px(8.0))
        .border_b(px(1.0))
        .border_color(color::HAIRLINE)
        .font_weight(gpui::FontWeight::EXTRA_BOLD)
        .text_size(px(10.0))
        .text_color(color::INK_TERTIARY)
        .child(div().flex_1().child("NAME"))
        .child(div().w(px(90.0)).child("SIZE"))
        .child(div().w(px(120.0)).child("MODIFIED"))
}

fn row(
    entry: &DirEntry,
    index: usize,
    selected: bool,
    is_last: bool,
    on_entry_click: OnEntryClick,
) -> impl IntoElement {
    let clickable = entry.kind != EntryKind::Other;
    let (bg, text_color) = if selected {
        (Some(color::INK), color::INK_ON_DARK)
    } else {
        (
            None,
            match entry.kind {
                EntryKind::Folder => color::INK_TERTIARY,
                EntryKind::Ledger | EntryKind::Other => color::INK_SECONDARY,
            },
        )
    };
    let icon = match entry.kind {
        EntryKind::Folder => DesktopIcon::Folder,
        EntryKind::Ledger | EntryKind::Other => DesktopIcon::File,
    };
    let size_text = entry
        .size
        .map(format_size)
        .unwrap_or_else(|| "\u{2014}".to_string());
    let modified_text = entry
        .modified
        .map(|modified| format_modified(modified, SystemTime::now()))
        .unwrap_or_else(|| "\u{2014}".to_string());
    let path = entry.path.clone();

    div()
        .id(SharedString::from(format!("explorer-row-{index}")))
        .when(clickable, |this| this.cursor_pointer())
        .when(!is_last, |this| {
            this.border_b(px(1.0)).border_color(color::HAIRLINE_LIGHT)
        })
        .when_some(bg, |this, bg| this.bg(bg))
        .flex()
        .items_center()
        .px(px(20.0))
        .py(px(9.0))
        .text_color(text_color)
        .on_click(move |event, window, cx| {
            on_entry_click(path.clone(), event.click_count(), window, cx)
        })
        .child(
            icon.icon()
                .with_size(px(14.0))
                .text_color(text_color)
                .mr(px(9.0)),
        )
        .child(
            div()
                .flex_1()
                .when(selected, |this| {
                    this.font_weight(gpui::FontWeight::EXTRA_BOLD)
                })
                .child(entry.name.clone()),
        )
        .child(div().w(px(90.0)).child(size_text))
        .child(div().w(px(120.0)).child(modified_text))
}

fn footer(
    mode: ExplorerMode,
    can_open: bool,
    on_cancel: OnCancel,
    on_open: OnOpen,
) -> impl IntoElement {
    let confirm_label = match mode {
        ExplorerMode::Open => "Open",
        ExplorerMode::New => "New",
    };
    div()
        .flex()
        .items_center()
        .gap(px(10.0))
        .px(px(20.0))
        .py(px(16.0))
        .border_t(px(1.0))
        .border_color(color::HAIRLINE)
        .text_size(px(11.5))
        .text_color(color::INK_SECONDARY)
        .child(
            div()
                .flex()
                .child("only ")
                .child(
                    div()
                        .font_weight(gpui::FontWeight::EXTRA_BOLD)
                        .text_color(color::INK)
                        .child(".pldb"),
                )
                .child(" files can be opened"),
        )
        .child(div().flex_1())
        .child(
            div()
                .id("explorer-cancel")
                .cursor_pointer()
                .py(px(8.0))
                .px(px(16.0))
                .border(px(1.0))
                .border_color(gpui::rgba(0x201e1d4d)) // rgba(32,30,29,.30)
                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                .text_color(color::INK)
                .on_click(move |_event, window, cx| on_cancel(window, cx))
                .child("Cancel"),
        )
        .child(
            div()
                .id("explorer-open")
                .when(can_open, |this| this.cursor_pointer())
                .py(px(8.0))
                .px(px(16.0))
                .bg(if can_open {
                    color::INK
                } else {
                    color::INSET_TRACK
                })
                .text_color(if can_open {
                    color::INK_ON_DARK
                } else {
                    color::INK_TERTIARY
                })
                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                .when(can_open, |this| {
                    this.on_click(move |_event, window, cx| on_open(window, cx))
                })
                .child(confirm_label),
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn touch(path: &Path) {
        std::fs::write(path, b"stub").expect("write should succeed");
    }

    #[test]
    fn open_at_records_its_mode() {
        let dir = tempfile::tempdir().expect("tempdir should be creatable");

        let explorer = FileExplorer::open_at(ExplorerMode::New, dir.path().to_path_buf());

        assert_eq!(explorer.mode(), ExplorerMode::New);
    }

    #[test]
    fn lists_folders_before_files_case_insensitively() {
        let dir = tempfile::tempdir().expect("tempdir should be creatable");
        std::fs::create_dir(dir.path().join("Zeta")).unwrap();
        std::fs::create_dir(dir.path().join("archive")).unwrap();
        touch(&dir.path().join("beta.pldb"));
        touch(&dir.path().join("Alpha.csv"));

        let explorer = FileExplorer::open_at(ExplorerMode::Open, dir.path().to_path_buf());

        let names: Vec<_> = explorer.entries().iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names, vec!["archive", "Zeta", "Alpha.csv", "beta.pldb"]);
    }

    #[test]
    fn only_a_real_pldb_extension_is_a_ledger() {
        let dir = tempfile::tempdir().expect("tempdir should be creatable");
        touch(&dir.path().join("real.pldb"));
        // A distinct base name, not just a different case of "real.pldb" -- macOS (APFS) and
        // Windows (NTFS) are case-insensitive-but-preserving by default, so two paths differing
        // only by case silently collide onto the one directory entry there (CI caught this: it
        // passed on Linux, panicked on both macOS and Windows).
        touch(&dir.path().join("second.PLDB"));
        touch(&dir.path().join("decoy.pldb.bak"));
        touch(&dir.path().join("plain.csv"));

        let explorer = FileExplorer::open_at(ExplorerMode::Open, dir.path().to_path_buf());

        let kind_of = |name: &str| {
            explorer
                .entries()
                .iter()
                .find(|e| e.name == name)
                .map(|e| e.kind)
                .unwrap()
        };
        assert_eq!(kind_of("real.pldb"), EntryKind::Ledger);
        assert_eq!(
            kind_of("second.PLDB"),
            EntryKind::Ledger,
            "extension check is case-insensitive"
        );
        assert_eq!(
            kind_of("decoy.pldb.bak"),
            EntryKind::Other,
            "a `.pldb.bak` file's real extension is `bak`, not `pldb`"
        );
        assert_eq!(kind_of("plain.csv"), EntryKind::Other);
    }

    #[test]
    fn clicking_a_folder_navigates_into_it() {
        let dir = tempfile::tempdir().expect("tempdir should be creatable");
        let sub = dir.path().join("ledgers");
        std::fs::create_dir(&sub).unwrap();
        touch(&sub.join("inner.pldb"));

        let mut explorer = FileExplorer::open_at(ExplorerMode::Open, dir.path().to_path_buf());
        explorer.click_entry(&sub);

        assert_eq!(explorer.current_path(), sub);
        assert_eq!(explorer.entries().len(), 1);
        assert_eq!(explorer.entries()[0].name, "inner.pldb");
    }

    #[test]
    fn clicking_a_pldb_file_selects_it_but_does_not_open_it() {
        let dir = tempfile::tempdir().expect("tempdir should be creatable");
        let pldb = dir.path().join("real.pldb");
        touch(&pldb);

        let mut explorer = FileExplorer::open_at(ExplorerMode::Open, dir.path().to_path_buf());
        assert!(!explorer.can_open());

        explorer.click_entry(&pldb);

        assert_eq!(explorer.selected(), Some(pldb.as_path()));
        assert!(explorer.can_open());
    }

    #[test]
    fn clicking_a_non_pldb_file_does_nothing() {
        let dir = tempfile::tempdir().expect("tempdir should be creatable");
        let csv = dir.path().join("plain.csv");
        touch(&csv);

        let mut explorer = FileExplorer::open_at(ExplorerMode::Open, dir.path().to_path_buf());
        explorer.click_entry(&csv);

        assert_eq!(explorer.selected(), None);
        assert!(!explorer.can_open());
    }

    #[test]
    fn navigating_into_a_folder_clears_any_prior_selection() {
        let dir = tempfile::tempdir().expect("tempdir should be creatable");
        let pldb = dir.path().join("real.pldb");
        touch(&pldb);
        let sub = dir.path().join("ledgers");
        std::fs::create_dir(&sub).unwrap();

        let mut explorer = FileExplorer::open_at(ExplorerMode::Open, dir.path().to_path_buf());
        explorer.click_entry(&pldb);
        assert!(explorer.can_open());

        explorer.click_entry(&sub);

        assert!(!explorer.can_open());
    }

    #[test]
    fn navigate_to_jumps_straight_to_an_ancestor() {
        let dir = tempfile::tempdir().expect("tempdir should be creatable");
        let sub = dir.path().join("ledgers");
        std::fs::create_dir(&sub).unwrap();

        let mut explorer = FileExplorer::open_at(ExplorerMode::Open, sub.clone());
        explorer.navigate_to(dir.path().to_path_buf());

        assert_eq!(explorer.current_path(), dir.path());
    }

    #[test]
    fn breadcrumb_segments_collapse_the_home_directory_to_a_tilde() {
        let Some(home) = dirs::home_dir() else {
            return; // No home directory on this platform/sandbox -- nothing to assert.
        };
        let path = home.join("documents").join("ledgers");

        let segments = breadcrumb_segments(&path);

        assert_eq!(segments[0].0, "~");
        assert_eq!(segments[0].1, home);
        assert_eq!(segments.last().unwrap().0, "ledgers");
        assert_eq!(segments.last().unwrap().1, path);
    }

    #[test]
    fn format_size_matches_the_mockups_kb_then_two_decimal_mb_style() {
        assert_eq!(format_size(184_000), "184 kb");
        assert_eq!(format_size(2_610_000), "2.61 mb");
    }

    #[test]
    fn format_modified_buckets_elapsed_time_with_correct_pluralisation() {
        let now = SystemTime::now();
        assert_eq!(
            format_modified(now - std::time::Duration::from_secs(30), now),
            "just now"
        );
        assert_eq!(
            format_modified(now - std::time::Duration::from_secs(60), now),
            "1 minute ago"
        );
        assert_eq!(
            format_modified(now - std::time::Duration::from_secs(600), now),
            "10 minutes ago"
        );
        assert_eq!(
            format_modified(now - std::time::Duration::from_secs(6 * 86_400), now),
            "6 days ago"
        );
        assert_eq!(
            format_modified(now - std::time::Duration::from_secs(21 * 86_400), now),
            "3 weeks ago"
        );
        assert_eq!(
            format_modified(now - std::time::Duration::from_secs(400 * 86_400), now),
            "1 year ago"
        );
    }
}
