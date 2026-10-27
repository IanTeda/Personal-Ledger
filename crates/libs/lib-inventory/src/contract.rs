//! The contract every stored Inventory port keeps (ADR-0046), run against the in-memory port here
//! and, once Inventory persists, a `lib-database` adapter, so the fake the tests use can be
//! trusted.
//!
//! Each check is a function generic over a [`PortsFactory`]: a consumer runs it from its own test,
//! since an adapter's tests need their own set-up (a fresh database per test). Each writes through
//! the Inventory Interactors, as a Client does, reloads as a restart would, and expects the same
//! rows back, in the same order. Remove Location is `lib-ledger`'s write, not an Interactor.

use chrono::NaiveDate;
use lib_core::{Money, UnitId};

use crate::fake::InMemoryInventory;
use crate::interactor::{
    AddArea, AddItem, AddLocation, EditArea, EditItem, EditLocation, MoveArea, RemoveArea,
    RemoveItem, SetCover,
};
use crate::{
    AreaError, CoverDraft, Inventory, InventoryRepository, ItemDraft, LocationDraft, LocationError,
    NameError, default_inventory,
};

/// Builds the port a contract check runs on.
pub trait PortsFactory {
    type Port: InventoryRepository;

    /// A port holding `seed`'s rows, in order, as if stored and loaded.
    fn create(&mut self, seed: Inventory) -> Self::Port;

    /// Ends the writes made on `port`, then loads a fresh one from wherever they were stored, as a
    /// restart would.
    fn reload(&mut self, port: Self::Port) -> Self::Port;

    /// Ends one write made on `port`, sending its changes to storage as an adapter's owner does
    /// after each input.
    fn commit(&mut self, port: &mut Self::Port);

    /// Makes every write from now on fail, as a store that refuses it would.
    fn break_writes(&mut self, port: &mut Self::Port);

    /// Settles the writes sent so far, then returns `port` reloaded to the last committed rows,
    /// with the number of failures reported naming the Inventory domain.
    fn recover(&mut self, port: Self::Port) -> (Self::Port, usize);
}

/// The in-memory port is its own storage, so a reload hands it back as it is. It keeps a copy as of
/// the last commit, which a failed write falls back to.
#[derive(Debug, Default)]
pub struct InMemoryFactory {
    committed: Option<InMemoryInventory>,
    broken: bool,
    failures: usize,
}

impl PortsFactory for InMemoryFactory {
    type Port = InMemoryInventory;

    fn create(&mut self, seed: Inventory) -> InMemoryInventory {
        let port = InMemoryInventory {
            locations: seed.locations,
            items: seed.items,
        };
        self.committed = Some(port.clone());
        port
    }

    fn reload(&mut self, mut port: InMemoryInventory) -> InMemoryInventory {
        self.commit(&mut port);
        port
    }

    fn commit(&mut self, port: &mut InMemoryInventory) {
        if self.broken {
            self.failures += 1;
        } else {
            self.committed = Some(port.clone());
        }
    }

    fn break_writes(&mut self, _port: &mut InMemoryInventory) {
        self.broken = true;
    }

    fn recover(&mut self, _port: InMemoryInventory) -> (InMemoryInventory, usize) {
        let committed = self.committed.clone().expect("create ran first");
        (committed, std::mem::take(&mut self.failures))
    }
}

fn today() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 10, 2).expect("a real date")
}

fn stub() -> Inventory {
    default_inventory(today())
}

fn dollars(amount: i64) -> Money {
    Money(amount.into())
}

fn draft(name: &str, cover: CoverDraft) -> LocationDraft {
    LocationDraft {
        name: name.into(),
        address: "1 Test St".into(),
        cover,
    }
}

fn cover() -> CoverDraft {
    CoverDraft {
        insurer: "AAMI".into(),
        policy_no: "P-1".into(),
        renews_on: Some(today()),
        sum_insured: Some(dollars(50_000)),
        item_limit: Some(dollars(1_000)),
    }
}

/// A Location added with a cover reloads with its Unit, address and every cover field; one with
/// no insurer reloads with none.
pub fn a_location_with_cover_reloads_the_same<F: PortsFactory>(factory: &mut F) {
    let mut port = factory.create(stub());

    let covered = AddLocation {
        draft: draft("Cabin", cover()),
        unit: UnitId::SEED_BTC,
    }
    .run(&mut port)
    .expect("a new name");
    let bare = AddLocation {
        draft: draft("Shed", CoverDraft::default()),
        unit: UnitId::SEED_AUD,
    }
    .run(&mut port)
    .expect("a new name");
    factory.commit(&mut port);

    let before = port.locations();
    let port = factory.reload(port);
    assert_eq!(port.locations(), before);
    let cabin = port.location(covered).expect("stored");
    assert_eq!(cabin.unit, UnitId::SEED_BTC, "the Unit is held by id");
    assert_eq!(cabin.address, "1 Test St");
    assert!(cabin.areas.is_empty());
    let stored = cabin.cover.expect("the cover is stored");
    assert_eq!(
        (stored.insurer.as_str(), stored.policy_no.as_deref()),
        ("AAMI", Some("P-1"))
    );
    assert_eq!(stored.renews_on, Some(today()));
    assert_eq!(stored.sum_insured, dollars(50_000));
    assert_eq!(stored.item_limit, Some(dollars(1_000)));
    assert_eq!(port.location(bare).expect("stored").cover, None);
}

/// An edit reloads with the details written and the Unit and Areas as they were; setting and
/// clearing the cover reload as written. Editing a Location that isn't there is refused.
pub fn an_edited_location_reloads_the_same<F: PortsFactory>(factory: &mut F) {
    let mut port = factory.create(stub());
    let before = port.locations()[0].clone();

    EditLocation {
        id: before.id,
        draft: LocationDraft {
            name: "Elm".into(),
            ..LocationDraft::from(&before)
        },
    }
    .run(&mut port)
    .expect("a free name");
    SetCover {
        location: port.locations()[2].id,
        cover: cover(),
    }
    .run(&mut port)
    .expect("a valid cover");
    SetCover {
        location: port.locations()[1].id,
        cover: CoverDraft::default(),
    }
    .run(&mut port)
    .expect("clearing is valid");
    assert_eq!(
        EditLocation {
            id: lib_core::LocationId::new(),
            draft: draft("Nowhere", CoverDraft::default()),
        }
        .run(&mut port),
        Err(LocationError::Unknown)
    );
    factory.commit(&mut port);

    let expected = port.locations();
    let port = factory.reload(port);
    assert_eq!(port.locations(), expected);
    let after = port.location(before.id).expect("stored");
    assert_eq!(after.name, "Elm");
    assert_eq!((after.unit, &after.areas), (before.unit, &before.areas));
    assert!(port.locations()[2].cover.is_some());
    assert_eq!(port.locations()[1].cover, None);
}

/// Areas reload in tab order: added ones last, a rename keeping the id, and a move swapping two.
pub fn areas_reload_in_order<F: PortsFactory>(factory: &mut F) {
    let mut port = factory.create(stub());
    let location = port.locations()[2].id;
    let first = AddArea {
        location,
        name: "Roof rack".into(),
    }
    .run(&mut port)
    .expect("a free name");
    let second = AddArea {
        location,
        name: "Bonnet".into(),
    }
    .run(&mut port)
    .expect("a free name");
    EditArea {
        area: second,
        name: "Canopy".into(),
    }
    .run(&mut port)
    .expect("a free name");
    let third = AddArea {
        location,
        name: "Boot".into(),
    }
    .run(&mut port)
    .expect("a free name");
    assert!(
        MoveArea {
            area: third,
            delta: -1
        }
        .run(&mut port)
    );
    assert!(
        !MoveArea {
            area: port.location(location).expect("stored").areas[0].id,
            delta: -1
        }
        .run(&mut port),
        "the first tab can't go further"
    );
    assert_eq!(
        AddArea {
            location,
            name: "ROOF RACK".into()
        }
        .run(&mut port),
        Err(AreaError::Name(NameError::Taken))
    );
    factory.commit(&mut port);

    let port = factory.reload(port);
    let areas = port.location(location).expect("stored").areas;
    let named: Vec<_> = areas
        .iter()
        .filter(|a| [first, second, third].contains(&a.id))
        .map(|a| (a.id, a.name.as_str()))
        .collect();
    assert_eq!(
        named,
        [(first, "Roof rack"), (third, "Boot"), (second, "Canopy")]
    );
}

/// An Item added, edited and removed reloads as written, in the Area it was put in.
pub fn items_reload_the_same<F: PortsFactory>(factory: &mut F) {
    let mut port = factory.create(stub());
    let storage = port.locations()[1].clone();
    let area = storage.areas[0].id;
    let seeded = port.items().len();

    let kept = AddItem {
        location: storage.id,
        area,
        draft: ItemDraft {
            name: " Ladder ".into(),
            replacement: dollars(90),
        },
    }
    .run(&mut port)
    .expect("the Area is the Location's");
    let gone = AddItem {
        location: storage.id,
        area,
        draft: ItemDraft {
            name: "Rake".into(),
            replacement: dollars(20),
        },
    }
    .run(&mut port)
    .expect("the Area is the Location's");
    EditItem {
        id: kept,
        draft: ItemDraft {
            name: "Tall ladder".into(),
            replacement: dollars(120),
        },
    }
    .run(&mut port)
    .expect("the Item is there");
    RemoveItem(gone).run(&mut port).expect("the Item is there");
    factory.commit(&mut port);

    let expected = port.items();
    let port = factory.reload(port);
    assert_eq!(port.items(), expected);
    assert_eq!(port.items().len(), seeded + 1);
    let item = port.item(kept).expect("stored");
    assert_eq!(
        (item.name.as_str(), item.area, item.location),
        ("Tall ladder", area, storage.id)
    );
    assert_eq!(item.replacement, dollars(120));
    assert_eq!(port.item(gone), None);
}

/// Remove Area moves its Items to the destination and drops the Area, and both reload; an Area
/// holding Items with nowhere to put them is refused and writes nothing.
pub fn remove_area_moves_its_items<F: PortsFactory>(factory: &mut F) {
    let mut port = factory.create(stub());
    let elm = port.locations()[0].clone();
    let (from, to) = (elm.areas[0].id, elm.areas[1].id);
    let held: Vec<_> = port
        .items()
        .into_iter()
        .filter(|i| i.area == from)
        .map(|i| i.id)
        .collect();
    let in_to = port.items().iter().filter(|i| i.area == to).count();
    assert!(!held.is_empty());

    assert_eq!(
        RemoveArea {
            area: from,
            destination: None
        }
        .run(&mut port),
        Err(AreaError::NeedsDestination)
    );
    assert_eq!(port.locations()[0], elm, "a refusal writes nothing");

    let removed = RemoveArea {
        area: from,
        destination: Some(to),
    }
    .run(&mut port)
    .expect("a sibling takes them");
    assert_eq!(removed.moved_items, held);
    factory.commit(&mut port);

    let expected = port.inventory();
    let port = factory.reload(port);
    assert_eq!(port.inventory(), expected);
    assert!(
        port.location(elm.id)
            .expect("stored")
            .areas
            .iter()
            .all(|a| a.id != from)
    );
    assert!(
        held.iter()
            .all(|id| port.item(*id).expect("kept").area == to)
    );
    assert_eq!(
        port.items().iter().filter(|i| i.area == to).count(),
        in_to + held.len()
    );
}

/// An empty Area just goes, taking no Item with it.
pub fn an_empty_area_is_removed_alone<F: PortsFactory>(factory: &mut F) {
    let mut port = factory.create(stub());
    let location = port.locations()[2].id;
    let area = AddArea {
        location,
        name: "Spare".into(),
    }
    .run(&mut port)
    .expect("a free name");
    factory.commit(&mut port);
    let items = port.items();

    let removed = RemoveArea {
        area,
        destination: None,
    }
    .run(&mut port)
    .expect("an empty Area needs no destination");
    assert!(removed.moved_items.is_empty());
    factory.commit(&mut port);

    let port = factory.reload(port);
    assert!(
        port.location(location)
            .expect("stored")
            .areas
            .iter()
            .all(|a| a.id != area)
    );
    assert_eq!(port.items(), items);
}

/// A write the store refuses is reported, and the rows fall back to the last committed ones.
pub fn a_failed_write_is_reported_and_rolls_back<F: PortsFactory>(factory: &mut F) {
    let mut port = factory.create(stub());
    let before = port.inventory();
    factory.break_writes(&mut port);

    AddLocation {
        draft: draft("Cabin", CoverDraft::default()),
        unit: UnitId::SEED_AUD,
    }
    .run(&mut port)
    .expect("the rule passes; the store is what refuses");
    factory.commit(&mut port);

    let (port, failures) = factory.recover(port);
    assert_eq!(failures, 1);
    assert_eq!(port.inventory(), before);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn in_memory_a_location_with_cover_reloads_the_same() {
        a_location_with_cover_reloads_the_same(&mut InMemoryFactory::default());
    }

    #[test]
    fn in_memory_an_edited_location_reloads_the_same() {
        an_edited_location_reloads_the_same(&mut InMemoryFactory::default());
    }

    #[test]
    fn in_memory_areas_reload_in_order() {
        areas_reload_in_order(&mut InMemoryFactory::default());
    }

    #[test]
    fn in_memory_items_reload_the_same() {
        items_reload_the_same(&mut InMemoryFactory::default());
    }

    #[test]
    fn in_memory_remove_area_moves_its_items() {
        remove_area_moves_its_items(&mut InMemoryFactory::default());
    }

    #[test]
    fn in_memory_an_empty_area_is_removed_alone() {
        an_empty_area_is_removed_alone(&mut InMemoryFactory::default());
    }

    #[test]
    fn in_memory_a_failed_write_is_reported_and_rolls_back() {
        a_failed_write_is_reported_and_rolls_back(&mut InMemoryFactory::default());
    }
}
