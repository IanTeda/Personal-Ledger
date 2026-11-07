//! The Account Deletion module's Desktop half (`docs/development/viper-architecture.md`, #854): the
//! Delete account Dialog's View and the entry point the Accounts page opens it through. Every
//! rule, every piece of text and the Presenter are `lib-account-deletion`'s; this folder turns gpui
//! keys and clicks into Presenter events and draws the view model.
//!
//! - `view/dialog/delete_account.rs` holds the Presenter in an Entity in the Dialog host and draws
//!   it.
//! - `router.rs` is the module's entry point ([`router::open_delete`]), which the legacy Accounts
//!   page calls until the Accounts page is a module (#855).
//!
//! Account delete writes through `lib-database`'s `AccountDeletionAdapter`, which `LedgerPorts`
//! builds over the Accounts, Transactions and Bills stores (#1084), so the Account, its
//! Transactions and the Unmatch of the entries they paid join the one pending batch and land in
//! one commit.

pub mod router;
pub(crate) mod view;

use lib_account_deletion::SharedDeletion;
use lib_database::{LedgerPorts, StoredAccountReads};

pub use view::dialog::delete_account::DeleteAccountDialog;

/// What the Delete Dialog reads and writes through: the deletion port, the Bills ports whose
/// reads a confirm copies back, and the figures it quotes as it opens: the Transactions counted by
/// the Transactions store, the Budgets by the Budgets store and the Unit codes read from the Units
/// store (#952, #1052).
#[derive(Clone)]
pub struct DeletionStores {
    pub deletion: SharedDeletion,
    /// The Accounts ports, which read the Account the Dialog deletes.
    pub accounts: lib_accounts::SharedPorts,
    pub bills: lib_bills::SharedPorts,
    pub reads: StoredAccountReads,
}

impl DeletionStores {
    /// The deletion port and reads `ports` built over the stores, beside the Accounts and Bills
    /// ports.
    pub fn of(ports: &LedgerPorts) -> Self {
        Self {
            deletion: ports.account_deletion.clone(),
            accounts: ports.accounts.clone(),
            bills: ports.bills.clone(),
            reads: ports.account_reads.clone(),
        }
    }
}
