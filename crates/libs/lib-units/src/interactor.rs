//! One Interactor per Units intent, each a request value whose `run` takes the ports it needs.
//! Every rule is checked before anything is written, so a refusal leaves the ports untouched.
//! A refusal is a variant of the crate's one [`Error`](crate::Error), and each Presenter maps the
//! variants its Interactor can return to Messages.

mod add_price_source;
mod add_unit;
mod deactivate_unit;
mod delete_price_source;
mod delete_unit;
mod edit_price_source;
mod edit_unit;
mod reactivate_unit;
mod set_base_unit;

pub use add_price_source::AddPriceSource;
pub use add_unit::AddUnit;
pub use deactivate_unit::DeactivateUnit;
pub use delete_price_source::{DeletePriceSource, PriceSourceDeleted};
pub use delete_unit::DeleteUnit;
pub use edit_price_source::EditPriceSource;
pub use edit_unit::EditUnit;
pub use reactivate_unit::ReactivateUnit;
pub use set_base_unit::SetBaseUnit;
