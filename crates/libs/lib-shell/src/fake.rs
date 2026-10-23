//! [`RecordingRouter`], a root Router that only records what it was asked, for tests.

use chrono::NaiveDate;
use lib_toast::{Notice, Notify};

use crate::keymap::ShellCommand;
use crate::protocols::ShellRouter;

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
