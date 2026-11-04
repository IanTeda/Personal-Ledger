//! The Inventory ports as the Desktop holds them (#866, #867, #1001): the store
//! `shell::router::build` is given, which the Inventory module's Presenters write through
//! `lib-inventory`'s Interactors. The writes are committed by the input they were made in
//! (`crate::input::send`), not here.
//!
//! What the module reads of other domains goes through the Ledger, in [`DialogReads`]: the fiat
//! Units the Location form offers and the Documents the Remove location Dialog quotes. A Dialog
//! copies them in when it opens.

use std::cell::RefCell;
use std::rc::Rc;

use gpui::{App, Entity};
use lib_core::{ItemId, LocationId, UnitId, UnitKind};
use lib_database::SharedInventoryStore;
use lib_inventory::fake::InMemoryInventory;
use lib_inventory::{
    Inventory, InventoryRepository, InventoryUnits, Item, Location, LocationUsage, LocationUse,
    SharedInventory, UnitChoice,
};
use lib_ledger::Ledger;

use crate::ledger_store::LedgerStore;

/// `ledger`'s own Inventory in memory, for a test that needs no storage.
pub fn in_memory(ledger: &Ledger) -> SharedInventoryStore {
    let Inventory { locations, items } = ledger.inventory().clone();
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

/// The reads of other domains the Inventory Dialogs quote, built for one call around the gpui
/// context: the fiat Units and the Documents a Location's Items are linked from.
pub struct DialogReads<'a> {
    pub ledger: &'a Entity<LedgerStore>,
    pub cx: &'a mut App,
}

impl DialogReads<'_> {
    fn ledger(&self) -> &Ledger {
        self.ledger.read(self.cx).ledger()
    }
}

impl InventoryUnits for DialogReads<'_> {
    fn currencies(&self) -> Vec<UnitChoice> {
        self.ledger()
            .units()
            .iter()
            .filter(|unit| unit.kind == UnitKind::Currency)
            .map(|unit| UnitChoice {
                id: unit.id,
                code: unit.code.clone(),
                name: unit.name.clone(),
            })
            .collect()
    }

    fn unit_code(&self, id: UnitId) -> Option<String> {
        self.ledger()
            .units()
            .iter()
            .find(|unit| unit.id == id)
            .map(|unit| unit.code.clone())
    }
}

impl LocationUsage for DialogReads<'_> {
    /// The Documents counted from the Documents store: those with a Link to one of the Location's
    /// Items, which the file's cascade drops when it goes (#964).
    fn location_usage(&self, id: LocationId) -> LocationUse {
        LocationUse {
            documents: self.ledger.read(self.cx).documents_linking_location(id),
        }
    }
}

#[cfg(test)]
mod tests {
    use gpui::{AppContext as _, TestAppContext};

    use super::*;
    use crate::documents::DocumentLink;
    use crate::ledger_store::LedgerSetup;

    fn stored(store: &SharedInventoryStore) -> Inventory {
        store.borrow().inventory().inventory()
    }

    fn today() -> chrono::NaiveDate {
        chrono::NaiveDate::from_ymd_opt(2026, 10, 11).expect("a real date")
    }

    #[gpui::test]
    fn the_ports_read_the_ledgers_units_and_documents(cx: &mut TestAppContext) {
        let ledger = cx.new(|_| {
            LedgerStore::with_storage(Ledger::stub_on(today()), LedgerSetup::stub(today()))
        });
        let linked = ledger.read_with(cx, |store, _| {
            store
                .ledger()
                .stub_documents(today())
                .iter()
                .flat_map(|document| &document.links)
                .find_map(|link| match link {
                    DocumentLink::InventoryItem(id) => Some(*id),
                    _ => None,
                })
                .and_then(|item| store.ledger().inventory().item(item).map(|i| i.location))
                .expect("the stub links a Document to an Item")
        });

        cx.update(|cx| {
            let ports = DialogReads {
                ledger: &ledger,
                cx,
            };
            let units = ports.currencies();
            assert!(units.iter().any(|unit| unit.id == UnitId::SEED_AUD));
            assert!(!units.iter().any(|unit| unit.id == UnitId::SEED_BTC));
            assert_eq!(ports.unit_code(UnitId::SEED_BTC).as_deref(), Some("btc"));
            assert!(ports.location_usage(linked).documents > 0);
            assert_eq!(
                ports.location_usage(LocationId::seed(9_999)),
                LocationUse::default()
            );
        });
    }
}
