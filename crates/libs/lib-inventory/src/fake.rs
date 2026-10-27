//! The in-memory port and the stub seed, for tests and UI-first work. It holds its rows in plain
//! fields so a test can set up or read state directly.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use chrono::NaiveDate;
use lib_core::{ItemId, LocationId, UnitId};
use lib_toast::{Notice, Notify};

use crate::protocols::{
    InventoryDialog, InventoryOutcome, InventoryReassign, InventoryRepository, InventoryRouter,
    InventoryUnits, LocationRemoved, LocationUsage, LocationUse, SharedInventory, UnitChoice,
};
use crate::{Inventory, Item, Location, LocationError, default_inventory};

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

impl InMemoryInventory {
    /// These rows shared, as the Presenters hold them, with a handle a test reads them back
    /// through.
    pub fn shared(self) -> (Rc<RefCell<Self>>, SharedInventory) {
        let rows = Rc::new(RefCell::new(self));
        let shared: SharedInventory = rows.clone();
        (rows, shared)
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

/// The fiat Units a test sets; any other Unit has no code.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InMemoryUnits {
    pub units: Vec<UnitChoice>,
}

impl InMemoryUnits {
    /// Australian and US dollars, the first with the seed id `lib-ledger`'s stub gives AUD.
    pub fn stub() -> Self {
        let unit = |id, code: &str, name: &str| UnitChoice {
            id,
            code: code.into(),
            name: name.into(),
        };
        Self {
            units: vec![
                unit(UnitId::SEED_AUD, "aud", "Australian dollar"),
                unit(UnitId::seed(20), "usd", "US dollar"),
            ],
        }
    }
}

impl InventoryUnits for InMemoryUnits {
    fn currencies(&self) -> Vec<UnitChoice> {
        self.units.clone()
    }

    fn unit_code(&self, id: UnitId) -> Option<String> {
        self.units
            .iter()
            .find(|unit| unit.id == id)
            .map(|unit| unit.code.clone())
    }
}

/// What names each Location in another domain, set per Location; one not listed has none.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InMemoryUsage {
    pub uses: HashMap<LocationId, LocationUse>,
}

impl LocationUsage for InMemoryUsage {
    fn location_usage(&self, id: LocationId) -> LocationUse {
        self.uses.get(&id).copied().unwrap_or_default()
    }
}

/// The reassign port over in-memory rows: it removes the Location with its Items. It drops no
/// Document Links, since no Documents are held here.
#[derive(Debug, Clone)]
pub struct InMemoryReassign {
    pub inventory: Rc<RefCell<InMemoryInventory>>,
}

impl InMemoryReassign {
    pub fn new(inventory: Rc<RefCell<InMemoryInventory>>) -> Self {
        Self { inventory }
    }
}

impl InventoryReassign for InMemoryReassign {
    fn remove_location(&mut self, id: LocationId) -> Result<LocationRemoved, LocationError> {
        let mut rows = self.inventory.borrow_mut();
        let location = rows
            .locations
            .iter()
            .find(|row| row.id == id)
            .ok_or(LocationError::Unknown)?;
        let name = location.name.clone();
        rows.locations.retain(|row| row.id != id);
        rows.items.retain(|item| item.location != id);
        Ok(LocationRemoved { name })
    }
}

/// What a [`RecordingRouter`] was asked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RouterCall {
    OpenDialog(InventoryDialog),
    CloseDialog(Option<InventoryOutcome>),
    Notice(Notice),
}

/// A Router that does nothing but record each call, in order.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RecordingRouter {
    pub calls: Vec<RouterCall>,
}

impl RecordingRouter {
    /// The calls made since the last `take`.
    pub fn take(&mut self) -> Vec<RouterCall> {
        std::mem::take(&mut self.calls)
    }
}

impl InventoryRouter for RecordingRouter {
    /// Always opens: the recording Router builds no Presenter that could fail.
    fn open_dialog(&mut self, dialog: InventoryDialog) -> bool {
        self.calls.push(RouterCall::OpenDialog(dialog));
        true
    }

    fn close_dialog(&mut self, outcome: Option<InventoryOutcome>) {
        self.calls.push(RouterCall::CloseDialog(outcome));
    }
}

impl Notify for RecordingRouter {
    fn notify(&mut self, notice: Notice) {
        self.calls.push(RouterCall::Notice(notice));
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
            id: LocationId::new(),
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
