//! The contract every stored Categories port keeps (ADR-0046), run against the in-memory port
//! here and the `lib-database` adapter (#858), so the fake the tests use can be trusted.
//!
//! Each check is a function generic over a [`PortsFactory`]: a consumer runs it from its own test,
//! since an adapter's tests need their own set-up (a fresh database per test). Each writes through
//! the Categories Interactors, as a Client does, reloads as a restart would, and expects the same
//! tree back, in the same order. There is no delete check: delete is `lib-category-deletion`'s
//! atomic port (#1004), whose own contract checks it.

use std::collections::BTreeMap;

use lib_core::{CategoryId, CategoryTypes, Money};

use crate::fake::{InMemoryCategories, InMemoryCategoryBudget};
use crate::interactor::{AddCategory, BudgetChange, MoveCategory, RenameCategory};
use crate::{
    Category, CategoryError, CategoryRepository, CategorySavePorts, Holdings, sample_categories,
};

/// A stored Categories port.
pub trait StoredPorts {
    fn categories(&mut self) -> &mut dyn CategoryRepository;
}

/// Builds the port a contract check runs on.
pub trait PortsFactory {
    type Ports: StoredPorts;

    /// A port holding `seed`'s Categories, in order, as if stored and loaded.
    fn create(&mut self, seed: Vec<Category>) -> Self::Ports;

    /// Ends the writes made on `ports`, then loads a fresh port from wherever they were stored, as
    /// a restart would.
    fn reload(&mut self, ports: Self::Ports) -> Self::Ports;

    /// Ends one write made on `ports`, sending its changes to storage as an adapter's owner does
    /// after each input.
    fn commit(&mut self, ports: &mut Self::Ports);

    /// Makes every write from now on fail, as a store that refuses it would.
    fn break_writes(&mut self, ports: &mut Self::Ports);

    /// Settles the writes sent so far, then returns `ports` reloaded to the last committed rows,
    /// with the number of failures reported naming the Categories domain.
    fn recover(&mut self, ports: Self::Ports) -> (Self::Ports, usize);
}

impl StoredPorts for InMemoryCategories {
    fn categories(&mut self) -> &mut dyn CategoryRepository {
        self
    }
}

/// The in-memory port is its own storage, so a reload hands it back as it is. It keeps a copy as
/// of the last commit, which a failed write falls back to.
#[derive(Debug, Default)]
pub struct InMemoryFactory {
    committed: Option<InMemoryCategories>,
    broken: bool,
    failures: usize,
}

impl PortsFactory for InMemoryFactory {
    type Ports = InMemoryCategories;

    fn create(&mut self, seed: Vec<Category>) -> InMemoryCategories {
        let ports = InMemoryCategories { categories: seed };
        self.committed = Some(ports.clone());
        ports
    }

    fn reload(&mut self, mut ports: InMemoryCategories) -> InMemoryCategories {
        self.commit(&mut ports);
        ports
    }

    fn commit(&mut self, ports: &mut InMemoryCategories) {
        if self.broken {
            self.failures += 1;
        } else {
            self.committed = Some(ports.clone());
        }
    }

    fn break_writes(&mut self, _ports: &mut InMemoryCategories) {
        self.broken = true;
    }

    fn recover(&mut self, _ports: InMemoryCategories) -> (InMemoryCategories, usize) {
        let committed = self.committed.clone().expect("create ran first");
        (committed, std::mem::take(&mut self.failures))
    }
}

const HOUSING: CategoryId = CategoryId::seed(1);
const UTILITIES: CategoryId = CategoryId::seed(3);
const ELECTRICITY: CategoryId = CategoryId::seed(4);
const FOOD: CategoryId = CategoryId::seed(6);
const GROCERIES: CategoryId = CategoryId::seed(7);
const TRANSPORT: CategoryId = CategoryId::seed(9);
const SALARY: CategoryId = CategoryId::seed(11);

fn add(name: &str, parent_id: Option<CategoryId>, category_type: CategoryTypes) -> AddCategory {
    AddCategory {
        name: name.into(),
        parent_id,
        category_type,
        monthly: BudgetChange::Keep,
    }
}

fn names(ports: &mut impl StoredPorts) -> Vec<String> {
    ports
        .categories()
        .categories()
        .into_iter()
        .map(|category| category.name)
        .collect()
}

/// Added Categories reload with their ids, names, parents and types: a top-level Income Category,
/// a child under it, and a grandchild at the depth cap under an Expense parent.
pub fn added_categories_reload_the_same<F: PortsFactory>(factory: &mut F) {
    let mut ports = factory.create(sample_categories());

    let dividends = add("Dividends", None, CategoryTypes::Income)
        .run(ports.categories(), &mut InMemoryCategoryBudget::default())
        .expect("a top-level Category");
    factory.commit(&mut ports);
    let franked = add("Franked", Some(dividends.id), CategoryTypes::Income)
        .run(ports.categories(), &mut InMemoryCategoryBudget::default())
        .expect("Dividends takes children");
    factory.commit(&mut ports);
    let gas = add("Gas", Some(UTILITIES), CategoryTypes::Expense)
        .run(ports.categories(), &mut InMemoryCategoryBudget::default())
        .expect("Utilities sits one level down");
    factory.commit(&mut ports);

    let before = ports.categories().categories();
    let mut ports = factory.reload(ports);
    assert_eq!(ports.categories().categories(), before);
    assert_eq!(
        ports.categories().category(dividends.id),
        Some(Category {
            id: dividends.id,
            name: "Dividends".into(),
            parent: None,
            category_type: CategoryTypes::Income,
        })
    );
    assert_eq!(
        ports.categories().category(franked.id),
        Some(Category {
            id: franked.id,
            name: "Franked".into(),
            parent: Some(dividends.id),
            category_type: CategoryTypes::Income,
        })
    );
    assert_eq!(
        ports.categories().category(gas.id),
        Some(Category {
            id: gas.id,
            name: "Gas".into(),
            parent: Some(UTILITIES),
            category_type: CategoryTypes::Expense,
        })
    );
}

/// A rename reloads with the new name and the parent and type as they were.
pub fn a_renamed_category_reloads_the_same<F: PortsFactory>(factory: &mut F) {
    let mut ports = factory.create(sample_categories());
    let before = ports
        .categories()
        .category(GROCERIES)
        .expect("Groceries is seeded");

    RenameCategory {
        id: GROCERIES,
        name: "Supermarket".into(),
    }
    .run(ports.categories())
    .expect("the Category is there");
    factory.commit(&mut ports);

    let mut ports = factory.reload(ports);
    assert_eq!(
        ports.categories().category(GROCERIES),
        Some(Category {
            name: "Supermarket".into(),
            ..before
        })
    );
    assert_eq!(
        ports.categories().categories().len(),
        sample_categories().len()
    );
}

/// A move reloads under its new parent, its children still under it; a move to top level reloads
/// with no parent.
pub fn a_moved_category_reloads_under_its_new_parent<F: PortsFactory>(factory: &mut F) {
    let mut ports = factory.create(sample_categories());

    MoveCategory {
        id: FOOD,
        parent_id: Some(TRANSPORT),
    }
    .run(ports.categories())
    .expect("Transport takes children");
    factory.commit(&mut ports);
    MoveCategory {
        id: UTILITIES,
        parent_id: None,
    }
    .run(ports.categories())
    .expect("any Category can go to top level");
    factory.commit(&mut ports);

    let mut ports = factory.reload(ports);
    let parent = |ports: &mut F::Ports, id| {
        ports
            .categories()
            .category(id)
            .expect("the Category is there")
            .parent
    };
    assert_eq!(parent(&mut ports, FOOD), Some(TRANSPORT));
    assert_eq!(parent(&mut ports, GROCERIES), Some(FOOD));
    assert_eq!(parent(&mut ports, UTILITIES), None);
    assert_eq!(parent(&mut ports, ELECTRICITY), Some(UTILITIES));
}

/// The tree lists each parent before its children, depth first, siblings in the order added,
/// before and after a reload: a child added under an early parent lists among its siblings, not
/// last, and a Category moved under a parent added after it lists after that parent.
pub fn categories_list_in_tree_order<F: PortsFactory>(factory: &mut F) {
    let mut ports = factory.create(sample_categories());

    add("Insurance", Some(HOUSING), CategoryTypes::Expense)
        .run(ports.categories(), &mut InMemoryCategoryBudget::default())
        .expect("Housing takes children");
    factory.commit(&mut ports);
    MoveCategory {
        id: FOOD,
        parent_id: Some(TRANSPORT),
    }
    .run(ports.categories())
    .expect("Transport takes children");
    factory.commit(&mut ports);

    let expected = [
        "Housing",
        "Rent",
        "Utilities",
        "Electricity",
        "Water",
        "Insurance",
        "Transport",
        "Food",
        "Groceries",
        "Dining",
        "Household",
        "Salary",
        "Interest",
    ];
    assert_eq!(names(&mut ports), expected);
    let mut ports = factory.reload(ports);
    assert_eq!(names(&mut ports), expected, "a reload keeps the order");
}

/// The tree rules refuse before anything is written, and a reload shows the tree as it was: a
/// child past the three-level cap, a child or a move under another type, and a move into the
/// Category's own subtree.
pub fn a_refused_write_leaves_the_tree_as_it_was<F: PortsFactory>(factory: &mut F) {
    let mut ports = factory.create(sample_categories());
    let stored = ports.categories().categories();

    assert_eq!(
        add("Peak", Some(ELECTRICITY), CategoryTypes::Expense)
            .run(ports.categories(), &mut InMemoryCategoryBudget::default()),
        Err(CategoryError::DepthExceeded.into())
    );
    assert_eq!(
        add("Bonus", Some(SALARY), CategoryTypes::Expense)
            .run(ports.categories(), &mut InMemoryCategoryBudget::default()),
        Err(CategoryError::TypeMismatch.into())
    );
    assert_eq!(
        MoveCategory {
            id: TRANSPORT,
            parent_id: Some(ELECTRICITY),
        }
        .run(ports.categories()),
        Err(CategoryError::DepthExceeded)
    );
    assert_eq!(
        MoveCategory {
            id: HOUSING,
            parent_id: Some(TRANSPORT),
        }
        .run(ports.categories()),
        Err(CategoryError::DepthExceeded)
    );
    assert_eq!(
        MoveCategory {
            id: TRANSPORT,
            parent_id: Some(SALARY),
        }
        .run(ports.categories()),
        Err(CategoryError::TypeMismatch)
    );
    assert_eq!(
        MoveCategory {
            id: HOUSING,
            parent_id: Some(ELECTRICITY),
        }
        .run(ports.categories()),
        Err(CategoryError::CycleDetected)
    );
    assert_eq!(
        RenameCategory {
            id: CategoryId::seed(999),
            name: "Nowhere".into(),
        }
        .run(ports.categories()),
        Err(CategoryError::NotFound)
    );
    factory.commit(&mut ports);

    let mut ports = factory.reload(ports);
    assert_eq!(ports.categories().categories(), stored);
}

/// A write that fails is reported once, naming the Categories domain, and the port goes back to
/// the last committed rows rather than keeping the change the store refused.
pub fn a_failed_write_reloads_and_reports<F: PortsFactory>(factory: &mut F) {
    let mut ports = factory.create(sample_categories());
    let stored = ports.categories().categories();

    factory.break_writes(&mut ports);
    add("Dividends", None, CategoryTypes::Income)
        .run(ports.categories(), &mut InMemoryCategoryBudget::default())
        .expect("a top-level Category");
    factory.commit(&mut ports);

    let (mut ports, failures) = factory.recover(ports);
    assert_eq!(failures, 1, "one failure naming the Categories domain");
    assert_eq!(
        ports.categories().categories(),
        stored,
        "the refused write is gone"
    );
}

/// The ports an Add or Edit save writes through beyond the Categories (the Monthly budget and the
/// handover), with the Categories repository they share.
pub trait SavePorts {
    /// The repository and the save ports, as an Interactor takes them.
    fn parts(&mut self) -> (&mut dyn CategoryRepository, &mut dyn CategorySavePorts);
}

/// Builds the save ports a contract check runs on, and reads what was committed.
///
/// `create` makes the sample Categories with [`GROCERIES`] budgeted this month in the Personal
/// spending Budget and holding at least one Split, as the Sample Ledger does; `Dining` is a
/// leaf beside it. Writes are stored only on `commit`.
pub trait SaveFactory {
    type Ports: SavePorts;

    fn create(&mut self) -> Self::Ports;

    /// Ends one input's writes, as `send` does.
    fn commit(&mut self, ports: &mut Self::Ports);

    /// The Categories as committed, read afresh.
    fn stored_categories(&mut self) -> Vec<Category>;

    /// The Personal spending Budget's amount for `category` this month as committed, read afresh.
    fn stored_amount(&mut self, category: CategoryId) -> Option<Money>;

    /// What names `category` as committed, read afresh.
    fn stored_holdings(&mut self, category: CategoryId) -> Holdings;
}

/// The in-memory save ports: the fakes together.
#[derive(Debug, Clone, Default)]
pub struct InMemorySavePorts {
    pub categories: InMemoryCategories,
    pub budget: InMemoryCategoryBudget,
}

impl SavePorts for InMemorySavePorts {
    fn parts(&mut self) -> (&mut dyn CategoryRepository, &mut dyn CategorySavePorts) {
        (&mut self.categories, &mut self.budget)
    }
}

/// The in-memory ports keep a copy as of the last commit, which the `stored_*` reads answer from.
#[derive(Debug, Default)]
pub struct InMemorySaveFactory {
    committed: InMemorySavePorts,
}

impl SaveFactory for InMemorySaveFactory {
    type Ports = InMemorySavePorts;

    fn create(&mut self) -> InMemorySavePorts {
        let ports = InMemorySavePorts {
            categories: InMemoryCategories {
                categories: sample_categories(),
            },
            budget: InMemoryCategoryBudget {
                amounts: BTreeMap::from([(GROCERIES, amount("400"))]),
                other_budgets: 1,
                held: BTreeMap::from([(
                    GROCERIES,
                    Holdings {
                        splits: 3,
                        bill_plans: 1,
                        payee_defaults: 2,
                    },
                )]),
                ..InMemoryCategoryBudget::default()
            },
        };
        self.committed = ports.clone();
        ports
    }

    fn commit(&mut self, ports: &mut InMemorySavePorts) {
        self.committed = ports.clone();
    }

    fn stored_categories(&mut self) -> Vec<Category> {
        self.committed.categories.categories.clone()
    }

    fn stored_amount(&mut self, category: CategoryId) -> Option<Money> {
        self.committed.budget.amounts.get(&category).cloned()
    }

    fn stored_holdings(&mut self, category: CategoryId) -> Holdings {
        crate::CategoryHandover::holdings(&self.committed.budget, category)
    }
}

const DINING: CategoryId = CategoryId::seed(8);

fn amount(text: &str) -> Money {
    text.parse().expect("an amount")
}

fn loaded<P: SavePorts>(ports: &mut P) -> Vec<Category> {
    ports.parts().0.categories()
}

/// A Monthly budget a rule refuses writes nothing, the Category included: not in the port, and not
/// in the store after the input's commit.
pub fn a_refused_monthly_budget_writes_nothing<F: SaveFactory>(factory: &mut F) {
    let mut ports = factory.create();
    let tree = loaded(&mut ports);
    let budgeted = factory.stored_amount(GROCERIES);

    let (categories, save) = ports.parts();
    let refused = AddCategory {
        name: "Dividends".into(),
        parent_id: None,
        category_type: CategoryTypes::Income,
        monthly: BudgetChange::Set(amount("50")),
    }
    .run(categories, save);
    assert!(refused.is_err(), "an Income Category takes no budget");
    factory.commit(&mut ports);

    assert_eq!(loaded(&mut ports), tree);
    assert_eq!(factory.stored_categories(), tree);
    assert_eq!(factory.stored_amount(GROCERIES), budgeted);
}

/// A budget that is set lands in the Personal spending Budget for this month, and a check that
/// passes means the set passes: the check refuses a parent and passes a leaf, and the set it
/// passed lands in the store at the commit.
pub fn a_set_budget_lands_this_month_and_a_passing_check_means_the_set_passes<F: SaveFactory>(
    factory: &mut F,
) {
    let mut ports = factory.create();
    let tree = loaded(&mut ports);
    let new = amount("612");

    assert!(
        ports
            .parts()
            .1
            .check_monthly_budget(&tree, FOOD, Some(&new))
            .is_err(),
        "a parent takes no budget"
    );
    assert_eq!(
        ports
            .parts()
            .1
            .check_monthly_budget(&tree, DINING, Some(&new)),
        Ok(())
    );
    assert_eq!(
        ports
            .parts()
            .1
            .set_monthly_budget(DINING, Some(new.clone())),
        Ok(()),
        "the set a check passed passes"
    );
    factory.commit(&mut ports);

    assert_eq!(factory.stored_amount(DINING), Some(new.clone()));
    assert_eq!(
        ports.parts().1.monthly_budget(&tree, Some(DINING)).amount,
        Some(new)
    );
}

/// A leaf gaining its first child hands its Splits, Bill Plans and Payee defaults to it, across
/// the three stores, and stops its own budget.
pub fn a_first_child_takes_the_leafs_rows<F: SaveFactory>(factory: &mut F) {
    let mut ports = factory.create();
    let held = factory.stored_holdings(GROCERIES);
    assert!(!held.is_empty(), "the sample's Groceries holds rows");

    let (categories, save) = ports.parts();
    let child = add("Organic", Some(GROCERIES), CategoryTypes::Expense)
        .run(categories, save)
        .expect("Groceries sits two levels down");
    factory.commit(&mut ports);

    assert_eq!(factory.stored_holdings(GROCERIES), Holdings::default());
    assert_eq!(factory.stored_holdings(child.id), held);
    assert_eq!(
        factory.stored_amount(GROCERIES),
        None,
        "the parent only rolls up"
    );
}

/// A new parent's budget is stopped in every active Budget: it is budgeted before and nowhere
/// after, and the stop is stored at the commit.
pub fn a_new_parents_budget_is_stopped_in_every_active_budget<F: SaveFactory>(factory: &mut F) {
    let mut ports = factory.create();
    let tree = loaded(&mut ports);
    let stop = ports
        .parts()
        .1
        .parent_stop(GROCERIES)
        .expect("budgeted in the seed");
    assert!(stop.budgets >= 1);

    ports.parts().1.stop_new_parent(GROCERIES);
    factory.commit(&mut ports);

    assert_eq!(ports.parts().1.parent_stop(GROCERIES), None);
    assert_eq!(
        ports
            .parts()
            .1
            .monthly_budget(&tree, Some(GROCERIES))
            .amount,
        None
    );
    assert_eq!(factory.stored_amount(GROCERIES), None);
}

/// The budget set, the stop and the handover wait for the one commit: nothing is stored before it,
/// all of it after.
pub fn the_writes_wait_for_the_one_commit<F: SaveFactory>(factory: &mut F) {
    let mut ports = factory.create();
    let held = factory.stored_holdings(GROCERIES);
    let budgeted = factory.stored_amount(GROCERIES);
    let tree = loaded(&mut ports);

    let (categories, save) = ports.parts();
    let child = AddCategory {
        name: "Organic".into(),
        parent_id: Some(GROCERIES),
        category_type: CategoryTypes::Expense,
        monthly: BudgetChange::Set(amount("75")),
    }
    .run(categories, save)
    .expect("Groceries takes a child");

    assert_eq!(factory.stored_categories(), tree);
    assert_eq!(factory.stored_holdings(GROCERIES), held);
    assert_eq!(factory.stored_amount(GROCERIES), budgeted);
    assert_eq!(factory.stored_amount(child.id), None);

    factory.commit(&mut ports);

    assert_eq!(factory.stored_holdings(child.id), held);
    assert_eq!(factory.stored_amount(child.id), Some(amount("75")));
    assert_eq!(factory.stored_categories().len(), tree.len() + 1);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn in_memory_added_categories_reload_the_same() {
        added_categories_reload_the_same(&mut InMemoryFactory::default());
    }

    #[test]
    fn in_memory_a_renamed_category_reloads_the_same() {
        a_renamed_category_reloads_the_same(&mut InMemoryFactory::default());
    }

    #[test]
    fn in_memory_a_moved_category_reloads_under_its_new_parent() {
        a_moved_category_reloads_under_its_new_parent(&mut InMemoryFactory::default());
    }

    #[test]
    fn in_memory_categories_list_in_tree_order() {
        categories_list_in_tree_order(&mut InMemoryFactory::default());
    }

    #[test]
    fn in_memory_a_refused_write_leaves_the_tree_as_it_was() {
        a_refused_write_leaves_the_tree_as_it_was(&mut InMemoryFactory::default());
    }

    #[test]
    fn in_memory_a_failed_write_reloads_and_reports() {
        a_failed_write_reloads_and_reports(&mut InMemoryFactory::default());
    }

    #[test]
    fn in_memory_a_refused_monthly_budget_writes_nothing() {
        a_refused_monthly_budget_writes_nothing(&mut InMemorySaveFactory::default());
    }

    #[test]
    fn in_memory_a_set_budget_lands_this_month() {
        a_set_budget_lands_this_month_and_a_passing_check_means_the_set_passes(
            &mut InMemorySaveFactory::default(),
        );
    }

    #[test]
    fn in_memory_a_first_child_takes_the_leafs_rows() {
        a_first_child_takes_the_leafs_rows(&mut InMemorySaveFactory::default());
    }

    #[test]
    fn in_memory_a_new_parents_budget_is_stopped() {
        a_new_parents_budget_is_stopped_in_every_active_budget(&mut InMemorySaveFactory::default());
    }

    #[test]
    fn in_memory_the_save_writes_wait_for_the_one_commit() {
        the_writes_wait_for_the_one_commit(&mut InMemorySaveFactory::default());
    }
}
