//! The port Inventory reaches storage through. It is synchronous (ADR-0046): an adapter serves
//! reads from memory and queues each write, so a write never fails here; a failed save is the
//! adapter's to report. `fake` implements it in memory.
//!
//! There is no delete for a Location: removing one also drops the Document Links to its Items, so
//! `lib-ledger` runs it and the Client's forwarding shim deletes the row (Q18).

use crate::{Inventory, Item, Location};
use lib_core::{ItemId, LocationId};

/// The Locations and Items on the Ledger.
pub trait InventoryRepository {
    /// Every Location, in the order they were added, each with its cover and its Areas in tab
    /// order.
    fn locations(&self) -> Vec<Location>;

    /// Every Item, in the order they were added.
    fn items(&self) -> Vec<Item>;

    fn location(&self, id: LocationId) -> Option<Location> {
        self.locations().into_iter().find(|l| l.id == id)
    }

    fn item(&self, id: ItemId) -> Option<Item> {
        self.items().into_iter().find(|i| i.id == id)
    }

    /// Both lists as the one value the rules run over.
    fn inventory(&self) -> Inventory {
        Inventory {
            locations: self.locations(),
            items: self.items(),
        }
    }

    /// Writes `location` over the row with its id, or as a new row after every row already there.
    /// Its cover and its Areas go with it, in order: an Area the row had and `location` no longer
    /// lists is deleted, so the Items in it are moved first.
    fn upsert_location(&mut self, location: Location);

    /// Writes `item` over the row with its id, or as a new row after every row already there.
    fn upsert_item(&mut self, item: Item);

    fn delete_item(&mut self, id: ItemId);
}
