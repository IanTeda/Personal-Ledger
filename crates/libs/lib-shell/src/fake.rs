//! [`RecordingRouter`], a root Router that only records what it was asked, [`FakeRailCounts`],
//! rail counts a test sets, and [`FakeLedgerFolders`], folders held in memory, for tests.

use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use chrono::NaiveDate;
use lib_toast::{Notice, Notify};

use crate::explorer::{EntryKind, FolderEntry, FolderUnreadable, LedgerFolders};
use crate::keymap::ShellCommand;
use crate::protocols::{RailCounts, ShellRailCounts, ShellRouter};

/// One call the Presenter made on its Router, in order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Routed {
    MonthTurned(NaiveDate),
    Notice(Notice),
    WriteFailed(String),
    Ran(ShellCommand),
    NotAJump(String),
}

#[derive(Debug, Default)]
pub struct RecordingRouter {
    pub routed: Vec<Routed>,
}

impl Notify for RecordingRouter {
    fn notify(&mut self, notice: Notice) {
        self.routed.push(Routed::Notice(notice));
    }
}

impl ShellRouter for RecordingRouter {
    /// A test's failure is its description.
    type Failure = String;

    fn month_turned(&mut self, today: NaiveDate) {
        self.routed.push(Routed::MonthTurned(today));
    }

    fn write_failed(&mut self, failure: String) {
        self.routed.push(Routed::WriteFailed(failure));
    }

    fn run(&mut self, command: ShellCommand) {
        self.routed.push(Routed::Ran(command));
    }

    fn not_a_jump(&mut self, sequence: String) {
        self.routed.push(Routed::NotAJump(sequence));
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
