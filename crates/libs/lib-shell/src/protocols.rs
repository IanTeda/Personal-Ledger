//! What the Shell's Presenter asks of the Client: the root Router, which the bin implements.

use chrono::NaiveDate;

/// The Client's root Router, which acts on what the Shell's Presenter decides.
pub trait ShellRouter {
    /// The Clock has crossed into a new calendar month, now `today`: generate its Bill Schedule
    /// and move every View dated by the month on to it.
    fn month_turned(&mut self, today: NaiveDate);
}
