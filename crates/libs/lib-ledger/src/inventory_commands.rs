//! The Inventory writes (ADR-0038): each a command value with its own refusal, checked before
//! anything is written so a refusal leaves the Ledger untouched, and recording a change per row it
//! wrote (ADR-0039).
//!
//! Single-entity rules stay in `lib_inventory`. What lives here is the rule that crosses domains: a
//! removed Property drops the Document Links to its Items, since a Document names an Item by id.

use lib_core::{ItemId, PropertyId, RoomId};
use lib_inventory::{PropertyDraft, PropertyError, RemovedProperty, RemovedRoom, RoomError};

use crate::{Ledger, LedgerChange, LedgerCommand};

/// Why a Room did not move: it is unknown, or already at the end it was asked to move past.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum MoveRoomRefusal {
    #[error("room did not move")]
    NotMoved,
}

impl Ledger {
    /// Adds a Property with no Rooms and returns its id.
    pub fn add_property(
        &mut self,
        draft: &PropertyDraft,
        unit: &str,
    ) -> Result<PropertyId, PropertyError> {
        let id = self.inventory.add_property(draft, unit)?;
        self.record_property(id);
        Ok(id)
    }

    /// Edits a Property's name, address and cover. Its Unit is fixed once added.
    pub fn edit_property(
        &mut self,
        id: PropertyId,
        draft: &PropertyDraft,
    ) -> Result<(), PropertyError> {
        self.inventory.edit_property(id, draft)?;
        self.record_property(id);
        Ok(())
    }

    /// Removes a Property with its Rooms and Items, and drops the Document Links to those Items.
    pub fn remove_property(&mut self, id: PropertyId) -> Result<RemovedProperty, PropertyError> {
        let removed = self.inventory.remove_property(id)?;
        for room in &removed.rooms {
            self.pending.push(LedgerChange::DeleteRoom(*room));
        }
        for item in &removed.items {
            self.pending.push(LedgerChange::DeleteItem(*item));
        }
        self.pending.push(LedgerChange::DeleteProperty(id));
        self.drop_document_inventory_links(&removed.items);
        Ok(removed)
    }

    /// Adds a Room at the end of a Property's tabs and returns its id.
    pub fn add_room(&mut self, property: PropertyId, name: &str) -> Result<RoomId, RoomError> {
        let id = self.inventory.add_room(property, name)?;
        self.record_room(id);
        Ok(id)
    }

    /// Renames a Room.
    pub fn edit_room(&mut self, room: RoomId, name: &str) -> Result<(), RoomError> {
        self.inventory.edit_room(room, name)?;
        self.record_room(room);
        Ok(())
    }

    /// Removes a Room, moving its Items to `destination` first when it holds any.
    pub fn remove_room(
        &mut self,
        room: RoomId,
        destination: Option<RoomId>,
    ) -> Result<RemovedRoom, RoomError> {
        let removed = self.inventory.remove_room(room, destination)?;
        for item in &removed.moved_items {
            self.record_item(*item);
        }
        self.pending.push(LedgerChange::DeleteRoom(room));
        Ok(removed)
    }

    /// Moves a Room one place down (`delta` > 0) or up within its Property. The Room and the
    /// neighbour it swapped with both change position.
    pub fn move_room(&mut self, room: RoomId, delta: isize) -> Result<(), MoveRoomRefusal> {
        let [moved, neighbour] = self
            .inventory
            .move_room(room, delta)
            .ok_or(MoveRoomRefusal::NotMoved)?;
        self.record_room(moved);
        self.record_room(neighbour);
        Ok(())
    }

    fn record_property(&mut self, id: PropertyId) {
        if let Some(property) = self.inventory.property(id) {
            self.pending
                .push(LedgerChange::UpsertProperty(property.clone()));
        }
    }

    fn record_room(&mut self, room: RoomId) {
        if let Some((property, room)) = self.inventory.room(room) {
            self.pending.push(LedgerChange::UpsertRoom {
                property: property.id,
                room: room.clone(),
            });
        }
    }

    fn record_item(&mut self, item: ItemId) {
        if let Some(item) = self.inventory.item(item) {
            self.pending.push(LedgerChange::UpsertItem(item.clone()));
        }
    }
}

/// [`Ledger::add_property`] as a value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddProperty {
    pub draft: PropertyDraft,
    pub unit: String,
}

impl LedgerCommand for AddProperty {
    type Outcome = PropertyId;
    type Refusal = PropertyError;
    fn run(self, ledger: &mut Ledger) -> Result<PropertyId, PropertyError> {
        ledger.add_property(&self.draft, &self.unit)
    }
}

/// [`Ledger::edit_property`] as a value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditProperty {
    pub id: PropertyId,
    pub draft: PropertyDraft,
}

impl LedgerCommand for EditProperty {
    type Outcome = ();
    type Refusal = PropertyError;
    fn run(self, ledger: &mut Ledger) -> Result<(), PropertyError> {
        ledger.edit_property(self.id, &self.draft)
    }
}

/// [`Ledger::remove_property`] as a value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RemoveProperty(pub PropertyId);

impl LedgerCommand for RemoveProperty {
    type Outcome = RemovedProperty;
    type Refusal = PropertyError;
    fn run(self, ledger: &mut Ledger) -> Result<RemovedProperty, PropertyError> {
        ledger.remove_property(self.0)
    }
}

/// [`Ledger::add_room`] as a value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddRoom {
    pub property: PropertyId,
    pub name: String,
}

impl LedgerCommand for AddRoom {
    type Outcome = RoomId;
    type Refusal = RoomError;
    fn run(self, ledger: &mut Ledger) -> Result<RoomId, RoomError> {
        ledger.add_room(self.property, &self.name)
    }
}

/// [`Ledger::edit_room`] as a value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditRoom {
    pub room: RoomId,
    pub name: String,
}

impl LedgerCommand for EditRoom {
    type Outcome = ();
    type Refusal = RoomError;
    fn run(self, ledger: &mut Ledger) -> Result<(), RoomError> {
        ledger.edit_room(self.room, &self.name)
    }
}

/// [`Ledger::remove_room`] as a value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RemoveRoom {
    pub room: RoomId,
    pub destination: Option<RoomId>,
}

impl LedgerCommand for RemoveRoom {
    type Outcome = RemovedRoom;
    type Refusal = RoomError;
    fn run(self, ledger: &mut Ledger) -> Result<RemovedRoom, RoomError> {
        ledger.remove_room(self.room, self.destination)
    }
}

/// [`Ledger::move_room`] as a value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MoveRoom {
    pub room: RoomId,
    pub delta: isize,
}

impl LedgerCommand for MoveRoom {
    type Outcome = ();
    type Refusal = MoveRoomRefusal;
    fn run(self, ledger: &mut Ledger) -> Result<(), MoveRoomRefusal> {
        ledger.move_room(self.room, self.delta)
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

    fn draft(name: &str) -> PropertyDraft {
        PropertyDraft {
            name: name.to_string(),
            ..PropertyDraft::default()
        }
    }

    /// A Room in the seed that holds Items, with another Room of the same Property to receive
    /// them, and the Property both belong to.
    fn room_with_items(inventory: &Inventory) -> (PropertyId, RoomId, RoomId) {
        let item = inventory.items.first().expect("seeded Items");
        let (property, room) = inventory.room(item.room).expect("an Item's Room exists");
        let other = property
            .rooms
            .iter()
            .find(|r| r.id != room.id)
            .expect("the seed's Property has a second Room")
            .id;
        (property.id, room.id, other)
    }

    /// The Ledger must come out of a refused command exactly as it went in.
    fn assert_untouched(ledger: &mut Ledger, before: &Inventory) {
        assert_eq!(ledger.inventory(), before);
        assert!(ledger.drain_changes().is_empty());
    }

    #[test]
    fn add_property_records_the_new_property() {
        let mut ledger = ledger();
        let id = ledger
            .add_property(&draft("Beach shack"), "nzd")
            .expect("a new name lands");
        let changes = ledger.drain_changes();
        assert_eq!(changes.len(), 1);
        match &changes[0] {
            LedgerChange::UpsertProperty(property) => {
                assert_eq!(property.id, id);
                assert_eq!(property.name, "Beach shack");
                assert_eq!(property.unit, "nzd");
            }
            other => panic!("expected an upsert of the Property, got {other:?}"),
        }
    }

    #[test]
    fn a_refused_add_property_leaves_the_ledger_untouched() {
        let mut ledger = ledger();
        let before = ledger.inventory().clone();
        let refused = ledger.add_property(&draft("  "), "aud");
        assert_eq!(refused, Err(PropertyError::Name(NameError::Empty)));
        assert_untouched(&mut ledger, &before);
    }

    #[test]
    fn edit_property_records_the_saved_property() {
        let mut ledger = ledger();
        let id = ledger.inventory().properties[0].id;
        ledger
            .edit_property(id, &draft("Renamed place"))
            .expect("a free name lands");
        let changes = ledger.drain_changes();
        assert!(matches!(
            changes.as_slice(),
            [LedgerChange::UpsertProperty(p)] if p.id == id && p.name == "Renamed place"
        ));
    }

    #[test]
    fn a_refused_edit_property_leaves_the_ledger_untouched() {
        let mut ledger = ledger();
        let before = ledger.inventory().clone();
        let taken = ledger.inventory().properties[1].name.clone();
        let id = ledger.inventory().properties[0].id;
        assert_eq!(
            ledger.edit_property(id, &draft(&taken)),
            Err(PropertyError::Name(NameError::Taken))
        );
        assert_untouched(&mut ledger, &before);
    }

    #[test]
    fn remove_property_records_its_rooms_items_and_the_property() {
        let mut ledger = ledger();
        let id = ledger.inventory().properties[0].id;
        let rooms: Vec<RoomId> = ledger.inventory().properties[0]
            .rooms
            .iter()
            .map(|r| r.id)
            .collect();
        let items: Vec<ItemId> = ledger
            .inventory()
            .items
            .iter()
            .filter(|i| i.property == id)
            .map(|i| i.id)
            .collect();
        assert!(!items.is_empty(), "the seeded Property holds Items");

        let removed = ledger.remove_property(id).expect("the Property exists");
        assert_eq!(removed.rooms, rooms);
        assert_eq!(removed.items, items);

        let changes = ledger.drain_changes();
        assert!(changes.contains(&LedgerChange::DeleteProperty(id)));
        for room in &rooms {
            assert!(changes.contains(&LedgerChange::DeleteRoom(*room)));
        }
        for item in &items {
            assert!(changes.contains(&LedgerChange::DeleteItem(*item)));
        }
    }

    #[test]
    fn remove_property_drops_the_document_links_to_its_items() {
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
        let property = ledger
            .inventory()
            .item(linked)
            .expect("the Item exists")
            .property;

        ledger
            .remove_property(property)
            .expect("the Property exists");

        let still_linked = ledger
            .documents()
            .iter()
            .flat_map(|d| &d.links)
            .any(|link| *link == DocumentLink::InventoryItem(linked));
        assert!(!still_linked, "no Link names a removed Item");
        assert!(domains(&ledger.drain_changes()).contains(&Domain::Documents));
    }

    #[test]
    fn a_refused_remove_property_leaves_the_ledger_untouched() {
        let mut ledger = ledger();
        let before = ledger.inventory().clone();
        assert_eq!(
            ledger.remove_property(PropertyId::seed(9_999)).unwrap_err(),
            PropertyError::Unknown
        );
        assert_untouched(&mut ledger, &before);
    }

    #[test]
    fn add_room_records_the_new_room_under_its_property() {
        let mut ledger = ledger();
        let property = ledger.inventory().properties[0].id;
        let id = ledger
            .add_room(property, "Attic")
            .expect("a free name lands");
        let changes = ledger.drain_changes();
        assert!(matches!(
            changes.as_slice(),
            [LedgerChange::UpsertRoom { property: p, room }] if *p == property && room.id == id && room.name == "Attic"
        ));
    }

    #[test]
    fn a_refused_add_room_leaves_the_ledger_untouched() {
        let mut ledger = ledger();
        let before = ledger.inventory().clone();
        let property = ledger.inventory().properties[0].id;
        let taken = ledger.inventory().properties[0].rooms[0].name.clone();
        assert_eq!(
            ledger.add_room(property, &taken),
            Err(RoomError::Name(NameError::Taken))
        );
        assert_untouched(&mut ledger, &before);
    }

    #[test]
    fn edit_room_records_the_renamed_room() {
        let mut ledger = ledger();
        let room = ledger.inventory().properties[0].rooms[0].id;
        ledger.edit_room(room, "Lounge").expect("a free name lands");
        let changes = ledger.drain_changes();
        assert!(matches!(
            changes.as_slice(),
            [LedgerChange::UpsertRoom { room: r, .. }] if r.id == room && r.name == "Lounge"
        ));
    }

    #[test]
    fn a_refused_edit_room_leaves_the_ledger_untouched() {
        let mut ledger = ledger();
        let before = ledger.inventory().clone();
        assert_eq!(
            ledger.edit_room(RoomId::seed(9_999), "Nowhere"),
            Err(RoomError::UnknownRoom)
        );
        assert_untouched(&mut ledger, &before);
    }

    #[test]
    fn remove_room_moves_its_items_and_records_each_one() {
        let mut ledger = ledger();
        let (_, room, other) = room_with_items(ledger.inventory());
        let moving: Vec<ItemId> = ledger
            .inventory()
            .items
            .iter()
            .filter(|i| i.room == room)
            .map(|i| i.id)
            .collect();

        let removed = ledger
            .remove_room(room, Some(other))
            .expect("the destination is another Room of the Property");
        assert_eq!(removed.moved_items, moving);

        let changes = ledger.drain_changes();
        for item in &moving {
            assert!(changes.contains(&LedgerChange::UpsertItem(
                ledger.inventory().item(*item).unwrap().clone()
            )));
        }
        assert!(changes.contains(&LedgerChange::DeleteRoom(room)));
    }

    #[test]
    fn a_refused_remove_room_leaves_the_ledger_untouched() {
        let mut ledger = ledger();
        let before = ledger.inventory().clone();
        let (_, room, _) = room_with_items(&before);
        assert_eq!(
            ledger.remove_room(room, None),
            Err(RoomError::NeedsDestination)
        );
        assert_untouched(&mut ledger, &before);
    }

    #[test]
    fn move_room_records_both_rooms_that_swapped() {
        let mut ledger = ledger();
        let property = &ledger.inventory().properties[0];
        let (first, second) = (property.rooms[0].id, property.rooms[1].id);

        ledger
            .move_room(first, 1)
            .expect("the first Room can move down");

        let changes = ledger.drain_changes();
        assert_eq!(changes.len(), 2);
        assert!(changes.iter().any(|c| matches!(
            c, LedgerChange::UpsertRoom { room, .. } if room.id == first
        )));
        assert!(changes.iter().any(|c| matches!(
            c, LedgerChange::UpsertRoom { room, .. } if room.id == second
        )));
    }

    #[test]
    fn a_refused_move_room_leaves_the_ledger_untouched() {
        let mut ledger = ledger();
        let before = ledger.inventory().clone();
        let last = ledger.inventory().properties[0].rooms.last().unwrap().id;
        assert_eq!(
            ledger.move_room(last, 1),
            Err(MoveRoomRefusal::NotMoved),
            "the last Room cannot move further down"
        );
        assert_untouched(&mut ledger, &before);
    }
}
