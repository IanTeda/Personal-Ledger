//! One Interactor per Units intent, each a request value whose `run` takes the ports it needs.
//! Every rule is checked before anything is written, so a refusal leaves the ports untouched.
//! Refusals are typed, and each Interactor's refusal type holds only what it can return.

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
pub use add_unit::{AddUnit, AddUnitRefusal};
pub use deactivate_unit::{DeactivateUnit, DeactivateUnitRefusal};
pub use delete_price_source::{DeletePriceSource, DeletePriceSourceRefusal, PriceSourceDeleted};
pub use delete_unit::{DeleteUnit, DeleteUnitRefusal};
pub use edit_price_source::{EditPriceSource, EditPriceSourceRefusal};
pub use edit_unit::{EditUnit, EditUnitRefusal};
pub use reactivate_unit::ReactivateUnit;
pub use set_base_unit::{SetBaseUnit, SetBaseUnitRefusal};

/// The Unit a request addressed is gone, which only happens when it was removed while its Dialog
/// was open.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("unit not found")]
pub struct UnitGone;
