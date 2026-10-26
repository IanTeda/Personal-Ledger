use lib_core::CategoryId;

use crate::protocols::CategoryRepository;
use crate::{Category, CategoryError, edit_category, get};

/// Rename Category `id`, leaving its parent and type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenameCategory {
    pub id: CategoryId,
    pub name: String,
}

impl RenameCategory {
    /// Returns the Category as written, or refuses a Category that isn't there.
    pub fn run(self, categories: &mut dyn CategoryRepository) -> Result<Category, CategoryError> {
        let mut rows = categories.categories();
        let outcome = edit_category(&mut rows, self.id, self.name)
            .and_then(|()| get(&rows, self.id).cloned().ok_or(CategoryError::NotFound));
        match &outcome {
            Ok(category) => {
                categories.upsert_category(category.clone());
                tracing::info!(category_id = %category.id, "Renamed a Category");
            }
            Err(refusal) => {
                tracing::info!(category_id = %self.id, %refusal, "Refused to rename a Category");
            }
        }
        outcome
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fake::InMemoryCategories;

    #[test]
    fn renaming_a_category_keeps_its_parent_and_type() {
        let mut categories = InMemoryCategories::stub();
        let before = categories
            .category(CategoryId::seed(8))
            .expect("Dining is seeded");

        let renamed = RenameCategory {
            id: before.id,
            name: "Eating out".into(),
        }
        .run(&mut categories)
        .expect("the Category is there");

        assert_eq!(
            renamed,
            Category {
                name: "Eating out".into(),
                ..before
            }
        );
        assert_eq!(categories.category(renamed.id), Some(renamed));
    }

    #[test]
    fn renaming_a_category_that_isnt_there_is_refused_and_nothing_is_written() {
        let mut categories = InMemoryCategories::stub();
        let before = categories.clone();

        assert_eq!(
            RenameCategory {
                id: CategoryId::seed(999),
                name: "Nowhere".into(),
            }
            .run(&mut categories),
            Err(CategoryError::NotFound)
        );
        assert_eq!(categories, before);
    }
}
