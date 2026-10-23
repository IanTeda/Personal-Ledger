//! What the Shell's Presenter asks of the Client: the root Router, which the bin implements.

use chrono::NaiveDate;
use lib_toast::Notify;

/// The Client's root Router, which acts on what the Shell's Presenter decides. It takes the
/// Shell's Notices as every module Router does (ADR-0048): until the Presenter holds the Toast
/// stack and status line (#826), it hands them on to the Client's.
pub trait ShellRouter: Notify {
    /// What the Client's storage reports when a write fails. The Client's own, since the Shell
    /// is toolkit-free and storage-free and can't name it.
    type Failure;

    /// The Clock has crossed into a new calendar month, now `today`: generate its Bill Schedule
    /// and move every View dated by the month on to it.
    fn month_turned(&mut self, today: NaiveDate);

    /// A write the Client committed failed: put what it named back as stored and tell the person
    /// at once, with an Error Toast (ADR-0046).
    fn write_failed(&mut self, failure: Self::Failure);
}
