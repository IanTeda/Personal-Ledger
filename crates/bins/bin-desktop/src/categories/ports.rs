//! The Categories module's one test seam on the Desktop: the Categories ports themselves are
//! `lib-database`'s `StoredCategoriesPorts` (#858, #859, #1083), which `LedgerPorts` hands the page
//! as `lib_categories::SharedCategories` and the Add and Edit Dialogs as the save ports. The Ledger
//! keeps no read copy (#1055), and the input a write is made in announces it (#1081).

use std::cell::RefCell;
use std::rc::Rc;

use lib_database::SharedCategoriesStore;

/// The sample Categories in memory, for a test that needs no storage.
pub fn in_memory() -> SharedCategoriesStore {
    Rc::new(RefCell::new(lib_categories::fake::InMemoryCategories {
        categories: lib_categories::sample_categories(),
    }))
}
