//! The Categories store as the Desktop holds it (#858): the store `shell::router::build` is given,
//! shared by the Ledger's forwarding shim and read copy, the Bills ports (`BillCategories`) and the
//! Payees ports and Import's Category select (`PayeeCategories`). The writes are committed by the
//! input they were made in (`crate::input::send`), not here.
//!
//! Every Categories writer is still a `LedgerCommand`: the legacy Categories page's Add, Edit and
//! the type and move it carries, and Category reassign (delete), until the page writes the port
//! itself (#859) and reassign moves to `lib-reassign` (#813). [`forward`] and [`forward_deletes`]
//! carry them to the store.

use lib_database::{CategoriesStore, SharedCategoriesStore};
use lib_ledger::{Ledger, LedgerChange};

/// `ledger`'s own Categories in memory, for a test that needs no storage.
pub fn in_memory(ledger: &Ledger) -> SharedCategoriesStore {
    std::rc::Rc::new(std::cell::RefCell::new(
        lib_categories::fake::InMemoryCategories {
            categories: ledger.categories().to_vec(),
        },
    ))
}

/// The forwarding shim's first half (#858): sends each Category row `changes` upserts to `store`,
/// in the order recorded, returning whether it wrote anything. Run before the Payees, Transactions
/// and Bills, since Category reassign re-points their rows to an Uncategorised it may create.
/// Writers carried: the legacy Categories page's Add and Edit, and the Uncategorised reassign
/// creates.
pub fn forward(store: &mut dyn CategoriesStore, changes: &[LedgerChange]) -> bool {
    let port = store.categories_mut();
    let mut wrote = false;
    for change in changes {
        if let LedgerChange::UpsertCategory(category) = change {
            port.upsert_category(category.clone());
            wrote = true;
        }
    }
    wrote
}

/// The forwarding shim's second half: sends each Category `changes` deletes to `store`, returning
/// whether it wrote anything. Run after the Payees, Transactions and Bills, since the rows naming
/// the Category must be re-pointed before it goes (their foreign keys). Writer carried: Category
/// reassign, until `lib-reassign` (#813).
pub fn forward_deletes(store: &mut dyn CategoriesStore, changes: &[LedgerChange]) -> bool {
    let mut wrote = false;
    for change in changes {
        if let LedgerChange::DeleteCategory(id) = change {
            store.delete_category(*id);
            wrote = true;
        }
    }
    wrote
}

#[cfg(test)]
mod tests {
    use lib_core::{CategoryId, TagId};

    use super::*;

    #[test]
    fn category_upserts_and_deletes_reach_the_store_in_their_own_halves() {
        let ledger = Ledger::stub();
        let store = in_memory(&ledger);
        let mut groceries = ledger.categories()[6].clone();
        groceries.name = "Supermarket".into();
        let gone = CategoryId::seed(8);
        let changes = [
            LedgerChange::UpsertCategory(groceries.clone()),
            LedgerChange::DeleteCategory(gone),
            LedgerChange::DeleteTag(TagId::seed(1)),
        ];

        assert!(forward(&mut *store.borrow_mut(), &changes));
        assert!(
            store.borrow().categories().category(gone).is_some(),
            "not yet"
        );
        assert!(forward_deletes(&mut *store.borrow_mut(), &changes));

        let store = store.borrow();
        assert_eq!(store.categories().category(groceries.id), Some(groceries));
        assert!(store.categories().category(gone).is_none());
        assert!(!forward(
            &mut lib_categories::fake::InMemoryCategories::default(),
            &changes[2..]
        ));
    }
}
