use lib_core::{AreaId, LocationId};

use crate::AreaError;
use crate::protocols::InventoryRepository;

/// Add an Area at the end of a Location's tabs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddArea {
    pub location: LocationId,
    pub name: String,
}

impl AddArea {
    pub fn run(self, inventory: &mut dyn InventoryRepository) -> Result<AreaId, AreaError> {
        let mut rows = inventory.inventory();
        let outcome = rows.add_area(self.location, &self.name);
        match &outcome {
            Ok(id) => {
                if let Some(location) = rows.location(self.location) {
                    inventory.upsert_location(location.clone());
                }
                tracing::info!(area_id = %id, "Added an Area");
            }
            Err(refusal) => tracing::info!(?refusal, "Refused to add an Area"),
        }
        outcome
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::NameError;
    use crate::fake::InMemoryInventory;

    fn stub() -> InMemoryInventory {
        InMemoryInventory::stub(chrono::NaiveDate::from_ymd_opt(2026, 10, 2).expect("a date"))
    }

    #[test]
    fn an_area_lands_last_and_a_taken_name_is_refused() {
        let mut inventory = stub();
        let location = inventory.locations()[0].id;
        let id = AddArea {
            location,
            name: " Attic ".into(),
        }
        .run(&mut inventory)
        .expect("a free name");

        let areas = inventory.location(location).expect("seeded").areas;
        assert_eq!(
            areas.last().map(|a| (a.id, a.name.as_str())),
            Some((id, "Attic"))
        );

        let before = inventory.clone();
        assert_eq!(
            AddArea {
                location,
                name: "attic".into()
            }
            .run(&mut inventory),
            Err(AreaError::Name(NameError::Taken))
        );
        assert_eq!(inventory, before);
    }
}
