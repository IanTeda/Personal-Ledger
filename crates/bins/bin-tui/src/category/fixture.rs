//! [`CategoryFixture`]: the in-memory [`CategoryStore`] every ticket in the "Categories
//! screen, views and popup" map builds against, seeded with `docs/ux/tui/categories/
//! README.md`'s own mock tree and amounts. Where the handoff draws a folded branch without
//! showing its children (e.g. `Housing`, `Transport`, `Health`), this fixture invents
//! plausible leaf children rather than leaving the parent childless — the tree needs real
//! multi-level structure for fold/unfold and depth to mean anything. Leaf `direct` amounts are
//! chosen so a subtree's derived `rollup` lands on the handoff's own displayed total wherever
//! the handoff shows enough of that subtree to check (`Housing`, `Transport`, `Health`,
//! `Salary`, `Investments` all land exactly; `Food`'s only differs by the cents the handoff's
//! narrow `12M` tree column truncates away — see the summary-box example, which does show
//! `Groceries`' cents). Root-level totals (`Income`/`Expenses`) are **not** forced to match —
//! rollup is derived, not stored, so it's whatever its children actually sum to; the handoff's
//! own totals don't quite sum either; it's hand-drawn wireframe, not a spreadsheet.

use lib_core::{Money, RowID};

use super::{CategoryError, CategoryKind, CategoryNode, CategoryStore};

/// An in-memory `CategoryStore`, seeded once with a fixed demo tree. Every mutating method
/// mutates this same `Vec` in place — the "in-memory mutable fixture" this map's ticket
/// decided on, so fold/new/edit/move genuinely change what's on screen for the rest of the
/// session (discarded on quit, since nothing here is persisted).
pub struct CategoryFixture {
    nodes: Vec<CategoryNode>,
}

impl Default for CategoryFixture {
    fn default() -> Self {
        Self::new()
    }
}

/// Builds an exact `Money` amount from whole dollars and cents — avoids both `f64` rounding
/// error and `unwrap`/`expect` on parsed string literals for seed data that's obviously always
/// well-formed (the repo convention against `unwrap`/`expect` outside tests is aimed at
/// fallible runtime input, which this isn't).
fn money(dollars: i64, cents: i64) -> Money {
    use bigdecimal::BigDecimal;
    Money(BigDecimal::from(dollars) + BigDecimal::from(cents) / BigDecimal::from(100))
}

fn node(
    id: RowID,
    parent_id: Option<RowID>,
    name: &str,
    note: Option<&str>,
    direct: Money,
    transaction_count: u32,
) -> CategoryNode {
    CategoryNode {
        id,
        parent_id,
        name: name.to_string(),
        note: note.map(str::to_string),
        active: true,
        direct,
        transaction_count,
    }
}

impl CategoryFixture {
    /// Seeds the demo tree. Ids are `RowID::from_timestamp` over a fixed, strictly-increasing
    /// sequence of dates (not `RowID::new()`/`RowID::mock()`) so a fresh fixture is
    /// deterministic across runs, matching the repo's "deterministic seeds for generated test
    /// data" convention.
    pub fn new() -> Self {
        use chrono::{DateTime, Utc};

        // A fresh, distinct RowID for each seeded node, ordered by seed position.
        let mut next = DateTime::parse_from_rfc3339("2024-01-01T00:00:00Z")
            .expect("fixed literal is a valid RFC3339 timestamp")
            .with_timezone(&Utc);
        let mut id = move || {
            let this = next;
            next += chrono::Duration::seconds(1);
            RowID::from_timestamp(this)
        };

        let income = id();
        let salary = id();
        let primary_job = id();
        let bonus = id();
        let investments = id();
        let dividends = id();
        let interest_income = id();
        let capital_gains = id();

        let expenses = id();
        let housing = id();
        let mortgage = id();
        let mortgage_interest = id();
        let mortgage_principal = id();
        let insurance = id();
        let food = id();
        let groceries = id();
        let restaurants = id();
        let transport = id();
        let fuel = id();
        let public_transit = id();
        let parking = id();
        let health = id();
        let pharmacy = id();
        let gp_visits = id();
        let dental = id();

        let zero = money(0, 0);
        let nodes = vec![
            node(income, None, "Income", None, zero.clone(), 0),
            node(salary, Some(income), "Salary", None, zero.clone(), 0),
            node(
                primary_job,
                Some(salary),
                "Primary Job",
                None,
                money(98_000, 0),
                24,
            ),
            node(bonus, Some(salary), "Bonus", None, money(20_400, 0), 2),
            node(
                investments,
                Some(income),
                "Investments",
                None,
                zero.clone(),
                0,
            ),
            node(
                dividends,
                Some(investments),
                "Dividends",
                None,
                money(12_000, 0),
                8,
            ),
            node(
                interest_income,
                Some(investments),
                "Interest",
                None,
                money(6_200, 0),
                12,
            ),
            node(
                capital_gains,
                Some(investments),
                "Capital Gains",
                None,
                money(5_000, 0),
                3,
            ),
            node(expenses, None, "Expenses", None, zero.clone(), 0),
            node(housing, Some(expenses), "Housing", None, zero.clone(), 0),
            node(mortgage, Some(housing), "Mortgage", None, zero.clone(), 0),
            node(
                mortgage_interest,
                Some(mortgage),
                "Interest",
                None,
                money(8_200, 0),
                12,
            ),
            node(
                mortgage_principal,
                Some(mortgage),
                "Principal",
                None,
                money(33_800, 0),
                12,
            ),
            node(
                insurance,
                Some(housing),
                "Insurance",
                None,
                money(10_400, 0),
                4,
            ),
            node(food, Some(expenses), "Food", None, zero.clone(), 0),
            node(
                groceries,
                Some(food),
                "Groceries",
                Some("supermarket, greengrocer"),
                money(12_480, 40),
                148,
            ),
            node(
                restaurants,
                Some(food),
                "Restaurants",
                None,
                money(5_760, 0),
                62,
            ),
            node(
                transport,
                Some(expenses),
                "Transport",
                None,
                zero.clone(),
                0,
            ),
            node(fuel, Some(transport), "Fuel", None, money(9_200, 0), 36),
            node(
                public_transit,
                Some(transport),
                "Public Transit",
                None,
                money(4_200, 0),
                52,
            ),
            node(
                parking,
                Some(transport),
                "Parking",
                None,
                money(3_500, 0),
                21,
            ),
            node(health, Some(expenses), "Health", None, zero.clone(), 0),
            node(
                pharmacy,
                Some(health),
                "Pharmacy",
                None,
                money(3_200, 0),
                14,
            ),
            node(
                gp_visits,
                Some(health),
                "GP Visits",
                None,
                money(2_800, 0),
                6,
            ),
            node(dental, Some(health), "Dental", None, money(2_200, 0), 3),
        ];

        Self { nodes }
    }

    /// The `income` and `expenses` roots — always exactly these two, per the handoff.
    fn is_root(&self, id: RowID) -> bool {
        self.find(id).is_some_and(|node| node.parent_id.is_none())
    }
}

impl CategoryStore for CategoryFixture {
    fn nodes(&self) -> &[CategoryNode] {
        &self.nodes
    }

    fn find(&self, id: RowID) -> Option<&CategoryNode> {
        self.nodes.iter().find(|node| node.id == id)
    }

    fn children(&self, parent: RowID) -> Vec<&CategoryNode> {
        self.nodes
            .iter()
            .filter(|node| node.parent_id == Some(parent))
            .collect()
    }

    fn root(&self, id: RowID) -> Option<&CategoryNode> {
        let mut current = self.find(id)?;
        while let Some(parent_id) = current.parent_id {
            current = self.find(parent_id)?;
        }
        Some(current)
    }

    fn kind(&self, id: RowID) -> Option<CategoryKind> {
        let root = self.root(id)?;
        match root.name.to_lowercase().as_str() {
            "income" => Some(CategoryKind::Income),
            "expenses" => Some(CategoryKind::Expense),
            _ => None,
        }
    }

    fn descendants(&self, id: RowID) -> Vec<RowID> {
        let mut ids = vec![id];
        let mut frontier = vec![id];
        while let Some(current) = frontier.pop() {
            for child in self.children(current) {
                ids.push(child.id);
                frontier.push(child.id);
            }
        }
        ids
    }

    fn rollup(&self, id: RowID) -> Money {
        let Some(this) = self.find(id) else {
            return money(0, 0);
        };
        let mut total = this.direct.0.clone();
        for child in self.children(id) {
            total += self.rollup(child.id).0;
        }
        Money(total)
    }

    fn insert(
        &mut self,
        parent: RowID,
        name: String,
        note: Option<String>,
    ) -> Result<RowID, CategoryError> {
        if self.find(parent).is_none() {
            return Err(CategoryError::NotFound);
        }
        if self
            .children(parent)
            .iter()
            .any(|sibling| sibling.name.eq_ignore_ascii_case(&name))
        {
            return Err(CategoryError::DuplicateSibling { name });
        }

        let new_id = RowID::new();
        self.nodes.push(node(
            new_id,
            Some(parent),
            &name,
            note.as_deref(),
            money(0, 0),
            0,
        ));
        Ok(new_id)
    }

    fn rename(&mut self, id: RowID, name: String) -> Result<(), CategoryError> {
        if self.is_root(id) {
            return Err(CategoryError::IsRoot);
        }
        let Some(parent_id) = self.find(id).and_then(|n| n.parent_id) else {
            return Err(CategoryError::NotFound);
        };
        if self
            .children(parent_id)
            .iter()
            .any(|sibling| sibling.id != id && sibling.name.eq_ignore_ascii_case(&name))
        {
            return Err(CategoryError::DuplicateSibling { name });
        }

        let Some(this) = self.nodes.iter_mut().find(|node| node.id == id) else {
            return Err(CategoryError::NotFound);
        };
        this.name = name;
        Ok(())
    }

    fn move_to(&mut self, id: RowID, new_parent: RowID) -> Result<(), CategoryError> {
        let Some(this) = self.find(id) else {
            return Err(CategoryError::NotFound);
        };
        if this.parent_id.is_none() {
            return Err(CategoryError::IsRoot);
        }
        if self.find(new_parent).is_none() {
            return Err(CategoryError::NotFound);
        }
        if self.descendants(id).contains(&new_parent) {
            return Err(CategoryError::WouldCycle {
                name: this.name.clone(),
            });
        }

        let crosses_root = self.root(id).map(|r| r.id) != self.root(new_parent).map(|r| r.id);
        if crosses_root {
            let has_transactions =
                self.descendants(id)
                    .iter()
                    .any(|descendant_id| match self.find(*descendant_id) {
                        Some(node) => node.transaction_count > 0,
                        None => false,
                    });
            if has_transactions {
                return Err(CategoryError::CrossRootWithTransactions {
                    name: this.name.clone(),
                });
            }
        }

        let Some(this) = self.nodes.iter_mut().find(|node| node.id == id) else {
            return Err(CategoryError::NotFound);
        };
        this.parent_id = Some(new_parent);
        Ok(())
    }

    fn set_active(&mut self, id: RowID, active: bool) -> Result<(), CategoryError> {
        if self.is_root(id) {
            return Err(CategoryError::IsRoot);
        }
        let Some(this) = self.nodes.iter_mut().find(|node| node.id == id) else {
            return Err(CategoryError::NotFound);
        };
        this.active = active;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn find_by_name<'a>(store: &'a CategoryFixture, name: &str) -> &'a CategoryNode {
        store
            .nodes()
            .iter()
            .find(|node| node.name == name)
            .unwrap_or_else(|| panic!("fixture should seed a category named {name}"))
    }

    #[test]
    fn seeds_exactly_two_roots() {
        let store = CategoryFixture::new();
        let roots: Vec<_> = store
            .nodes()
            .iter()
            .filter(|node| node.parent_id.is_none())
            .collect();
        assert_eq!(roots.len(), 2);
        assert!(roots.iter().any(|n| n.name == "Income"));
        assert!(roots.iter().any(|n| n.name == "Expenses"));
    }

    #[test]
    fn rollup_of_a_leaf_equals_its_direct_amount() {
        let store = CategoryFixture::new();
        let groceries = find_by_name(&store, "Groceries");
        assert_eq!(store.rollup(groceries.id), groceries.direct);
    }

    #[test]
    fn rollup_of_a_parent_sums_its_descendants() {
        let store = CategoryFixture::new();
        let food = find_by_name(&store, "Food");
        assert_eq!(store.rollup(food.id).to_string(), "18240.4");
    }

    #[test]
    fn rollup_matches_the_handoffs_displayed_total_where_it_shows_enough_to_check() {
        let store = CategoryFixture::new();
        for (name, expected) in [
            ("Salary", "118400"),
            ("Investments", "23200"),
            ("Housing", "52400"),
            ("Transport", "16900"),
            ("Health", "8200"),
        ] {
            let node = find_by_name(&store, name);
            assert_eq!(store.rollup(node.id).to_string(), expected, "{name}");
        }
    }

    #[test]
    fn kind_is_derived_from_the_root_not_stored_per_node() {
        let store = CategoryFixture::new();
        assert_eq!(
            store.kind(find_by_name(&store, "Groceries").id),
            Some(CategoryKind::Expense)
        );
        assert_eq!(
            store.kind(find_by_name(&store, "Salary").id),
            Some(CategoryKind::Income)
        );
    }

    #[test]
    fn descendants_includes_every_depth_and_the_node_itself() {
        let store = CategoryFixture::new();
        let housing = find_by_name(&store, "Housing");
        let names: Vec<&str> = store
            .descendants(housing.id)
            .iter()
            .map(|id| store.find(*id).unwrap().name.as_str())
            .collect();
        for expected in ["Housing", "Mortgage", "Interest", "Principal", "Insurance"] {
            assert!(names.contains(&expected), "missing {expected} in {names:?}");
        }
    }

    #[test]
    fn insert_rejects_a_case_insensitive_duplicate_sibling() {
        let mut store = CategoryFixture::new();
        let food = find_by_name(&store, "Food").id;
        store
            .insert(food, "Snacks".to_string(), None)
            .expect("first insert should succeed");

        let result = store.insert(food, "snacks".to_string(), None);
        assert_eq!(
            result,
            Err(CategoryError::DuplicateSibling {
                name: "snacks".to_string()
            })
        );
    }

    #[test]
    fn insert_lands_the_new_node_as_a_leaf_under_its_parent() {
        let mut store = CategoryFixture::new();
        let food = find_by_name(&store, "Food").id;
        let new_id = store
            .insert(
                food,
                "Snacks".to_string(),
                Some("vending machine".to_string()),
            )
            .expect("insert should succeed");

        let new_node = store.find(new_id).expect("inserted node should exist");
        assert_eq!(new_node.parent_id, Some(food));
        assert_eq!(new_node.note.as_deref(), Some("vending machine"));
        assert_eq!(store.rollup(new_id).to_string(), "0");
    }

    #[test]
    fn rename_rejects_renaming_a_root() {
        let mut store = CategoryFixture::new();
        let income = find_by_name(&store, "Income").id;
        assert_eq!(
            store.rename(income, "Revenue".to_string()),
            Err(CategoryError::IsRoot)
        );
    }

    #[test]
    fn rename_succeeds_when_the_new_name_has_no_sibling_clash() {
        let mut store = CategoryFixture::new();
        let restaurants = find_by_name(&store, "Restaurants").id;
        store
            .rename(restaurants, "Dining".to_string())
            .expect("rename should succeed");
        assert_eq!(store.find(restaurants).unwrap().name, "Dining");
    }

    #[test]
    fn move_refuses_a_cycle_into_its_own_descendant() {
        let mut store = CategoryFixture::new();
        let food = find_by_name(&store, "Food").id;
        let groceries = find_by_name(&store, "Groceries").id;

        assert_eq!(
            store.move_to(food, groceries),
            Err(CategoryError::WouldCycle {
                name: "Food".to_string()
            })
        );
    }

    #[test]
    fn move_refuses_crossing_roots_while_the_subtree_has_transactions() {
        let mut store = CategoryFixture::new();
        let groceries = find_by_name(&store, "Groceries").id;
        let salary = find_by_name(&store, "Salary").id;

        assert_eq!(
            store.move_to(groceries, salary),
            Err(CategoryError::CrossRootWithTransactions {
                name: "Groceries".to_string()
            })
        );
    }

    #[test]
    fn move_allows_crossing_roots_when_the_subtree_is_empty() {
        let mut store = CategoryFixture::new();
        let food = find_by_name(&store, "Food").id;
        let salary = find_by_name(&store, "Salary").id;
        let empty_id = store
            .insert(food, "Untouched".to_string(), None)
            .expect("insert should succeed");

        store
            .move_to(empty_id, salary)
            .expect("an empty subtree should be allowed to cross roots");
        assert_eq!(store.find(empty_id).unwrap().parent_id, Some(salary));
    }

    #[test]
    fn move_within_the_same_root_recomputes_the_old_and_new_ancestors_rollups() {
        let mut store = CategoryFixture::new();
        let groceries = find_by_name(&store, "Groceries").id;
        let transport = find_by_name(&store, "Transport").id;
        let food = find_by_name(&store, "Food").id;

        let food_rollup_before = store.rollup(food);
        store
            .move_to(groceries, transport)
            .expect("same-root move should succeed");

        assert!(store.rollup(food) < food_rollup_before);
        assert_eq!(store.find(groceries).unwrap().parent_id, Some(transport));
    }

    #[test]
    fn archive_sets_active_false_and_keeps_direct_and_rollup_intact() {
        let mut store = CategoryFixture::new();
        let groceries = find_by_name(&store, "Groceries").id;
        let rollup_before = store.rollup(groceries);

        store
            .set_active(groceries, false)
            .expect("archiving a non-root should succeed");

        assert!(!store.find(groceries).unwrap().active);
        assert_eq!(store.rollup(groceries), rollup_before);
    }

    #[test]
    fn archive_refuses_a_root() {
        let mut store = CategoryFixture::new();
        let expenses = find_by_name(&store, "Expenses").id;
        assert_eq!(
            store.set_active(expenses, false),
            Err(CategoryError::IsRoot)
        );
    }
}
