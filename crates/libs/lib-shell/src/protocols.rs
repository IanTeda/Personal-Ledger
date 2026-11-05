//! What the Shell's Presenter asks of the Client: the root Router, which the bin implements, and
//! the rail's badge counts, which the bin answers from the open Ledger's stores.

use chrono::NaiveDate;
use lib_toast::Notify;

use crate::keymap::ShellCommand;

/// The Client's root Router, which acts on what the Shell's Presenter decides. It takes the
/// Shell's Notices as every module Router does (ADR-0048), queueing them with theirs for the
/// Client to show on the Presenter once the input is handled.
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

    /// A global key the Shell keeps was pressed: act on its command.
    fn run(&mut self, command: ShellCommand);

    /// A prefix was followed by a key that finishes no sequence (`g x`): say so on the status
    /// line.
    fn not_a_jump(&mut self, sequence: String);
}

/// The counts the primary rail's badges show, each worked out by the domain that owns its rule.
/// A zero draws no badge.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RailCounts {
    /// The Bills badge: how many Bill Schedule entries need attention.
    pub bills_attention: usize,
    /// The Budgets badge: how many of the default Budget's Categories are over this month.
    pub budgets_over: usize,
    /// The Documents badge: how many Documents wait in the Inbox.
    pub documents_inbox: usize,
}

/// The rail's badge counts on `today`, read from the open Ledger. The Shell depends on no feature
/// crate, so it receives each count through this port rather than reaching the rules itself.
pub trait ShellRailCounts {
    fn rail_counts(&self, today: NaiveDate) -> RailCounts;
}
