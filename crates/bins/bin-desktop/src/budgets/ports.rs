//! The Budgets module's one test seam on the Desktop: the Budgets ports themselves are
//! `lib-database`'s `StoredBudgetsPorts` (#952, #1083), which `LedgerPorts` hands the page and its
//! Dialogs as `lib_budgets::SharedPorts`. The writes are committed by the input they were made in
//! (`crate::input::send`), not here.

use std::cell::RefCell;
use std::rc::Rc;

use lib_database::FakeStores;
use lib_database::SharedBudgetsStore;

/// `sample`'s Budgets in memory, for a test that needs no storage.
pub fn in_memory(sample: &FakeStores) -> SharedBudgetsStore {
    Rc::new(RefCell::new(sample.budgets.clone()))
}
