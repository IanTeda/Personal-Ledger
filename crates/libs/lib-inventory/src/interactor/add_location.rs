use lib_core::{LocationId, UnitId};

use crate::protocols::InventoryRepository;
use crate::{LocationDraft, LocationError};

/// Add the Location a [`LocationDraft`] describes, in `unit`, with no Areas, after every Location
/// already there.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddLocation {
    pub draft: LocationDraft,
    pub unit: UnitId,
}

impl AddLocation {
    /// Returns the new Location's id, or refuses a name that is empty or taken, or a bad cover.
    pub fn run(self, inventory: &mut dyn InventoryRepository) -> Result<LocationId, LocationError> {
        let mut rows = inventory.inventory();
        let outcome = rows.add_location(&self.draft, self.unit);
        match &outcome {
            Ok(id) => {
                if let Some(location) = rows.location(*id) {
                    inventory.upsert_location(location.clone());
                }
                tracing::info!(location_id = %id, "Added a Location");
            }
            Err(refusal) => tracing::info!(?refusal, "Refused to add a Location"),
        }
        outcome
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::NameError;
    use crate::fake::InMemoryInventory;

    fn draft(name: &str) -> LocationDraft {
        LocationDraft {
            name: name.into(),
            ..LocationDraft::default()
        }
    }

    #[test]
    fn adding_writes_the_location_in_its_unit_after_the_others() {
        let mut inventory = InMemoryInventory::default();
        let id = AddLocation {
            draft: draft(" Cabin "),
            unit: UnitId::SEED_AUD,
        }
        .run(&mut inventory)
        .expect("a new name");

        let location = inventory.location(id).expect("written");
        assert_eq!(
            (location.name.as_str(), location.unit),
            ("Cabin", UnitId::SEED_AUD)
        );
        assert!(location.areas.is_empty());
    }

    #[test]
    fn a_refusal_writes_nothing() {
        let mut inventory = InMemoryInventory::default();
        AddLocation {
            draft: draft("Cabin"),
            unit: UnitId::SEED_AUD,
        }
        .run(&mut inventory)
        .expect("a new name");
        let before = inventory.clone();

        assert_eq!(
            AddLocation {
                draft: draft("CABIN"),
                unit: UnitId::SEED_AUD,
            }
            .run(&mut inventory),
            Err(LocationError::Name(NameError::Taken))
        );
        assert_eq!(inventory, before);
    }
}
