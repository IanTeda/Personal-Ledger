use lib_core::LocationId;

use crate::protocols::InventoryRepository;
use crate::{LocationError, RemovedLocation};

/// Remove a Location with its Areas and Items, used or not. The last Location may go.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RemoveLocation(pub LocationId);

impl RemoveLocation {
    /// Returns the Areas and Items that went with it. An unknown Location is refused before
    /// anything is written. The port's delete takes the Items, then the Areas, then the Location,
    /// and the file's cascade drops the Document Links to the Items.
    pub fn run(
        self,
        inventory: &mut dyn InventoryRepository,
    ) -> Result<RemovedLocation, LocationError> {
        let outcome = inventory.inventory().remove_location(self.0);
        match &outcome {
            Ok(removed) => {
                inventory.delete_location(self.0);
                tracing::info!(
                    location_id = %self.0,
                    areas = removed.areas.len(),
                    items = removed.items.len(),
                    "Removed a Location"
                );
            }
            Err(refusal) => tracing::info!(?refusal, "Refused to remove a Location"),
        }
        outcome
    }
}

#[cfg(test)]
mod tests {
    use lib_core::{AreaId, ItemId};

    use super::*;
    use crate::fake::InMemoryInventory;

    fn sample() -> InMemoryInventory {
        InMemoryInventory::sample(chrono::NaiveDate::from_ymd_opt(2026, 10, 15).expect("a date"))
    }

    #[test]
    fn a_location_goes_with_its_areas_and_items_and_reports_each() {
        let mut inventory = sample();
        let id = inventory.locations()[0].id;
        let areas: Vec<AreaId> = inventory.locations()[0]
            .areas
            .iter()
            .map(|area| area.id)
            .collect();
        let items: Vec<ItemId> = inventory
            .items()
            .iter()
            .filter(|item| item.location == id)
            .map(|item| item.id)
            .collect();
        assert!(!items.is_empty(), "the seeded Location holds Items");
        let others = inventory.locations().len() - 1;

        let removed = RemoveLocation(id).run(&mut inventory).expect("it exists");

        assert_eq!(removed.areas, areas);
        assert_eq!(removed.items, items);
        assert!(inventory.location(id).is_none());
        assert_eq!(inventory.locations().len(), others);
        assert!(inventory.items().iter().all(|item| item.location != id));
    }

    #[test]
    fn an_unknown_location_is_refused_and_nothing_is_written() {
        let mut inventory = sample();
        let before = inventory.clone();

        assert_eq!(
            RemoveLocation(LocationId::seed(9_999)).run(&mut inventory),
            Err(LocationError::Unknown)
        );
        assert_eq!(inventory, before);
    }

    #[test]
    fn the_last_location_can_go() {
        let mut inventory = sample();
        let ids: Vec<_> = inventory.locations().iter().map(|l| l.id).collect();
        for id in ids {
            RemoveLocation(id).run(&mut inventory).expect("it exists");
        }
        assert!(inventory.locations().is_empty());
        assert!(inventory.items().is_empty());
    }
}
