//! One Interactor per Inventory intent, each a request value whose `run` takes the port it needs.
//! It reads the rows, applies the Entity's rule to them and writes only the rows that changed, so
//! a refusal leaves the port untouched. Remove Location writes only Inventory, since the file's
//! cascade drops the Document Links to its Items (#964).

mod add_area;
mod add_item;
mod add_location;
mod edit_area;
mod edit_item;
mod edit_location;
mod move_area;
mod remove_area;
mod remove_item;
mod remove_location;
mod set_cover;

pub use add_area::AddArea;
pub use add_item::AddItem;
pub use add_location::AddLocation;
pub use edit_area::EditArea;
pub use edit_item::EditItem;
pub use edit_location::EditLocation;
pub use move_area::MoveArea;
pub use remove_area::RemoveArea;
pub use remove_item::RemoveItem;
pub use remove_location::RemoveLocation;
pub use set_cover::SetCover;
