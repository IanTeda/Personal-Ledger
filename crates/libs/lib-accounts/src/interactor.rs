//! One Interactor per Accounts intent, each a request value whose `run` takes the port it needs.
//! Every rule is the Entity's and is checked before anything is written, so a refusal leaves the
//! port untouched. Delete is a cross-domain write, `lib-account-deletion`'s (#854).

mod add_account;
mod edit_account;

pub use add_account::AddAccount;
pub use edit_account::EditAccount;
