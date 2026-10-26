//! The port Categories reach storage through, in the shape of `lib-payees'`. It is synchronous
//! (ADR-0046): an adapter serves reads from memory and queues each write, so a write never fails
//! here; a failed save is the adapter's to report. `fake` implements it in memory.
//!
//! There is no delete: a Category delete re-points the Splits and Bill Plans filed under it, so it
//! stays a `lib-ledger` command until `lib-reassign` (#813). The page's Presenters and its Router
//! trait arrive with the page (#859).

use lib_core::CategoryId;

use crate::Category;

/// The Categories on the Ledger.
pub trait CategoryRepository {
    /// Every Category in tree order: each parent before its children, depth first, and siblings in
    /// the order they were added (see [`tree_order`](crate::tree_order)).
    fn categories(&self) -> Vec<Category>;

    fn category(&self, id: CategoryId) -> Option<Category> {
        self.categories()
            .into_iter()
            .find(|category| category.id == id)
    }

    /// Writes `category` over the row with its id, keeping that row's place among its siblings, or
    /// as a new row after every row already there.
    fn upsert_category(&mut self, category: Category);
}
