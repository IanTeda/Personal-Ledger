//! The floating "1e" file explorer behind `:open` and `:new`, and "1e·pick" behind `:data`
//! (`docs/ux/desktop-mockups/01-shell/README.md`), drawn from `lib-shell`'s explorer Presenter
//! ([`lib_shell::explorer`]), which the Shell's Presenter holds, and [`FsLedgerFolders`], the
//! `LedgerFolders` port it reads the disk through. `render` draws the Presenter's view model; the
//! row list's scroll handle is the Desktop's own.
//!
//! The handoff's Implementation note 11 asks for row eligibility to be "type-driven, not
//! name-driven... gate selectability on the actual file extension read from the filesystem", so
//! the adapter reads the real folder (`std::fs::read_dir`) and each entry's kind is its real
//! extension ([`lib_shell::EntryKind::of`]). `Root` acts on the file the Presenter chooses:
//! opening it, or creating the one `:new`'s File name names (#980).

use std::{
    path::{Path, PathBuf},
    rc::Rc,
    time::SystemTime,
};

use gpui::{App, BoxShadow, SharedString, Window, div, point, prelude::*, px};
use gpui_component::Sizable;
use lib_shell::{
    EntryKind, ExplorerFilter, ExplorerFilters, ExplorerMode, ExplorerViewModel, FolderEntry,
    FolderUnreadable, LedgerFolders,
};

use crate::{assets::icon::DesktopIcon, msg, shell::view::statusline::under_home, theme::color};

pub use lib_shell::explorer::display_name;

/// Dialog width: the "1e" spec's own `640px`.
pub const WIDTH: gpui::Pixels = px(640.0);

/// Distance from the window's top edge: the "1e" spec's own `top: 80px`.
pub const TOP_OFFSET: gpui::Pixels = px(80.0);

/// Caps the row list at roughly the mockup's own 6 visible rows (9px vertical padding + a
/// ~13px line, per row) before it scrolls -- the mockup's own illustrative listing never shows
/// more than that, but a real directory can hold far more entries than fit the dialog.
const MAX_ROWS_HEIGHT: gpui::Pixels = px(6.0 * 31.0);

/// The explorer's folders, read from the disk. An entry whose metadata can't be read (a race with
/// its deletion) is left out.
#[derive(Debug, Clone, Copy, Default)]
pub struct FsLedgerFolders;

impl LedgerFolders for FsLedgerFolders {
    fn read_folder(&self, folder: &Path) -> Result<Vec<FolderEntry>, FolderUnreadable> {
        let read_dir =
            std::fs::read_dir(folder).map_err(|error| FolderUnreadable(error.to_string()))?;
        Ok(read_dir
            .flatten()
            .filter_map(|entry| {
                let path = entry.path();
                let metadata = entry.metadata().ok()?;
                let is_folder = metadata.is_dir();
                Some(FolderEntry {
                    name: entry.file_name().to_string_lossy().into_owned(),
                    kind: EntryKind::of(&path, is_folder),
                    size: (!is_folder).then_some(metadata.len()),
                    modified: metadata.modified().ok(),
                    path,
                })
            })
            .collect())
    }

    /// A broken link is there too: `symlink_metadata` doesn't follow it.
    fn exists(&self, path: &Path) -> bool {
        std::fs::symlink_metadata(path).is_ok()
    }
}

/// Breadcrumb segments for `path`, oldest first: a display label paired with the real ancestor
/// path a click on it should jump to (the Presenter's `explorer_browse`). A path under the home
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
pub(crate) fn format_modified(modified: SystemTime, now: SystemTime) -> String {
    let elapsed = now.duration_since(modified).unwrap_or_default().as_secs();
    if elapsed < 60 {
        return msg::desktop_explorer_modified_just_now();
    }
    if elapsed < 3_600 {
        msg::desktop_explorer_modified_minutes(i64::try_from(elapsed / 60).unwrap_or(i64::MAX))
    } else if elapsed < 86_400 {
        msg::desktop_explorer_modified_hours(i64::try_from(elapsed / 3_600).unwrap_or(i64::MAX))
    } else if elapsed < 7 * 86_400 {
        msg::desktop_explorer_modified_days(i64::try_from(elapsed / 86_400).unwrap_or(i64::MAX))
    } else if elapsed < 30 * 86_400 {
        msg::desktop_explorer_modified_weeks(
            i64::try_from(elapsed / (7 * 86_400)).unwrap_or(i64::MAX),
        )
    } else if elapsed < 365 * 86_400 {
        msg::desktop_explorer_modified_months(
            i64::try_from(elapsed / (30 * 86_400)).unwrap_or(i64::MAX),
        )
    } else {
        msg::desktop_explorer_modified_years(
            i64::try_from(elapsed / (365 * 86_400)).unwrap_or(i64::MAX),
        )
    }
}

/// A row click: the clicked entry's path and the platform's own click count (`2` for a real
/// double-click) -- `Root::handle_explorer_entry_click` opens immediately on a double-click
/// landing on an already-selected `.pldb` row (README: "double-click a `.pldb` row opens
/// immediately"), mirroring `shell::view::rail::primary::OnRowClick`'s own `Rc`-shared-closure shape.
pub type OnEntryClick = Rc<dyn Fn(PathBuf, usize, &mut Window, &mut App)>;
pub type OnBreadcrumbClick = Rc<dyn Fn(PathBuf, &mut Window, &mut App)>;
pub type OnFilterToggle = Rc<dyn Fn(ExplorerFilter, &mut Window, &mut App)>;
pub type OnCancel = Rc<dyn Fn(&mut Window, &mut App)>;
pub type OnOpen = Rc<dyn Fn(&mut Window, &mut App)>;

/// The floating dialog itself: `position: absolute`, centred at [`TOP_OFFSET`] from the window's
/// top edge -- the same centring technique as `shell::view::palette::render`'s own doc explains
/// (`gpui` has no CSS-style transform to recentre a percentage position). `scroll_handle` keeps
/// the row list's scroll, which `Root` puts back at the top on each new folder.
#[expect(
    clippy::too_many_arguments,
    reason = "one handler per affordance, as the palette and rails take theirs"
)]
pub fn render(
    explorer: &ExplorerViewModel,
    scroll_handle: &gpui::ScrollHandle,
    on_entry_click: OnEntryClick,
    on_breadcrumb_click: OnBreadcrumbClick,
    on_filter_toggle: OnFilterToggle,
    on_cancel: OnCancel,
    on_open: OnOpen,
    cx: &App,
) -> gpui::AnyElement {
    let entries = &explorer.entries;
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
                .bg(color::background(cx))
                .border(px(2.0))
                .border_color(color::foreground(cx))
                .shadow(vec![BoxShadow {
                    color: color::dialog_shadow(cx).into(),
                    offset: point(px(0.0), px(16.0)),
                    blur_radius: px(48.0),
                    spread_radius: px(0.0),
                }])
                .child(header(explorer.mode, explorer.current.as_deref(), cx))
                .child(path_bar(
                    &explorer.folder,
                    entries.len(),
                    on_breadcrumb_click,
                    cx,
                ))
                .child(column_header(cx))
                .child(
                    div()
                        .id("explorer-rows")
                        .max_h(MAX_ROWS_HEIGHT)
                        .overflow_y_scroll()
                        .track_scroll(scroll_handle)
                        .children(entries.iter().enumerate().map(|(index, entry)| {
                            let is_last = index == entries.len() - 1;
                            let selected =
                                explorer.selected.as_deref() == Some(entry.path.as_path());
                            row(
                                entry,
                                index,
                                explorer.mode,
                                selected,
                                is_last,
                                on_entry_click.clone(),
                                cx,
                            )
                        })),
                )
                .when(explorer.mode == ExplorerMode::New, |this| {
                    this.child(file_name_field(
                        &explorer.file_name,
                        explorer.name_taken.clone(),
                        &explorer.folder,
                        cx,
                    ))
                })
                .child(footer(
                    explorer.mode,
                    &explorer.folder,
                    explorer.filters,
                    explorer.can_confirm,
                    on_filter_toggle,
                    on_cancel,
                    on_open,
                    cx,
                )),
        )
        .into_any_element()
}

/// The title, and in the folder picker the Data folder now on its right.
fn header(mode: ExplorerMode, current: Option<&Path>, cx: &App) -> impl IntoElement {
    let title = match mode {
        ExplorerMode::Open => msg::desktop_explorer_open(),
        ExplorerMode::New => msg::desktop_explorer_new(),
        ExplorerMode::PickFolder => msg::desktop_explorer_pick(),
    };
    let now = current
        .filter(|_| mode == ExplorerMode::PickFolder)
        .map(|folder| {
            let home = dirs::home_dir();
            msg::desktop_explorer_pick_now(&under_home(folder, home.as_deref()))
        });
    div()
        .flex()
        .items_center()
        .gap(px(12.0))
        .px(px(20.0))
        .py(px(16.0))
        .border_b(px(2.0))
        .border_color(color::border(cx))
        .child(
            div()
                .debug_selector(|| "explorer-title".to_string())
                .flex_1()
                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                .text_size(px(16.0))
                .child(title),
        )
        .children(now.map(|now| {
            div()
                .debug_selector(|| "explorer-current".to_string())
                .min_w(px(0.0))
                .truncate()
                .font_family("monospace")
                .text_size(px(12.0))
                .text_color(color::muted(cx))
                .child(now)
        }))
}

fn path_bar(
    current_path: &Path,
    entry_count: usize,
    on_breadcrumb_click: OnBreadcrumbClick,
    cx: &App,
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
        .border_color(color::hairline(cx))
        .text_size(px(12.0))
        .text_color(color::muted(cx))
        .child(
            DesktopIcon::Folder
                .icon()
                .with_size(px(13.0))
                .text_color(color::muted(cx)),
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
                            cx,
                        )
                    }),
            ),
        )
        .child(div().flex_1())
        .child(div().child(msg::desktop_explorer_item_count(i64::from(
            entry_count as u32,
        ))))
}

fn breadcrumb_segment(
    label: String,
    path: PathBuf,
    current: bool,
    on_breadcrumb_click: OnBreadcrumbClick,
    cx: &App,
) -> impl IntoElement {
    div()
        .id(SharedString::from(format!("explorer-breadcrumb-{label}")))
        .cursor_pointer()
        .when(current, |this| {
            this.font_weight(gpui::FontWeight::EXTRA_BOLD)
                .text_color(color::foreground(cx))
        })
        .on_click(move |_event, window, cx| on_breadcrumb_click(path.clone(), window, cx))
        .child(label)
}

fn column_header(cx: &App) -> impl IntoElement {
    div()
        .flex()
        .px(px(20.0))
        .py(px(8.0))
        .border_b(px(1.0))
        .border_color(color::hairline(cx))
        .font_weight(gpui::FontWeight::EXTRA_BOLD)
        .text_size(px(10.0))
        .text_color(color::faint_text(cx))
        .child(div().flex_1().child(msg::desktop_explorer_column_name()))
        .child(div().w(px(90.0)).child(msg::desktop_explorer_column_size()))
        .child(
            div()
                .w(px(120.0))
                .child(msg::desktop_explorer_column_modified()),
        )
}

/// One row. In the folder picker the folders are the targets, drawn in full, and the files are
/// dimmed and inert.
fn row(
    entry: &FolderEntry,
    index: usize,
    mode: ExplorerMode,
    selected: bool,
    is_last: bool,
    on_entry_click: OnEntryClick,
    cx: &App,
) -> impl IntoElement {
    let picking = mode == ExplorerMode::PickFolder;
    let clickable = if picking {
        entry.kind == EntryKind::Folder
    } else {
        entry.kind != EntryKind::Other
    };
    let (bg, text_color) = if selected {
        (
            Some(color::selection_background(cx)),
            color::selection_text(cx),
        )
    } else {
        (
            None,
            match (picking, entry.kind) {
                (true, EntryKind::Folder) => color::foreground(cx),
                (true, _) | (false, EntryKind::Folder) => color::faint_text(cx),
                (false, EntryKind::Ledger | EntryKind::Other) => color::muted(cx),
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
        .debug_selector(move || format!("explorer-row-{index}"))
        .when(clickable, |this| this.cursor_pointer())
        .when(!is_last, |this| {
            this.border_b(px(1.0)).border_color(color::hairline(cx))
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

/// 1e·new's File name field under the listing, focused: the typed name, the caret and the
/// `.pldb` Create adds, and under it the inline refusal when `taken` names a file already in
/// `folder`, with the field's border in the refusal's colour.
fn file_name_field(
    file_name: &str,
    taken: Option<String>,
    folder: &Path,
    cx: &App,
) -> impl IntoElement {
    let refused = taken.is_some();
    let border = if refused {
        color::negative_text(cx)
    } else {
        color::foreground(cx)
    };
    let folder_name = display_name(folder);
    div()
        .flex()
        .flex_col()
        .gap(px(6.0))
        .pt(px(14.0))
        .pb(px(4.0))
        .px(px(20.0))
        .border_t(px(1.0))
        .border_color(color::hairline(cx))
        .child(
            div()
                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                .text_size(px(12.0))
                .text_color(color::foreground(cx))
                .child(msg::desktop_explorer_file_name()),
        )
        .child(
            div()
                .debug_selector(|| "explorer-file-name".to_string())
                .flex()
                .items_center()
                .px(px(10.0))
                .py(px(8.0))
                .border(px(2.0))
                .border_color(border)
                .text_color(color::foreground(cx))
                .child(file_name.to_string())
                .child(div().w(px(2.0)).h(px(15.0)).bg(color::cursor(cx)))
                .child(div().flex_1())
                // The extension Create adds, as the file is named: not prose, so not a Message.
                .child(div().text_color(color::faint_text(cx)).child(".pldb")),
        )
        .children(taken.map(|name| {
            div()
                .debug_selector(|| "explorer-name-taken".to_string())
                .flex()
                .items_center()
                .gap(px(8.0))
                .text_size(px(11.5))
                .text_color(color::negative_text(cx))
                .child(
                    div()
                        .w(px(8.0))
                        .h(px(8.0))
                        .flex_none()
                        .bg(color::negative_text(cx)),
                )
                .child(msg::desktop_explorer_name_taken(&name, &folder_name))
        }))
        .when(!refused, |this| this.pb(px(4.0)))
}

#[expect(
    clippy::too_many_arguments,
    reason = "the mode and folder it names its button for, its filters, and one handler per affordance"
)]
fn footer(
    mode: ExplorerMode,
    folder: &Path,
    filters: ExplorerFilters,
    can_open: bool,
    on_filter_toggle: OnFilterToggle,
    on_cancel: OnCancel,
    on_open: OnOpen,
    cx: &App,
) -> impl IntoElement {
    let picking = mode == ExplorerMode::PickFolder;
    let confirm_label = match mode {
        ExplorerMode::Open => msg::desktop_explorer_open_button(),
        ExplorerMode::New => msg::desktop_explorer_new_button(),
        ExplorerMode::PickFolder => msg::desktop_explorer_use_button(&display_name(folder)),
    };
    div()
        .flex()
        .items_center()
        .gap(px(10.0))
        .px(px(20.0))
        .py(px(16.0))
        .border_t(px(1.0))
        .border_color(color::hairline(cx))
        .text_size(px(11.5))
        .text_color(color::muted(cx))
        .child(
            div()
                .flex()
                .flex_col()
                .gap(px(8.0))
                // Files are never the folder picker's targets, so it hides only dot entries.
                .when(!picking, |this| {
                    this.child(filter_checkbox(
                        "explorer-filter-pldb-only",
                        filters.pldb_only,
                        div().child(msg::desktop_explorer_filter_pldb_only()),
                        ExplorerFilter::PldbOnly,
                        on_filter_toggle.clone(),
                        cx,
                    ))
                })
                .child(filter_checkbox(
                    "explorer-filter-hide-hidden",
                    filters.hide_hidden,
                    div()
                        .flex()
                        .gap(px(4.0))
                        .child(msg::desktop_explorer_filter_hide_hidden())
                        .child(
                            div()
                                .text_color(color::muted(cx))
                                .child(msg::desktop_explorer_filter_hide_hidden_detail()),
                        ),
                    ExplorerFilter::HideHidden,
                    on_filter_toggle,
                    cx,
                ))
                .when(picking, |this| {
                    this.child(
                        div()
                            .debug_selector(|| "explorer-pick-note".to_string())
                            .child(msg::desktop_explorer_pick_note()),
                    )
                }),
        )
        .child(div().flex_1())
        .child(
            div()
                .id("explorer-cancel")
                .debug_selector(|| "explorer-cancel".to_string())
                .cursor_pointer()
                .py(px(8.0))
                .px(px(16.0))
                .border(px(1.0))
                .border_color(color::border(cx))
                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                .text_color(color::foreground(cx))
                .on_click(move |_event, window, cx| on_cancel(window, cx))
                .child(msg::desktop_explorer_cancel()),
        )
        .child(
            div()
                .id("explorer-open")
                .debug_selector(|| "explorer-open".to_string())
                .when(can_open, |this| this.cursor_pointer())
                .py(px(8.0))
                .px(px(16.0))
                .bg(if can_open {
                    color::foreground(cx)
                } else {
                    color::inset_track(cx)
                })
                .text_color(if can_open {
                    color::background(cx)
                } else {
                    color::faint_text(cx)
                })
                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                .when(can_open, |this| {
                    this.on_click(move |_event, window, cx| on_open(window, cx))
                })
                .child(confirm_label),
        )
}

/// A 14×14 square checkbox, ink-filled when checked and without a radius (frame 1e).
fn filter_checkbox(
    id: &'static str,
    checked: bool,
    label: impl IntoElement,
    filter: ExplorerFilter,
    on_toggle: OnFilterToggle,
    cx: &App,
) -> impl IntoElement {
    div()
        .id(id)
        .cursor_pointer()
        .flex()
        .items_center()
        .gap(px(8.0))
        .text_color(color::foreground(cx))
        .on_click(move |_event, window, cx| on_toggle(filter, window, cx))
        .child(
            div()
                .w(px(14.0))
                .h(px(14.0))
                .flex_none()
                .border_1()
                .border_color(color::foreground(cx))
                .bg(if checked {
                    color::foreground(cx)
                } else {
                    color::background(cx)
                }),
        )
        .child(label)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_disk_adapter_reads_a_real_folder_and_says_what_is_there() {
        let dir = tempfile::tempdir().expect("tempdir should be creatable");
        std::fs::create_dir(dir.path().join("archive")).expect("a folder");
        std::fs::write(dir.path().join("teda.pldb"), b"stub").expect("a file");
        std::fs::write(dir.path().join("notes.csv"), b"stub").expect("a file");

        let mut entries = FsLedgerFolders
            .read_folder(dir.path())
            .expect("the folder reads");
        entries.sort_by(|a, b| a.name.cmp(&b.name));

        let read: Vec<_> = entries
            .iter()
            .map(|entry| (entry.name.as_str(), entry.kind, entry.size))
            .collect();
        assert_eq!(
            read,
            [
                ("archive", EntryKind::Folder, None),
                ("notes.csv", EntryKind::Other, Some(4)),
                ("teda.pldb", EntryKind::Ledger, Some(4)),
            ]
        );
        assert!(entries.iter().all(|entry| entry.modified.is_some()));
        assert!(FsLedgerFolders.exists(&dir.path().join("teda.pldb")));
        assert!(!FsLedgerFolders.exists(&dir.path().join("teda.pldb-wal")));
        assert!(
            FsLedgerFolders
                .read_folder(&dir.path().join("missing"))
                .is_err()
        );
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
