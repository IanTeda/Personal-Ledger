use lib_core::CategoryId;

use crate::protocols::CategoryRepository;
use crate::{Category, CategoryError, get, move_category};

/// Move Category `id` under `parent_id`, or to top level when `None`, refused when the parent
/// isn't there, is another type, sits at the depth cap, or is the Category or one nested under it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MoveCategory {
    pub id: CategoryId,
    pub parent_id: Option<CategoryId>,
}

impl MoveCategory {
    /// Returns the Category as written.
    pub fn run(self, categories: &mut dyn CategoryRepository) -> Result<Category, CategoryError> {
        let mut rows = categories.categories();
        let outcome = move_category(&mut rows, self.id, self.parent_id)
            .and_then(|()| get(&rows, self.id).cloned().ok_or(CategoryError::NotFound));
        match &outcome {
            Ok(category) => {
                categories.upsert_category(category.clone());
                tracing::info!(category_id = %category.id, "Moved a Category");
            }
            Err(refusal) => {
                tracing::info!(category_id = %self.id, %refusal, "Refused to move a Category");
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
    fn moving_a_category_writes_its_new_parent() {
        let mut categories = InMemoryCategories::stub();
        let transport = CategoryId::seed(9);
        let food = CategoryId::seed(6);

        let moved = MoveCategory {
            id: transport,
            parent_id: Some(food),
        }
        .run(&mut categories)
        .expect("Food takes children");

        assert_eq!(moved.parent, Some(food));
        assert_eq!(categories.category(transport), Some(moved));
    }

    #[test]
    fn a_move_into_its_own_subtree_or_under_another_type_is_refused_and_nothing_is_written() {
        let mut categories = InMemoryCategories::stub();
        let before = categories.clone();
        let food = CategoryId::seed(6);

        assert_eq!(
            MoveCategory {
                id: food,
                parent_id: Some(CategoryId::seed(7)),
            }
            .run(&mut categories),
            Err(CategoryError::CycleDetected)
        );
        assert_eq!(
            MoveCategory {
                id: food,
                parent_id: Some(CategoryId::seed(11)),
            }
            .run(&mut categories),
            Err(CategoryError::TypeMismatch)
        );
        assert_eq!(categories, before);
    }
}
