//! Settings' Accounts page (2o): its Entity, which renders the page (`settings::accounts`)
//! and builds the Add, Edit and Delete dialogs drawn here. The standalone 3a page left the primary
//! rail for Settings (#420).

pub mod add_dialog;
pub mod delete_dialog;
pub mod edit_dialog;
pub(crate) mod hints;
pub(crate) mod select_field;
pub mod snapshot;
pub mod state;
