use lib_core::LocationId;

use crate::protocols::InventoryRepository;
use crate::{CoverDraft, LocationError};

/// Set a Location's cover from a [`CoverDraft`], or clear it when the insurer is empty.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetCover {
    pub location: LocationId,
    pub cover: CoverDraft,
}

impl SetCover {
    pub fn run(self, inventory: &mut dyn InventoryRepository) -> Result<(), LocationError> {
        let mut rows = inventory.inventory();
        let outcome = rows.set_cover(self.location, &self.cover);
        match &outcome {
            Ok(()) => {
                if let Some(location) = rows.location(self.location) {
                    inventory.upsert_location(location.clone());
                }
                tracing::info!(location_id = %self.location, "Set a Location's cover");
            }
            Err(refusal) => tracing::info!(?refusal, "Refused to set a Location's cover"),
        }
        outcome
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CoverError;
    use crate::fake::InMemoryInventory;

    fn stub() -> InMemoryInventory {
        InMemoryInventory::stub(chrono::NaiveDate::from_ymd_opt(2026, 10, 2).expect("a date"))
    }

    #[test]
    fn an_empty_insurer_clears_the_cover_and_a_bad_one_is_refused() {
        let mut inventory = stub();
        let id = inventory.locations()[0].id;
        assert!(inventory.location(id).expect("seeded").cover.is_some());

        let bad = CoverDraft {
            insurer: "AAMI".into(),
            ..CoverDraft::default()
        };
        assert_eq!(
            SetCover {
                location: id,
                cover: bad
            }
            .run(&mut inventory),
            Err(LocationError::Cover(CoverError::SumMissing))
        );
        assert!(inventory.location(id).expect("seeded").cover.is_some());

        SetCover {
            location: id,
            cover: CoverDraft::default(),
        }
        .run(&mut inventory)
        .expect("clearing is always valid");
        assert_eq!(inventory.location(id).expect("seeded").cover, None);
    }
}
