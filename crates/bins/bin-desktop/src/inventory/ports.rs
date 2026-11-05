//! The Inventory ports as the Desktop holds them (#866, #867, #1001): the store
//! `shell::router::build` is given, which the Inventory module's Presenters write through
//! `lib-inventory`'s Interactors. The writes are committed by the input they were made in
//! (`crate::input::send`), not here.
//!
//! What the module reads of other domains goes through `lib-database`'s `StoredInventoryReads`
//! (#1014): the fiat Units the Location form offers and the Documents the Remove location Dialog
//! quotes. A Dialog copies them in when it opens.

use std::cell::RefCell;
use std::rc::Rc;

use lib_core::{ItemId, LocationId};
use lib_database::SharedInventoryStore;
use lib_inventory::fake::InMemoryInventory;
use lib_inventory::{Inventory, InventoryRepository, Item, Location, SharedInventory};

/// The stub Inventory as of `today` in memory, for a test that needs no storage.
pub fn in_memory(today: chrono::NaiveDate) -> SharedInventoryStore {
    let Inventory { locations, items } = lib_inventory::default_inventory(today);
    Rc::new(RefCell::new(InMemoryInventory { locations, items }))
}

/// The shared store as the module's Presenters hold the repository, borrowed for each call: the
/// store is shared behind a `RefCell` whose guard can't be lent out of it.
pub struct StoreInventory(SharedInventoryStore);

impl InventoryRepository for StoreInventory {
    fn locations(&self) -> Vec<Location> {
        self.0.borrow().inventory().locations()
    }

    fn items(&self) -> Vec<Item> {
        self.0.borrow().inventory().items()
    }

    fn upsert_location(&mut self, location: Location) {
        self.0
            .borrow_mut()
            .inventory_mut()
            .upsert_location(location);
    }

    fn upsert_item(&mut self, item: Item) {
        self.0.borrow_mut().inventory_mut().upsert_item(item);
    }

    fn delete_item(&mut self, id: ItemId) {
        self.0.borrow_mut().inventory_mut().delete_item(id);
    }

    fn delete_location(&mut self, id: LocationId) {
        self.0.borrow_mut().inventory_mut().delete_location(id);
    }
}

/// `store` as the Inventory module's Presenters share it.
pub fn shared(store: &SharedInventoryStore) -> SharedInventory {
    Rc::new(RefCell::new(StoreInventory(store.clone())))
}
