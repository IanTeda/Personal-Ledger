//! The Account Deletion module's Desktop half (`docs/development/viper-architecture.md`, #854): the
//! Delete account Dialog's View and the entry point the Accounts page opens it through. Every
//! rule, every piece of text and the Presenter are `lib-account-deletion`'s; this folder turns gpui
//! keys and clicks into Presenter events and draws the view model.
//!
//! - `view/dialog/delete_account.rs` is the Presenter's `DialogView`, hosted in a generic
//!   `ModuleDialog` (ADR-0056), and draws it.
//! - `router.rs` is the module's entry point ([`router::open_delete`]), which the Accounts page's
//!   Router calls.
//!
//! The Dialog reads and writes through `lib-account-deletion`'s `DeletionPorts`, which the open
//! Ledger hands out whole (#1204): its deletion port writes the Accounts, Transactions and Bills
//! stores together (#1084), so the Account, its Transactions and the Unmatch of the entries they
//! paid join the one pending batch and land in one commit.

pub mod router;
pub(crate) mod view;
