//! The Ledger's Inventory: Properties, their Rooms and the Items in them (`CONTEXT.md`'s
//! Property, Room and Inventory Item). `gpui`-free and side-effect free, so every rule is
//! unit-tested without a window.
//!
//! Documents depends on this module (a Document Link names an Item by id), never the reverse, so
//! removing a Property reports the Items that went and leaves the Documents to drop their Links.
//! Amounts are in the owning Property's Unit, at two decimal places like the Transaction stubs.

pub(crate) mod form;

use bigdecimal::BigDecimal;
use chrono::NaiveDate;
use lib_core::Money;

/// A Property's insurance cover: one of its details, not a record of its own.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cover {
    pub insurer: String,
    pub policy_no: Option<String>,
    /// Display only: a past date changes nothing.
    pub renews_on: Option<NaiveDate>,
    pub sum_insured: Money,
    pub item_limit: Option<Money>,
}

/// A named area of one Property. `id` is stable across renames, so an Item never moves on one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Room {
    pub id: u32,
    pub name: String,
}

/// A place whose contents are kept as one register. A Room's position in `rooms` is its room-tab
/// (and `J`/`K`) order; persistence adds the column later.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Property {
    pub id: u32,
    pub name: String,
    pub address: String,
    /// The fiat Unit's code, fixed when the Property is added.
    pub unit: String,
    pub cover: Option<Cover>,
    pub rooms: Vec<Room>,
}

/// A physical possession, in one Room of one Property.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Item {
    pub id: u32,
    pub property: u32,
    pub room: u32,
    pub name: String,
    /// In the Property's Unit.
    pub replacement: Money,
}

/// Every Property and Item. The Items sit in one list so Documents can resolve a Link by id.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Inventory {
    pub properties: Vec<Property>,
    pub items: Vec<Item>,
}

// ---------------------------------------------------------------------------------------------
// Lookups and derived queries
// ---------------------------------------------------------------------------------------------

impl Inventory {
    pub fn property(&self, id: u32) -> Option<&Property> {
        self.properties.iter().find(|p| p.id == id)
    }

    pub fn item(&self, id: u32) -> Option<&Item> {
        self.items.iter().find(|i| i.id == id)
    }

    /// The Room and the Property holding it.
    pub fn room(&self, id: u32) -> Option<(&Property, &Room)> {
        self.properties
            .iter()
            .find_map(|p| p.rooms.iter().find(|r| r.id == id).map(|r| (p, r)))
    }

    pub fn property_count(&self) -> usize {
        self.properties.len()
    }

    pub fn room_count(&self) -> usize {
        self.properties.iter().map(|p| p.rooms.len()).sum()
    }

    pub fn item_count(&self) -> usize {
        self.items.len()
    }

    /// Items in a Room; an empty or unknown Room is 0.
    pub fn room_items(&self, room: u32) -> usize {
        self.items.iter().filter(|i| i.room == room).count()
    }

    /// The replacement value of a Room's Items; an empty or unknown Room is zero.
    pub fn room_value(&self, room: u32) -> Money {
        sum(self.items.iter().filter(|i| i.room == room))
    }

    pub fn property_items(&self, property: u32) -> usize {
        self.items.iter().filter(|i| i.property == property).count()
    }

    pub fn property_value(&self, property: u32) -> Money {
        sum(self.items.iter().filter(|i| i.property == property))
    }

    fn next_property_id(&self) -> u32 {
        self.properties.iter().map(|p| p.id).max().unwrap_or(0) + 1
    }

    fn next_room_id(&self) -> u32 {
        self.properties
            .iter()
            .flat_map(|p| &p.rooms)
            .map(|r| r.id)
            .max()
            .unwrap_or(0)
            + 1
    }
}

fn sum<'a>(items: impl Iterator<Item = &'a Item>) -> Money {
    Money(items.map(|i| i.replacement.0.clone()).sum())
}

fn zero() -> Money {
    Money(BigDecimal::from(0))
}

// ---------------------------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------------------------

/// Why a Property or Room name is refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NameError {
    Empty,
    /// Another Property (or another Room of the same Property) has it, ignoring case.
    Taken,
}

/// Why a Property's cover is refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoverError {
    /// An insurer is set but no sum insured.
    SumMissing,
    SumNotPositive,
    LimitNotPositive,
    LimitAboveSum,
}

/// The cover as the dialog holds it: an empty insurer means no cover, whatever else is filled in.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CoverDraft {
    pub insurer: String,
    pub policy_no: String,
    pub renews_on: Option<NaiveDate>,
    pub sum_insured: Option<Money>,
    pub item_limit: Option<Money>,
}

impl CoverDraft {
    /// `Ok(None)` when there is no insurer, which clears all five fields.
    pub fn validate(&self) -> Result<Option<Cover>, CoverError> {
        let insurer = self.insurer.trim();
        if insurer.is_empty() {
            return Ok(None);
        }
        let sum_insured = self.sum_insured.clone().ok_or(CoverError::SumMissing)?;
        if sum_insured.0 <= 0 {
            return Err(CoverError::SumNotPositive);
        }
        if let Some(limit) = &self.item_limit {
            if limit.0 <= 0 {
                return Err(CoverError::LimitNotPositive);
            }
            if limit.0 > sum_insured.0 {
                return Err(CoverError::LimitAboveSum);
            }
        }
        let policy_no = self.policy_no.trim();
        Ok(Some(Cover {
            insurer: insurer.to_string(),
            policy_no: (!policy_no.is_empty()).then(|| policy_no.to_string()),
            renews_on: self.renews_on,
            sum_insured,
            item_limit: self.item_limit.clone(),
        }))
    }
}

/// The editable details of a Property: everything but its Unit, which is fixed on Add.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PropertyDraft {
    pub name: String,
    pub address: String,
    pub cover: CoverDraft,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PropertyError {
    Unknown,
    Name(NameError),
    Cover(CoverError),
    /// Add needs a Unit.
    UnitMissing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoomError {
    UnknownProperty,
    UnknownRoom,
    Name(NameError),
    /// The Room holds Items and another Room of the Property must take them.
    NeedsDestination,
    /// The destination is the Room itself, or not a Room of the same Property.
    BadDestination,
    /// The Room holds Items and is the Property's only Room, so there is nowhere to move them.
    LastRoomHoldsItems,
}

fn taken<'a>(mut names: impl Iterator<Item = &'a str>, name: &str) -> bool {
    names.any(|n| n.to_lowercase() == name.to_lowercase())
}

// ---------------------------------------------------------------------------------------------
// Property mutations
// ---------------------------------------------------------------------------------------------

/// Adds a Property with no Rooms and returns its id.
pub fn add_property(
    inventory: &mut Inventory,
    draft: &PropertyDraft,
    unit: &str,
) -> Result<u32, PropertyError> {
    if unit.trim().is_empty() {
        return Err(PropertyError::UnitMissing);
    }
    let (name, cover) = checked(inventory, None, draft)?;
    let id = inventory.next_property_id();
    inventory.properties.push(Property {
        id,
        name,
        address: draft.address.trim().to_string(),
        unit: unit.to_string(),
        cover,
        rooms: Vec::new(),
    });
    Ok(id)
}

/// Edits everything but the Unit.
pub fn edit_property(
    inventory: &mut Inventory,
    id: u32,
    draft: &PropertyDraft,
) -> Result<(), PropertyError> {
    let (name, cover) = checked(inventory, Some(id), draft)?;
    let property = inventory
        .properties
        .iter_mut()
        .find(|p| p.id == id)
        .ok_or(PropertyError::Unknown)?;
    property.name = name;
    property.address = draft.address.trim().to_string();
    property.cover = cover;
    Ok(())
}

fn checked(
    inventory: &Inventory,
    own_id: Option<u32>,
    draft: &PropertyDraft,
) -> Result<(String, Option<Cover>), PropertyError> {
    if let Some(id) = own_id
        && inventory.property(id).is_none()
    {
        return Err(PropertyError::Unknown);
    }
    let name = draft.name.trim();
    if name.is_empty() {
        return Err(PropertyError::Name(NameError::Empty));
    }
    let others = inventory
        .properties
        .iter()
        .filter(|p| Some(p.id) != own_id)
        .map(|p| p.name.as_str());
    if taken(others, name) {
        return Err(PropertyError::Name(NameError::Taken));
    }
    let cover = draft.cover.validate().map_err(PropertyError::Cover)?;
    Ok((name.to_string(), cover))
}

/// What removing a Property took with it, for the Documents surface to drop the Links.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemovedProperty {
    /// The ids of the Items that went with it.
    pub items: Vec<u32>,
}

/// Removes a Property, its Rooms and its Items. The last Property may go.
pub fn remove_property(
    inventory: &mut Inventory,
    id: u32,
) -> Result<RemovedProperty, PropertyError> {
    let at = inventory
        .properties
        .iter()
        .position(|p| p.id == id)
        .ok_or(PropertyError::Unknown)?;
    inventory.properties.remove(at);
    let items = inventory
        .items
        .iter()
        .filter(|i| i.property == id)
        .map(|i| i.id)
        .collect();
    inventory.items.retain(|i| i.property != id);
    Ok(RemovedProperty { items })
}

// ---------------------------------------------------------------------------------------------
// Room mutations
// ---------------------------------------------------------------------------------------------

fn checked_room_name(
    property: &Property,
    own_id: Option<u32>,
    name: &str,
) -> Result<String, RoomError> {
    let name = name.trim();
    if name.is_empty() {
        return Err(RoomError::Name(NameError::Empty));
    }
    let others = property
        .rooms
        .iter()
        .filter(|r| Some(r.id) != own_id)
        .map(|r| r.name.as_str());
    if taken(others, name) {
        return Err(RoomError::Name(NameError::Taken));
    }
    Ok(name.to_string())
}

/// Adds a Room at the end of the Property's tabs and returns its id.
pub fn add_room(inventory: &mut Inventory, property: u32, name: &str) -> Result<u32, RoomError> {
    let id = inventory.next_room_id();
    let property = inventory
        .properties
        .iter_mut()
        .find(|p| p.id == property)
        .ok_or(RoomError::UnknownProperty)?;
    let name = checked_room_name(property, None, name)?;
    property.rooms.push(Room { id, name });
    Ok(id)
}

/// Renames a Room. Nothing else about a Room is editable: it never changes Property.
pub fn edit_room(inventory: &mut Inventory, room: u32, name: &str) -> Result<(), RoomError> {
    let property = inventory
        .properties
        .iter_mut()
        .find(|p| p.rooms.iter().any(|r| r.id == room))
        .ok_or(RoomError::UnknownRoom)?;
    let name = checked_room_name(property, Some(room), name)?;
    if let Some(r) = property.rooms.iter_mut().find(|r| r.id == room) {
        r.name = name;
    }
    Ok(())
}

/// The Room the remove dialog pre-selects: the one above, or the one below when it is first.
pub fn default_destination(inventory: &Inventory, room: u32) -> Option<u32> {
    let (property, _) = inventory.room(room)?;
    let at = property.rooms.iter().position(|r| r.id == room)?;
    let other = at.checked_sub(1).unwrap_or(1);
    property.rooms.get(other).map(|r| r.id)
}

/// Removes a Room. An empty one just goes; one with Items moves them to `destination`, which must
/// be another Room of the same Property.
pub fn remove_room(
    inventory: &mut Inventory,
    room: u32,
    destination: Option<u32>,
) -> Result<(), RoomError> {
    let (property, _) = inventory.room(room).ok_or(RoomError::UnknownRoom)?;
    let property_id = property.id;
    if inventory.room_items(room) > 0 {
        if property.rooms.len() == 1 {
            return Err(RoomError::LastRoomHoldsItems);
        }
        let to = destination.ok_or(RoomError::NeedsDestination)?;
        if to == room || !property.rooms.iter().any(|r| r.id == to) {
            return Err(RoomError::BadDestination);
        }
        for item in inventory.items.iter_mut().filter(|i| i.room == room) {
            item.room = to;
        }
    }
    if let Some(p) = inventory
        .properties
        .iter_mut()
        .find(|p| p.id == property_id)
    {
        p.rooms.retain(|r| r.id != room);
    }
    Ok(())
}

/// Moves a Room down (`delta` > 0) or up within its own Property, stopping at the ends. Returns
/// whether it moved.
pub fn move_room(inventory: &mut Inventory, room: u32, delta: isize) -> bool {
    let Some(property) = inventory
        .properties
        .iter_mut()
        .find(|p| p.rooms.iter().any(|r| r.id == room))
    else {
        return false;
    };
    let Some(from) = property.rooms.iter().position(|r| r.id == room) else {
        return false;
    };
    let Some(to) = from
        .checked_add_signed(delta)
        .filter(|&to| to < property.rooms.len())
    else {
        return false;
    };
    property.rooms.swap(from, to);
    from != to
}

// ---------------------------------------------------------------------------------------------
// Seed
// ---------------------------------------------------------------------------------------------

/// One seeded Room: its 16q count and total in whole dollars, and the hand-written Items in it
/// (`(id, name, dollars)`). The filler makes up the rest of both, exactly.
struct RoomSeed {
    name: &'static str,
    count: u32,
    total: i64,
    named: &'static [(u32, &'static str, i64)],
}

/// Filler Items take ids from here, clear of the hand-written ones.
const FILLER_ID_START: u32 = 100;

const ELM_ST: &[RoomSeed] = &[
    RoomSeed {
        name: "Living",
        count: 38,
        total: 24_150,
        named: &[
            (5, "Sonos Era 100", 449),
            (7, "Samsung 65\" OLED TV", 2_400),
            (8, "Three-seat sofa", 3_200),
        ],
    },
    RoomSeed {
        name: "Kitchen",
        count: 46,
        total: 22_310,
        named: &[
            (3, "Fridge", 2_800),
            (9, "Dishwasher", 1_100),
            (10, "Induction cooktop", 1_900),
        ],
    },
    RoomSeed {
        name: "Bedroom 1",
        count: 24,
        total: 18_420,
        named: &[
            (4, "Engagement ring", 6_500),
            (11, "King bed and mattress", 3_600),
        ],
    },
    RoomSeed {
        name: "Bedroom 2",
        count: 16,
        total: 8_060,
        named: &[(12, "Queen bed and mattress", 2_400)],
    },
    RoomSeed {
        name: "Bedroom 3",
        count: 12,
        total: 5_540,
        named: &[(13, "Single bed and mattress", 1_200)],
    },
    RoomSeed {
        name: "Office",
        count: 19,
        total: 15_240,
        named: &[
            (2, "Sony A7 IV", 4_380),
            (14, "MacBook Pro 16\"", 3_900),
            (15, "Standing desk", 1_100),
        ],
    },
    RoomSeed {
        name: "Garage",
        count: 27,
        total: 15_060,
        named: &[
            (6, "Toyota Corolla", 8_500),
            (16, "Cordless tool set", 1_250),
        ],
    },
    RoomSeed {
        name: "Laundry",
        count: 11,
        total: 3_120,
        named: &[(17, "Washing machine", 1_150), (18, "Dryer", 900)],
    },
    RoomSeed {
        name: "Bathroom",
        count: 9,
        total: 1_880,
        named: &[(19, "Heated towel rail", 420)],
    },
    RoomSeed {
        name: "Outdoor & shed",
        count: 12,
        total: 49_700,
        named: &[
            (20, "Ride-on mower", 4_200),
            (21, "Garden shed contents", 9_500),
            (22, "Outdoor dining setting", 3_800),
        ],
    },
];

const STORAGE_UNIT: &[RoomSeed] = &[RoomSeed {
    name: "Storage",
    count: 18,
    total: 6_200,
    named: &[
        (23, "Boxed winter wardrobe", 900),
        (24, "Spare mattress", 450),
    ],
}];

fn dollars(amount: i64) -> Money {
    Money(BigDecimal::new(amount.saturating_mul(100).into(), 2))
}

/// `count - named` filler amounts in whole dollars that sum to `remaining`, none below 1: a
/// deterministic 1–5 weighting so the values vary without any randomness.
fn filler_amounts(count: usize, remaining: i64) -> Vec<i64> {
    let weights: Vec<i64> = (0..count).map(|k| 1 + (k as i64 * 7) % 5).collect();
    let total_weight: i64 = weights.iter().sum();
    let mut amounts: Vec<i64> = weights
        .iter()
        .map(|w| (remaining * w / total_weight).max(1))
        .collect();
    let drift = remaining - amounts.iter().sum::<i64>();
    if let Some(last) = amounts.last_mut() {
        *last += drift;
    }
    amounts
}

fn seed_property(
    inventory: &mut Inventory,
    next_filler_id: &mut u32,
    name: &str,
    address: &str,
    cover: Cover,
    rooms: &[RoomSeed],
) {
    let property_id = inventory.next_property_id();
    let mut property = Property {
        id: property_id,
        name: name.to_string(),
        address: address.to_string(),
        unit: "aud".to_string(),
        cover: Some(cover),
        rooms: Vec::new(),
    };
    for seed in rooms {
        let room_id =
            inventory.next_room_id() + u32::try_from(property.rooms.len()).unwrap_or_default();
        property.rooms.push(Room {
            id: room_id,
            name: seed.name.to_string(),
        });
        let named_total: i64 = seed.named.iter().map(|(_, _, d)| d).sum();
        for (id, item, amount) in seed.named {
            inventory.items.push(Item {
                id: *id,
                property: property_id,
                room: room_id,
                name: (*item).to_string(),
                replacement: dollars(*amount),
            });
        }
        let filler = usize::try_from(seed.count)
            .unwrap_or_default()
            .saturating_sub(seed.named.len());
        for (k, amount) in filler_amounts(filler, seed.total - named_total)
            .into_iter()
            .enumerate()
        {
            inventory.items.push(Item {
                id: *next_filler_id,
                property: property_id,
                room: room_id,
                name: format!("{} item {}", seed.name, k + 1),
                replacement: dollars(amount),
            });
            *next_filler_id += 1;
        }
    }
    // The rooms were not in `inventory` yet when their ids were allocated, so push them now.
    inventory.properties.push(property);
}

/// The two stub Properties (`16q`'s 12 Elm St contents and Storage unit) with their 11 Rooms and
/// 232 Items. 12 Elm St holds 214 Items worth 163,480; the Storage unit 18, worth 6,200, under
/// its own copy of the NRMA policy number.
pub fn default_inventory(today: NaiveDate) -> Inventory {
    let mut inventory = Inventory::default();
    let mut next_filler_id = FILLER_ID_START;
    let renews = today + chrono::Duration::days(45);
    seed_property(
        &mut inventory,
        &mut next_filler_id,
        "12 Elm St contents",
        "12 Elm St, Ainslie ACT",
        Cover {
            insurer: "NRMA".to_string(),
            policy_no: Some("HC-4471902".to_string()),
            renews_on: Some(renews),
            sum_insured: dollars(150_000),
            item_limit: Some(dollars(2_000)),
        },
        ELM_ST,
    );
    seed_property(
        &mut inventory,
        &mut next_filler_id,
        "Storage unit",
        "Kennards, Unit 114 \u{b7} Mitchell ACT",
        Cover {
            insurer: "NRMA".to_string(),
            policy_no: Some("HC-4471902".to_string()),
            renews_on: Some(renews),
            sum_insured: dollars(20_000),
            item_limit: Some(dollars(1_000)),
        },
        STORAGE_UNIT,
    );
    inventory
}

#[cfg(test)]
mod tests {
    use super::*;

    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 10, 2).unwrap()
    }

    fn seed() -> Inventory {
        default_inventory(today())
    }

    fn room_id(inv: &Inventory, property: &str, room: &str) -> u32 {
        inv.properties
            .iter()
            .find(|p| p.name == property)
            .and_then(|p| p.rooms.iter().find(|r| r.name == room))
            .unwrap()
            .id
    }

    fn property_id(inv: &Inventory, name: &str) -> u32 {
        inv.properties.iter().find(|p| p.name == name).unwrap().id
    }

    fn draft(name: &str) -> PropertyDraft {
        PropertyDraft {
            name: name.to_string(),
            ..PropertyDraft::default()
        }
    }

    fn cover(insurer: &str, sum: Option<i64>, limit: Option<i64>) -> CoverDraft {
        CoverDraft {
            insurer: insurer.to_string(),
            sum_insured: sum.map(dollars),
            item_limit: limit.map(dollars),
            ..CoverDraft::default()
        }
    }

    // --- seed -------------------------------------------------------------------------------

    #[test]
    fn seed_holds_232_items_across_two_properties_and_eleven_rooms() {
        let inv = seed();
        assert_eq!(inv.property_count(), 2);
        assert_eq!(inv.room_count(), 11);
        assert_eq!(inv.item_count(), 232);
        assert_eq!(
            inv.property_items(property_id(&inv, "12 Elm St contents")),
            214
        );
        assert_eq!(inv.property_items(property_id(&inv, "Storage unit")), 18);
        assert_eq!(
            inv.property_value(property_id(&inv, "12 Elm St contents")),
            dollars(163_480)
        );
        assert_eq!(
            inv.property_value(property_id(&inv, "Storage unit")),
            dollars(6_200)
        );
    }

    #[test]
    fn seed_rooms_hit_16q_counts_and_totals_in_tab_order() {
        let inv = seed();
        let elm = inv
            .property(property_id(&inv, "12 Elm St contents"))
            .unwrap();
        let rooms: Vec<_> = elm
            .rooms
            .iter()
            .map(|r| (r.name.as_str(), inv.room_items(r.id), inv.room_value(r.id)))
            .collect();
        let expected = [
            ("Living", 38, 24_150),
            ("Kitchen", 46, 22_310),
            ("Bedroom 1", 24, 18_420),
            ("Bedroom 2", 16, 8_060),
            ("Bedroom 3", 12, 5_540),
            ("Office", 19, 15_240),
            ("Garage", 27, 15_060),
            ("Laundry", 11, 3_120),
            ("Bathroom", 9, 1_880),
            ("Outdoor & shed", 12, 49_700),
        ];
        assert_eq!(rooms.len(), expected.len());
        for ((name, count, value), (want_name, want_count, want_value)) in
            rooms.iter().zip(expected)
        {
            assert_eq!(*name, want_name);
            assert_eq!(*count, want_count, "{name} count");
            assert_eq!(*value, dollars(want_value), "{name} value");
        }
    }

    #[test]
    fn seed_items_have_unique_ids_and_positive_values_in_their_own_property() {
        let inv = seed();
        let mut ids: Vec<_> = inv.items.iter().map(|i| i.id).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), inv.items.len());
        for item in &inv.items {
            assert!(item.replacement.0 > 0, "{}", item.name);
            let (property, _) = inv.room(item.room).unwrap();
            assert_eq!(
                property.id, item.property,
                "{} is in its Room's Property",
                item.name
            );
        }
    }

    #[test]
    fn seed_keeps_the_stub_ids_and_drops_the_blob_item() {
        let inv = seed();
        let named = [
            (2, "Sony A7 IV", "Office"),
            (3, "Fridge", "Kitchen"),
            (4, "Engagement ring", "Bedroom 1"),
            (5, "Sonos Era 100", "Living"),
            (6, "Toyota Corolla", "Garage"),
        ];
        for (id, name, room) in named {
            let item = inv.item(id).unwrap();
            assert_eq!(item.name, name);
            assert_eq!(inv.room(item.room).unwrap().1.name, room);
        }
        assert!(inv.items.iter().all(|i| i.name != "12 Elm St contents"));
    }

    #[test]
    fn seed_storage_unit_cover_reads_the_nrma_policy() {
        let inv = seed();
        let storage = inv.property(property_id(&inv, "Storage unit")).unwrap();
        let cover = storage.cover.as_ref().unwrap();
        assert_eq!(cover.insurer, "NRMA");
        assert_eq!(cover.policy_no.as_deref(), Some("HC-4471902"));
        assert_eq!(cover.sum_insured, dollars(20_000));
        assert_eq!(cover.item_limit, Some(dollars(1_000)));
    }

    #[test]
    fn filler_amounts_sum_exactly_and_never_drop_below_a_dollar() {
        for (count, remaining) in [(35, 14_000), (1, 7), (9, 9), (20, 4_321)] {
            let amounts = filler_amounts(count, remaining);
            assert_eq!(amounts.len(), count);
            assert_eq!(amounts.iter().sum::<i64>(), remaining);
            assert!(amounts.iter().all(|a| *a >= 1));
        }
    }

    // --- derived queries --------------------------------------------------------------------

    #[test]
    fn an_empty_or_unknown_room_is_zero_items_and_zero_value() {
        let mut inv = seed();
        let storage = property_id(&inv, "Storage unit");
        let spare = add_room(&mut inv, storage, "Spare").unwrap();
        assert_eq!(inv.room_items(spare), 0);
        assert_eq!(inv.room_value(spare), zero());
        assert_eq!(inv.room_items(9_999), 0);
        assert_eq!(inv.room_value(9_999), zero());
    }

    // --- property mutations -----------------------------------------------------------------

    #[test]
    fn add_property_starts_with_no_rooms_and_fixes_the_unit() {
        let mut inv = seed();
        let id = add_property(&mut inv, &draft("  Beach shack "), "nzd").unwrap();
        let property = inv.property(id).unwrap();
        assert_eq!(property.name, "Beach shack");
        assert_eq!(property.unit, "nzd");
        assert!(property.rooms.is_empty());
        assert_eq!(property.cover, None);
        assert_eq!(inv.property_count(), 3);
    }

    #[test]
    fn add_property_refuses_empty_taken_and_unitless() {
        let mut inv = seed();
        assert_eq!(
            add_property(&mut inv, &draft("  "), "aud"),
            Err(PropertyError::Name(NameError::Empty))
        );
        assert_eq!(
            add_property(&mut inv, &draft("storage UNIT"), "aud"),
            Err(PropertyError::Name(NameError::Taken))
        );
        assert_eq!(
            add_property(&mut inv, &draft("Beach shack"), " "),
            Err(PropertyError::UnitMissing)
        );
        assert_eq!(inv.property_count(), 2);
    }

    #[test]
    fn edit_property_changes_details_but_never_the_unit() {
        let mut inv = seed();
        let id = property_id(&inv, "Storage unit");
        let mut edit = draft("Storage");
        edit.address = " Fyshwick ".to_string();
        edit_property(&mut inv, id, &edit).unwrap();
        let property = inv.property(id).unwrap();
        assert_eq!(property.name, "Storage");
        assert_eq!(property.address, "Fyshwick");
        assert_eq!(property.unit, "aud");
        assert_eq!(property.cover, None, "an empty insurer clears the cover");
    }

    #[test]
    fn edit_property_may_keep_its_own_name_but_not_take_anothers() {
        let mut inv = seed();
        let id = property_id(&inv, "Storage unit");
        assert_eq!(edit_property(&mut inv, id, &draft("STORAGE unit")), Ok(()));
        assert_eq!(
            edit_property(&mut inv, id, &draft("12 elm st contents")),
            Err(PropertyError::Name(NameError::Taken))
        );
        assert_eq!(
            edit_property(&mut inv, 999, &draft("Anything")),
            Err(PropertyError::Unknown)
        );
    }

    #[test]
    fn remove_property_takes_its_rooms_and_items_and_reports_the_item_ids() {
        let mut inv = seed();
        let storage = property_id(&inv, "Storage unit");
        let expected: Vec<u32> = inv
            .items
            .iter()
            .filter(|i| i.property == storage)
            .map(|i| i.id)
            .collect();
        let removed = remove_property(&mut inv, storage).unwrap();
        assert_eq!(removed.items, expected);
        assert_eq!(removed.items.len(), 18);
        assert_eq!(inv.property_count(), 1);
        assert_eq!(inv.room_count(), 10);
        assert_eq!(inv.item_count(), 214);
        assert_eq!(
            remove_property(&mut inv, storage),
            Err(PropertyError::Unknown)
        );
    }

    #[test]
    fn the_last_property_may_be_removed() {
        let mut inv = seed();
        for id in inv.properties.iter().map(|p| p.id).collect::<Vec<_>>() {
            remove_property(&mut inv, id).unwrap();
        }
        assert_eq!(inv, Inventory::default());
    }

    // --- cover validation -------------------------------------------------------------------

    #[test]
    fn no_insurer_means_no_cover_whatever_else_is_filled_in() {
        let mut draft = cover("  ", Some(-5), Some(0));
        draft.policy_no = "X".to_string();
        assert_eq!(draft.validate(), Ok(None));
    }

    #[test]
    fn cover_needs_a_positive_sum_insured() {
        assert_eq!(
            cover("NRMA", None, None).validate(),
            Err(CoverError::SumMissing)
        );
        assert_eq!(
            cover("NRMA", Some(0), None).validate(),
            Err(CoverError::SumNotPositive)
        );
        assert_eq!(
            cover("NRMA", Some(-1), None).validate(),
            Err(CoverError::SumNotPositive)
        );
    }

    #[test]
    fn item_limit_is_optional_positive_and_within_the_sum() {
        assert!(cover("NRMA", Some(100), None).validate().unwrap().is_some());
        assert_eq!(
            cover("NRMA", Some(100), Some(0)).validate(),
            Err(CoverError::LimitNotPositive)
        );
        assert_eq!(
            cover("NRMA", Some(100), Some(101)).validate(),
            Err(CoverError::LimitAboveSum)
        );
        let at_sum = cover("NRMA", Some(100), Some(100))
            .validate()
            .unwrap()
            .unwrap();
        assert_eq!(at_sum.item_limit, Some(dollars(100)));
    }

    #[test]
    fn a_blank_policy_number_is_none_and_fields_are_trimmed() {
        let mut draft = cover(" AAMI ", Some(100), None);
        draft.policy_no = "  ".to_string();
        let cover = draft.validate().unwrap().unwrap();
        assert_eq!(cover.insurer, "AAMI");
        assert_eq!(cover.policy_no, None);
    }

    #[test]
    fn a_bad_cover_refuses_the_whole_property_change() {
        let mut inv = seed();
        let mut bad = draft("Beach shack");
        bad.cover = cover("NRMA", Some(100), Some(500));
        assert_eq!(
            add_property(&mut inv, &bad, "aud"),
            Err(PropertyError::Cover(CoverError::LimitAboveSum))
        );
        assert_eq!(inv.property_count(), 2);
    }

    // --- room mutations ---------------------------------------------------------------------

    #[test]
    fn add_room_appends_and_names_are_unique_per_property_ignoring_case() {
        let mut inv = seed();
        let elm = property_id(&inv, "12 Elm St contents");
        let storage = property_id(&inv, "Storage unit");
        let id = add_room(&mut inv, elm, " Attic ").unwrap();
        assert_eq!(inv.property(elm).unwrap().rooms.last().unwrap().id, id);
        assert_eq!(
            add_room(&mut inv, elm, "attic"),
            Err(RoomError::Name(NameError::Taken))
        );
        assert_eq!(
            add_room(&mut inv, elm, " "),
            Err(RoomError::Name(NameError::Empty))
        );
        // The same name is fine in another Property.
        assert!(add_room(&mut inv, storage, "Attic").is_ok());
        assert_eq!(
            add_room(&mut inv, 999, "X"),
            Err(RoomError::UnknownProperty)
        );
    }

    #[test]
    fn room_ids_are_unique_across_properties() {
        let mut inv = seed();
        let beach = add_property(&mut inv, &draft("Beach shack"), "aud").unwrap();
        add_room(&mut inv, beach, "Deck").unwrap();
        let mut ids: Vec<_> = inv
            .properties
            .iter()
            .flat_map(|p| &p.rooms)
            .map(|r| r.id)
            .collect();
        let total = ids.len();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), total);
    }

    #[test]
    fn edit_room_renames_without_moving_items() {
        let mut inv = seed();
        let living = room_id(&inv, "12 Elm St contents", "Living");
        let before = inv.room_items(living);
        edit_room(&mut inv, living, "Lounge").unwrap();
        assert_eq!(inv.room(living).unwrap().1.name, "Lounge");
        assert_eq!(inv.room_items(living), before);
        assert_eq!(edit_room(&mut inv, living, "LOUNGE"), Ok(()));
        assert_eq!(
            edit_room(&mut inv, living, "kitchen"),
            Err(RoomError::Name(NameError::Taken))
        );
        assert_eq!(edit_room(&mut inv, 999, "X"), Err(RoomError::UnknownRoom));
    }

    #[test]
    fn removing_an_empty_room_needs_no_destination() {
        let mut inv = seed();
        let elm = property_id(&inv, "12 Elm St contents");
        let attic = add_room(&mut inv, elm, "Attic").unwrap();
        assert_eq!(remove_room(&mut inv, attic, None), Ok(()));
        assert!(inv.room(attic).is_none());
    }

    #[test]
    fn removing_a_room_with_items_moves_them_to_the_chosen_room() {
        let mut inv = seed();
        let laundry = room_id(&inv, "12 Elm St contents", "Laundry");
        let bathroom = room_id(&inv, "12 Elm St contents", "Bathroom");
        let value = inv.property_value(property_id(&inv, "12 Elm St contents"));
        assert_eq!(
            remove_room(&mut inv, laundry, None),
            Err(RoomError::NeedsDestination)
        );
        remove_room(&mut inv, laundry, Some(bathroom)).unwrap();
        assert_eq!(inv.room_items(bathroom), 9 + 11);
        assert_eq!(inv.room_value(bathroom), dollars(1_880 + 3_120));
        assert_eq!(
            inv.property_value(property_id(&inv, "12 Elm St contents")),
            value
        );
        assert_eq!(inv.item_count(), 232);
    }

    #[test]
    fn items_only_move_to_another_room_of_the_same_property() {
        let mut inv = seed();
        let laundry = room_id(&inv, "12 Elm St contents", "Laundry");
        let storage = room_id(&inv, "Storage unit", "Storage");
        assert_eq!(
            remove_room(&mut inv, laundry, Some(storage)),
            Err(RoomError::BadDestination)
        );
        assert_eq!(
            remove_room(&mut inv, laundry, Some(laundry)),
            Err(RoomError::BadDestination)
        );
        assert_eq!(
            remove_room(&mut inv, laundry, Some(999)),
            Err(RoomError::BadDestination)
        );
        assert!(inv.room(laundry).is_some());
    }

    #[test]
    fn the_last_room_cannot_go_while_it_holds_items() {
        let mut inv = seed();
        let storage = room_id(&inv, "Storage unit", "Storage");
        assert_eq!(
            remove_room(&mut inv, storage, None),
            Err(RoomError::LastRoomHoldsItems)
        );
        // Emptied by removing the Property's Items, the last Room may go.
        let property = property_id(&inv, "Storage unit");
        inv.items.retain(|i| i.property != property);
        assert_eq!(remove_room(&mut inv, storage, None), Ok(()));
        assert!(inv.property(property).unwrap().rooms.is_empty());
        assert_eq!(
            remove_room(&mut inv, storage, None),
            Err(RoomError::UnknownRoom)
        );
    }

    #[test]
    fn the_default_destination_is_the_room_above_or_below_when_first() {
        let inv = seed();
        let living = room_id(&inv, "12 Elm St contents", "Living");
        let kitchen = room_id(&inv, "12 Elm St contents", "Kitchen");
        let bedroom = room_id(&inv, "12 Elm St contents", "Bedroom 1");
        let storage = room_id(&inv, "Storage unit", "Storage");
        assert_eq!(default_destination(&inv, living), Some(kitchen));
        assert_eq!(default_destination(&inv, bedroom), Some(kitchen));
        assert_eq!(
            default_destination(&inv, storage),
            None,
            "an only Room has none"
        );
        assert_eq!(default_destination(&inv, 999), None);
    }

    #[test]
    fn move_room_reorders_within_its_property_and_stops_at_the_ends() {
        let mut inv = seed();
        let elm = property_id(&inv, "12 Elm St contents");
        let names = |inv: &Inventory| -> Vec<String> {
            inv.property(elm)
                .unwrap()
                .rooms
                .iter()
                .map(|r| r.name.clone())
                .collect()
        };
        let living = room_id(&inv, "12 Elm St contents", "Living");
        let shed = room_id(&inv, "12 Elm St contents", "Outdoor & shed");
        assert!(!move_room(&mut inv, living, -1), "K at the top is a no-op");
        assert!(!move_room(&mut inv, shed, 1), "J at the bottom is a no-op");
        assert!(move_room(&mut inv, living, 1));
        assert_eq!(&names(&inv)[..2], ["Kitchen", "Living"]);
        assert!(move_room(&mut inv, living, -1));
        assert_eq!(&names(&inv)[..2], ["Living", "Kitchen"]);
        assert!(!move_room(&mut inv, 999, 1));
    }

    #[test]
    fn a_lone_room_cannot_move_and_never_crosses_properties() {
        let mut inv = seed();
        let storage = room_id(&inv, "Storage unit", "Storage");
        assert!(!move_room(&mut inv, storage, -1));
        assert!(!move_room(&mut inv, storage, 1));
        assert_eq!(inv.room_count(), 11);
    }
}
