use lib_core::AreaId;

use crate::protocols::InventoryRepository;
use crate::{AreaError, RemovedArea};

/// Remove an Area. An empty one just goes; one with Items moves them to `destination`, another
/// Area of the same Location, first.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RemoveArea {
    pub area: AreaId,
    pub destination: Option<AreaId>,
}

impl RemoveArea {
    /// Returns the Items moved. The moved Items are written before the Location drops the Area,
    /// so a stored Item never names an Area that is gone.
    pub fn run(self, inventory: &mut dyn InventoryRepository) -> Result<RemovedArea, AreaError> {
        let mut rows = inventory.inventory();
        let holder = rows.area(self.area).map(|(location, _)| location.id);
        let outcome = rows.remove_area(self.area, self.destination);
        match (&outcome, holder) {
            (Ok(removed), Some(holder)) => {
                for id in &removed.moved_items {
                    if let Some(item) = rows.item(*id) {
                        inventory.upsert_item(item.clone());
                    }
                }
                if let Some(location) = rows.location(holder) {
                    inventory.upsert_location(location.clone());
                }
                tracing::info!(
                    area_id = %self.area,
                    moved = removed.moved_items.len(),
                    "Removed an Area"
                );
            }
            (Err(refusal), _) => tracing::info!(?refusal, "Refused to remove an Area"),
            (Ok(_), None) => {}
        }
        outcome
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fake::InMemoryInventory;

    fn stub() -> InMemoryInventory {
        InMemoryInventory::stub(chrono::NaiveDate::from_ymd_opt(2026, 10, 2).expect("a date"))
    }

    #[test]
    fn an_areas_items_move_to_the_destination_before_it_goes() {
        let mut inventory = stub();
        let areas = inventory.locations()[0].areas.clone();
        let (from, to) = (areas[0].id, areas[1].id);
        let held = inventory.items().iter().filter(|i| i.area == from).count();
        let in_to = inventory.items().iter().filter(|i| i.area == to).count();
        assert!(held > 0);

        let removed = RemoveArea {
            area: from,
            destination: Some(to),
        }
        .run(&mut inventory)
        .expect("a sibling takes them");

        assert_eq!(removed.moved_items.len(), held);
        assert!(inventory.locations()[0].areas.iter().all(|a| a.id != from));
        assert!(inventory.items().iter().all(|i| i.area != from));
        assert_eq!(
            inventory.items().iter().filter(|i| i.area == to).count(),
            in_to + held
        );
    }

    #[test]
    fn an_area_with_items_and_no_destination_is_refused_and_nothing_is_written() {
        let mut inventory = stub();
        let before = inventory.clone();
        let from = inventory.locations()[0].areas[0].id;

        assert_eq!(
            RemoveArea {
                area: from,
                destination: None
            }
            .run(&mut inventory),
            Err(AreaError::NeedsDestination)
        );
        assert_eq!(inventory, before);
    }
}
