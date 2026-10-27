use lib_core::LocationId;

use crate::protocols::InventoryRepository;
use crate::{LocationDraft, LocationError};

/// Edit a Location's name, address and cover. Its Unit is fixed once added.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditLocation {
    pub id: LocationId,
    pub draft: LocationDraft,
}

impl EditLocation {
    pub fn run(self, inventory: &mut dyn InventoryRepository) -> Result<(), LocationError> {
        let mut rows = inventory.inventory();
        let outcome = rows.edit_location(self.id, &self.draft);
        match &outcome {
            Ok(()) => {
                if let Some(location) = rows.location(self.id) {
                    inventory.upsert_location(location.clone());
                }
                tracing::info!(location_id = %self.id, "Edited a Location");
            }
            Err(refusal) => tracing::info!(?refusal, "Refused to edit a Location"),
        }
        outcome
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fake::InMemoryInventory;
    use crate::{CoverDraft, CoverError};

    fn stub() -> InMemoryInventory {
        InMemoryInventory::stub(chrono::NaiveDate::from_ymd_opt(2026, 10, 2).expect("a date"))
    }

    #[test]
    fn an_edit_keeps_the_unit_and_the_areas() {
        let mut inventory = stub();
        let before = inventory.locations()[0].clone();
        let mut draft = LocationDraft::from(&before);
        draft.name = "Elm".into();

        EditLocation {
            id: before.id,
            draft,
        }
        .run(&mut inventory)
        .expect("a free name");

        let after = inventory.location(before.id).expect("still there");
        assert_eq!(after.name, "Elm");
        assert_eq!((after.unit, &after.areas), (before.unit, &before.areas));
    }

    #[test]
    fn a_bad_cover_is_refused_and_nothing_is_written() {
        let mut inventory = stub();
        let before = inventory.clone();
        let id = inventory.locations()[0].id;
        let draft = LocationDraft {
            name: "Elm".into(),
            cover: CoverDraft {
                insurer: "NRMA".into(),
                ..CoverDraft::default()
            },
            ..LocationDraft::default()
        };

        assert_eq!(
            EditLocation { id, draft }.run(&mut inventory),
            Err(LocationError::Cover(CoverError::SumMissing))
        );
        assert_eq!(inventory, before);
    }
}
