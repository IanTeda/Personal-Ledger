use lib_core::{AreaId, ItemId, LocationId};

use crate::protocols::InventoryRepository;
use crate::{ItemDraft, ItemError};

/// Add the Item an [`ItemDraft`] describes to an Area of a Location.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddItem {
    pub location: LocationId,
    pub area: AreaId,
    pub draft: ItemDraft,
}

impl AddItem {
    pub fn run(self, inventory: &mut dyn InventoryRepository) -> Result<ItemId, ItemError> {
        let mut rows = inventory.inventory();
        let outcome = rows.add_item(self.location, self.area, &self.draft);
        match &outcome {
            Ok(id) => {
                if let Some(item) = rows.item(*id) {
                    inventory.upsert_item(item.clone());
                }
                tracing::info!(item_id = %id, "Added an Item");
            }
            Err(refusal) => tracing::info!(?refusal, "Refused to add an Item"),
        }
        outcome
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fake::InMemoryInventory;
    use lib_core::Money;

    #[test]
    fn an_item_lands_in_its_area_and_one_for_another_locations_area_is_refused() {
        let mut inventory =
            InMemoryInventory::stub(chrono::NaiveDate::from_ymd_opt(2026, 10, 2).expect("a date"));
        let locations = inventory.locations();
        let (elm, storage) = (&locations[0], &locations[1]);
        let draft = ItemDraft {
            name: " Ladder ".into(),
            replacement: Money(90.into()),
        };

        let id = AddItem {
            location: storage.id,
            area: storage.areas[0].id,
            draft: draft.clone(),
        }
        .run(&mut inventory)
        .expect("the Area is the Location's");
        assert_eq!(inventory.item(id).map(|i| i.name), Some("Ladder".into()));

        let before = inventory.clone();
        assert_eq!(
            AddItem {
                location: storage.id,
                area: elm.areas[0].id,
                draft
            }
            .run(&mut inventory),
            Err(ItemError::UnknownArea)
        );
        assert_eq!(inventory, before);
    }
}
