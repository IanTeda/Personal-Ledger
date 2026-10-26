//! One Interactor per Tags intent, each a request value whose `run` takes the port it needs. Every
//! rule is the Entity's and is checked before anything is written, so a refusal leaves the port
//! untouched. Remove and merge rewrite Splits, so they stay `lib-ledger`'s.

mod add_tag;
mod deactivate_tag;
mod edit_tag;
mod reactivate_tag;

pub use add_tag::AddTag;
pub use deactivate_tag::DeactivateTag;
pub use edit_tag::EditTag;
pub use reactivate_tag::ReactivateTag;
