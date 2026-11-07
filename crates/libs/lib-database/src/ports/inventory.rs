//! `lib-inventory`'s repository answered over the Inventory store (#866, #867, #1001, #1083). What
//! the module reads of other domains is [`StoredInventoryReads`](crate::StoredInventoryReads).

use lib_core::{ItemId, LocationId};
use lib_inventory::{InventoryRepository, Item, Location};

use crate::adapter::store::SharedInventoryStore;

/// [`InventoryRepository`] over the shared Inventory store, borrowed for each call: the store is
/// shared behind a `RefCell` whose guard can't be lent out of it. A write reaches the store's
/// recorder, so the commit names the Inventory.
pub struct StoredInventoryPorts {
    store: SharedInventoryStore,
}

impl StoredInventoryPorts {
    pub fn new(store: SharedInventoryStore) -> Self {
        Self { store }
    }
}

impl InventoryRepository for StoredInventoryPorts {
    fn locations(&self) -> Vec<Location> {
        self.store.borrow().inventory().locations()
    }

    fn items(&self) -> Vec<Item> {
        self.store.borrow().inventory().items()
    }

    fn upsert_location(&mut self, location: Location) {
        self.store
            .borrow_mut()
            .inventory_mut()
            .upsert_location(location);
    }

    fn upsert_item(&mut self, item: Item) {
        self.store.borrow_mut().inventory_mut().upsert_item(item);
    }

    fn delete_item(&mut self, id: ItemId) {
        self.store.borrow_mut().inventory_mut().delete_item(id);
    }

    fn delete_location(&mut self, id: LocationId) {
        self.store.borrow_mut().inventory_mut().delete_location(id);
    }
}

#[cfg(test)]
mod tests {
    use crate::ports::test_support::{fakes, file};
    use crate::{Domain, LedgerPorts};

    #[test]
    fn an_inventory_write_reports_the_inventory_written_on_either() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        for ledger in [fakes(), file(dir.path())] {
            let LedgerPorts {
                inventory, commit, ..
            } = ledger;
            let location = inventory
                .borrow()
                .locations()
                .into_iter()
                .next()
                .expect("a seeded Location");
            assert!(commit.commit().is_empty(), "a read writes nothing");

            inventory.borrow_mut().upsert_location(location);

            assert_eq!(commit.commit().wrote, [Domain::Inventory]);
        }
    }
}
