use lib_core::AreaId;

use crate::protocols::InventoryRepository;

/// Move an Area down (`delta` > 0) or up within its Location, stopping at the ends.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MoveArea {
    pub area: AreaId,
    pub delta: isize,
}

impl MoveArea {
    /// Returns whether the Area moved; one already at the end it was asked to pass does not.
    pub fn run(self, inventory: &mut dyn InventoryRepository) -> bool {
        let mut rows = inventory.inventory();
        let moved = rows.move_area(self.area, self.delta).is_some();
        if moved && let Some((location, _)) = rows.area(self.area) {
            inventory.upsert_location(location.clone());
            tracing::info!(area_id = %self.area, delta = self.delta, "Moved an Area");
        }
        moved
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fake::InMemoryInventory;

    #[test]
    fn an_area_swaps_with_its_neighbour_and_stops_at_the_ends() {
        let mut inventory =
            InMemoryInventory::stub(chrono::NaiveDate::from_ymd_opt(2026, 10, 2).expect("a date"));
        let areas: Vec<_> = inventory.locations()[0]
            .areas
            .iter()
            .map(|a| a.id)
            .collect();

        assert!(
            !MoveArea {
                area: areas[0],
                delta: -1
            }
            .run(&mut inventory)
        );
        assert!(
            MoveArea {
                area: areas[0],
                delta: 1
            }
            .run(&mut inventory)
        );

        let after: Vec<_> = inventory.locations()[0]
            .areas
            .iter()
            .map(|a| a.id)
            .collect();
        assert_eq!(&after[..2], &[areas[1], areas[0]]);
    }
}
