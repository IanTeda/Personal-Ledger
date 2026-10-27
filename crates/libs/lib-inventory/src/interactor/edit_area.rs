use lib_core::AreaId;

use crate::AreaError;
use crate::protocols::InventoryRepository;

/// Rename an Area. Its id and its Items stay.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditArea {
    pub area: AreaId,
    pub name: String,
}

impl EditArea {
    pub fn run(self, inventory: &mut dyn InventoryRepository) -> Result<(), AreaError> {
        let mut rows = inventory.inventory();
        let outcome = rows.edit_area(self.area, &self.name);
        match &outcome {
            Ok(()) => {
                if let Some((location, _)) = rows.area(self.area) {
                    inventory.upsert_location(location.clone());
                }
                tracing::info!(area_id = %self.area, "Renamed an Area");
            }
            Err(refusal) => tracing::info!(?refusal, "Refused to rename an Area"),
        }
        outcome
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fake::InMemoryInventory;

    #[test]
    fn a_rename_keeps_the_id_and_the_items() {
        let mut inventory =
            InMemoryInventory::stub(chrono::NaiveDate::from_ymd_opt(2026, 10, 2).expect("a date"));
        let area = inventory.locations()[0].areas[0].clone();
        let items = inventory.items();

        EditArea {
            area: area.id,
            name: "Lounge".into(),
        }
        .run(&mut inventory)
        .expect("a free name");

        let renamed = &inventory.locations()[0].areas[0];
        assert_eq!((renamed.id, renamed.name.as_str()), (area.id, "Lounge"));
        assert_eq!(inventory.items(), items);
    }
}
