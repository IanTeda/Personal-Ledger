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
mod fetch_all;
mod fetch_due;
mod fetch_now;
mod fetch_source;
mod reactivate_unit;
mod record_fetched_prices;
mod record_price_history;
mod save_price_source_key;
mod set_base_unit;
mod set_price;
mod test_price_source;

pub use crate::fetching::{FetchedPrice, RoundOutcome};
pub use add_price_source::AddPriceSource;
pub use add_unit::AddUnit;
pub use deactivate_unit::DeactivateUnit;
pub use delete_price_source::{DeletePriceSource, PriceSourceDeleted};
pub use delete_unit::{CheckedDelete, DeleteUnit};
pub use edit_price_source::EditPriceSource;
pub use edit_unit::EditUnit;
pub use fetch_all::FetchAll;
pub use fetch_due::FetchDue;
pub use fetch_now::FetchNow;
pub use fetch_source::FetchSource;
pub use reactivate_unit::ReactivateUnit;
pub use record_fetched_prices::RecordFetchedPrices;
pub use record_price_history::RecordPriceHistory;
pub use save_price_source_key::{KeyChange, SavePriceSourceKey};
pub use set_base_unit::SetBaseUnit;
pub use set_price::SetPrice;
pub use test_price_source::TestPriceSource;
