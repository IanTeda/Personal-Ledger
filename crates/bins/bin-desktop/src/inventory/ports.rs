//! The Inventory ports as the Desktop holds them (#866, #867): the store `shell::router::build` is
//! given, shared by the Inventory module and the Ledger's forwarding shim and read copy. The writes
//! are committed by the input they were made in (`crate::input::send`), not here.
//!
//! The Inventory page writes the repository through `lib-inventory`'s Interactors. What the module
//! reaches of other domains goes through the Ledger, in [`LedgerPorts`]: the fiat Units the
//! Location form offers, the Documents the Remove location Dialog quotes, and Remove Location, the
//! reassign port, answered by running `lib_ledger::RemoveLocation` through `LedgerStore::apply`,
//! which also drops the Document Links to the Location's Items; the forwarding shim carries the
//! delete to the store in the input's one commit (Q6, until `lib-reassign`, #813). [`forward`] is
//! the Inventory shim.

use std::cell::RefCell;
use std::rc::Rc;

use gpui::{App, Entity};
use lib_core::{ItemId, LocationId, UnitId, UnitKind};
use lib_database::{InventoryStore, SharedInventoryStore};
use lib_inventory::fake::InMemoryInventory;
use lib_inventory::{
    Inventory, InventoryReassign, InventoryRepository, InventoryUnits, Item, Location,
    LocationError, LocationRemoved, LocationUsage, LocationUse, SharedInventory, UnitChoice,
};
use lib_ledger::{Ledger, LedgerChange, RemoveLocation};

use crate::documents::DocumentLink;
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
}

/// `store` as the Inventory module's Presenters share it.
pub fn shared(store: &SharedInventoryStore) -> SharedInventory {
    Rc::new(RefCell::new(StoreInventory(store.clone())))
}

/// The forwarding shim (#866): sends each Location delete in `changes` to `store`, returning
/// whether it wrote anything. The delete takes the Location's Areas and Items with it, so the
/// Area and Item deletes `lib_ledger::RemoveLocation` also records need no write of their own.
/// Writer carried: Remove Location, which also drops the Document Links to its Items, until
/// `lib-reassign` (#813); the Inventory page writes every other row itself.
pub fn forward(store: &mut dyn InventoryStore, changes: &[LedgerChange]) -> bool {
    let mut wrote = false;
    for change in changes {
        if let LedgerChange::DeleteLocation(id) = change {
            store.delete_location(*id);
            wrote = true;
        }
    }
    wrote
}

/// Brings the Ledger read copy of the Inventory in line with the store, announcing
/// `Domain::Inventory` when it changed. Run before and after every Inventory event and Dialog
/// confirm, so the Documents see a write at once, and a remove runs on the rows the page shows.
pub fn refresh(ledger: &Entity<LedgerStore>, cx: &mut App) {
    ledger.update(cx, |store, cx| store.refresh_inventory(cx));
}

/// The Inventory ports the Desktop answers from its Ledger, built for one call around its gpui
/// context: the fiat Units, the Documents a Location's Items are linked from, and Remove Location.
pub struct LedgerPorts<'a> {
    pub ledger: &'a Entity<LedgerStore>,
    pub cx: &'a mut App,
}

impl LedgerPorts<'_> {
    fn ledger(&self) -> &Ledger {
        self.ledger.read(self.cx).ledger()
    }
}

impl InventoryUnits for LedgerPorts<'_> {
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

impl LocationUsage for LedgerPorts<'_> {
    /// The Documents counted from the Ledger's read copy: those with a Link to one of the
    /// Location's Items.
    fn location_usage(&self, id: LocationId) -> LocationUse {
        let ledger = self.ledger();
        let items: Vec<ItemId> = ledger
            .inventory()
            .items
            .iter()
            .filter(|item| item.location == id)
            .map(|item| item.id)
            .collect();
        LocationUse {
            documents: ledger
                .documents()
                .iter()
                .filter(|document| {
                    document.links.iter().any(|link| {
                        matches!(link, DocumentLink::InventoryItem(item) if items.contains(item))
                    })
                })
                .count(),
        }
    }
}

impl InventoryReassign for LedgerPorts<'_> {
    /// Today's `LedgerCommand`, on the read copy brought in line with the store first. The name is
    /// read before the write takes it.
    fn remove_location(&mut self, id: LocationId) -> Result<LocationRemoved, LocationError> {
        self.ledger.update(self.cx, |store, cx| {
            store.refresh_inventory(cx);
            let name = store
                .ledger()
                .inventory()
                .location(id)
                .map(|location| location.name.clone())
                .ok_or(LocationError::Unknown)?;
            store.apply(cx, RemoveLocation(id))?;
            Ok(LocationRemoved { name })
        })
    }
}

#[cfg(test)]
mod tests {
    use gpui::{AppContext as _, TestAppContext};
    use lib_core::AreaId;

    use super::*;

    fn stored(store: &SharedInventoryStore) -> Inventory {
        store.borrow().inventory().inventory()
    }

    #[test]
    fn a_location_delete_reaches_the_store_with_its_areas_and_items() {
        let ledger = Ledger::stub();
        let store = in_memory(&ledger);
        let elm = ledger.inventory().locations[0].id;
        let changes = [
            LedgerChange::DeleteArea(AreaId::seed(1)),
            LedgerChange::DeleteLocation(elm),
            LedgerChange::DeleteTag(lib_core::TagId::seed(1)),
        ];

        assert!(forward(&mut *store.borrow_mut(), &changes));

        let after = stored(&store);
        assert!(after.location(elm).is_none());
        assert!(after.items.iter().all(|item| item.location != elm));
        assert!(!forward(&mut *store.borrow_mut(), &changes[2..]));
    }

    fn today() -> chrono::NaiveDate {
        chrono::NaiveDate::from_ymd_opt(2026, 10, 11).expect("a real date")
    }

    #[gpui::test]
    fn the_ports_read_the_ledgers_units_and_documents(cx: &mut TestAppContext) {
        let ledger = cx.new(|_| LedgerStore::new(Ledger::stub_on(today())));
        let linked = ledger.read_with(cx, |store, _| {
            store
                .ledger()
                .documents()
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
            let ports = LedgerPorts {
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

    #[gpui::test]
    fn the_reassign_port_removes_through_the_ledger_and_drops_the_links(cx: &mut TestAppContext) {
        let ledger = cx.new(|_| LedgerStore::new(Ledger::stub_on(today())));
        let elm = ledger.read_with(cx, |store, _| store.ledger().inventory().locations[0].id);

        let removed = cx.update(|cx| {
            LedgerPorts {
                ledger: &ledger,
                cx,
            }
            .remove_location(elm)
        });

        assert_eq!(removed.expect("a stub Location").name, "12 Elm St contents");
        ledger.read_with(cx, |store, _| {
            assert!(store.ledger().inventory().location(elm).is_none());
        });
        let gone = cx.update(|cx| {
            LedgerPorts {
                ledger: &ledger,
                cx,
            }
            .remove_location(elm)
        });
        assert_eq!(gone, Err(LocationError::Unknown));
    }
}
