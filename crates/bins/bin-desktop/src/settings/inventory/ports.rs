//! The Inventory store as the Desktop holds it (#866): the store `shell::router::build` is given,
//! shared by the Ledger's forwarding shim and read copy. The writes are committed by the input
//! they were made in (`crate::input::send`), not here.
//!
//! The Inventory page still writes through `lib_ledger`'s Inventory commands until it runs on
//! `lib-inventory`'s Interactors (#867), and Remove Location, which also drops the Document Links
//! to its Items, stays a Ledger command until `lib-reassign` (#813): [`forward`] is the Inventory
//! shim, carrying their changes to the store.

use std::cell::RefCell;
use std::rc::Rc;

use lib_core::{AreaId, LocationId};
use lib_database::{InventoryStore, SharedInventoryStore};
use lib_inventory::Inventory;
use lib_inventory::fake::InMemoryInventory;
use lib_ledger::{Ledger, LedgerChange};

/// `ledger`'s own Inventory in memory, for a test that needs no storage.
pub fn in_memory(ledger: &Ledger) -> SharedInventoryStore {
    let Inventory { locations, items } = ledger.inventory().clone();
    Rc::new(RefCell::new(InMemoryInventory { locations, items }))
}

/// The forwarding shim (#866): sends each Inventory change in `changes` to `store`, in the order
/// recorded, returning whether it wrote anything. `written` is the Ledger's Inventory once the
/// command ran: an Area change names only its own row, so the shim writes the Location holding
/// it as `written` has it, with its Areas in tab order. A Location `written` no longer holds is
/// being removed, so its Area deletes wait for its own delete, which takes its Areas and Items.
/// Remove Area records the Items it moves before the Area, so they leave the Area first.
pub fn forward(
    store: &mut dyn InventoryStore,
    changes: &[LedgerChange],
    written: &Inventory,
) -> bool {
    let mut wrote = false;
    for change in changes {
        wrote |= match change {
            LedgerChange::UpsertLocation(location) => {
                store.inventory_mut().upsert_location(location.clone());
                true
            }
            LedgerChange::DeleteLocation(id) => {
                store.delete_location(*id);
                true
            }
            LedgerChange::UpsertItem(item) => {
                store.inventory_mut().upsert_item(item.clone());
                true
            }
            LedgerChange::DeleteItem(id) => {
                store.inventory_mut().delete_item(*id);
                true
            }
            LedgerChange::UpsertArea { location, .. } => write_location(store, written, *location),
            LedgerChange::DeleteArea(area) => {
                holder_of(store, *area).is_some_and(|id| write_location(store, written, id))
            }
            _ => false,
        };
    }
    wrote
}

/// The Location the store holds Area `area` in.
fn holder_of(store: &dyn InventoryStore, area: AreaId) -> Option<LocationId> {
    store
        .inventory()
        .locations()
        .into_iter()
        .find(|location| location.areas.iter().any(|a| a.id == area))
        .map(|location| location.id)
}

/// Writes Location `id` as `written` has it, unless the store already holds it so: a Move Area
/// records both Areas it swapped, which is one write of their Location. Returns whether it wrote.
fn write_location(store: &mut dyn InventoryStore, written: &Inventory, id: LocationId) -> bool {
    let Some(location) = written.location(id) else {
        return false;
    };
    if store.inventory().location(id).as_ref() == Some(location) {
        return false;
    }
    store.inventory_mut().upsert_location(location.clone());
    true
}

#[cfg(test)]
mod tests {
    use lib_ledger::{AddArea, LedgerCommand, MoveArea, RemoveArea, RemoveLocation};

    use super::*;

    /// Runs `command` on `ledger` and forwards what it recorded to `store`.
    fn run<C: LedgerCommand>(
        ledger: &mut Ledger,
        store: &SharedInventoryStore,
        command: C,
    ) -> bool {
        command
            .run(ledger)
            .map_err(|_| ())
            .expect("the command lands");
        let changes = ledger.drain_changes();
        forward(&mut *store.borrow_mut(), &changes, ledger.inventory())
    }

    fn stored(store: &SharedInventoryStore) -> Inventory {
        store.borrow().inventory().inventory()
    }

    #[test]
    fn each_inventory_command_leaves_the_store_as_the_ledger() {
        let mut ledger = Ledger::stub();
        let store = in_memory(&ledger);
        let elm = ledger.inventory().locations[0].clone();
        let (from, to) = (elm.areas[0].id, elm.areas[1].id);
        assert!(ledger.inventory().area_items(from) > 0);

        assert!(run(
            &mut ledger,
            &store,
            AddArea {
                location: elm.id,
                name: "Attic".into()
            }
        ));
        assert_eq!(&stored(&store), ledger.inventory());

        assert!(run(
            &mut ledger,
            &store,
            MoveArea {
                area: to,
                delta: -1
            }
        ));
        assert_eq!(&stored(&store), ledger.inventory());

        assert!(run(
            &mut ledger,
            &store,
            RemoveArea {
                area: from,
                destination: Some(to)
            }
        ));
        assert_eq!(&stored(&store), ledger.inventory());

        assert!(run(&mut ledger, &store, RemoveLocation(elm.id)));
        assert_eq!(&stored(&store), ledger.inventory());
        assert!(stored(&store).location(elm.id).is_none());
    }

    #[test]
    fn other_domains_changes_write_nothing() {
        let ledger = Ledger::stub();
        let store = in_memory(&ledger);
        let changes = [
            LedgerChange::DeleteTag(lib_core::TagId::seed(1)),
            LedgerChange::DeleteArea(AreaId::seed(9_999)),
        ];

        assert!(!forward(
            &mut *store.borrow_mut(),
            &changes,
            ledger.inventory()
        ));
        assert_eq!(&stored(&store), ledger.inventory());
    }
}
