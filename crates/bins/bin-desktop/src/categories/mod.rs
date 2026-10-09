pub(crate) mod form;

pub use lib_categories::*;

#[cfg(test)]
mod tests {
    use lib_core::CategoryTypes;

    use super::*;

    #[test]
    fn the_seed_matches_the_categories_bundles_twelve_category_tree() {
        let categories = default_categories();
        assert_eq!(categories.len(), 12);
        let mut ids: Vec<_> = categories.iter().map(|c| c.id).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), 12, "ids are unique");
        for category in &categories {
            if let Some(parent) = category.parent {
                assert!(
                    get(&categories, parent).is_some(),
                    "{} has a parent",
                    category.name
                );
            }
        }
    }

    #[test]
    fn paths_join_ancestors_with_the_separator() {
        let categories = default_categories();
        let electricity = find_by_name(&categories, "Electricity").unwrap();
        assert_eq!(
            path(&categories, electricity).as_deref(),
            Some("Housing \u{203a} Utilities \u{203a} Electricity")
        );
        let groceries = find_by_name(&categories, "Groceries").unwrap();
        assert_eq!(
            path(&categories, groceries).as_deref(),
            Some("Food \u{203a} Groceries")
        );
        let transport = find_by_name(&categories, "Transport").unwrap();
        assert_eq!(path(&categories, transport).as_deref(), Some("Transport"));
        assert_eq!(path(&categories, 999), None);
    }

    #[test]
    fn leaves_are_categories_with_no_children() {
        let categories = default_categories();
        let leaf = |name: &str| is_leaf(&categories, find_by_name(&categories, name).unwrap());
        assert!(leaf("Groceries"));
        assert!(leaf("Electricity"));
        assert!(
            leaf("Transport"),
            "a top-level category with no children is a leaf"
        );
        assert!(!leaf("Food"));
        assert!(!leaf("Utilities"));
        assert!(!leaf("Housing"));
    }

    #[test]
    fn descendants_include_the_category_and_everything_nested_at_any_depth() {
        let categories = default_categories();
        let id = |name: &str| find_by_name(&categories, name).unwrap();
        let mut housing = descendants_inclusive(&categories, id("Housing"));
        housing.sort_unstable();
        let mut expected = vec![
            id("Housing"),
            id("Rent"),
            id("Utilities"),
            id("Electricity"),
            id("Water"),
        ];
        expected.sort_unstable();
        assert_eq!(housing, expected);
        assert_eq!(
            descendants_inclusive(&categories, id("Groceries")),
            vec![id("Groceries")]
        );
    }

    #[test]
    fn paths_in_tree_order_lists_parents_before_their_children() {
        let categories = default_categories();
        let labels: Vec<_> = paths_in_tree_order(&categories)
            .into_iter()
            .map(|(_, label)| label)
            .collect();
        assert_eq!(labels.len(), 12);
        assert_eq!(labels[0], "Housing");
        assert_eq!(labels[1], "Housing \u{203a} Rent");
        assert_eq!(labels[2], "Housing \u{203a} Utilities");
        assert_eq!(labels[3], "Housing \u{203a} Utilities \u{203a} Electricity");
        assert!(labels.contains(&"Salary".to_string()));
    }

    #[test]
    fn income_and_expense_types_are_set() {
        let categories = default_categories();
        let salary = find_by_name(&categories, "Salary").unwrap();
        assert_eq!(
            get(&categories, salary).unwrap().category_type,
            CategoryTypes::Income
        );
        let groceries = find_by_name(&categories, "Groceries").unwrap();
        assert_eq!(
            get(&categories, groceries).unwrap().category_type,
            CategoryTypes::Expense
        );
    }

    #[test]
    fn depth_is_zero_for_top_level() {
        let categories = default_categories();
        let housing = find_by_name(&categories, "Housing").unwrap();
        assert_eq!(depth(&categories, housing), 0);
    }

    #[test]
    fn depth_increases_for_nested_categories() {
        let categories = default_categories();
        let utilities = find_by_name(&categories, "Utilities").unwrap();
        let electricity = find_by_name(&categories, "Electricity").unwrap();
        assert_eq!(depth(&categories, utilities), 1);
        assert_eq!(depth(&categories, electricity), 2);
    }

    #[test]
    fn get_or_create_uncategorised_creates_one_per_type() {
        let mut categories = default_categories();
        let original_count = categories.len();

        let uncategorised_expense =
            get_or_create_uncategorised(&mut categories, CategoryTypes::Expense);
        assert_eq!(categories.len(), original_count + 1);
        assert_eq!(
            categories
                .iter()
                .find(|c| c.id == uncategorised_expense)
                .unwrap()
                .name,
            "Uncategorised"
        );

        let uncategorised_expense_again =
            get_or_create_uncategorised(&mut categories, CategoryTypes::Expense);
        assert_eq!(
            uncategorised_expense, uncategorised_expense_again,
            "should reuse"
        );
        assert_eq!(
            categories.len(),
            original_count + 1,
            "should not create duplicate"
        );

        let _uncategorised_income =
            get_or_create_uncategorised(&mut categories, CategoryTypes::Income);
        assert_eq!(
            categories.len(),
            original_count + 2,
            "should create second for Income type"
        );
    }

    #[test]
    fn insert_category_succeeds_with_valid_parent() {
        let mut categories = default_categories();
        let food = find_by_name(&categories, "Food").unwrap();
        let result = insert_category(
            &mut categories,
            "Takeaway".to_string(),
            Some(food),
            CategoryTypes::Expense,
        );
        assert!(result.is_ok());
        let new_id = result.unwrap();
        let new_cat = get(&categories, new_id).unwrap();
        assert_eq!(new_cat.name, "Takeaway");
        assert_eq!(new_cat.parent, Some(food));
    }

    #[test]
    fn insert_category_rejects_mismatched_type() {
        let mut categories = default_categories();
        let salary = find_by_name(&categories, "Salary").unwrap();
        let result = insert_category(
            &mut categories,
            "Bonus".to_string(),
            Some(salary),
            CategoryTypes::Expense,
        );
        assert_eq!(result, Err(CategoryError::TypeMismatch));
    }

    #[test]
    fn insert_category_rejects_nonexistent_parent() {
        let mut categories = default_categories();
        let result = insert_category(
            &mut categories,
            "Invalid".to_string(),
            Some(999),
            CategoryTypes::Expense,
        );
        assert_eq!(result, Err(CategoryError::ParentNotFound));
    }

    #[test]
    fn insert_category_rejects_depth_exceeded() {
        let mut categories = default_categories();
        let electricity = find_by_name(&categories, "Electricity").unwrap();
        // Electricity is at depth 2, can't add a child
        let result = insert_category(
            &mut categories,
            "SubElectric".to_string(),
            Some(electricity),
            CategoryTypes::Expense,
        );
        assert_eq!(result, Err(CategoryError::DepthExceeded));
    }

    #[test]
    fn move_category_succeeds_with_valid_parent() {
        let mut categories = default_categories();
        let transport = find_by_name(&categories, "Transport").unwrap();
        let food = find_by_name(&categories, "Food").unwrap();

        // Transport is at depth 0, food is at depth 0; moving Transport to be under Food
        let result = move_category(&mut categories, transport, Some(food));
        assert!(result.is_ok());
        assert_eq!(get(&categories, transport).unwrap().parent, Some(food));
    }

    #[test]
    fn move_category_rejects_cycle() {
        let mut categories = default_categories();
        let food = find_by_name(&categories, "Food").unwrap();
        let groceries = find_by_name(&categories, "Groceries").unwrap();

        // Try to make Food a child of Groceries (its own child) -- cycle
        let result = move_category(&mut categories, food, Some(groceries));
        assert_eq!(result, Err(CategoryError::CycleDetected));
    }

    #[test]
    fn move_category_rejects_type_mismatch() {
        let mut categories = default_categories();
        let transport = find_by_name(&categories, "Transport").unwrap();
        let salary = find_by_name(&categories, "Salary").unwrap();

        // Can't move an Expense under an Income parent
        let result = move_category(&mut categories, transport, Some(salary));
        assert_eq!(result, Err(CategoryError::TypeMismatch));
    }

    #[test]
    fn delete_category_leaf_succeeds() {
        let mut categories = default_categories();
        let groceries = find_by_name(&categories, "Groceries").unwrap();
        let result = delete_category(&mut categories, groceries);
        assert!(result.is_ok());
        assert!(!categories.iter().any(|c| c.id == groceries));
    }

    #[test]
    fn delete_category_non_leaf_fails() {
        let mut categories = default_categories();
        let food = find_by_name(&categories, "Food").unwrap();

        // Food has children (Groceries, Dining), so deletion should fail
        let result = delete_category(&mut categories, food);
        assert_eq!(result, Err(CategoryError::NonLeafDeletion));
        assert!(categories.iter().any(|c| c.id == food));
    }

    #[test]
    fn edit_category_succeeds() {
        let mut categories = default_categories();
        let housing = find_by_name(&categories, "Housing").unwrap();
        let result = edit_category(&mut categories, housing, "Real Estate".to_string());
        assert!(result.is_ok());
        assert_eq!(get(&categories, housing).unwrap().name, "Real Estate");
    }

    #[test]
    fn tree_rows_includes_all_when_all_expanded() {
        let categories = default_categories();
        let expanded = vec![1, 3, 6, 9, 10, 11, 12]; // All parents expanded
        let rows = tree_rows(&categories, &expanded);
        assert_eq!(
            rows.len(),
            categories.len(),
            "should include all categories when all expanded"
        );
    }

    #[test]
    fn tree_rows_respects_collapsed_state() {
        let categories = default_categories();
        let expanded = vec![1]; // Only Housing expanded, not Food
        let rows = tree_rows(&categories, &expanded);

        // Should include Housing and its children (Rent, Utilities, Electricity, Water)
        // All top-level categories appear, but Food's children don't (it's not expanded)
        assert!(rows.iter().any(|r| r.id == 1)); // Housing (expanded)
        assert!(rows.iter().any(|r| r.id == 2)); // Rent (child of Housing, shown)
        assert!(rows.iter().any(|r| r.id == 6)); // Food (top-level, always shown)
        assert!(!rows.iter().any(|r| r.id == 7)); // Groceries (child of Food, not shown because Food not expanded)
    }

    #[test]
    fn tree_rows_depth_increases_for_nested() {
        let categories = default_categories();
        let expanded = vec![1, 3, 6];
        let rows = tree_rows(&categories, &expanded);

        let housing = rows.iter().find(|r| r.id == 1).unwrap();
        let utilities = rows.iter().find(|r| r.id == 3).unwrap();
        let electricity = rows.iter().find(|r| r.id == 4).unwrap();

        assert_eq!(housing.depth, 0);
        assert_eq!(utilities.depth, 1);
        assert_eq!(electricity.depth, 2);
    }

    #[test]
    fn toggle_expanded_adds_and_removes() {
        let mut expanded = vec![1];
        let food = find_by_name(&default_categories(), "Food").unwrap();

        toggle_expanded(&mut expanded, food, false);
        assert!(expanded.contains(&food));

        toggle_expanded(&mut expanded, food, false);
        assert!(!expanded.contains(&food));
    }

    #[test]
    fn toggle_expanded_ignores_leaves() {
        let mut expanded = vec![];
        let groceries = find_by_name(&default_categories(), "Groceries").unwrap();

        toggle_expanded(&mut expanded, groceries, true);
        assert!(
            !expanded.contains(&groceries),
            "leaves should not be expanded"
        );
    }

    #[test]
    fn settings_rows_list_expense_before_income_and_skip_collapsed_children() {
        let categories = default_categories();
        let names = |expanded: &[u32]| -> Vec<String> {
            settings_rows(&categories, expanded)
                .into_iter()
                .map(|row| row.name)
                .collect()
        };
        assert_eq!(
            names(&[]),
            [
                "Housing",
                "Food",
                "Transport",
                "Household",
                "Salary",
                "Interest"
            ]
        );
        assert_eq!(
            names(&[1, 3]),
            [
                "Housing",
                "Rent",
                "Utilities",
                "Electricity",
                "Water",
                "Food",
                "Transport",
                "Household",
                "Salary",
                "Interest"
            ]
        );
    }

    #[test]
    fn the_seed_is_three_levels_deep_and_housing_has_two_children() {
        let categories = default_categories();
        assert_eq!(levels_deep(&categories), 3);
        assert_eq!(levels_deep(&[]), 0);
        assert_eq!(child_count(&categories, 1), 2);
        assert_eq!(child_count(&categories, 2), 0);
    }
}
