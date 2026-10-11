//! `ViewEvent` -- the requests a View makes of `Shell`. A View emits one of these with
//! `cx.emit` and never holds a handle to `Shell` (ADR-0032): `Shell` subscribes to each View
//! and handles every event in one place (`app/view_events.rs`).

use lib_toast::ToastKind;

use crate::{
    bills::EntryId, chrome::dialog_host::OpenDialog, navigation::nav::Noun, period::Period,
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
    /// Enter Insert mode for an edit the View has opened in place (Budgets' Plan cell). While
    /// it lasts, `Shell` hands the View every key ahead of the router.
    EnterInsert,
    /// Leave Insert mode, once the View's edit in place is saved or abandoned.
    ExitInsert,
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
}

impl Reveal {
    /// The destination that owns the record.
    pub fn noun(self) -> Noun {
        match self {
            Self::Transaction(_) => Noun::Transactions,
            Self::BillEntry(_) | Self::BillPlan(_) | Self::BillsPeriod(_) => Noun::Bills,
        }
    }
}
