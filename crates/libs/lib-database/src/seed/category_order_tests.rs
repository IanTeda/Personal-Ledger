//! The feature crates hold hand-copied sample Categories in tree order, since they can't depend on
//! `lib-categories` to compute it. This checks each copy still lists its ids in the order
//! `lib_categories::tree_order` gives, so renaming or reordering the Categories sample fails here
//! rather than breaking the copies silently.

use lib_core::CategoryId;

/// The sample Categories' ids in the order `lib-categories` lists them.
fn tree_order_ids() -> Vec<CategoryId> {
    lib_categories::tree_order(&lib_categories::sample_categories())
        .into_iter()
        .map(|category| category.id)
        .collect()
}

/// `ids` is the copy's ids in its own order. It must match the tree order restricted to the ids
/// the copy holds, so an id the copy leaves out is not a failure.
fn assert_in_tree_order(name: &str, ids: Vec<CategoryId>) {
    let expected: Vec<CategoryId> = tree_order_ids()
        .into_iter()
        .filter(|id| ids.contains(id))
        .collect();
    assert_eq!(
        ids, expected,
        "{name}'s sample Categories are out of tree order"
    );
}

#[test]
fn the_bills_budgets_and_payees_sample_categories_list_their_ids_in_tree_order() {
    assert_in_tree_order(
        "lib-bills",
        lib_bills::fake::sample_categories()
            .into_iter()
            .map(|category| category.id)
            .collect(),
    );
    assert_in_tree_order(
        "lib-budgets",
        lib_budgets::fake::sample_categories()
            .into_iter()
            .map(|category| category.id)
            .collect(),
    );
    assert_in_tree_order(
        "lib-payees",
        lib_payees::fake::sample_categories()
            .into_iter()
            .map(|category| category.id)
            .collect(),
    );
}
