//! The Entity: Locations, their Areas and the Items in them (`GLOSSARY.md`'s Location, Area and
//! Inventory Item), the rules over them and the stub seed. Amounts are in the owning Location's
//! Unit, at two decimal places like the Transaction stubs.

use bigdecimal::BigDecimal;
use chrono::NaiveDate;
use lib_core::{AreaId, ItemId, LocationId, Money, UnitId};

/// A Location's insurance cover: one of its details, not a record of its own.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cover {
    pub insurer: String,
    pub policy_no: Option<String>,
    /// Display only: a past date changes nothing.
    pub renews_on: Option<NaiveDate>,
    pub sum_insured: Money,
    pub item_limit: Option<Money>,
}

/// A named area of one Location. `id` is stable across renames, so an Item never moves on one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Area {
    pub id: AreaId,
    pub name: String,
}

/// A place whose contents are kept as one register. An Area's position in `areas` is its area-tab
/// (and `J`/`K`) order; persistence adds the column later.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Location {
    pub id: LocationId,
    pub name: String,
    pub address: String,
    /// The fiat Unit, fixed when the Location is added. Held by id, so a rename of the Unit's code
    /// leaves the Location on its Unit.
    pub unit: UnitId,
    pub cover: Option<Cover>,
    pub areas: Vec<Area>,
}

/// A physical possession, in one Area of one Location.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Item {
    pub id: ItemId,
    pub location: LocationId,
    pub area: AreaId,
    pub name: String,
    /// In the Location's Unit.
    pub replacement: Money,
}

/// Every Location and Item. The Items sit in one list so Documents can resolve a Link by id.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Inventory {
    pub locations: Vec<Location>,
    pub items: Vec<Item>,
}

// ---------------------------------------------------------------------------------------------
// Lookups and derived queries
// ---------------------------------------------------------------------------------------------

impl Inventory {
    pub fn location(&self, id: LocationId) -> Option<&Location> {
        self.locations.iter().find(|l| l.id == id)
    }

    pub fn item(&self, id: ItemId) -> Option<&Item> {
        self.items.iter().find(|i| i.id == id)
    }

    /// The Area and the Location holding it.
    pub fn area(&self, id: AreaId) -> Option<(&Location, &Area)> {
        self.locations
            .iter()
            .find_map(|l| l.areas.iter().find(|a| a.id == id).map(|a| (l, a)))
    }

    pub fn location_count(&self) -> usize {
        self.locations.len()
    }

    pub fn area_count(&self) -> usize {
        self.locations.iter().map(|l| l.areas.len()).sum()
    }

    pub fn item_count(&self) -> usize {
        self.items.len()
    }

    /// Items in an Area; an empty or unknown Area is 0.
    pub fn area_items(&self, area: AreaId) -> usize {
        self.items.iter().filter(|i| i.area == area).count()
    }

    /// The replacement value of an Area's Items; an empty or unknown Area is zero.
    pub fn area_value(&self, area: AreaId) -> Money {
        sum(self.items.iter().filter(|i| i.area == area))
    }

    pub fn location_items(&self, location: LocationId) -> usize {
        self.items.iter().filter(|i| i.location == location).count()
    }

    pub fn location_value(&self, location: LocationId) -> Money {
        sum(self.items.iter().filter(|i| i.location == location))
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

/// Why an Item is refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemError {
    Unknown,
    UnknownLocation,
    /// The Area is not one of the Location's.
    UnknownArea,
    NameEmpty,
    /// A replacement value below zero.
    ReplacementNegative,
}

/// The editable details of an Item: where it sits is fixed once added, bar a move to another Area
/// of its Location.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemDraft {
    pub name: String,
    pub replacement: Money,
}

/// Why a Location or Area name is refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NameError {
    Empty,
    /// Another Location (or another Area of the same Location) has it, ignoring case.
    Taken,
}

/// Why a Location's cover is refused.
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

impl From<&Cover> for CoverDraft {
    fn from(cover: &Cover) -> Self {
        Self {
            insurer: cover.insurer.clone(),
            policy_no: cover.policy_no.clone().unwrap_or_default(),
            renews_on: cover.renews_on,
            sum_insured: Some(cover.sum_insured.clone()),
            item_limit: cover.item_limit.clone(),
        }
    }
}

/// The editable details of a Location: everything but its Unit, which is fixed on Add.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LocationDraft {
    pub name: String,
    pub address: String,
    pub cover: CoverDraft,
}

impl From<&Location> for LocationDraft {
    /// The Location as it is, so an edit that changes only some fields keeps the rest.
    fn from(location: &Location) -> Self {
        Self {
            name: location.name.clone(),
            address: location.address.clone(),
            cover: location
                .cover
                .as_ref()
                .map(CoverDraft::from)
                .unwrap_or_default(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocationError {
    Unknown,
    Name(NameError),
    Cover(CoverError),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AreaError {
    UnknownLocation,
    UnknownArea,
    Name(NameError),
    /// The Area holds Items and another Area of the Location must take them.
    NeedsDestination,
    /// The destination is the Area itself, or not an Area of the same Location.
    BadDestination,
    /// The Area holds Items and is the Location's only Area, so there is nowhere to move them.
    LastAreaHoldsItems,
}

fn taken<'a>(mut names: impl Iterator<Item = &'a str>, name: &str) -> bool {
    names.any(|n| n.to_lowercase() == name.to_lowercase())
}

// ---------------------------------------------------------------------------------------------
// Location mutations
// ---------------------------------------------------------------------------------------------

/// What removing a Location took with it: the Areas and Items the Ledger records as gone, and the
/// Items whose Document Links must drop.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemovedLocation {
    pub areas: Vec<AreaId>,
    /// The ids of the Items that went with it.
    pub items: Vec<ItemId>,
}

/// What removing an Area changed beyond the Area itself: the Items it moved to the destination.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemovedArea {
    pub moved_items: Vec<ItemId>,
}

/// The Inventory's writes. Each refuses before it changes anything, and reports the rows it
/// changed so the Ledger can record them (ADR-0039).
impl Inventory {
    /// Adds a Location with no Areas and returns its id.
    pub fn add_location(
        &mut self,
        draft: &LocationDraft,
        unit: UnitId,
    ) -> Result<LocationId, LocationError> {
        let (name, cover) = checked(self, None, draft)?;
        let id = LocationId::new();
        self.locations.push(Location {
            id,
            name,
            address: draft.address.trim().to_string(),
            unit,
            cover,
            areas: Vec::new(),
        });
        Ok(id)
    }

    /// Edits everything but the Unit.
    pub fn edit_location(
        &mut self,
        id: LocationId,
        draft: &LocationDraft,
    ) -> Result<(), LocationError> {
        let (name, cover) = checked(self, Some(id), draft)?;
        let location = self
            .locations
            .iter_mut()
            .find(|l| l.id == id)
            .ok_or(LocationError::Unknown)?;
        location.name = name;
        location.address = draft.address.trim().to_string();
        location.cover = cover;
        Ok(())
    }

    /// Removes a Location, its Areas and its Items, and reports each. The last Location may go.
    pub fn remove_location(&mut self, id: LocationId) -> Result<RemovedLocation, LocationError> {
        let at = self
            .locations
            .iter()
            .position(|l| l.id == id)
            .ok_or(LocationError::Unknown)?;
        let removed = self.locations.remove(at);
        let items = self
            .items
            .iter()
            .filter(|i| i.location == id)
            .map(|i| i.id)
            .collect();
        self.items.retain(|i| i.location != id);
        Ok(RemovedLocation {
            areas: removed.areas.iter().map(|a| a.id).collect(),
            items,
        })
    }

    /// Adds an Area at the end of the Location's tabs and returns its id.
    pub fn add_area(&mut self, location: LocationId, name: &str) -> Result<AreaId, AreaError> {
        let id = AreaId::new();
        let location = self
            .locations
            .iter_mut()
            .find(|l| l.id == location)
            .ok_or(AreaError::UnknownLocation)?;
        let name = checked_area_name(location, None, name)?;
        location.areas.push(Area { id, name });
        Ok(id)
    }

    /// Renames an Area. Nothing else about an Area is editable: it never changes Location.
    pub fn edit_area(&mut self, area: AreaId, name: &str) -> Result<(), AreaError> {
        let location = self
            .locations
            .iter_mut()
            .find(|l| l.areas.iter().any(|a| a.id == area))
            .ok_or(AreaError::UnknownArea)?;
        let name = checked_area_name(location, Some(area), name)?;
        if let Some(a) = location.areas.iter_mut().find(|a| a.id == area) {
            a.name = name;
        }
        Ok(())
    }

    /// Removes an Area. An empty one just goes; one with Items moves them to `destination`, which
    /// must be another Area of the same Location. Reports the Items it moved.
    pub fn remove_area(
        &mut self,
        area: AreaId,
        destination: Option<AreaId>,
    ) -> Result<RemovedArea, AreaError> {
        let (location, _) = self.area(area).ok_or(AreaError::UnknownArea)?;
        let location_id = location.id;
        let mut moved_items = Vec::new();
        if self.area_items(area) > 0 {
            if location.areas.len() == 1 {
                return Err(AreaError::LastAreaHoldsItems);
            }
            let to = destination.ok_or(AreaError::NeedsDestination)?;
            if to == area || !location.areas.iter().any(|a| a.id == to) {
                return Err(AreaError::BadDestination);
            }
            for item in self.items.iter_mut().filter(|i| i.area == area) {
                item.area = to;
                moved_items.push(item.id);
            }
        }
        if let Some(l) = self.locations.iter_mut().find(|l| l.id == location_id) {
            l.areas.retain(|a| a.id != area);
        }
        Ok(RemovedArea { moved_items })
    }

    /// Moves an Area down (`delta` > 0) or up within its own Location, stopping at the ends.
    /// Returns the Area and the neighbour it swapped with, or `None` when nothing moved.
    pub fn move_area(&mut self, area: AreaId, delta: isize) -> Option<[AreaId; 2]> {
        let location = self
            .locations
            .iter_mut()
            .find(|l| l.areas.iter().any(|a| a.id == area))?;
        let from = location.areas.iter().position(|a| a.id == area)?;
        let to = from
            .checked_add_signed(delta)
            .filter(|&to| to < location.areas.len())?;
        if from == to {
            return None;
        }
        location.areas.swap(from, to);
        Some([area, location.areas[from].id])
    }
}

/// The Items' and cover writes.
impl Inventory {
    /// Sets or clears a Location's cover; an empty insurer clears it.
    pub fn set_cover(
        &mut self,
        location: LocationId,
        draft: &CoverDraft,
    ) -> Result<(), LocationError> {
        let cover = draft.validate().map_err(LocationError::Cover)?;
        self.locations
            .iter_mut()
            .find(|l| l.id == location)
            .ok_or(LocationError::Unknown)?
            .cover = cover;
        Ok(())
    }

    /// Adds an Item to an Area of a Location and returns its id.
    pub fn add_item(
        &mut self,
        location: LocationId,
        area: AreaId,
        draft: &ItemDraft,
    ) -> Result<ItemId, ItemError> {
        let holder = self.location(location).ok_or(ItemError::UnknownLocation)?;
        if !holder.areas.iter().any(|a| a.id == area) {
            return Err(ItemError::UnknownArea);
        }
        let (name, replacement) = checked_item(draft)?;
        let id = ItemId::new();
        self.items.push(Item {
            id,
            location,
            area,
            name,
            replacement,
        });
        Ok(id)
    }

    /// Edits an Item's name and replacement value.
    pub fn edit_item(&mut self, id: ItemId, draft: &ItemDraft) -> Result<(), ItemError> {
        let (name, replacement) = checked_item(draft)?;
        let item = self
            .items
            .iter_mut()
            .find(|i| i.id == id)
            .ok_or(ItemError::Unknown)?;
        item.name = name;
        item.replacement = replacement;
        Ok(())
    }

    /// Removes an Item and returns it.
    pub fn remove_item(&mut self, id: ItemId) -> Result<Item, ItemError> {
        let at = self
            .items
            .iter()
            .position(|i| i.id == id)
            .ok_or(ItemError::Unknown)?;
        Ok(self.items.remove(at))
    }
}

fn checked_item(draft: &ItemDraft) -> Result<(String, Money), ItemError> {
    let name = draft.name.trim();
    if name.is_empty() {
        return Err(ItemError::NameEmpty);
    }
    if draft.replacement.0 < 0 {
        return Err(ItemError::ReplacementNegative);
    }
    Ok((name.to_string(), draft.replacement.clone()))
}

fn checked(
    inventory: &Inventory,
    own_id: Option<LocationId>,
    draft: &LocationDraft,
) -> Result<(String, Option<Cover>), LocationError> {
    if let Some(id) = own_id
        && inventory.location(id).is_none()
    {
        return Err(LocationError::Unknown);
    }
    let name = draft.name.trim();
    if name.is_empty() {
        return Err(LocationError::Name(NameError::Empty));
    }
    let others = inventory
        .locations
        .iter()
        .filter(|l| Some(l.id) != own_id)
        .map(|l| l.name.as_str());
    if taken(others, name) {
        return Err(LocationError::Name(NameError::Taken));
    }
    let cover = draft.cover.validate().map_err(LocationError::Cover)?;
    Ok((name.to_string(), cover))
}

fn checked_area_name(
    location: &Location,
    own_id: Option<AreaId>,
    name: &str,
) -> Result<String, AreaError> {
    let name = name.trim();
    if name.is_empty() {
        return Err(AreaError::Name(NameError::Empty));
    }
    let others = location
        .areas
        .iter()
        .filter(|a| Some(a.id) != own_id)
        .map(|a| a.name.as_str());
    if taken(others, name) {
        return Err(AreaError::Name(NameError::Taken));
    }
    Ok(name.to_string())
}

/// The Area the remove dialog pre-selects: the one above, or the one below when it is first.
pub fn default_destination(inventory: &Inventory, area: AreaId) -> Option<AreaId> {
    let (location, _) = inventory.area(area)?;
    let at = location.areas.iter().position(|a| a.id == area)?;
    let other = at.checked_sub(1).unwrap_or(1);
    location.areas.get(other).map(|a| a.id)
}

// ---------------------------------------------------------------------------------------------
// Seed
// ---------------------------------------------------------------------------------------------

/// One seeded Area: its 16q count and total in whole dollars, and the hand-written Items in it
/// (`(id, name, dollars)`). The filler makes up the rest of both, exactly.
struct AreaSeed {
    name: &'static str,
    count: u32,
    total: i64,
    named: &'static [(u32, &'static str, i64)],
}

/// Filler Items take ids from here, clear of the hand-written ones.
const FILLER_ID_START: u32 = 100;

const ELM_ST: &[AreaSeed] = &[
    AreaSeed {
        name: "Living",
        count: 38,
        total: 24_150,
        named: &[
            (5, "Sonos Era 100", 449),
            (7, "Samsung 65\" OLED TV", 2_400),
            (8, "Three-seat sofa", 3_200),
        ],
    },
    AreaSeed {
        name: "Kitchen",
        count: 46,
        total: 22_310,
        named: &[
            (3, "Fridge", 2_800),
            (9, "Dishwasher", 1_100),
            (10, "Induction cooktop", 1_900),
        ],
    },
    AreaSeed {
        name: "Bedarea 1",
        count: 24,
        total: 18_420,
        named: &[
            (4, "Engagement ring", 6_500),
            (11, "King bed and mattress", 3_600),
        ],
    },
    AreaSeed {
        name: "Bedarea 2",
        count: 16,
        total: 8_060,
        named: &[(12, "Queen bed and mattress", 2_400)],
    },
    AreaSeed {
        name: "Bedarea 3",
        count: 12,
        total: 5_540,
        named: &[(13, "Single bed and mattress", 1_200)],
    },
    AreaSeed {
        name: "Office",
        count: 19,
        total: 15_240,
        named: &[
            (2, "Sony A7 IV", 4_380),
            (14, "MacBook Pro 16\"", 3_900),
            (15, "Standing desk", 1_100),
        ],
    },
    AreaSeed {
        name: "Garage",
        count: 27,
        total: 15_060,
        named: &[
            (6, "Toyota Corolla", 8_500),
            (16, "Cordless tool set", 1_250),
        ],
    },
    AreaSeed {
        name: "Laundry",
        count: 11,
        total: 3_120,
        named: &[(17, "Washing machine", 1_150), (18, "Dryer", 900)],
    },
    AreaSeed {
        name: "Batharea",
        count: 9,
        total: 1_880,
        named: &[(19, "Heated towel rail", 420)],
    },
    AreaSeed {
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

const STORAGE_UNIT: &[AreaSeed] = &[AreaSeed {
    name: "Storage",
    count: 18,
    total: 6_200,
    named: &[
        (23, "Boxed winter wardrobe", 900),
        (24, "Spare mattress", 450),
    ],
}];

/// A vehicle: its Areas are the places in it where things sit.
const HILUX: &[AreaSeed] = &[
    AreaSeed {
        name: "Cab",
        count: 3,
        total: 1_450,
        named: &[(25, "Dash camera", 380)],
    },
    AreaSeed {
        name: "Tray",
        count: 2,
        total: 2_300,
        named: &[(26, "Tray canopy", 1_900)],
    },
    AreaSeed {
        name: "Toolbox",
        count: 4,
        total: 3_100,
        named: &[(27, "Recovery kit", 650)],
    },
];

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

fn seed_location(
    inventory: &mut Inventory,
    next_filler_id: &mut u32,
    name: &str,
    address: &str,
    cover: Option<Cover>,
    areas: &[AreaSeed],
) {
    let location_id =
        LocationId::seed(u32::try_from(inventory.locations.len() + 1).unwrap_or_default());
    let mut location = Location {
        id: location_id,
        name: name.to_string(),
        address: address.to_string(),
        unit: UnitId::SEED_AUD,
        cover,
        areas: Vec::new(),
    };
    for seed in areas {
        let areas_so_far = inventory
            .locations
            .iter()
            .map(|l| l.areas.len())
            .sum::<usize>()
            + location.areas.len();
        let area_id = AreaId::seed(u32::try_from(areas_so_far + 1).unwrap_or_default());
        location.areas.push(Area {
            id: area_id,
            name: seed.name.to_string(),
        });
        let named_total: i64 = seed.named.iter().map(|(_, _, d)| d).sum();
        for (id, item, amount) in seed.named {
            inventory.items.push(Item {
                id: ItemId::seed(*id),
                location: location_id,
                area: area_id,
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
                id: ItemId::seed(*next_filler_id),
                location: location_id,
                area: area_id,
                name: format!("{} item {}", seed.name, k + 1),
                replacement: dollars(amount),
            });
            *next_filler_id += 1;
        }
    }
    // The areas were not in `inventory` yet when their ids were allocated, so push them now.
    inventory.locations.push(location);
}

/// The three stub Locations (`16q`'s 12 Elm St contents, Storage unit and Hilux) with their 14
/// Areas and 241 Items. 12 Elm St holds 214 Items worth 163,480; the Storage unit 18, worth 6,200,
/// under its own copy of the NRMA policy number; the Hilux, a vehicle with no cover, 9 worth 6,850.
pub fn default_inventory(today: NaiveDate) -> Inventory {
    let mut inventory = Inventory::default();
    let mut next_filler_id = FILLER_ID_START;
    let renews = today + chrono::Duration::days(45);
    seed_location(
        &mut inventory,
        &mut next_filler_id,
        "12 Elm St contents",
        "12 Elm St, Ainslie ACT",
        Some(Cover {
            insurer: "NRMA".to_string(),
            policy_no: Some("HC-4471902".to_string()),
            renews_on: Some(renews),
            sum_insured: dollars(150_000),
            item_limit: Some(dollars(2_000)),
        }),
        ELM_ST,
    );
    seed_location(
        &mut inventory,
        &mut next_filler_id,
        "Storage unit",
        "Kennards, Unit 114 \u{b7} Mitchell ACT",
        Some(Cover {
            insurer: "NRMA".to_string(),
            policy_no: Some("HC-4471902".to_string()),
            renews_on: Some(renews),
            sum_insured: dollars(20_000),
            item_limit: Some(dollars(1_000)),
        }),
        STORAGE_UNIT,
    );
    seed_location(
        &mut inventory,
        &mut next_filler_id,
        "Hilux",
        "Rego YHL 482 \u{b7} ACT",
        None,
        HILUX,
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

    fn area_id(inv: &Inventory, location: &str, area: &str) -> AreaId {
        inv.locations
            .iter()
            .find(|l| l.name == location)
            .and_then(|l| l.areas.iter().find(|a| a.name == area))
            .unwrap()
            .id
    }

    fn location_id(inv: &Inventory, name: &str) -> LocationId {
        inv.locations.iter().find(|l| l.name == name).unwrap().id
    }

    fn draft(name: &str) -> LocationDraft {
        LocationDraft {
            name: name.to_string(),
            ..LocationDraft::default()
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

    // --- writes ----------------------------------------------------------------------------

    #[test]
    fn writes_land_in_the_inventory_it_reads() {
        let mut service = seed();
        let before = service.locations.len();

        let location = service
            .add_location(&draft("Beach shack"), UnitId::SEED_BTC)
            .unwrap();
        let area = service.add_area(location, "Deck").unwrap();
        service.edit_area(area, "Verandah").unwrap();

        let inv = service;
        assert_eq!(inv.locations.len(), before + 1);
        assert_eq!(inv.area(area).unwrap().1.name, "Verandah");
    }

    #[test]
    fn refusals_leave_the_inventory_untouched() {
        let mut service = seed();
        let before = service.clone();

        assert_eq!(
            service.add_area(LocationId::seed(9_999), "Nowhere"),
            Err(AreaError::UnknownLocation)
        );
        assert_eq!(
            service
                .remove_location(LocationId::seed(9_999))
                .unwrap_err(),
            LocationError::Unknown
        );
        assert!(service.move_area(AreaId::seed(9_999), 1).is_none());

        assert_eq!(&service, &before);
    }

    #[test]
    fn remove_location_reports_the_items_it_took() {
        let mut service = seed();
        let elm = location_id(&service, "12 Elm St contents");
        let expected: Vec<ItemId> = service
            .items
            .iter()
            .filter(|item| item.location == elm)
            .map(|item| item.id)
            .collect();

        let removed = service.remove_location(elm).unwrap();

        assert_eq!(removed.items, expected);
        assert!(service.location(elm).is_none());
    }

    // --- seed -------------------------------------------------------------------------------

    #[test]
    fn seed_holds_241_items_across_three_locations_and_fourteen_areas() {
        let inv = seed();
        assert_eq!(inv.location_count(), 3);
        assert_eq!(inv.area_count(), 14);
        assert_eq!(inv.item_count(), 241);
        assert_eq!(
            inv.location_items(location_id(&inv, "12 Elm St contents")),
            214
        );
        assert_eq!(inv.location_items(location_id(&inv, "Storage unit")), 18);
        assert_eq!(
            inv.location_value(location_id(&inv, "12 Elm St contents")),
            dollars(163_480)
        );
        assert_eq!(
            inv.location_value(location_id(&inv, "Storage unit")),
            dollars(6_200)
        );
        assert_eq!(inv.location_items(location_id(&inv, "Hilux")), 9);
    }

    #[test]
    fn seed_hilux_is_a_vehicle_with_three_areas_and_no_cover() {
        let inv = seed();
        let hilux = inv.location(location_id(&inv, "Hilux")).unwrap();
        let areas: Vec<_> = hilux
            .areas
            .iter()
            .map(|a| (a.name.as_str(), inv.area_items(a.id), inv.area_value(a.id)))
            .collect();
        assert_eq!(
            areas,
            [
                ("Cab", 3, dollars(1_450)),
                ("Tray", 2, dollars(2_300)),
                ("Toolbox", 4, dollars(3_100)),
            ]
        );
        assert_eq!(hilux.cover, None);
    }

    #[test]
    fn seed_areas_hit_16q_counts_and_totals_in_tab_order() {
        let inv = seed();
        let elm = inv
            .location(location_id(&inv, "12 Elm St contents"))
            .unwrap();
        let areas: Vec<_> = elm
            .areas
            .iter()
            .map(|a| (a.name.as_str(), inv.area_items(a.id), inv.area_value(a.id)))
            .collect();
        let expected = [
            ("Living", 38, 24_150),
            ("Kitchen", 46, 22_310),
            ("Bedarea 1", 24, 18_420),
            ("Bedarea 2", 16, 8_060),
            ("Bedarea 3", 12, 5_540),
            ("Office", 19, 15_240),
            ("Garage", 27, 15_060),
            ("Laundry", 11, 3_120),
            ("Batharea", 9, 1_880),
            ("Outdoor & shed", 12, 49_700),
        ];
        assert_eq!(areas.len(), expected.len());
        for ((name, count, value), (want_name, want_count, want_value)) in
            areas.iter().zip(expected)
        {
            assert_eq!(*name, want_name);
            assert_eq!(*count, want_count, "{name} count");
            assert_eq!(*value, dollars(want_value), "{name} value");
        }
    }

    #[test]
    fn seed_items_have_unique_ids_and_positive_values_in_their_own_location() {
        let inv = seed();
        let mut ids: Vec<_> = inv.items.iter().map(|i| i.id).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), inv.items.len());
        for item in &inv.items {
            assert!(item.replacement.0 > 0, "{}", item.name);
            let (location, _) = inv.area(item.area).unwrap();
            assert_eq!(
                location.id, item.location,
                "{} is in its Area's Location",
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
            (4, "Engagement ring", "Bedarea 1"),
            (5, "Sonos Era 100", "Living"),
            (6, "Toyota Corolla", "Garage"),
        ];
        for (id, name, area) in named {
            let item = inv.item(ItemId::seed(id)).unwrap();
            assert_eq!(item.name, name);
            assert_eq!(inv.area(item.area).unwrap().1.name, area);
        }
        assert!(inv.items.iter().all(|i| i.name != "12 Elm St contents"));
    }

    #[test]
    fn seed_storage_unit_cover_reads_the_nrma_policy() {
        let inv = seed();
        let storage = inv.location(location_id(&inv, "Storage unit")).unwrap();
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
    fn an_empty_or_unknown_area_is_zero_items_and_zero_value() {
        let mut inv = seed();
        let storage = location_id(&inv, "Storage unit");
        let spare = inv.add_area(storage, "Spare").unwrap();
        assert_eq!(inv.area_items(spare), 0);
        assert_eq!(inv.area_value(spare), zero());
        assert_eq!(inv.area_items(AreaId::seed(9_999)), 0);
        assert_eq!(inv.area_value(AreaId::seed(9_999)), zero());
    }

    // --- location mutations -----------------------------------------------------------------

    #[test]
    fn add_location_starts_with_no_areas_and_fixes_the_unit() {
        let mut inv = seed();
        let id = inv
            .add_location(&draft("  Beach shack "), UnitId::SEED_BTC)
            .unwrap();
        let location = inv.location(id).unwrap();
        assert_eq!(location.name, "Beach shack");
        assert_eq!(location.unit, UnitId::SEED_BTC);
        assert!(location.areas.is_empty());
        assert_eq!(location.cover, None);
        assert_eq!(inv.location_count(), 4);
    }

    #[test]
    fn add_location_refuses_empty_and_taken() {
        let mut inv = seed();
        assert_eq!(
            inv.add_location(&draft("  "), UnitId::SEED_AUD),
            Err(LocationError::Name(NameError::Empty))
        );
        assert_eq!(
            inv.add_location(&draft("storage UNIT"), UnitId::SEED_AUD),
            Err(LocationError::Name(NameError::Taken))
        );
        assert_eq!(inv.location_count(), 3);
    }

    #[test]
    fn edit_location_changes_details_but_never_the_unit() {
        let mut inv = seed();
        let id = location_id(&inv, "Storage unit");
        let mut edit = draft("Storage");
        edit.address = " Fyshwick ".to_string();
        inv.edit_location(id, &edit).unwrap();
        let location = inv.location(id).unwrap();
        assert_eq!(location.name, "Storage");
        assert_eq!(location.address, "Fyshwick");
        assert_eq!(location.unit, UnitId::SEED_AUD);
        assert_eq!(location.cover, None, "an empty insurer clears the cover");
    }

    #[test]
    fn a_draft_from_the_location_renames_it_and_keeps_its_address_and_cover() {
        let mut inv = seed();
        let id = location_id(&inv, "12 Elm St contents");
        let before = inv.location(id).unwrap().clone();
        let mut rename = LocationDraft::from(&before);
        rename.name = "Elm St".to_string();
        inv.edit_location(id, &rename).unwrap();
        let after = inv.location(id).unwrap();
        assert_eq!(after.name, "Elm St");
        assert_eq!(after.address, before.address);
        assert_eq!(after.cover, before.cover);
    }

    #[test]
    fn edit_location_may_keep_its_own_name_but_not_take_anothers() {
        let mut inv = seed();
        let id = location_id(&inv, "Storage unit");
        assert_eq!(inv.edit_location(id, &draft("STORAGE unit")), Ok(()));
        assert_eq!(
            inv.edit_location(id, &draft("12 elm st contents")),
            Err(LocationError::Name(NameError::Taken))
        );
        assert_eq!(
            inv.edit_location(LocationId::seed(999), &draft("Anything")),
            Err(LocationError::Unknown)
        );
    }

    #[test]
    fn remove_location_takes_its_areas_and_items_and_reports_the_item_ids() {
        let mut inv = seed();
        let storage = location_id(&inv, "Storage unit");
        let expected: Vec<ItemId> = inv
            .items
            .iter()
            .filter(|i| i.location == storage)
            .map(|i| i.id)
            .collect();
        let removed = inv.remove_location(storage).unwrap();
        assert_eq!(removed.items, expected);
        assert_eq!(removed.items.len(), 18);
        assert_eq!(inv.location_count(), 2);
        assert_eq!(inv.area_count(), 13);
        assert_eq!(inv.item_count(), 223);
        assert_eq!(inv.remove_location(storage), Err(LocationError::Unknown));
    }

    #[test]
    fn the_last_location_may_be_removed() {
        let mut inv = seed();
        for id in inv.locations.iter().map(|l| l.id).collect::<Vec<_>>() {
            inv.remove_location(id).unwrap();
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
    fn a_bad_cover_refuses_the_whole_location_change() {
        let mut inv = seed();
        let mut bad = draft("Beach shack");
        bad.cover = cover("NRMA", Some(100), Some(500));
        assert_eq!(
            inv.add_location(&bad, UnitId::SEED_AUD),
            Err(LocationError::Cover(CoverError::LimitAboveSum))
        );
        assert_eq!(inv.location_count(), 3);
    }

    // --- cover and Item mutations -----------------------------------------------------------

    #[test]
    fn set_cover_sets_validates_and_clears() {
        let mut inv = seed();
        let hilux = location_id(&inv, "Hilux");
        inv.set_cover(hilux, &cover("AAMI", Some(30_000), None))
            .unwrap();
        assert_eq!(
            inv.location(hilux).unwrap().cover.as_ref().unwrap().insurer,
            "AAMI"
        );
        assert_eq!(
            inv.set_cover(hilux, &cover("AAMI", None, None)),
            Err(LocationError::Cover(CoverError::SumMissing))
        );
        inv.set_cover(hilux, &cover("", None, None)).unwrap();
        assert_eq!(inv.location(hilux).unwrap().cover, None);
        assert_eq!(
            inv.set_cover(LocationId::new(), &cover("", None, None)),
            Err(LocationError::Unknown)
        );
    }

    fn item_draft(name: &str, amount: i64) -> ItemDraft {
        ItemDraft {
            name: name.to_string(),
            replacement: dollars(amount),
        }
    }

    #[test]
    fn items_are_added_edited_and_removed_within_the_locations_areas() {
        let mut inv = seed();
        let storage = location_id(&inv, "Storage unit");
        let area = area_id(&inv, "Storage unit", "Storage");
        let before = inv.item_count();

        let id = inv
            .add_item(storage, area, &item_draft(" Ladder ", 90))
            .unwrap();
        assert_eq!(inv.item(id).unwrap().name, "Ladder");
        assert_eq!(inv.item_count(), before + 1);

        inv.edit_item(id, &item_draft("Tall ladder", 120)).unwrap();
        assert_eq!(inv.item(id).unwrap().replacement, dollars(120));

        assert_eq!(inv.remove_item(id).unwrap().name, "Tall ladder");
        assert_eq!(inv.item_count(), before);
        assert_eq!(inv.remove_item(id), Err(ItemError::Unknown));
    }

    #[test]
    fn an_item_is_refused_without_a_name_a_positive_value_or_one_of_the_locations_areas() {
        let mut inv = seed();
        let storage = location_id(&inv, "Storage unit");
        let area = area_id(&inv, "Storage unit", "Storage");
        let elm_area = area_id(&inv, "12 Elm St contents", "Kitchen");
        assert_eq!(
            inv.add_item(storage, area, &item_draft("  ", 10)),
            Err(ItemError::NameEmpty)
        );
        assert_eq!(
            inv.add_item(storage, area, &item_draft("Ladder", -1)),
            Err(ItemError::ReplacementNegative)
        );
        assert_eq!(
            inv.add_item(storage, elm_area, &item_draft("Ladder", 10)),
            Err(ItemError::UnknownArea)
        );
        assert_eq!(
            inv.add_item(LocationId::new(), area, &item_draft("Ladder", 10)),
            Err(ItemError::UnknownLocation)
        );
        assert_eq!(
            inv.edit_item(ItemId::new(), &item_draft("Ladder", 10)),
            Err(ItemError::Unknown)
        );
    }

    // --- area mutations ---------------------------------------------------------------------

    #[test]
    fn add_area_appends_and_names_are_unique_per_location_ignoring_case() {
        let mut inv = seed();
        let elm = location_id(&inv, "12 Elm St contents");
        let storage = location_id(&inv, "Storage unit");
        let id = inv.add_area(elm, " Attic ").unwrap();
        assert_eq!(inv.location(elm).unwrap().areas.last().unwrap().id, id);
        assert_eq!(
            inv.add_area(elm, "attic"),
            Err(AreaError::Name(NameError::Taken))
        );
        assert_eq!(
            inv.add_area(elm, " "),
            Err(AreaError::Name(NameError::Empty))
        );
        // The same name is fine in another Location.
        assert!(inv.add_area(storage, "Attic").is_ok());
        assert_eq!(
            inv.add_area(LocationId::seed(999), "X"),
            Err(AreaError::UnknownLocation)
        );
    }

    #[test]
    fn area_ids_are_unique_across_locations() {
        let mut inv = seed();
        let beach = inv
            .add_location(&draft("Beach shack"), UnitId::SEED_AUD)
            .unwrap();
        inv.add_area(beach, "Deck").unwrap();
        let mut ids: Vec<_> = inv
            .locations
            .iter()
            .flat_map(|l| &l.areas)
            .map(|a| a.id)
            .collect();
        let total = ids.len();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), total);
    }

    #[test]
    fn edit_area_renames_without_moving_items() {
        let mut inv = seed();
        let living = area_id(&inv, "12 Elm St contents", "Living");
        let before = inv.area_items(living);
        inv.edit_area(living, "Lounge").unwrap();
        assert_eq!(inv.area(living).unwrap().1.name, "Lounge");
        assert_eq!(inv.area_items(living), before);
        assert_eq!(inv.edit_area(living, "LOUNGE"), Ok(()));
        assert_eq!(
            inv.edit_area(living, "kitchen"),
            Err(AreaError::Name(NameError::Taken))
        );
        assert_eq!(
            inv.edit_area(AreaId::seed(999), "X"),
            Err(AreaError::UnknownArea)
        );
    }

    #[test]
    fn removing_an_empty_area_needs_no_destination() {
        let mut inv = seed();
        let elm = location_id(&inv, "12 Elm St contents");
        let attic = inv.add_area(elm, "Attic").unwrap();
        assert_eq!(inv.remove_area(attic, None).map(|_| ()), Ok(()));
        assert!(inv.area(attic).is_none());
    }

    #[test]
    fn removing_an_area_with_items_moves_them_to_the_chosen_area() {
        let mut inv = seed();
        let laundry = area_id(&inv, "12 Elm St contents", "Laundry");
        let batharea = area_id(&inv, "12 Elm St contents", "Batharea");
        let value = inv.location_value(location_id(&inv, "12 Elm St contents"));
        assert_eq!(
            inv.remove_area(laundry, None),
            Err(AreaError::NeedsDestination)
        );
        inv.remove_area(laundry, Some(batharea)).unwrap();
        assert_eq!(inv.area_items(batharea), 9 + 11);
        assert_eq!(inv.area_value(batharea), dollars(1_880 + 3_120));
        assert_eq!(
            inv.location_value(location_id(&inv, "12 Elm St contents")),
            value
        );
        assert_eq!(inv.item_count(), 241);
    }

    #[test]
    fn items_only_move_to_another_area_of_the_same_location() {
        let mut inv = seed();
        let laundry = area_id(&inv, "12 Elm St contents", "Laundry");
        let storage = area_id(&inv, "Storage unit", "Storage");
        assert_eq!(
            inv.remove_area(laundry, Some(storage)),
            Err(AreaError::BadDestination)
        );
        assert_eq!(
            inv.remove_area(laundry, Some(laundry)),
            Err(AreaError::BadDestination)
        );
        assert_eq!(
            inv.remove_area(laundry, Some(AreaId::seed(999))),
            Err(AreaError::BadDestination)
        );
        assert!(inv.area(laundry).is_some());
    }

    #[test]
    fn the_last_area_cannot_go_while_it_holds_items() {
        let mut inv = seed();
        let storage = area_id(&inv, "Storage unit", "Storage");
        assert_eq!(
            inv.remove_area(storage, None),
            Err(AreaError::LastAreaHoldsItems)
        );
        // Emptied by removing the Location's Items, the last Area may go.
        let location = location_id(&inv, "Storage unit");
        inv.items.retain(|i| i.location != location);
        assert_eq!(inv.remove_area(storage, None).map(|_| ()), Ok(()));
        assert!(inv.location(location).unwrap().areas.is_empty());
        assert_eq!(inv.remove_area(storage, None), Err(AreaError::UnknownArea));
    }

    #[test]
    fn the_default_destination_is_the_area_above_or_below_when_first() {
        let inv = seed();
        let living = area_id(&inv, "12 Elm St contents", "Living");
        let kitchen = area_id(&inv, "12 Elm St contents", "Kitchen");
        let bedarea = area_id(&inv, "12 Elm St contents", "Bedarea 1");
        let storage = area_id(&inv, "Storage unit", "Storage");
        assert_eq!(default_destination(&inv, living), Some(kitchen));
        assert_eq!(default_destination(&inv, bedarea), Some(kitchen));
        assert_eq!(
            default_destination(&inv, storage),
            None,
            "an only Area has none"
        );
        assert_eq!(default_destination(&inv, AreaId::seed(999)), None);
    }

    #[test]
    fn move_area_reorders_within_its_location_and_stops_at_the_ends() {
        let mut inv = seed();
        let elm = location_id(&inv, "12 Elm St contents");
        let names = |inv: &Inventory| -> Vec<String> {
            inv.location(elm)
                .unwrap()
                .areas
                .iter()
                .map(|a| a.name.clone())
                .collect()
        };
        let living = area_id(&inv, "12 Elm St contents", "Living");
        let shed = area_id(&inv, "12 Elm St contents", "Outdoor & shed");
        assert!(
            inv.move_area(living, -1).is_none(),
            "K at the top is a no-op"
        );
        assert!(
            inv.move_area(shed, 1).is_none(),
            "J at the bottom is a no-op"
        );
        assert!(inv.move_area(living, 1).is_some());
        assert_eq!(&names(&inv)[..2], ["Kitchen", "Living"]);
        assert!(inv.move_area(living, -1).is_some());
        assert_eq!(&names(&inv)[..2], ["Living", "Kitchen"]);
        assert!(inv.move_area(AreaId::seed(999), 1).is_none());
    }

    #[test]
    fn a_lone_area_cannot_move_and_never_crosses_locations() {
        let mut inv = seed();
        let storage = area_id(&inv, "Storage unit", "Storage");
        assert!(inv.move_area(storage, -1).is_none());
        assert!(inv.move_area(storage, 1).is_none());
        assert_eq!(inv.area_count(), 14);
    }
}
