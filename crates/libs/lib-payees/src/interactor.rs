//! One Interactor per Payees intent, each a request value whose `run` takes the ports it needs.
//! Every rule is the Entity's and is checked before anything is written, so a refusal leaves the
//! ports untouched. Each Presenter maps the refusals its Interactor can return to Messages.

mod add_payee;
mod deactivate_payee;
mod delete_payee;
mod edit_payee;
mod reactivate_payee;

pub use add_payee::AddPayee;
pub use deactivate_payee::DeactivatePayee;
pub use delete_payee::DeletePayee;
pub use edit_payee::EditPayee;
pub use reactivate_payee::ReactivatePayee;
