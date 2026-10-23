//! `ViewEvent` -- the requests a View makes of `Shell`. A View emits one of these with
//! `cx.emit` and never holds a handle to `Shell` (ADR-0032): `Shell` subscribes to each View
//! and handles every event in one place (`app/view_events.rs`).

use lib_core::{AccountId, BillPlanId, PayeeId, TransactionId};
use lib_toast::ToastKind;

use crate::{
    bills::EntryId,
    chrome::{
        dialog_entity::DialogHandle,
        dialog_host::{HostedOpen, OpenDialog},
    },
    navigation::nav::{InputMode, Noun},
    period::Period,
    settings::{SettingsPage, display::DisplayPreferences},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ViewEvent {
    /// Raise a Toast whose Message the View has already resolved to text.
    RaiseToast { kind: ToastKind, text: String },
    /// Open a Dialog in the Dialog host.
    OpenDialog(Box<OpenDialog>),
    /// Open a hosted Dialog, whose confirm hands its outcome back to the page that opened it
    /// (`dialog_host::open_dialog`).
    OpenHostedDialog(Box<HostedOpen>),
    /// Open a module's Dialog, an Entity holding its Presenter (`dialog_entity::host_dialog`).
    HostDialog(DialogHandle),
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
    /// Run a palette command by name, as its click target on a View would (the Dashboard's
    /// `:open`/`:new` empty state).
    RunCommand(&'static str),
    /// The Display Preferences changed on Settings. They are cross-cutting: `Shell` hands them to
    /// the Views that draw with them, and the Toasts Preference to its Toasts.
    DisplayChanged(DisplayPreferences),
    /// Settings swapped its open page; `Shell` records it as a step for Back and Forward.
    SettingsPageChanged(SettingsPage),
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
    Transaction(TransactionId),
    /// A Bill Schedule entry, selected on the Schedule tab in the period that shows it.
    BillEntry(EntryId),
    /// A Bill Plan, by id, selected on the Planner tab.
    BillPlan(BillPlanId),
    /// The Bills Schedule tab on a period, with no filter hiding its rows.
    BillsPeriod(Period),
    /// An Account, by id, selected on Settings › Accounts.
    Account(AccountId),
    /// An Account's ledger: Transactions filtered to the Account, from their first row.
    AccountTransactions(AccountId),
    /// A Payee, by id, selected on Settings › Payees.
    Payee(PayeeId),
}

impl Reveal {
    /// The destination that owns the record.
    pub fn noun(self) -> Noun {
        match self {
            Self::Transaction(_) | Self::AccountTransactions(_) => Noun::Transactions,
            Self::BillEntry(_) | Self::BillPlan(_) | Self::BillsPeriod(_) => Noun::Bills,
            Self::Account(_) | Self::Payee(_) => Noun::Settings,
        }
    }
}
