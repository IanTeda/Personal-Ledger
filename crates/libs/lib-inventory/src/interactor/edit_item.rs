use lib_core::ItemId;

use crate::protocols::InventoryRepository;
use crate::{ItemDraft, ItemError};

/// Edit an Item's name and replacement value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditItem {
    pub id: ItemId,
    pub draft: ItemDraft,
}

impl EditItem {
    pub fn run(self, inventory: &mut dyn InventoryRepository) -> Result<(), ItemError> {
        let mut rows = inventory.inventory();
        let outcome = rows.edit_item(self.id, &self.draft);
        match &outcome {
            Ok(()) => {
                if let Some(item) = rows.item(self.id) {
                    inventory.upsert_item(item.clone());
                }
                tracing::info!(item_id = %self.id, "Edited an Item");
            }
            Err(refusal) => tracing::info!(?refusal, "Refused to edit an Item"),
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
    fn an_edit_changes_the_name_and_value_and_keeps_the_area() {
        let mut inventory =
            InMemoryInventory::stub(chrono::NaiveDate::from_ymd_opt(2026, 10, 2).expect("a date"));
        let before = inventory.items()[0].clone();

        EditItem {
            id: before.id,
            draft: ItemDraft {
                name: "Renamed".into(),
                replacement: Money(5.into()),
            },
        }
        .run(&mut inventory)
        .expect("the Item is there");

        let after = inventory.item(before.id).expect("still there");
        assert_eq!(after.name, "Renamed");
        assert_eq!(after.replacement, Money(5.into()));
        assert_eq!((after.area, after.location), (before.area, before.location));
    }
}
