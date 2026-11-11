//! What the Shell's Presenter asks of the Client: the root Router, which the bin implements, and
//! the rail's badge counts, which the bin answers from the open Ledger's stores.

use std::path::Path;

use chrono::NaiveDate;
use lib_toast::Notify;

use crate::explorer::LedgerFolders;
use crate::help::{HelpGroup, HelpPresenter};
use crate::navigation::{Movement, Noun};
use crate::palette::PaletteRun;
use crate::toast_history::ToastHistoryPresenter;
use crate::window_size::WindowSize;

/// A Dialog of the Shell's own, whose Presenter the Shell builds and the Client hosts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShellDialog {
    Help(HelpPresenter),
    ToastHistory(ToastHistoryPresenter),
}

/// The Client's root Router, which acts on what the Shell's Presenter decides, each call inside the
/// input the Presenter is handling, so one commit covers whatever the key ran. The Presenter runs
/// the commands that change only its own state (focus, Insert, a jump, the palette, Help and the
/// Toast history) and calls a typed method here for each that reaches a View or the frame. A View
/// is named by the [`Noun`] showing, since where the person is lives in the Presenter.
///
/// It takes the Shell's Notices as every module Router does (ADR-0048), queueing them with theirs
/// for the Client to show on the Presenter once the input is handled.
pub trait ShellRouter: Notify {
    /// What the Client's storage reports when a write fails. The Client's own, since the Shell
    /// is toolkit-free and storage-free and can't name it.
    type Failure;

    /// What one of the Client's palette Commands does ([`crate::PaletteEffect::Client`]).
    type Effect;

    /// The Clock has crossed into a new calendar month, now `today`: generate its Bill Schedule
    /// and move every View dated by the month on to it.
    fn month_turned(&mut self, today: NaiveDate);

    /// A write the Client committed failed: put what it named back as stored and tell the person
    /// at once, with an Error Toast (ADR-0046).
    fn write_failed(&mut self, failure: Self::Failure);

    /// A prefix was followed by a key that finishes no sequence (`g x`): say so on the status
    /// line.
    fn not_a_jump(&mut self, sequence: String);

    /// Whether a Ledger is open, which a noun or Command needing one is refused without.
    fn ledger_open(&self) -> bool;

    /// Something that needs a Ledger was refused with none open: say so on the status line.
    fn refused_without_ledger(&mut self);

    /// `b`: the frame drops what hangs off the rail's shape (its hover tooltip) and says the
    /// Window Size it draws at and whether the rail starts hidden there, for the Shell to flip.
    fn toggle_rail(&mut self) -> (WindowSize, bool);

    /// `/`: opens the Search of the View showing `noun`. Whether it has one, which the Shell enters
    /// Search mode only for.
    fn start_search(&mut self, noun: Noun) -> bool;

    /// `esc`: the View showing `noun` backs out of whatever of its own is open. Whether it took
    /// the key.
    fn view_back(&mut self, noun: Noun) -> bool;

    /// The View's part of a movement, while it has focus: the rails' are the Shell's.
    fn move_selection(&mut self, noun: Noun, movement: Movement);

    /// How many rows the context rail lists beside `noun`, which its movement steps through.
    fn context_rail_count(&self, noun: Noun) -> usize;

    /// The View showing `noun` has just become the destination: its scroll goes back to the top.
    fn reset_view_scroll(&mut self, noun: Noun);

    /// The keys the View showing `noun` adds to the Help Dialog, if any.
    fn help_group(&mut self, noun: Noun) -> Option<HelpGroup>;

    /// Hosts one of the Shell's own Dialogs over the View.
    fn host_dialog(&mut self, dialog: ShellDialog);

    /// `:close`: closes the Open Ledger. The Shell lands on the Dashboard after.
    fn close_ledger(&mut self);

    /// Runs one of the Client's palette Commands, its destination already showing.
    fn run_effect(&mut self, run: PaletteRun<Self::Effect>);

    /// The folders the file explorer reads.
    fn folders(&self) -> &dyn LedgerFolders;

    /// The file explorer chose the Ledger file at `path`: opens it in place of the Open Ledger.
    /// Whether it opened; one that didn't says why, with an Error Toast.
    fn open_ledger(&mut self, path: &Path) -> bool;

    /// `:new`'s file explorer chose `path`: creates a Ledger file there and opens it. Whether it
    /// opened. A Client that asks first (the Desktop's Create ledger Dialog) returns `false` and
    /// reports the open later through `Presenter::explorer_chosen`.
    fn create_ledger(&mut self, path: &Path) -> bool;

    /// The folder picker picked `folder` as the Data folder: has it written, by the one place that
    /// writes it, which says how that went. Whether `folder` is now the Data folder in effect, so
    /// `:open` and `:new` start there: only when the write applied, not when it was already the
    /// folder shown, when a source above the configuration file still sets it, or when it wasn't
    /// written. lib-shell can't reach `lib-settings`, so the Desktop maps General's outcome to this
    /// `bool` in one place, `DataFolderOutcome::in_effect`.
    fn set_data_folder(&mut self, folder: &Path) -> bool;
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
