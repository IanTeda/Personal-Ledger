//! [`RecordingRouter`], a root Router that only records what it was asked, [`FakeRailCounts`],
//! rail counts a test sets, and [`FakeLedgerFolders`], folders held in memory, for tests.

use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use chrono::NaiveDate;
use lib_toast::{Notice, Notify};

use crate::explorer::{EntryKind, FolderEntry, FolderUnreadable, LedgerFolders};
use crate::help::HelpGroup;
use crate::navigation::{Movement, Noun};
use crate::palette::PaletteRun;
use crate::protocols::{RailCounts, ShellDialog, ShellRailCounts, ShellRouter};
use crate::window_size::WindowSize;

/// One call the Presenter made on its Router, in order. The Router's questions (whether a Ledger
/// is open, the context rail's count, the folders) aren't calls.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Routed {
    MonthTurned(NaiveDate),
    Notice(Notice),
    WriteFailed(String),
    NotAJump(String),
    RefusedWithoutLedger,
    ToggledRail,
    StartedSearch(Noun),
    ViewBack(Noun),
    Moved(Noun, Movement),
    ViewScrollReset(Noun),
    HelpGroup(Noun),
    Hosted(ShellDialog),
    ClosedLedger,
    RanEffect(PaletteRun<&'static str>),
    OpenedLedger(PathBuf),
    CreatedLedger(PathBuf),
}

/// A root Router that records each call and answers as a test set it: by default a Ledger is
/// open, every View has a Search and takes `esc`, and every file opens.
#[derive(Debug)]
pub struct RecordingRouter {
    pub routed: Vec<Routed>,
    pub ledger_open: bool,
    pub has_search: bool,
    pub takes_back: bool,
    pub context_rows: usize,
    pub help_group: Option<HelpGroup>,
    /// What [`ShellRouter::toggle_rail`] says the frame draws at.
    pub drawn_at: (WindowSize, bool),
    pub opens: bool,
    pub folders: FakeLedgerFolders,
}

impl Default for RecordingRouter {
    fn default() -> Self {
        Self {
            routed: Vec::new(),
            ledger_open: true,
            has_search: true,
            takes_back: true,
            context_rows: 0,
            help_group: None,
            drawn_at: (WindowSize::Large, false),
            opens: true,
            folders: FakeLedgerFolders::default(),
        }
    }
}

impl Notify for RecordingRouter {
    fn notify(&mut self, notice: Notice) {
        self.routed.push(Routed::Notice(notice));
    }
}

impl ShellRouter for RecordingRouter {
    /// A test's failure is its description.
    type Failure = String;
    /// A test's Client effect is its name.
    type Effect = &'static str;

    fn month_turned(&mut self, today: NaiveDate) {
        self.routed.push(Routed::MonthTurned(today));
    }

    fn write_failed(&mut self, failure: String) {
        self.routed.push(Routed::WriteFailed(failure));
    }

    fn not_a_jump(&mut self, sequence: String) {
        self.routed.push(Routed::NotAJump(sequence));
    }

    fn ledger_open(&self) -> bool {
        self.ledger_open
    }

    fn refused_without_ledger(&mut self) {
        self.routed.push(Routed::RefusedWithoutLedger);
    }

    fn toggle_rail(&mut self) -> (WindowSize, bool) {
        self.routed.push(Routed::ToggledRail);
        self.drawn_at
    }

    fn start_search(&mut self, noun: Noun) -> bool {
        self.routed.push(Routed::StartedSearch(noun));
        self.has_search
    }

    fn view_back(&mut self, noun: Noun) -> bool {
        self.routed.push(Routed::ViewBack(noun));
        self.takes_back
    }

    fn move_selection(&mut self, noun: Noun, movement: Movement) {
        self.routed.push(Routed::Moved(noun, movement));
    }

    fn context_rail_count(&self, _noun: Noun) -> usize {
        self.context_rows
    }

    fn reset_view_scroll(&mut self, noun: Noun) {
        self.routed.push(Routed::ViewScrollReset(noun));
    }

    fn help_group(&mut self, noun: Noun) -> Option<HelpGroup> {
        self.routed.push(Routed::HelpGroup(noun));
        self.help_group.clone()
    }

    fn host_dialog(&mut self, dialog: ShellDialog) {
        self.routed.push(Routed::Hosted(dialog));
    }

    fn close_ledger(&mut self) {
        self.routed.push(Routed::ClosedLedger);
    }

    fn run_effect(&mut self, run: PaletteRun<&'static str>) {
        self.routed.push(Routed::RanEffect(run));
    }

    fn folders(&self) -> &dyn LedgerFolders {
        &self.folders
    }

    fn open_ledger(&mut self, path: &Path) -> bool {
        self.routed.push(Routed::OpenedLedger(path.to_path_buf()));
        self.opens
    }

    fn create_ledger(&mut self, path: &Path) -> bool {
        self.routed.push(Routed::CreatedLedger(path.to_path_buf()));
        self.opens
    }
}

/// Rail counts a test sets, recording each day they were read on.
#[derive(Debug, Default)]
pub struct FakeRailCounts {
    pub counts: Cell<RailCounts>,
    pub read_on: RefCell<Vec<NaiveDate>>,
}

impl ShellRailCounts for FakeRailCounts {
    fn rail_counts(&self, today: NaiveDate) -> RailCounts {
        self.read_on.borrow_mut().push(today);
        self.counts.get()
    }
}

/// Folders and files held in memory, by path, built up a test at a time. Every file is 1 kb and
/// was modified at the epoch; a folder marked unreadable fails to read, as one without permission
/// does.
#[derive(Debug, Default)]
pub struct FakeLedgerFolders {
    /// Each path held, `true` for a folder.
    paths: BTreeMap<PathBuf, bool>,
    unreadable: BTreeSet<PathBuf>,
}

impl FakeLedgerFolders {
    pub fn folder(mut self, path: PathBuf) -> Self {
        self.paths.insert(path, true);
        self
    }

    pub fn file(mut self, path: PathBuf) -> Self {
        self.paths.insert(path, false);
        self
    }

    pub fn unreadable(mut self, path: PathBuf) -> Self {
        self.unreadable.insert(path);
        self
    }
}

impl LedgerFolders for FakeLedgerFolders {
    fn read_folder(&self, folder: &Path) -> Result<Vec<FolderEntry>, FolderUnreadable> {
        if self.unreadable.contains(folder) || self.paths.get(folder) != Some(&true) {
            return Err(FolderUnreadable(format!(
                "{} can't be read",
                folder.display()
            )));
        }
        Ok(self
            .paths
            .iter()
            .filter(|(path, _)| path.parent() == Some(folder))
            .map(|(path, &is_folder)| FolderEntry {
                name: crate::explorer::display_name(path),
                path: path.clone(),
                kind: EntryKind::of(path, is_folder),
                size: (!is_folder).then_some(1_000),
                modified: Some(SystemTime::UNIX_EPOCH),
            })
            .collect())
    }

    fn exists(&self, path: &Path) -> bool {
        self.paths.contains_key(path)
    }
}
