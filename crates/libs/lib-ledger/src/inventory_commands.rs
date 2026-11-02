//! The Inventory write that crosses domains (ADR-0038): Remove Location, a command value with its
//! own refusal, checked before anything is written so a refusal leaves the Ledger untouched, and
//! recording a change per row it wrote (ADR-0039).
//!
//! Single-entity rules are `lib_inventory`, which the Inventory page writes itself (#867). Remove
//! Location stays here until `lib-reassign` (#813). The Document Links to its Items aren't its to
//! drop: the file's cascade does, and the Client reloads the Documents after the commit (#964).

use lib_core::LocationId;
use lib_inventory::{LocationError, RemovedLocation};

use crate::{Ledger, LedgerChange, LedgerCommand};

impl Ledger {
    /// Removes a Location with its Areas and Items. The file's cascade drops the Document Links to
    /// those Items, and the Client reloads the Documents after the commit (#964).
    pub fn remove_location(&mut self, id: LocationId) -> Result<RemovedLocation, LocationError> {
        let removed = self.inventory.remove_location(id)?;
        for area in &removed.areas {
            self.pending.push(LedgerChange::DeleteArea(*area));
        }
        for item in &removed.items {
            self.pending.push(LedgerChange::DeleteItem(*item));
        }
        self.pending.push(LedgerChange::DeleteLocation(id));
        Ok(removed)
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

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;
    use lib_core::{AreaId, ItemId};
    use lib_inventory::{Inventory, default_inventory};

    use super::*;
    use crate::LedgerParts;

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

    /// The Ledger must come out of a refused command exactly as it went in.
    fn assert_untouched(ledger: &mut Ledger, before: &Inventory) {
        assert_eq!(ledger.inventory(), before);
        assert!(ledger.drain_changes().is_empty());
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
    fn a_refused_remove_location_leaves_the_ledger_untouched() {
        let mut ledger = ledger();
        let before = ledger.inventory().clone();
        assert_eq!(
            ledger.remove_location(LocationId::seed(9_999)).unwrap_err(),
            LocationError::Unknown
        );
        assert_untouched(&mut ledger, &before);
    }
}
