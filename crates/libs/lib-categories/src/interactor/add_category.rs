use lib_core::{CategoryId, CategoryTypes};

use crate::protocols::CategoryRepository;
use crate::{Category, CategoryError, get, insert_category};

/// Add a Category at top level or under `parent_id`, refused when the parent isn't there, is
/// another type, or already sits at the depth cap.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddCategory {
    pub name: String,
    pub parent_id: Option<CategoryId>,
    pub category_type: CategoryTypes,
}

impl AddCategory {
    /// Returns the Category as written.
    pub fn run(self, categories: &mut dyn CategoryRepository) -> Result<Category, CategoryError> {
        let mut rows = categories.categories();
        let outcome = insert_category(&mut rows, self.name, self.parent_id, self.category_type)
            .and_then(|id| get(&rows, id).cloned().ok_or(CategoryError::NotFound));
        match &outcome {
            Ok(category) => {
                categories.upsert_category(category.clone());
                tracing::info!(category_id = %category.id, "Added a Category");
            }
            Err(refusal) => tracing::info!(%refusal, "Refused to add a Category"),
        }
        outcome
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fake::InMemoryCategories;

    fn add(name: &str, parent_id: Option<CategoryId>, category_type: CategoryTypes) -> AddCategory {
        AddCategory {
            name: name.into(),
            parent_id,
            category_type,
        }
    }

    #[test]
    fn adding_a_child_writes_it_under_its_parent() {
        let mut categories = InMemoryCategories::stub();
        let food = CategoryId::seed(6);

        let added = add("Takeaway", Some(food), CategoryTypes::Expense)
            .run(&mut categories)
            .expect("Food takes children");

        assert_eq!(added.name, "Takeaway");
        assert_eq!(added.parent, Some(food));
        assert_eq!(categories.category(added.id), Some(added));
    }

    #[test]
    fn a_child_of_another_type_or_past_the_depth_cap_is_refused_and_nothing_is_written() {
        let mut categories = InMemoryCategories::stub();
        let before = categories.clone();
        let salary = CategoryId::seed(11);
        let electricity = CategoryId::seed(4);

        assert_eq!(
            add("Bonus", Some(salary), CategoryTypes::Expense).run(&mut categories),
            Err(CategoryError::TypeMismatch)
        );
        assert_eq!(
            add("Peak", Some(electricity), CategoryTypes::Expense).run(&mut categories),
            Err(CategoryError::DepthExceeded)
        );
        assert_eq!(
            add(
                "Orphan",
                Some(CategoryId::seed(999)),
                CategoryTypes::Expense
            )
            .run(&mut categories),
            Err(CategoryError::ParentNotFound)
        );
        assert_eq!(categories, before);
    }
}
