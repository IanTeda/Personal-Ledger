//! The file explorer's Presenter (#1064): the floating window behind `:open` and `:new`, and the
//! Data folder picker behind `:data` and Settings › General's **Change…** (#1148), held by the
//! Shell's Presenter. It holds the mode, the folder browsed, the entries read there, the
//! filters, the selection, `:new`'s file name and where it starts. The filters and where it
//! starts outlive each opening, so the Presenter lives as long as the Shell and only the session
//! opens and closes.
//!
//! It reads the disk through [`LedgerFolders`], which the Client answers from its file system, so
//! the Shell stays I/O-free and its tests run on an in-memory folder ([`crate::fake`]). A row's
//! kind is decided from its real extension ([`EntryKind::of`]): folders open, Ledger files select,
//! other files are inert.
//!
//! Choosing a file hands it back ([`ExplorerOutcome::Chosen`]) for the Client to open or create,
//! with the explorer still open: it closes once the Client says the file opened
//! ([`ExplorerPresenter::chosen`]), and stays open on a failure, so the person can pick another.
//!
//! Picking a folder hands back the folder browsed ([`ExplorerOutcome::FolderPicked`]), never a
//! file: files are listed for what's there, inert, and the `.pldb`-only filter doesn't apply.
//!
//! Its keys are the Dialog keymap's, though the explorer is not a Dialog: back closes, and in
//! `:new` select creates the file and a key the keymap doesn't bind is typed into the File name,
//! `backspace` included. `:open` picks its file by mouse alone, so it takes no other key. The
//! folder picker walks by keys too: its list steps move a cursor over the folders, select or `l`
//! goes into the one under it, `h` or `backspace` goes up, and submit picks the folder browsed.

use std::path::{Component, Path, PathBuf};
use std::time::SystemTime;

use lib_list_cursor::Step;
use lib_viper::keymap::{BACK, Keymap};

use crate::dialog::DialogCommand;

/// Which command opened the explorer: `:open` browses to a Ledger file to open, `:new` names a
/// file to create in the folder browsed to, and `:data` picks the folder browsed to as the Data
/// folder.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExplorerMode {
    Open,
    New,
    PickFolder,
}

impl ExplorerMode {
    /// The Command that opened it, as the status line echoes it: a stable English id, not a
    /// Message.
    pub fn command(self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::New => "new",
            Self::PickFolder => "data",
        }
    }
}

/// What a row is, which decides its icon and what a click on it does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryKind {
    /// A click opens it.
    Folder,
    /// A file with a real `.pldb` extension: the only kind `:open` selects.
    Ledger,
    /// Any other file: listed, never selectable.
    Other,
}

impl EntryKind {
    /// The kind of what is at `path`: a folder, else a Ledger file by its real extension, else
    /// another file.
    pub fn of(path: &Path, is_folder: bool) -> Self {
        if is_folder {
            Self::Folder
        } else if is_ledger_file(path) {
            Self::Ledger
        } else {
            Self::Other
        }
    }
}

/// One entry of a folder, as [`LedgerFolders`] reads it.
#[derive(Debug, Clone, PartialEq)]
pub struct FolderEntry {
    pub name: String,
    pub path: PathBuf,
    pub kind: EntryKind,
    /// `None` for a folder, whose size column shows none.
    pub size: Option<u64>,
    pub modified: Option<SystemTime>,
}

/// A folder that couldn't be read (permissions, or gone), with why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FolderUnreadable(pub String);

/// The folders the explorer browses, which the Client reads from its file system.
pub trait LedgerFolders {
    /// Every entry in `folder`, in any order.
    fn read_folder(&self, folder: &Path) -> Result<Vec<FolderEntry>, FolderUnreadable>;

    /// Whether anything, a file, folder or link, is at `path`.
    fn exists(&self, path: &Path) -> bool;
}

/// The footer's two checkboxes, which outlive each opening and the Client keeps across a restart.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct ExplorerFilters {
    /// Show only folders and Ledger files. Folders stay, so browsing still works.
    pub pldb_only: bool,
    /// Hide entries whose name begins with `.`.
    pub hide_hidden: bool,
}

impl Default for ExplorerFilters {
    fn default() -> Self {
        Self {
            pldb_only: true,
            hide_hidden: true,
        }
    }
}

/// Which footer checkbox a click toggled.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExplorerFilter {
    PldbOnly,
    HideHidden,
}

/// A file chosen to open or create, for the Client to act on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExplorerChoice {
    pub mode: ExplorerMode,
    pub path: PathBuf,
}

/// What an input did to the explorer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExplorerOutcome {
    /// Nothing changed.
    Ignored,
    /// What it draws changed.
    Changed,
    /// It closed without choosing anything.
    Closed,
    /// A file was chosen, the explorer still open until the Client says how it went
    /// ([`ExplorerPresenter::chosen`]).
    Chosen(ExplorerChoice),
    /// The folder picker picked the folder browsed, for the Shell to make the Data folder.
    FolderPicked(PathBuf),
}

/// What the explorer draws.
#[derive(Debug, Clone, PartialEq)]
pub struct ExplorerViewModel {
    pub mode: ExplorerMode,
    /// The folder browsed.
    pub folder: PathBuf,
    /// Its entries after the filters, folders first, then files, each by name case-insensitively.
    pub entries: Vec<FolderEntry>,
    /// Open's selected Ledger file, or the folder under the folder picker's cursor.
    pub selected: Option<PathBuf>,
    pub filters: ExplorerFilters,
    /// Where each opening starts, the Data folder, which the folder picker's header names.
    pub current: Option<PathBuf>,
    /// `:new`'s File name, as typed.
    pub file_name: String,
    /// The file `:new`'s name clashes with in the folder browsed, refused inline.
    pub name_taken: Option<String>,
    /// Whether Open or Create has a file to act on.
    pub can_confirm: bool,
    /// The key that closes it, as bound now.
    pub close_key: String,
}

/// One opening of the explorer.
#[derive(Debug)]
struct Session {
    mode: ExplorerMode,
    folder: PathBuf,
    /// Everything read in `folder`, sorted and unfiltered, so a filter applies without reading
    /// it again.
    listing: Vec<FolderEntry>,
    selected: Option<PathBuf>,
    file_name: String,
    /// The file `file_name` clashes with, checked as the name or the folder changes.
    taken: Option<String>,
}

/// The file explorer's Presenter.
#[derive(Debug, Default)]
pub struct ExplorerPresenter {
    /// Where it starts: the Data folder, while it can be read.
    start: Option<PathBuf>,
    /// Where it starts when the Data folder can't be read, the home folder.
    fallback: Option<PathBuf>,
    filters: ExplorerFilters,
    session: Option<Session>,
}

impl ExplorerPresenter {
    /// Starts each opening in `folder`, the Data folder.
    pub fn set_start_folder(&mut self, folder: Option<PathBuf>) {
        self.start = folder;
    }

    /// Starts in `folder` when the Data folder can't be read; the current folder without one.
    pub fn set_fallback_folder(&mut self, folder: Option<PathBuf>) {
        self.fallback = folder;
    }

    pub fn filters(&self) -> ExplorerFilters {
        self.filters
    }

    pub fn set_filters(&mut self, filters: ExplorerFilters) {
        self.filters = filters;
    }

    /// Opens in `mode` at the Data folder, else the fallback, else the current folder. A start
    /// that can't be read is passed over for the next; the last lists nothing. The folder picker's
    /// cursor starts on the first folder.
    pub fn open(&mut self, mode: ExplorerMode, folders: &dyn LedgerFolders) {
        let candidates = [self.start.clone(), self.fallback.clone()];
        let (folder, listing) = candidates
            .into_iter()
            .flatten()
            .find_map(|folder| {
                let listing = folders.read_folder(&folder).ok()?;
                Some((folder, listing))
            })
            .unwrap_or_else(|| {
                let folder = PathBuf::from(".");
                let listing = folders.read_folder(&folder).unwrap_or_default();
                (folder, listing)
            });
        let mut session = Session {
            mode,
            folder,
            listing: Vec::new(),
            selected: None,
            file_name: String::new(),
            taken: None,
        };
        session.list(listing);
        session.cursor_on(self.filters, None);
        self.session = Some(session);
    }

    pub fn close(&mut self) {
        self.session = None;
    }

    pub fn is_open(&self) -> bool {
        self.session.is_some()
    }

    pub fn mode(&self) -> Option<ExplorerMode> {
        self.session.as_ref().map(|session| session.mode)
    }

    /// The Client's word on the file chosen: it opened, and the explorer closes, or it didn't,
    /// and the explorer stays open for another pick.
    pub fn chosen(&mut self, succeeded: bool) {
        if succeeded {
            self.close();
        }
    }

    /// A row click, `click_count` `2` for a double-click: a folder opens, and in `:open` a Ledger
    /// file selects, a double-click on the selected one choosing it. Any other file, any file in
    /// `:new`, and a path no longer listed do nothing.
    pub fn click_entry(
        &mut self,
        path: &Path,
        click_count: usize,
        folders: &dyn LedgerFolders,
    ) -> ExplorerOutcome {
        let filters = self.filters;
        let Some(session) = self.session.as_mut() else {
            return ExplorerOutcome::Ignored;
        };
        let Some(kind) = session
            .entries(filters)
            .find(|entry| entry.path == path)
            .map(|entry| entry.kind)
        else {
            return ExplorerOutcome::Ignored;
        };
        match kind {
            EntryKind::Folder => {
                session.browse(path.to_path_buf(), filters, folders);
                ExplorerOutcome::Changed
            }
            EntryKind::Ledger if session.mode == ExplorerMode::Open => {
                if click_count >= 2 && session.selected.as_deref() == Some(path) {
                    return self.confirm();
                }
                session.selected = Some(path.to_path_buf());
                ExplorerOutcome::Changed
            }
            EntryKind::Ledger | EntryKind::Other => ExplorerOutcome::Ignored,
        }
    }

    /// A breadcrumb click: browses `folder`, one of the folder browsed's ancestors.
    pub fn navigate_to(&mut self, folder: PathBuf, folders: &dyn LedgerFolders) -> ExplorerOutcome {
        let filters = self.filters;
        match self.session.as_mut() {
            Some(session) => {
                session.browse(folder, filters, folders);
                ExplorerOutcome::Changed
            }
            None => ExplorerOutcome::Ignored,
        }
    }

    /// Flips one footer checkbox. A selection the filters now hide is dropped, so Open never acts
    /// on a row the person can't see.
    pub fn toggle_filter(&mut self, filter: ExplorerFilter) -> ExplorerOutcome {
        match filter {
            ExplorerFilter::PldbOnly => self.filters.pldb_only = !self.filters.pldb_only,
            ExplorerFilter::HideHidden => self.filters.hide_hidden = !self.filters.hide_hidden,
        }
        let filters = self.filters;
        if let Some(session) = self.session.as_mut()
            && let Some(selected) = session.selected.clone()
            && !session.entries(filters).any(|entry| entry.path == selected)
        {
            session.selected = None;
            session.cursor_on(filters, None);
        }
        ExplorerOutcome::Changed
    }

    /// Open or Create: chooses the selected Ledger file, or the file `:new`'s name makes, while
    /// there is one. Use: picks the folder browsed.
    pub fn confirm(&self) -> ExplorerOutcome {
        if let Some(session) = self.session.as_ref()
            && session.mode == ExplorerMode::PickFolder
        {
            return ExplorerOutcome::FolderPicked(session.folder.clone());
        }
        match self.session.as_ref().and_then(Session::target) {
            Some(path) => ExplorerOutcome::Chosen(ExplorerChoice {
                mode: self.session.as_ref().map_or(ExplorerMode::Open, |s| s.mode),
                path,
            }),
            None => ExplorerOutcome::Ignored,
        }
    }

    /// Resolves `chord` through the Dialog keymap: back closes; in `:new` select creates and a
    /// key the keymap doesn't bind is typed, `backspace` deleting and `typed` the character the
    /// key types, if any. A list step is a letter, so it types too.
    pub fn key(
        &mut self,
        chord: &str,
        typed: Option<char>,
        dialog_keymap: &Keymap<DialogCommand>,
        folders: &dyn LedgerFolders,
    ) -> ExplorerOutcome {
        let Some(session) = self.session.as_mut() else {
            return ExplorerOutcome::Ignored;
        };
        let command = dialog_keymap.event(chord);
        if command == Some(DialogCommand::Back) {
            self.close();
            return ExplorerOutcome::Closed;
        }
        match session.mode {
            ExplorerMode::Open => return ExplorerOutcome::Ignored,
            ExplorerMode::PickFolder => return self.pick_key(chord, command, folders),
            ExplorerMode::New => {}
        }
        match command {
            Some(DialogCommand::Select) => self.confirm(),
            Some(command) if !command.is_list_step() => ExplorerOutcome::Ignored,
            _ if chord == "backspace" => {
                session.file_name.pop();
                session.check_name(folders);
                ExplorerOutcome::Changed
            }
            _ => match typed {
                Some(ch) => {
                    session.file_name.push(ch);
                    session.check_name(folders);
                    ExplorerOutcome::Changed
                }
                None => ExplorerOutcome::Ignored,
            },
        }
    }

    /// The folder picker's keys: a list step moves the cursor over the folders, select or `l` goes
    /// into the one under it, `h` or `backspace` goes up, and submit picks the folder browsed.
    fn pick_key(
        &mut self,
        chord: &str,
        command: Option<DialogCommand>,
        folders: &dyn LedgerFolders,
    ) -> ExplorerOutcome {
        let filters = self.filters;
        let Some(session) = self.session.as_mut() else {
            return ExplorerOutcome::Ignored;
        };
        match command {
            Some(DialogCommand::Submit) => self.confirm(),
            Some(DialogCommand::ListNext | DialogCommand::SelectNext) => {
                session.step_cursor(filters, Step::Next)
            }
            Some(DialogCommand::ListPrevious | DialogCommand::SelectPrevious) => {
                session.step_cursor(filters, Step::Prev)
            }
            Some(DialogCommand::Select) => session.enter_cursor(filters, folders),
            _ if chord == "l" => session.enter_cursor(filters, folders),
            _ if chord == "h" || chord == "backspace" => session.up(filters, folders),
            _ => ExplorerOutcome::Ignored,
        }
    }

    /// What the explorer draws, `None` while closed. `dialog_keymap` names the close key.
    pub fn view_model(&self, dialog_keymap: &Keymap<DialogCommand>) -> Option<ExplorerViewModel> {
        let session = self.session.as_ref()?;
        Some(ExplorerViewModel {
            mode: session.mode,
            folder: session.folder.clone(),
            entries: session.entries(self.filters).cloned().collect(),
            selected: session.selected.clone(),
            filters: self.filters,
            current: self.start.clone(),
            file_name: session.file_name.clone(),
            name_taken: session.taken.clone(),
            can_confirm: session.mode == ExplorerMode::PickFolder || session.target().is_some(),
            close_key: dialog_keymap.chord(BACK).unwrap_or_default().to_string(),
        })
    }
}

impl Session {
    /// Browses `folder`, dropping the selection. A folder that can't be read lists nothing: there
    /// is nothing the person can do about it here but browse elsewhere.
    fn browse(&mut self, folder: PathBuf, filters: ExplorerFilters, folders: &dyn LedgerFolders) {
        let listing = folders.read_folder(&folder).unwrap_or_default();
        self.folder = folder;
        self.selected = None;
        self.list(listing);
        self.cursor_on(filters, None);
        self.check_name(folders);
    }

    /// Puts the folder picker's cursor on the folder at `path`, else on the first folder shown
    /// under `filters`, else nowhere. Any other mode has no cursor.
    fn cursor_on(&mut self, filters: ExplorerFilters, path: Option<&Path>) {
        if self.mode != ExplorerMode::PickFolder {
            return;
        }
        let targets = self.targets(filters);
        let cursor = path
            .and_then(|path| targets.iter().find(|target| **target == path))
            .or(targets.first())
            .map(|target| target.to_path_buf());
        self.selected = cursor;
    }

    /// The folders the picker's cursor moves over: the folders shown under `filters`.
    fn targets(&self, filters: ExplorerFilters) -> Vec<&Path> {
        self.entries(filters)
            .filter(|entry| entry.kind == EntryKind::Folder)
            .map(|entry| entry.path.as_path())
            .collect()
    }

    /// Moves the picker's cursor a folder down or up, stopping at either end.
    fn step_cursor(&mut self, filters: ExplorerFilters, step: Step) -> ExplorerOutcome {
        let targets = self.targets(filters);
        if targets.is_empty() {
            return ExplorerOutcome::Ignored;
        }
        let at = self
            .selected
            .as_deref()
            .and_then(|selected| targets.iter().position(|path| *path == selected));
        let next = match at {
            Some(at) => lib_list_cursor::step(at, targets.len(), step),
            None => 0,
        };
        let next = targets[next].to_path_buf();
        if self.selected.as_ref() == Some(&next) {
            return ExplorerOutcome::Ignored;
        }
        self.selected = Some(next);
        ExplorerOutcome::Changed
    }

    /// Goes into the folder under the picker's cursor.
    fn enter_cursor(
        &mut self,
        filters: ExplorerFilters,
        folders: &dyn LedgerFolders,
    ) -> ExplorerOutcome {
        match self.selected.clone() {
            Some(folder) => {
                self.browse(folder, filters, folders);
                ExplorerOutcome::Changed
            }
            None => ExplorerOutcome::Ignored,
        }
    }

    /// Goes up to the folder browsed's parent, the cursor on the folder it came from.
    fn up(&mut self, filters: ExplorerFilters, folders: &dyn LedgerFolders) -> ExplorerOutcome {
        let Some(parent) = self
            .folder
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .map(Path::to_path_buf)
        else {
            return ExplorerOutcome::Ignored;
        };
        let from = self.folder.clone();
        self.browse(parent, filters, folders);
        self.cursor_on(filters, Some(&from));
        ExplorerOutcome::Changed
    }

    /// Lists `listing`, folders first, then files, each by name case-insensitively.
    fn list(&mut self, mut listing: Vec<FolderEntry>) {
        listing.sort_by(|a, b| {
            let a_folder = a.kind == EntryKind::Folder;
            let b_folder = b.kind == EntryKind::Folder;
            b_folder
                .cmp(&a_folder)
                .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
        });
        self.listing = listing;
    }

    /// The listing under `filters`: what the rows and the item count show. The folder picker lists
    /// every file, since none is a target.
    fn entries(&self, filters: ExplorerFilters) -> impl Iterator<Item = &FolderEntry> {
        let pldb_only = filters.pldb_only && self.mode != ExplorerMode::PickFolder;
        self.listing
            .iter()
            .filter(move |entry| !(filters.hide_hidden && entry.name.starts_with('.')))
            .filter(move |entry| !(pldb_only && entry.kind == EntryKind::Other))
    }

    /// The file Open or Create acts on: Open's selection, or the file `:new`'s name makes while
    /// nothing is there by that name.
    fn target(&self) -> Option<PathBuf> {
        match self.mode {
            ExplorerMode::Open => self.selected.clone(),
            ExplorerMode::New => self.new_file_path().filter(|_| self.taken.is_none()),
            ExplorerMode::PickFolder => None,
        }
    }

    /// The file `:new`'s name makes in the folder browsed, with `.pldb` added if the name doesn't
    /// end in it. `None` for a blank name, or one that isn't a single file name (`..`, or one
    /// holding a separator), since Create makes the file in the folder browsed and nowhere else.
    fn new_file_path(&self) -> Option<PathBuf> {
        let name = self.file_name.trim();
        let mut components = Path::new(name).components();
        let single = matches!(
            (components.next(), components.next()),
            (Some(Component::Normal(part)), None) if part == name
        );
        if !single {
            return None;
        }
        let file = if is_ledger_file(Path::new(name)) {
            name.to_string()
        } else {
            format!("{name}.pldb")
        };
        Some(self.folder.join(file))
    }

    /// Checks `:new`'s name against the folder browsed. It is taken while the file is there, or
    /// any journal SQLite keeps beside it ([`ledger_files`]): a stale `-wal` would replay an old
    /// Ledger into the new one, and a failed Create removes them all.
    fn check_name(&mut self, folders: &dyn LedgerFolders) {
        self.taken = self.new_file_path().and_then(|path| {
            ledger_files(&path)
                .iter()
                .any(|file| folders.exists(file))
                .then(|| display_name(&path))
        });
    }
}

/// Whether `path` has a real `.pldb` extension, in any case: `old-ledger.pldb.bak`'s is `bak`.
pub fn is_ledger_file(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("pldb"))
}

/// The Ledger file at `path` and the journal files SQLite keeps beside it. `:new` refuses a name
/// while any of them is there, so a failed Create removing them all removes only what it made.
pub fn ledger_files(path: &Path) -> Vec<PathBuf> {
    let mut files = vec![path.to_path_buf()];
    for suffix in ["-wal", "-shm", "-journal"] {
        let mut side = path.as_os_str().to_owned();
        side.push(suffix);
        files.push(side.into());
    }
    files
}

/// `path`'s file name as a Toast or the explorer names it, else the whole path (a root).
pub fn display_name(path: &Path) -> String {
    path.file_name().map_or_else(
        || path.display().to_string(),
        |name| name.to_string_lossy().into_owned(),
    )
}

#[cfg(test)]
mod tests {
    use lib_config::KeyBindingConfig;

    use super::*;
    use crate::dialog::dialog_keymap;
    use crate::fake::FakeLedgerFolders;

    fn root() -> PathBuf {
        PathBuf::from("/ledgers")
    }

    /// Every file kind shown, dot entries hidden, so listing and clicking see the fixtures.
    fn all_files() -> ExplorerFilters {
        ExplorerFilters {
            pldb_only: false,
            hide_hidden: true,
        }
    }

    fn opened(mode: ExplorerMode, folders: &FakeLedgerFolders) -> ExplorerPresenter {
        let mut explorer = ExplorerPresenter::default();
        explorer.set_filters(all_files());
        explorer.set_start_folder(Some(root()));
        explorer.open(mode, folders);
        explorer
    }

    fn model(explorer: &ExplorerPresenter) -> ExplorerViewModel {
        explorer
            .view_model(&dialog_keymap(&KeyBindingConfig::default()))
            .expect("the explorer is open")
    }

    fn names(explorer: &ExplorerPresenter) -> Vec<String> {
        model(explorer)
            .entries
            .into_iter()
            .map(|entry| entry.name)
            .collect()
    }

    fn press(
        explorer: &mut ExplorerPresenter,
        chord: &str,
        folders: &FakeLedgerFolders,
    ) -> ExplorerOutcome {
        let typed = (chord.chars().count() == 1).then(|| chord.chars().next().unwrap_or(' '));
        explorer.key(
            chord,
            typed,
            &dialog_keymap(&KeyBindingConfig::default()),
            folders,
        )
    }

    /// A `:new` explorer at the root with `name` typed into its File name.
    fn naming(name: &str, folders: &FakeLedgerFolders) -> ExplorerPresenter {
        let mut explorer = opened(ExplorerMode::New, folders);
        for ch in name.chars() {
            explorer.key(
                "",
                Some(ch),
                &dialog_keymap(&KeyBindingConfig::default()),
                folders,
            );
        }
        explorer
    }

    fn empty_root() -> FakeLedgerFolders {
        FakeLedgerFolders::default().folder(root())
    }

    fn filtered_fixture() -> FakeLedgerFolders {
        empty_root()
            .folder(root().join("archive"))
            .folder(root().join(".cache"))
            .file(root().join("teda.pldb"))
            .file(root().join(".hidden.pldb"))
            .file(root().join("old-ledger.pldb.bak"))
            .file(root().join("expenses.csv"))
    }

    #[test]
    fn it_opens_in_its_mode_at_the_data_folder() {
        let explorer = opened(ExplorerMode::New, &empty_root());

        assert_eq!(explorer.mode(), Some(ExplorerMode::New));
        assert_eq!(model(&explorer).folder, root());
    }

    #[test]
    fn an_unreadable_data_folder_opens_at_the_fallback() {
        let home = PathBuf::from("/home");
        let folders = FakeLedgerFolders::default().folder(home.clone());
        let mut explorer = ExplorerPresenter::default();
        explorer.set_start_folder(Some(root()));
        explorer.set_fallback_folder(Some(home.clone()));

        explorer.open(ExplorerMode::Open, &folders);

        assert_eq!(model(&explorer).folder, home);
    }

    #[test]
    fn it_lists_folders_before_files_case_insensitively() {
        let folders = empty_root()
            .folder(root().join("Zeta"))
            .folder(root().join("archive"))
            .file(root().join("beta.pldb"))
            .file(root().join("Alpha.csv"));

        let explorer = opened(ExplorerMode::Open, &folders);

        assert_eq!(
            names(&explorer),
            ["archive", "Zeta", "Alpha.csv", "beta.pldb"]
        );
    }

    #[test]
    fn only_a_real_pldb_extension_is_a_ledger() {
        assert_eq!(
            EntryKind::of(Path::new("real.pldb"), false),
            EntryKind::Ledger
        );
        assert_eq!(
            EntryKind::of(Path::new("second.PLDB"), false),
            EntryKind::Ledger,
            "the extension check ignores case"
        );
        assert_eq!(
            EntryKind::of(Path::new("decoy.pldb.bak"), false),
            EntryKind::Other,
            "a `.pldb.bak` file's real extension is `bak`"
        );
        assert_eq!(
            EntryKind::of(Path::new("plain.csv"), false),
            EntryKind::Other
        );
        assert_eq!(
            EntryKind::of(Path::new("ledgers.pldb"), true),
            EntryKind::Folder,
            "a folder is a folder whatever it is called"
        );
    }

    #[test]
    fn default_filters_hide_dot_entries_but_show_every_other_file() {
        let explorer = opened(ExplorerMode::Open, &filtered_fixture());

        assert_eq!(
            names(&explorer),
            [
                "archive",
                "expenses.csv",
                "old-ledger.pldb.bak",
                "teda.pldb"
            ]
        );
    }

    #[test]
    fn pldb_only_keeps_folders_and_real_pldb_files_but_not_the_bak_decoy() {
        let mut explorer = opened(ExplorerMode::Open, &filtered_fixture());

        explorer.toggle_filter(ExplorerFilter::PldbOnly);

        assert_eq!(names(&explorer), ["archive", "teda.pldb"]);
        assert!(
            explorer.filters().pldb_only,
            "the filters outlive the opening"
        );
    }

    #[test]
    fn showing_hidden_files_reveals_dot_entries() {
        let mut explorer = opened(ExplorerMode::Open, &filtered_fixture());

        explorer.toggle_filter(ExplorerFilter::HideHidden);

        assert_eq!(model(&explorer).entries.len(), 6);
        assert!(names(&explorer).contains(&".cache".to_string()));
    }

    #[test]
    fn a_filter_that_hides_the_selection_clears_it() {
        let folders = filtered_fixture();
        let mut explorer = opened(ExplorerMode::Open, &folders);
        explorer.toggle_filter(ExplorerFilter::HideHidden);
        explorer.click_entry(&root().join(".hidden.pldb"), 1, &folders);
        assert!(model(&explorer).can_confirm);

        explorer.toggle_filter(ExplorerFilter::HideHidden);

        assert!(!model(&explorer).can_confirm);
    }

    #[test]
    fn clicking_a_folder_browses_it_and_drops_the_selection() {
        let ledgers = root().join("ledgers");
        let folders = empty_root()
            .file(root().join("real.pldb"))
            .folder(ledgers.clone())
            .file(ledgers.join("inner.pldb"));
        let mut explorer = opened(ExplorerMode::Open, &folders);
        explorer.click_entry(&root().join("real.pldb"), 1, &folders);

        assert_eq!(
            explorer.click_entry(&ledgers, 1, &folders),
            ExplorerOutcome::Changed
        );

        assert_eq!(model(&explorer).folder, ledgers);
        assert_eq!(names(&explorer), ["inner.pldb"]);
        assert!(!model(&explorer).can_confirm);
    }

    #[test]
    fn a_folder_that_cannot_be_read_lists_nothing() {
        let locked = root().join("locked");
        let folders = empty_root()
            .folder(locked.clone())
            .unreadable(locked.clone());
        let mut explorer = opened(ExplorerMode::Open, &folders);

        explorer.click_entry(&locked, 1, &folders);

        assert_eq!(model(&explorer).folder, locked);
        assert!(names(&explorer).is_empty());
    }

    #[test]
    fn a_breadcrumb_browses_straight_to_an_ancestor() {
        let ledgers = root().join("ledgers");
        let folders = empty_root().folder(ledgers.clone());
        let mut explorer = ExplorerPresenter::default();
        explorer.set_start_folder(Some(ledgers));
        explorer.open(ExplorerMode::Open, &folders);

        explorer.navigate_to(root(), &folders);

        assert_eq!(model(&explorer).folder, root());
    }

    #[test]
    fn clicking_a_ledger_file_selects_it_and_a_double_click_chooses_it() {
        let ledger = root().join("real.pldb");
        let folders = empty_root().file(ledger.clone());
        let mut explorer = opened(ExplorerMode::Open, &folders);
        assert!(!model(&explorer).can_confirm);

        assert_eq!(
            explorer.click_entry(&ledger, 1, &folders),
            ExplorerOutcome::Changed
        );
        assert_eq!(model(&explorer).selected, Some(ledger.clone()));

        assert_eq!(
            explorer.click_entry(&ledger, 2, &folders),
            ExplorerOutcome::Chosen(ExplorerChoice {
                mode: ExplorerMode::Open,
                path: ledger,
            })
        );
        assert!(explorer.is_open(), "open until the Client says how it went");
    }

    #[test]
    fn a_double_click_on_an_unselected_ledger_file_only_selects_it() {
        let ledger = root().join("real.pldb");
        let folders = empty_root().file(ledger.clone());
        let mut explorer = opened(ExplorerMode::Open, &folders);

        assert_eq!(
            explorer.click_entry(&ledger, 2, &folders),
            ExplorerOutcome::Changed
        );
    }

    #[test]
    fn clicking_another_file_does_nothing() {
        let csv = root().join("plain.csv");
        let folders = empty_root().file(csv.clone());
        let mut explorer = opened(ExplorerMode::Open, &folders);

        assert_eq!(
            explorer.click_entry(&csv, 1, &folders),
            ExplorerOutcome::Ignored
        );
        assert_eq!(model(&explorer).selected, None);
    }

    #[test]
    fn open_confirms_the_selection_and_closes_only_once_it_opened() {
        let ledger = root().join("real.pldb");
        let folders = empty_root().file(ledger.clone());
        let mut explorer = opened(ExplorerMode::Open, &folders);
        assert_eq!(
            explorer.confirm(),
            ExplorerOutcome::Ignored,
            "nothing chosen"
        );
        explorer.click_entry(&ledger, 1, &folders);

        assert!(matches!(explorer.confirm(), ExplorerOutcome::Chosen(_)));
        explorer.chosen(false);
        assert!(explorer.is_open(), "a file that failed leaves it open");

        explorer.chosen(true);
        assert!(!explorer.is_open(), "a file that opened closes it");
    }

    #[test]
    fn back_closes_either_mode() {
        let folders = empty_root();
        for mode in [ExplorerMode::Open, ExplorerMode::New] {
            let mut explorer = opened(mode, &folders);

            assert_eq!(
                press(&mut explorer, "esc", &folders),
                ExplorerOutcome::Closed
            );
            assert!(!explorer.is_open());
        }
    }

    #[test]
    fn open_types_nothing() {
        let folders = empty_root();
        let mut explorer = opened(ExplorerMode::Open, &folders);

        assert_eq!(
            press(&mut explorer, "a", &folders),
            ExplorerOutcome::Ignored
        );
        assert_eq!(model(&explorer).file_name, "");
    }

    #[test]
    fn a_typed_name_creates_that_name_with_pldb_added() {
        let folders = empty_root();
        let mut explorer = naming("teda-2027", &folders);

        assert_eq!(model(&explorer).file_name, "teda-2027");
        assert_eq!(
            press(&mut explorer, "enter", &folders),
            ExplorerOutcome::Chosen(ExplorerChoice {
                mode: ExplorerMode::New,
                path: root().join("teda-2027.pldb"),
            })
        );
    }

    #[test]
    fn a_list_step_letter_is_typed_into_the_name() {
        let folders = empty_root();
        let mut explorer = naming("", &folders);

        press(&mut explorer, "j", &folders);
        press(&mut explorer, "k", &folders);

        assert_eq!(model(&explorer).file_name, "jk");
    }

    #[test]
    fn a_name_ending_in_pldb_in_any_case_is_not_given_it_again() {
        let folders = empty_root();
        let chosen = |name: &str| match naming(name, &folders).confirm() {
            ExplorerOutcome::Chosen(choice) => Some(choice.path),
            _ => None,
        };

        assert_eq!(chosen("teda.PLDB"), Some(root().join("teda.PLDB")));
        assert_eq!(
            chosen("teda.pldb.bak"),
            Some(root().join("teda.pldb.bak.pldb")),
            "only a real .pldb extension counts"
        );
    }

    #[test]
    fn a_name_already_in_the_folder_is_refused() {
        let folders = empty_root().file(root().join("teda.pldb"));

        let mut explorer = naming("teda", &folders);

        assert_eq!(
            model(&explorer).name_taken,
            Some("teda.pldb".to_string()),
            "names the file it clashes with"
        );
        assert!(!model(&explorer).can_confirm);
        assert_eq!(
            press(&mut explorer, "enter", &folders),
            ExplorerOutcome::Ignored
        );
    }

    #[test]
    fn a_name_whose_journal_is_still_in_the_folder_is_refused() {
        let folders = empty_root().file(root().join("teda.pldb-wal"));

        let explorer = naming("teda", &folders);

        assert_eq!(model(&explorer).name_taken, Some("teda.pldb".to_string()));
    }

    #[test]
    fn a_blank_name_or_one_naming_another_folder_creates_nothing() {
        let folders = empty_root();
        for name in ["", "   ", "sub/teda", "..", "."] {
            let explorer = naming(name, &folders);
            assert_eq!(explorer.confirm(), ExplorerOutcome::Ignored, "{name:?}");
            assert_eq!(model(&explorer).name_taken, None, "{name:?}");
        }
    }

    #[test]
    fn backspace_deletes_the_last_character_typed() {
        let folders = empty_root();
        let mut explorer = naming("tedx", &folders);

        press(&mut explorer, "backspace", &folders);
        press(&mut explorer, "a", &folders);

        assert_eq!(model(&explorer).file_name, "teda");
        for _ in 0..5 {
            press(&mut explorer, "backspace", &folders);
        }
        assert_eq!(model(&explorer).file_name, "", "nothing left to delete");
    }

    #[test]
    fn in_new_a_file_row_is_inert_and_a_folder_keeps_the_name() {
        let ledgers = root().join("ledgers");
        let folders = empty_root()
            .file(root().join("teda.pldb"))
            .folder(ledgers.clone())
            .file(ledgers.join("teda.pldb"));
        let mut explorer = naming("teda-2027", &folders);

        explorer.click_entry(&root().join("teda.pldb"), 2, &folders);
        assert_eq!(model(&explorer).selected, None, "nothing selects in :new");

        explorer.click_entry(&ledgers, 1, &folders);
        assert_eq!(
            explorer.confirm(),
            ExplorerOutcome::Chosen(ExplorerChoice {
                mode: ExplorerMode::New,
                path: ledgers.join("teda-2027.pldb"),
            }),
            "the name is kept, made in the folder browsed"
        );
    }

    #[test]
    fn browsing_rechecks_the_name_in_the_new_folder() {
        let ledgers = root().join("ledgers");
        let folders = empty_root()
            .folder(ledgers.clone())
            .file(ledgers.join("teda.pldb"));
        let mut explorer = naming("teda", &folders);
        assert_eq!(model(&explorer).name_taken, None);

        explorer.click_entry(&ledgers, 1, &folders);

        assert_eq!(model(&explorer).name_taken, Some("teda.pldb".to_string()));
    }

    /// `/ledgers` holding two folders, a Ledger, a decoy and a CSV, with a folder inside `archive`.
    fn picking_fixture() -> FakeLedgerFolders {
        filtered_fixture()
            .folder(root().join("finance"))
            .folder(root().join("archive").join("2024"))
    }

    fn picking(folders: &FakeLedgerFolders) -> ExplorerPresenter {
        let mut explorer = ExplorerPresenter::default();
        explorer.set_start_folder(Some(root()));
        explorer.open(ExplorerMode::PickFolder, folders);
        explorer
    }

    #[test]
    fn the_folder_picker_opens_in_the_data_folder_with_the_cursor_on_the_first_folder() {
        let explorer = picking(&picking_fixture());

        let model = model(&explorer);
        assert_eq!(model.mode, ExplorerMode::PickFolder);
        assert_eq!(model.folder, root());
        assert_eq!(
            model.current,
            Some(root()),
            "the header names the Data folder"
        );
        assert_eq!(model.selected, Some(root().join("archive")));
        assert!(model.can_confirm, "Use always picks the folder browsed");
    }

    #[test]
    fn the_folder_picker_lists_every_file_whatever_the_pldb_only_filter() {
        let explorer = picking(&picking_fixture());

        assert!(explorer.filters().pldb_only, "the default, kept for :open");
        assert_eq!(
            names(&explorer),
            [
                "archive",
                "finance",
                "expenses.csv",
                "old-ledger.pldb.bak",
                "teda.pldb"
            ]
        );
    }

    #[test]
    fn the_folder_pickers_files_are_inert() {
        let folders = picking_fixture();
        let mut explorer = picking(&folders);

        for file in ["teda.pldb", "expenses.csv"] {
            assert_eq!(
                explorer.click_entry(&root().join(file), 2, &folders),
                ExplorerOutcome::Ignored,
                "{file}"
            );
        }
        assert_eq!(model(&explorer).selected, Some(root().join("archive")));
    }

    #[test]
    fn j_and_k_move_the_cursor_over_the_folders_only() {
        let folders = picking_fixture();
        let mut explorer = picking(&folders);

        assert_eq!(
            press(&mut explorer, "j", &folders),
            ExplorerOutcome::Changed
        );
        assert_eq!(model(&explorer).selected, Some(root().join("finance")));
        assert_eq!(
            press(&mut explorer, "j", &folders),
            ExplorerOutcome::Ignored,
            "files are no stop"
        );
        press(&mut explorer, "k", &folders);
        assert_eq!(model(&explorer).selected, Some(root().join("archive")));
    }

    #[test]
    fn l_or_enter_walks_into_the_folder_and_h_or_backspace_goes_back_up() {
        let folders = picking_fixture();
        let mut explorer = picking(&folders);

        press(&mut explorer, "l", &folders);
        assert_eq!(model(&explorer).folder, root().join("archive"));
        assert_eq!(model(&explorer).selected, Some(root().join("archive/2024")));

        press(&mut explorer, "h", &folders);
        assert_eq!(model(&explorer).folder, root());
        assert_eq!(
            model(&explorer).selected,
            Some(root().join("archive")),
            "the cursor is back on the folder it came from"
        );

        press(&mut explorer, "j", &folders);
        press(&mut explorer, "enter", &folders);
        assert_eq!(model(&explorer).folder, root().join("finance"));
        assert_eq!(model(&explorer).selected, None, "finance holds no folder");
        assert_eq!(
            press(&mut explorer, "enter", &folders),
            ExplorerOutcome::Ignored
        );
        press(&mut explorer, "backspace", &folders);
        assert_eq!(model(&explorer).folder, root());
    }

    #[test]
    fn use_picks_the_folder_browsed_never_a_file() {
        let folders = picking_fixture();
        let mut explorer = picking(&folders);
        press(&mut explorer, "l", &folders);

        assert_eq!(
            press(&mut explorer, "ctrl+enter", &folders),
            ExplorerOutcome::FolderPicked(root().join("archive"))
        );
        assert_eq!(
            explorer.confirm(),
            ExplorerOutcome::FolderPicked(root().join("archive"))
        );
    }

    #[test]
    fn cancel_closes_the_folder_picker_with_nothing_picked() {
        let folders = picking_fixture();
        let mut explorer = picking(&folders);
        press(&mut explorer, "l", &folders);

        assert_eq!(
            press(&mut explorer, "esc", &folders),
            ExplorerOutcome::Closed
        );
        assert!(!explorer.is_open());
    }

    #[test]
    fn the_close_key_is_named_as_bound_now() {
        let explorer = opened(ExplorerMode::Open, &empty_root());

        assert_eq!(model(&explorer).close_key, "esc");
    }
}
