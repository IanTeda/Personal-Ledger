use lib_core::ItemId;

use crate::ItemError;
use crate::protocols::InventoryRepository;

/// Remove an Item. The Document Links to it are `lib-documents`' to drop.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RemoveItem(pub ItemId);

impl RemoveItem {
    pub fn run(self, inventory: &mut dyn InventoryRepository) -> Result<(), ItemError> {
        let mut rows = inventory.inventory();
        let outcome = rows.remove_item(self.0).map(|_| ());
        match &outcome {
            Ok(()) => {
                inventory.delete_item(self.0);
                tracing::info!(item_id = %self.0, "Removed an Item");
            }
            Err(refusal) => tracing::info!(?refusal, "Refused to remove an Item"),
        }
        outcome
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fake::InMemoryInventory;

    #[test]
    fn removing_deletes_the_row_and_an_unknown_item_is_refused() {
        let mut inventory =
            InMemoryInventory::stub(chrono::NaiveDate::from_ymd_opt(2026, 10, 2).expect("a date"));
        let id = inventory.items()[0].id;
        let before = inventory.items().len();

        RemoveItem(id)
            .run(&mut inventory)
            .expect("the Item is there");

        assert_eq!(inventory.items().len(), before - 1);
        assert_eq!(RemoveItem(id).run(&mut inventory), Err(ItemError::Unknown));
    }
}
