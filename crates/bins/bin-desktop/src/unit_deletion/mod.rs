//! The Unit Deletion module's Desktop half (`docs/development/viper-architecture.md`, #903): the
//! Delete unit Dialog's View, which the Units page's Router opens for a delete. Every rule, every
//! piece of text and the Presenter are `lib-unit-deletion`'s; this folder turns gpui keys and
//! clicks into Presenter events and draws the view model.
//!
//! - `view/dialog/delete_unit.rs` holds the Presenter in an Entity in the Dialog host and draws it.
//!
//! Unit delete writes through `lib-database`'s `UnitDeletionAdapter`, which `LedgerPorts` builds
//! over the Units and Institutions stores (#1084), so the Unit's removal, the base's move and the
//! Institutions' new Usual units join the one pending batch and land in one commit.

pub(crate) mod view;

pub use view::dialog::delete_unit::DeleteUnitDialog;
