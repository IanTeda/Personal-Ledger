//! The Inventory writes (ADR-0038): each a command value with its own refusal, checked before
//! anything is written so a refusal leaves the Ledger untouched, and recording a change per row it
//! wrote (ADR-0039).
//!
//! Single-entity rules stay in `lib_inventory`. What lives here is the rule that crosses domains: a
//! removed Location drops the Document Links to its Items, since a Document names an Item by id.

use lib_core::{AreaId, ItemId, LocationId, UnitId};
use lib_inventory::{AreaError, LocationDraft, LocationError, RemovedArea, RemovedLocation};

use crate::{Ledger, LedgerChange, LedgerCommand};

/// Why an Area did not move: it is unknown, or already at the end it was asked to move past.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum MoveAreaRefusal {
    #[error("area did not move")]
    NotMoved,
}

impl Ledger {
    /// Adds a Location with no Areas and returns its id.
    pub fn add_location(
        &mut self,
        draft: &LocationDraft,
        unit: UnitId,
    ) -> Result<LocationId, LocationError> {
        let id = self.inventory.add_location(draft, unit)?;
        self.record_location(id);
        Ok(id)
    }

    /// Edits a Location's name, address and cover. Its Unit is fixed once added.
    pub fn edit_location(
        &mut self,
        id: LocationId,
        draft: &LocationDraft,
    ) -> Result<(), LocationError> {
        self.inventory.edit_location(id, draft)?;
        self.record_location(id);
        Ok(())
    }

    /// Removes a Location with its Areas and Items, and drops the Document Links to those Items.
    pub fn remove_location(&mut self, id: LocationId) -> Result<RemovedLocation, LocationError> {
        let removed = self.inventory.remove_location(id)?;
        for area in &removed.areas {
            self.pending.push(LedgerChange::DeleteArea(*area));
        }
        for item in &removed.items {
            self.pending.push(LedgerChange::DeleteItem(*item));
        }
        self.pending.push(LedgerChange::DeleteLocation(id));
        self.drop_document_inventory_links(&removed.items);
        Ok(removed)
    }

    /// Adds an Area at the end of a Location's tabs and returns its id.
    pub fn add_area(&mut self, location: LocationId, name: &str) -> Result<AreaId, AreaError> {
        let id = self.inventory.add_area(location, name)?;
        self.record_area(id);
        Ok(id)
    }

    /// Renames an Area.
    pub fn edit_area(&mut self, area: AreaId, name: &str) -> Result<(), AreaError> {
        self.inventory.edit_area(area, name)?;
        self.record_area(area);
        Ok(())
    }

    /// Removes an Area, moving its Items to `destination` first when it holds any.
    pub fn remove_area(
        &mut self,
        area: AreaId,
        destination: Option<AreaId>,
    ) -> Result<RemovedArea, AreaError> {
        let removed = self.inventory.remove_area(area, destination)?;
        for item in &removed.moved_items {
            self.record_item(*item);
        }
        self.pending.push(LedgerChange::DeleteArea(area));
        Ok(removed)
    }

    /// Moves an Area one place down (`delta` > 0) or up within its Location. The Area and the
    /// neighbour it swapped with both change position.
    pub fn move_area(&mut self, area: AreaId, delta: isize) -> Result<(), MoveAreaRefusal> {
        let [moved, neighbour] = self
            .inventory
            .move_area(area, delta)
            .ok_or(MoveAreaRefusal::NotMoved)?;
        self.record_area(moved);
        self.record_area(neighbour);
        Ok(())
    }

    fn record_location(&mut self, id: LocationId) {
        if let Some(location) = self.inventory.location(id) {
            self.pending
                .push(LedgerChange::UpsertLocation(location.clone()));
        }
    }

    fn record_area(&mut self, area: AreaId) {
        if let Some((location, area)) = self.inventory.area(area) {
            self.pending.push(LedgerChange::UpsertArea {
                location: location.id,
                area: area.clone(),
            });
        }
    }

    fn record_item(&mut self, item: ItemId) {
        if let Some(item) = self.inventory.item(item) {
            self.pending.push(LedgerChange::UpsertItem(item.clone()));
        }
    }
}

/// [`Ledger::add_location`] as a value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddLocation {
    pub draft: LocationDraft,
    pub unit: UnitId,
}

impl LedgerCommand for AddLocation {
    type Outcome = LocationId;
    type Refusal = LocationError;
    fn run(self, ledger: &mut Ledger) -> Result<LocationId, LocationError> {
        ledger.add_location(&self.draft, self.unit)
    }
}

/// [`Ledger::edit_location`] as a value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditLocation {
    pub id: LocationId,
    pub draft: LocationDraft,
}

impl LedgerCommand for EditLocation {
    type Outcome = ();
    type Refusal = LocationError;
    fn run(self, ledger: &mut Ledger) -> Result<(), LocationError> {
        ledger.edit_location(self.id, &self.draft)
    }
}

/// [`Ledger::remove_location`] as a value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RemoveLocation(pub LocationId);

impl LedgerCommand for RemoveLocation {
    type Outcome = RemovedLocation;
    type Refusal = LocationError;
    fn run(self, ledger: &mut Ledger) -> Result<RemovedLocation, LocationError> {
        ledger.remove_location(self.0)
    }
}

/// [`Ledger::add_area`] as a value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddArea {
    pub location: LocationId,
    pub name: String,
}

impl LedgerCommand for AddArea {
    type Outcome = AreaId;
    type Refusal = AreaError;
    fn run(self, ledger: &mut Ledger) -> Result<AreaId, AreaError> {
        ledger.add_area(self.location, &self.name)
    }
}

/// [`Ledger::edit_area`] as a value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditArea {
    pub area: AreaId,
    pub name: String,
}

impl LedgerCommand for EditArea {
    type Outcome = ();
    type Refusal = AreaError;
    fn run(self, ledger: &mut Ledger) -> Result<(), AreaError> {
        ledger.edit_area(self.area, &self.name)
    }
}

/// [`Ledger::remove_area`] as a value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RemoveArea {
    pub area: AreaId,
    pub destination: Option<AreaId>,
}

impl LedgerCommand for RemoveArea {
    type Outcome = RemovedArea;
    type Refusal = AreaError;
    fn run(self, ledger: &mut Ledger) -> Result<RemovedArea, AreaError> {
        ledger.remove_area(self.area, self.destination)
    }
}

/// [`Ledger::move_area`] as a value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MoveArea {
    pub area: AreaId,
    pub delta: isize,
}

impl LedgerCommand for MoveArea {
    type Outcome = ();
    type Refusal = MoveAreaRefusal;
    fn run(self, ledger: &mut Ledger) -> Result<(), MoveAreaRefusal> {
        ledger.move_area(self.area, self.delta)
    }
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;
    use lib_documents::DocumentLink;
    use lib_inventory::{Inventory, NameError, default_inventory};

    use super::*;
    use crate::{Domain, LedgerParts, domains};

    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 10, 15).unwrap()
    }

    /// A Ledger holding only the seeded Inventory, so each test reads just the collection it
    /// writes.
    fn ledger() -> Ledger {
        Ledger::from_parts(LedgerParts {
            inventory: default_inventory(today()),
            ..LedgerParts::default()
        })
    }

    fn draft(name: &str) -> LocationDraft {
        LocationDraft {
            name: name.to_string(),
            ..LocationDraft::default()
        }
    }

    /// An Area in the seed that holds Items, with another Area of the same Location to receive
    /// them, and the Location both belong to.
    fn area_with_items(inventory: &Inventory) -> (LocationId, AreaId, AreaId) {
        let item = inventory.items.first().expect("seeded Items");
        let (location, area) = inventory.area(item.area).expect("an Item's Area exists");
        let other = location
            .areas
            .iter()
            .find(|r| r.id != area.id)
            .expect("the seed's Location has a second Area")
            .id;
        (location.id, area.id, other)
    }

    /// The Ledger must come out of a refused command exactly as it went in.
    fn assert_untouched(ledger: &mut Ledger, before: &Inventory) {
        assert_eq!(ledger.inventory(), before);
        assert!(ledger.drain_changes().is_empty());
    }

    #[test]
    fn add_location_records_the_new_location() {
        let mut ledger = ledger();
        let id = ledger
            .add_location(&draft("Beach shack"), UnitId::SEED_BTC)
            .expect("a new name lands");
        let changes = ledger.drain_changes();
        assert_eq!(changes.len(), 1);
        match &changes[0] {
            LedgerChange::UpsertLocation(location) => {
                assert_eq!(location.id, id);
                assert_eq!(location.name, "Beach shack");
                assert_eq!(location.unit, UnitId::SEED_BTC);
            }
            other => panic!("expected an upsert of the Location, got {other:?}"),
        }
    }

    #[test]
    fn a_location_keeps_its_unit_when_the_units_code_is_renamed() {
        let mut ledger = Ledger::stub_on(today());
        let id = ledger
            .add_location(&draft("Beach shack"), UnitId::SEED_AUD)
            .expect("a new name lands");
        let unit = ledger.inventory().location(id).expect("added").unit;
        assert_eq!(ledger.unit_code(unit), "aud");

        ledger
            .units
            .iter_mut()
            .find(|row| row.id == UnitId::SEED_AUD)
            .expect("the stub has aud")
            .code = "aus".to_string();

        let location = ledger.inventory().location(id).expect("still there");
        assert_eq!(location.unit, UnitId::SEED_AUD);
        assert_eq!(ledger.unit_code(location.unit), "aus");
    }

    #[test]
    fn a_refused_add_location_leaves_the_ledger_untouched() {
        let mut ledger = ledger();
        let before = ledger.inventory().clone();
        let refused = ledger.add_location(&draft("  "), UnitId::SEED_AUD);
        assert_eq!(refused, Err(LocationError::Name(NameError::Empty)));
        assert_untouched(&mut ledger, &before);
    }

    #[test]
    fn edit_location_records_the_saved_location() {
        let mut ledger = ledger();
        let id = ledger.inventory().locations[0].id;
        ledger
            .edit_location(id, &draft("Renamed place"))
            .expect("a free name lands");
        let changes = ledger.drain_changes();
        assert!(matches!(
            changes.as_slice(),
            [LedgerChange::UpsertLocation(p)] if p.id == id && p.name == "Renamed place"
        ));
    }

    #[test]
    fn a_refused_edit_location_leaves_the_ledger_untouched() {
        let mut ledger = ledger();
        let before = ledger.inventory().clone();
        let taken = ledger.inventory().locations[1].name.clone();
        let id = ledger.inventory().locations[0].id;
        assert_eq!(
            ledger.edit_location(id, &draft(&taken)),
            Err(LocationError::Name(NameError::Taken))
        );
        assert_untouched(&mut ledger, &before);
    }

    #[test]
    fn remove_location_records_its_areas_items_and_the_location() {
        let mut ledger = ledger();
        let id = ledger.inventory().locations[0].id;
        let areas: Vec<AreaId> = ledger.inventory().locations[0]
            .areas
            .iter()
            .map(|r| r.id)
            .collect();
        let items: Vec<ItemId> = ledger
            .inventory()
            .items
            .iter()
            .filter(|i| i.location == id)
            .map(|i| i.id)
            .collect();
        assert!(!items.is_empty(), "the seeded Location holds Items");

        let removed = ledger.remove_location(id).expect("the Location exists");
        assert_eq!(removed.areas, areas);
        assert_eq!(removed.items, items);

        let changes = ledger.drain_changes();
        assert!(changes.contains(&LedgerChange::DeleteLocation(id)));
        for area in &areas {
            assert!(changes.contains(&LedgerChange::DeleteArea(*area)));
        }
        for item in &items {
            assert!(changes.contains(&LedgerChange::DeleteItem(*item)));
        }
    }

    #[test]
    fn remove_location_drops_the_document_links_to_its_items() {
        let mut ledger = Ledger::stub_on(today());
        let linked = ledger
            .documents()
            .iter()
            .flat_map(|d| &d.links)
            .find_map(|link| match link {
                DocumentLink::InventoryItem(id) => Some(*id),
                _ => None,
            })
            .expect("the stub links a Document to an Inventory Item");
        let location = ledger
            .inventory()
            .item(linked)
            .expect("the Item exists")
            .location;

        ledger
            .remove_location(location)
            .expect("the Location exists");

        let still_linked = ledger
            .documents()
            .iter()
            .flat_map(|d| &d.links)
            .any(|link| *link == DocumentLink::InventoryItem(linked));
        assert!(!still_linked, "no Link names a removed Item");
        assert!(domains(&ledger.drain_changes()).contains(&Domain::Documents));
    }

    #[test]
    fn a_refused_remove_location_leaves_the_ledger_untouched() {
        let mut ledger = ledger();
        let before = ledger.inventory().clone();
        assert_eq!(
            ledger.remove_location(LocationId::seed(9_999)).unwrap_err(),
            LocationError::Unknown
        );
        assert_untouched(&mut ledger, &before);
    }

    #[test]
    fn add_area_records_the_new_area_under_its_location() {
        let mut ledger = ledger();
        let location = ledger.inventory().locations[0].id;
        let id = ledger
            .add_area(location, "Attic")
            .expect("a free name lands");
        let changes = ledger.drain_changes();
        assert!(matches!(
            changes.as_slice(),
            [LedgerChange::UpsertArea { location: p, area }] if *p == location && area.id == id && area.name == "Attic"
        ));
    }

    #[test]
    fn a_refused_add_area_leaves_the_ledger_untouched() {
        let mut ledger = ledger();
        let before = ledger.inventory().clone();
        let location = ledger.inventory().locations[0].id;
        let taken = ledger.inventory().locations[0].areas[0].name.clone();
        assert_eq!(
            ledger.add_area(location, &taken),
            Err(AreaError::Name(NameError::Taken))
        );
        assert_untouched(&mut ledger, &before);
    }

    #[test]
    fn edit_area_records_the_renamed_area() {
        let mut ledger = ledger();
        let area = ledger.inventory().locations[0].areas[0].id;
        ledger.edit_area(area, "Lounge").expect("a free name lands");
        let changes = ledger.drain_changes();
        assert!(matches!(
            changes.as_slice(),
            [LedgerChange::UpsertArea { area: r, .. }] if r.id == area && r.name == "Lounge"
        ));
    }

    #[test]
    fn a_refused_edit_area_leaves_the_ledger_untouched() {
        let mut ledger = ledger();
        let before = ledger.inventory().clone();
        assert_eq!(
            ledger.edit_area(AreaId::seed(9_999), "Nowhere"),
            Err(AreaError::UnknownArea)
        );
        assert_untouched(&mut ledger, &before);
    }

    #[test]
    fn remove_area_moves_its_items_and_records_each_one() {
        let mut ledger = ledger();
        let (_, area, other) = area_with_items(ledger.inventory());
        let moving: Vec<ItemId> = ledger
            .inventory()
            .items
            .iter()
            .filter(|i| i.area == area)
            .map(|i| i.id)
            .collect();

        let removed = ledger
            .remove_area(area, Some(other))
            .expect("the destination is another Area of the Location");
        assert_eq!(removed.moved_items, moving);

        let changes = ledger.drain_changes();
        for item in &moving {
            assert!(changes.contains(&LedgerChange::UpsertItem(
                ledger.inventory().item(*item).unwrap().clone()
            )));
        }
        assert!(changes.contains(&LedgerChange::DeleteArea(area)));
    }

    #[test]
    fn a_refused_remove_area_leaves_the_ledger_untouched() {
        let mut ledger = ledger();
        let before = ledger.inventory().clone();
        let (_, area, _) = area_with_items(&before);
        assert_eq!(
            ledger.remove_area(area, None),
            Err(AreaError::NeedsDestination)
        );
        assert_untouched(&mut ledger, &before);
    }

    #[test]
    fn move_area_records_both_areas_that_swapped() {
        let mut ledger = ledger();
        let location = &ledger.inventory().locations[0];
        let (first, second) = (location.areas[0].id, location.areas[1].id);

        ledger
            .move_area(first, 1)
            .expect("the first Area can move down");

        let changes = ledger.drain_changes();
        assert_eq!(changes.len(), 2);
        assert!(changes.iter().any(|c| matches!(
            c, LedgerChange::UpsertArea { area, .. } if area.id == first
        )));
        assert!(changes.iter().any(|c| matches!(
            c, LedgerChange::UpsertArea { area, .. } if area.id == second
        )));
    }

    #[test]
    fn a_refused_move_area_leaves_the_ledger_untouched() {
        let mut ledger = ledger();
        let before = ledger.inventory().clone();
        let last = ledger.inventory().locations[0].areas.last().unwrap().id;
        assert_eq!(
            ledger.move_area(last, 1),
            Err(MoveAreaRefusal::NotMoved),
            "the last Area cannot move further down"
        );
        assert_untouched(&mut ledger, &before);
    }
}
