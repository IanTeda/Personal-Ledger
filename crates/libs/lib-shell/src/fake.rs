//! [`RecordingRouter`], a root Router that only records what it was asked, for tests.

use chrono::NaiveDate;

use crate::protocols::ShellRouter;

/// One call the Presenter made on its Router, in order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Routed {
    MonthTurned(NaiveDate),
}

#[derive(Debug, Default)]
pub struct RecordingRouter {
    pub routed: Vec<Routed>,
}

impl ShellRouter for RecordingRouter {
    fn month_turned(&mut self, today: NaiveDate) {
        self.routed.push(Routed::MonthTurned(today));
    }
}
