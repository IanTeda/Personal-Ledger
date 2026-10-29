//! The ports Accounts reach storage and the other domains through, in the shape of `lib-payees'`.
//! They are synchronous (ADR-0046): an adapter serves reads from memory and queues each write, so a
//! write never fails here; a failed save is the adapter's to report. `fake` implements them in
//! memory.
//!
//! The adapter's [`AccountUsage`] counts the Transactions; the Budgets count is `lib-ledger`'s until
//! Budgets persist (#811). The Presenters reach every port together, through [`AccountsPorts`], and
//! ask for navigation through [`AccountsRouter`], which each Client implements (#855).

use std::cell::RefCell;
use std::rc::Rc;

use lib_core::{AccountId, InstitutionId, UnitId};
use lib_toast::Notify;

use crate::Account;

/// The Accounts on the Ledger.
pub trait AccountRepository {
    /// Every Account, in the order they were added.
    fn accounts(&self) -> Vec<Account>;

    fn account(&self, id: AccountId) -> Option<Account> {
        self.accounts().into_iter().find(|account| account.id == id)
    }

    /// Writes `account` over the row with its id, or as a new row after every row already there.
    fn upsert_account(&mut self, account: Account);

    /// Removes Account `id`'s row. The Transactions booked to it are not this port's to remove.
    fn delete_account(&mut self, id: AccountId);
}

/// How many rows of the other domains name one Account, as its Edit and Delete Dialogs show.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct AccountUse {
    /// The Transactions booked to it, counted from the Transactions themselves.
    pub transactions: usize,
    /// The Budgets that count it.
    pub budgets: usize,
}

/// The read the Edit and Delete Dialogs count an Account's Transactions and Budgets through, so
/// `lib-accounts` needn't depend on either domain's crate. Only a figure the Dialogs show, so a
/// read that can't be made is the answerer's to report, not a refusal here.
pub trait AccountUsage {
    /// What names Account `id`; an Account nothing names, or one that doesn't exist, has none.
    fn account_usage(&self, id: AccountId) -> AccountUse;
}

/// An Institution an Account may be at, as Accounts reads it: the id the Account keeps and the name
/// the Institution select shows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountInstitution {
    pub id: InstitutionId,
    pub name: String,
    /// The Unit a new Account at this Institution starts on, a hint only: the base Unit stays the
    /// Ledger's one default.
    pub usual_unit: Option<UnitId>,
}

/// The Institutions on the Ledger. A read port, so `lib-accounts` needn't depend on the
/// Institutions crate.
pub trait AccountInstitutions {
    /// Every Institution, in the Institutions' order.
    fn account_institutions(&self) -> Vec<AccountInstitution>;
}

/// A Unit an Account may count in, as Accounts reads it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountUnit {
    pub id: UnitId,
    pub code: String,
    /// A deactivated Unit takes no new Account, though one already on it keeps showing its code.
    pub is_active: bool,
    /// The Unit's Precision (GLOSSARY): the places an amount in it shows at, and an opening balance
    /// is padded to. The number, not `lib_units::Precision`, so `lib-accounts` needn't depend on the
    /// Units crate.
    pub precision: u8,
}

/// The Units on the Ledger and the base Unit. A read port, so `lib-accounts` needn't depend on the
/// Units crate.
pub trait AccountUnits {
    /// Every Unit, in the Units' order.
    fn account_units(&self) -> Vec<AccountUnit>;

    /// The Unit the net worth is summed in and a new Account starts on; `None` when none is set.
    fn base_unit(&self) -> Option<UnitId>;
}

/// Every Accounts port, as one value the Presenters share.
pub trait AccountsPorts {
    fn accounts(&self) -> &dyn AccountRepository;

    fn accounts_mut(&mut self) -> &mut dyn AccountRepository;

    fn usage(&self) -> &dyn AccountUsage;

    fn institutions(&self) -> &dyn AccountInstitutions;

    fn units(&self) -> &dyn AccountUnits;
}

/// The ports as the page and its Dialogs hold them: one set, shared, so an Account a Dialog adds is
/// on the page as soon as the Dialog closes. Single-threaded, as both Clients' UI threads are.
pub type SharedPorts = Rc<RefCell<dyn AccountsPorts>>;

/// An Accounts Dialog, as the page asks the Router to open it. Delete is not here: it is
/// `lib-account-deletion`'s, which the Router opens ([`AccountsRouter::open_delete`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AccountsDialog {
    /// A fresh form, its Name starting on `name`: empty from the page, the typed argument from
    /// `accounts new <account name>`.
    AddAccount {
        name: String,
    },
    EditAccount(AccountId),
}

/// What a confirmed Accounts Dialog did to an Account.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountsOutcome {
    /// The Account added or edited, for the page to select.
    pub id: AccountId,
}

/// Navigation out of the Accounts Presenters, implemented by each Client. The Client passes it into
/// each `handle` call rather than the Presenter holding it, since the Desktop can only open a
/// Dialog with the gpui context that call runs in. A Toast or status-line message goes to the
/// Shell through [`Notify`] (ADR-0048).
pub trait AccountsRouter: Notify {
    /// Builds `dialog`'s Presenter on the shared ports and hosts its View. Returns whether it
    /// opened: a Dialog on an Account removed since it was asked for has no Presenter to build.
    fn open_dialog(&mut self, dialog: AccountsDialog) -> bool;

    /// Opens the Account Deletion module's Delete Dialog on Account `id`. Returns whether it
    /// opened. The page selects the row that slides into the deleted one's place when the
    /// Dialog lands ([`AccountsPageEvent::Deleted`]).
    ///
    /// [`AccountsPageEvent::Deleted`]: crate::presenter::AccountsPageEvent::Deleted
    fn open_delete(&mut self, id: AccountId) -> bool;

    /// Shows Account `id`'s ledger: the Transactions filtered to it.
    fn open_ledger(&mut self, id: AccountId);

    /// Closes the open Accounts Dialog. `outcome` is what its confirm did, `None` when it was
    /// cancelled; the Client hands it to the page as [`AccountsPageEvent::DialogClosed`].
    ///
    /// [`AccountsPageEvent::DialogClosed`]: crate::presenter::AccountsPageEvent::DialogClosed
    fn close_dialog(&mut self, outcome: Option<AccountsOutcome>);
}
