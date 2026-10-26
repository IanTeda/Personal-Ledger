//! One Interactor per Categories intent, each a request value whose `run` takes the port it needs.
//! Every rule is the Entity's and is checked before anything is written, so a refusal leaves the
//! port untouched. Delete re-points other domains' rows, so it stays in `lib-ledger` (#847, Q6).

mod add_category;
mod move_category;
mod rename_category;

pub use add_category::AddCategory;
pub use move_category::MoveCategory;
pub use rename_category::RenameCategory;
