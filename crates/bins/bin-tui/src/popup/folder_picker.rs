//! The Data folder picker (TUI-064, `docs/ux/tui-mockups/05-settings/README.md`): a popup centred
//! over the dimmed View, the Settings page picker's treatment, hosting `lib-shell`'s
//! [`ExplorerPresenter`] in [`ExplorerMode::PickFolder`] so both Clients walk folders through one
//! model. It is opened from Settings › General and by `:data` over any View, with or without a
//! Ledger open, and the View behind it is untouched, so closing it returns there as it was. The
//! mode stays NORMAL.
//!
//! The TUI has no explorer, and `:open` and `:new` keep their typed paths; this is the only place
//! it walks folders. It reads the disk through `lib_database::FsLedgerFolders`, which the
//! Desktop's explorer shares, since `lib-shell` stays I/O-free.

use std::path::PathBuf;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use lib_config::KeyBindingConfig;
use lib_database::FsLedgerFolders;
use lib_shell::{
    DialogCommand, EntryKind, ExplorerMode, ExplorerOutcome, ExplorerViewModel, LedgerFolders,
    dialog_keymap,
    explorer::{ExplorerPresenter, display_name, under_home},
};
use lib_viper::keymap::Keymap;
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
};

use crate::colours::Colours;
use crate::msg;

/// The popup's width in cells, border included: the Settings page picker's treatment, wide enough
/// for a folder path and the `now` note beside the title.
const WIDTH: u16 = 76;

/// The most folder rows it shows before it scrolls.
const MAX_ROWS: u16 = 12;

/// What a key did to the picker, for the Shell to act on.
#[derive(Debug, PartialEq, Eq)]
pub enum Outcome {
    /// Nothing the Shell needs to do.
    Stay,
    /// `esc`: close it with nothing changed.
    Close,
    /// `u`: the folder shown, for the Shell to make the Data folder.
    Use(PathBuf),
}

/// The open picker.
pub struct FolderPickerPopup {
    presenter: ExplorerPresenter,
    keymap: Keymap<DialogCommand>,
    folders: Box<dyn LedgerFolders>,
}

impl std::fmt::Debug for FolderPickerPopup {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FolderPickerPopup")
            .field("presenter", &self.presenter)
            .finish_non_exhaustive()
    }
}

impl FolderPickerPopup {
    /// Opens on `current`, the Data folder, falling back to `home` when it can't be read, its
    /// keys the user's Dialog keys from `keybindings`, as the footer names them.
    pub fn open(current: PathBuf, home: Option<PathBuf>, keybindings: &KeyBindingConfig) -> Self {
        Self::open_on(current, home, keybindings, Box::new(FsLedgerFolders))
    }

    /// [`Self::open`] over any `folders`, so a test can walk a made-up disk.
    pub fn open_on(
        current: PathBuf,
        home: Option<PathBuf>,
        keybindings: &KeyBindingConfig,
        folders: Box<dyn LedgerFolders>,
    ) -> Self {
        let mut presenter = ExplorerPresenter::default();
        presenter.set_start_folder(Some(current));
        presenter.set_fallback_folder(home);
        presenter.open(ExplorerMode::PickFolder, folders.as_ref());
        Self {
            presenter,
            keymap: dialog_keymap(keybindings),
            folders,
        }
    }

    /// The folder shown, which Use picks.
    pub fn folder(&self) -> Option<PathBuf> {
        self.view_model().map(|view| view.folder)
    }

    fn view_model(&self) -> Option<ExplorerViewModel> {
        self.presenter.view_model(&self.keymap)
    }

    /// The picker takes every key while open. `u` stands in for the Desktop's `ctrl-enter`, which
    /// many terminals send as `enter`; the rest go through the Dialog keymap as the Desktop's do.
    pub fn handle_key(&mut self, key: KeyEvent) -> Outcome {
        if key.modifiers == KeyModifiers::NONE && key.code == KeyCode::Char('u') {
            return Self::outcome(self.presenter.confirm());
        }
        let Some(chord) = chord(key) else {
            return Outcome::Stay;
        };
        let outcome = self
            .presenter
            .key(&chord, None, &self.keymap, self.folders.as_ref());
        Self::outcome(outcome)
    }

    fn outcome(outcome: ExplorerOutcome) -> Outcome {
        match outcome {
            ExplorerOutcome::Closed => Outcome::Close,
            ExplorerOutcome::FolderPicked(folder) => Outcome::Use(folder),
            ExplorerOutcome::Ignored | ExplorerOutcome::Changed | ExplorerOutcome::Chosen(_) => {
                Outcome::Stay
            }
        }
    }

    /// The footer's keys, naming the folder `u` uses.
    pub fn footer(&self, back_key: &str) -> String {
        let folder = self
            .folder()
            .map(|folder| display_name(&folder))
            .unwrap_or_default();
        msg::tui_footer_folder_picker(&folder, back_key)
    }

    /// Draws the popup centred in `area`: the title with `now <current>` on its right, the folder
    /// shown, its folders with the files dim and inert, and the Use chip.
    pub fn render(&self, frame: &mut Frame<'_>, area: Rect, c: &Colours) {
        let Some(view) = self.view_model() else {
            return;
        };
        let rows = u16::try_from(view.entries.len() + 1).unwrap_or(u16::MAX);
        let shown = rows.clamp(2, MAX_ROWS);
        // Border, title and its rule, the path line, the rows, a blank and the Use chip.
        let popup = centred(area, WIDTH, shown + 8);
        frame.render_widget(Clear, popup);
        let block = Block::bordered();
        let inner = block.inner(popup);
        frame.render_widget(block, popup);

        let [heading, path, list, _, chip] = Layout::vertical([
            Constraint::Length(2),
            Constraint::Length(1),
            Constraint::Min(1),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .areas(inner);

        let home = dirs::home_dir();
        let now = view
            .current
            .as_deref()
            .map(|current| msg::tui_folder_picker_now(&under_home(current, home.as_deref())))
            .unwrap_or_default();
        render_heading(frame, heading, &msg::tui_folder_picker_title(), &now, c);

        frame.render_widget(
            Paragraph::new(Span::styled(
                under_home(&view.folder, home.as_deref()),
                Style::default().add_modifier(Modifier::BOLD),
            )),
            path,
        );

        self.render_rows(frame, list, &view, c);

        let label = msg::tui_folder_picker_use(&display_name(&view.folder));
        frame.render_widget(
            Paragraph::new(Span::styled(label, c.accent()))
                .alignment(ratatui::layout::Alignment::Right),
            chip,
        );
    }

    /// The `../` row, then the entries, scrolled so the cursor stays in view.
    fn render_rows(
        &self,
        frame: &mut Frame<'_>,
        area: Rect,
        view: &ExplorerViewModel,
        c: &Colours,
    ) {
        let capacity = usize::from(area.height);
        let up = (view
            .folder
            .parent()
            .is_some_and(|p| !p.as_os_str().is_empty()))
        .then(msg::tui_folder_picker_up);
        let mut lines: Vec<(Line<'static>, bool)> = Vec::new();
        if let Some(up) = up {
            lines.push((Line::from(Span::styled(up, c.muted())), false));
        }
        for entry in &view.entries {
            let name = if entry.kind == EntryKind::Folder {
                format!("{}/", entry.name)
            } else {
                entry.name.clone()
            };
            let selected = view.selected.as_deref() == Some(entry.path.as_path());
            let style = if entry.kind == EntryKind::Folder {
                Style::default()
            } else {
                c.muted()
            };
            lines.push((Line::from(Span::styled(name, style)), selected));
        }
        if lines.is_empty() || (lines.len() == 1 && up_only(&lines)) {
            lines.push((
                Line::from(Span::styled(msg::tui_folder_picker_empty(), c.muted())),
                false,
            ));
        }
        let at = lines
            .iter()
            .position(|(_, selected)| *selected)
            .unwrap_or(0);
        let first = (at + 1).saturating_sub(capacity);
        for (row, (line, selected)) in lines.into_iter().skip(first).take(capacity).enumerate() {
            let row_area = Rect {
                y: area.y + u16::try_from(row).unwrap_or(0),
                height: 1,
                ..area
            };
            if selected {
                frame.render_widget(Block::new().style(c.selection()), row_area);
            }
            frame.render_widget(Paragraph::new(line), row_area);
        }
    }
}

/// Whether the only row is the `../` one.
fn up_only(lines: &[(Line<'static>, bool)]) -> bool {
    lines.len() == 1 && !lines[0].1
}

/// The title on the left, `note` on the right, over a rule: the Settings page picker's heading.
fn render_heading(frame: &mut Frame<'_>, area: Rect, title: &str, note: &str, c: &Colours) {
    let block = Block::new()
        .borders(Borders::BOTTOM)
        .border_type(BorderType::Thick);
    let row = block.inner(area);
    frame.render_widget(block, area);
    let note_width = u16::try_from(note.chars().count()).unwrap_or(u16::MAX);
    let [left, right] =
        Layout::horizontal([Constraint::Min(0), Constraint::Length(note_width)]).areas(row);
    frame.render_widget(
        Paragraph::new(Span::styled(
            title.to_string(),
            Style::default().add_modifier(Modifier::BOLD),
        )),
        left,
    );
    frame.render_widget(
        Paragraph::new(Span::styled(note.to_string(), c.muted())),
        right,
    );
}

/// The Dialog keymap's spelling of `key`, `None` for one it has no spelling for.
fn chord(key: KeyEvent) -> Option<String> {
    let name = match key.code {
        KeyCode::Char(c) => c.to_string(),
        KeyCode::Enter => "enter".to_string(),
        KeyCode::Esc => "esc".to_string(),
        KeyCode::Backspace => "backspace".to_string(),
        KeyCode::Down => "down".to_string(),
        KeyCode::Up => "up".to_string(),
        KeyCode::Left => "left".to_string(),
        KeyCode::Right => "right".to_string(),
        KeyCode::Tab => "tab".to_string(),
        _ => return None,
    };
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        return Some(format!("ctrl+{name}"));
    }
    Some(name)
}

/// A `width` × `height` rect centred in `area`, shrunk to fit it.
fn centred(area: Rect, width: u16, height: u16) -> Rect {
    let width = width.min(area.width);
    let height = height.min(area.height);
    Rect {
        x: area.x + (area.width - width) / 2,
        y: area.y + (area.height - height) / 2,
        width,
        height,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn disk() -> tempfile::TempDir {
        let dir = tempfile::tempdir().expect("a temporary directory");
        std::fs::create_dir(dir.path().join("finance")).expect("a folder");
        std::fs::create_dir(dir.path().join(".hidden")).expect("a hidden folder");
        std::fs::write(dir.path().join("ledger.pldb"), b"x").expect("a Ledger file");
        std::fs::write(dir.path().join("notes.txt"), b"hello").expect("another file");
        dir
    }

    #[test]
    fn rebound_go_into_and_go_up_keys_walk_the_folders() {
        let dir = disk();
        let mut config = KeyBindingConfig::default();
        config.bindings.insert("go_into".into(), "right".into());
        config.bindings.insert("go_up".into(), "left".into());
        let mut picker = FolderPickerPopup::open(dir.path().to_path_buf(), None, &config);
        let press = |picker: &mut FolderPickerPopup, code| {
            picker.handle_key(KeyEvent::new(code, KeyModifiers::NONE));
        };

        press(&mut picker, KeyCode::Right);
        assert_eq!(picker.folder(), Some(dir.path().join("finance")));
        press(&mut picker, KeyCode::Char('h'));
        assert_eq!(
            picker.folder(),
            Some(dir.path().join("finance")),
            "the replaced default does nothing"
        );
        press(&mut picker, KeyCode::Left);
        assert_eq!(picker.folder(), Some(dir.path().to_path_buf()));
    }

    #[test]
    fn the_adapter_reads_each_entry_with_its_kind_and_size() {
        let dir = disk();
        let mut entries = FsLedgerFolders.read_folder(dir.path()).expect("readable");
        entries.sort_by(|a, b| a.name.cmp(&b.name));
        let seen: Vec<_> = entries
            .iter()
            .map(|entry| (entry.name.as_str(), entry.kind, entry.size))
            .collect();
        assert_eq!(
            seen,
            [
                (".hidden", EntryKind::Folder, None),
                ("finance", EntryKind::Folder, None),
                ("ledger.pldb", EntryKind::Ledger, Some(1)),
                ("notes.txt", EntryKind::Other, Some(5)),
            ]
        );
    }

    #[test]
    fn the_adapter_reports_a_folder_it_cannot_read() {
        let dir = disk();
        let missing = dir.path().join("gone");
        assert!(FsLedgerFolders.read_folder(&missing).is_err());
    }

    #[test]
    fn exists_sees_files_folders_and_broken_links() {
        let dir = disk();
        assert!(FsLedgerFolders.exists(&dir.path().join("finance")));
        assert!(FsLedgerFolders.exists(&dir.path().join("notes.txt")));
        assert!(!FsLedgerFolders.exists(&dir.path().join("gone")));
        #[cfg(unix)]
        {
            let link = dir.path().join("broken");
            std::os::unix::fs::symlink(dir.path().join("gone"), &link).expect("a link");
            assert!(FsLedgerFolders.exists(&link), "a broken link is there too");
        }
    }
}
