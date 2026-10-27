//! The in-memory port and the stub seed, for tests and UI-first work. It holds its rows in plain
//! fields so a test can set up or read state directly.

use chrono::NaiveDate;
use lib_core::ItemId;

use crate::protocols::InventoryRepository;
use crate::{Inventory, Item, Location, default_inventory};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InMemoryInventory {
    pub locations: Vec<Location>,
    pub items: Vec<Item>,
}

impl InMemoryInventory {
    /// The stub seed: the stub Ledger's Inventory as of `today` (see [`default_inventory`]). Ids
    /// match `lib-ledger`'s stub, so its Document Links still resolve their Items.
    pub fn stub(today: NaiveDate) -> Self {
        let Inventory { locations, items } = default_inventory(today);
        Self { locations, items }
    }
}

impl InventoryRepository for InMemoryInventory {
    fn locations(&self) -> Vec<Location> {
        self.locations.clone()
    }

    fn items(&self) -> Vec<Item> {
        self.items.clone()
    }

    fn upsert_location(&mut self, location: Location) {
        match self.locations.iter_mut().find(|row| row.id == location.id) {
            Some(row) => *row = location,
            None => self.locations.push(location),
        }
    }

    fn upsert_item(&mut self, item: Item) {
        match self.items.iter_mut().find(|row| row.id == item.id) {
            Some(row) => *row = item,
            None => self.items.push(item),
        }
    }

    fn delete_item(&mut self, id: ItemId) {
        self.items.retain(|row| row.id != id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 10, 2).expect("a real date")
    }

    #[test]
    fn upserting_replaces_the_row_with_its_id_or_appends_it() {
        let mut inventory = InMemoryInventory::stub(today());
        let seeded = inventory.locations().len();
        let mut elm = inventory.locations()[0].clone();
        elm.name = "Elm".into();
        inventory.upsert_location(elm.clone());
        let fresh = Location {
            id: lib_core::LocationId::new(),
            ..elm.clone()
        };
        inventory.upsert_location(fresh.clone());

        assert_eq!(inventory.locations().len(), seeded + 1);
        assert_eq!(inventory.location(elm.id), Some(elm));
        assert_eq!(inventory.locations().last(), Some(&fresh));
    }

    #[test]
    fn deleting_an_item_drops_only_that_row() {
        let mut inventory = InMemoryInventory::stub(today());
        let before = inventory.items().len();
        let id = inventory.items()[0].id;
        inventory.delete_item(id);
        assert_eq!(inventory.items().len(), before - 1);
        assert_eq!(inventory.item(id), None);
    }
}
