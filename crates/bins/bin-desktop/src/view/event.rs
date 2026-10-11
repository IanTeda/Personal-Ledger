//! `ViewEvent` -- the requests a View makes of `Shell`. A View emits one of these with
//! `cx.emit` and never holds a handle to `Shell` (ADR-0032): `Shell` subscribes to each View
//! and handles every event in one place (`app/view_events.rs`).

use lib_toast::ToastKind;

use crate::{
    bills::EntryId,
    chrome::dialog_host::OpenDialog,
    navigation::nav::{InputMode, Noun},
    period::Period,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ViewEvent {
    /// Raise a Toast whose Message the View has already resolved to text.
    RaiseToast { kind: ToastKind, text: String },
    /// Open a Dialog in the Dialog host.
    OpenDialog(Box<OpenDialog>),
    /// Make `Noun` the active destination, as the rail or a `g`-jump would.
    Navigate(Noun),
    /// Replace the status message shown in the Chrome; `None` clears it.
    SetStatus(Option<String>),
    /// Jump to a record another View owns (ADR-0037): `Shell` makes its destination active, then
    /// asks that View to show it.
    Reveal(Reveal),
    /// Enter a mode for something the View has opened itself: an edit in place (Budgets' Plan
    /// cell), its Search box or its filter popover (Transactions). While it lasts, `Shell` hands
    /// the View that mode's keys, and `Esc` comes back to the View as `escape`.
    EnterMode(ViewMode),
    /// Leave the mode the View entered, once what it opened is closed.
    ExitMode,
}

/// A mode a View enters for something it has opened itself: the [`InputMode`]s `Shell` hands
/// back to the View rather than reading itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
    /// An edit in place. `Shell` hands the View every key ahead of the router.
    Insert,
    /// The View's Search box. `Shell` hands the View every key through `search_key`.
    Search,
    /// A filter popover. `Shell` hands the View every key ahead of the router.
    Filter,
}

impl ViewMode {
    pub fn input_mode(self) -> InputMode {
        match self {
            Self::Insert => InputMode::Insert,
            Self::Search => InputMode::Search,
            Self::Filter => InputMode::Filter,
        }
    }
}

/// A record one View hands off to the View that owns it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reveal {
    /// A Transaction, by id, selected on the Transactions page.
    Transaction(u32),
    /// A Bill Schedule entry, selected on the Schedule tab in the period that shows it.
    BillEntry(EntryId),
    /// A Bill Plan, by id, selected on the Planner tab.
    BillPlan(u32),
    /// The Bills Schedule tab on a period, with no filter hiding its rows.
    BillsPeriod(Period),
    /// An Account, by id, selected on Settings › Accounts.
    Account(u32),
    /// A Payee, by id, selected on Settings › Payees.
    Payee(u32),
}

impl Reveal {
    /// The destination that owns the record.
    pub fn noun(self) -> Noun {
        match self {
            Self::Transaction(_) => Noun::Transactions,
            Self::BillEntry(_) | Self::BillPlan(_) | Self::BillsPeriod(_) => Noun::Bills,
            Self::Account(_) | Self::Payee(_) => Noun::Settings,
        }
    }
}
