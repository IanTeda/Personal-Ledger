//! The contract every stored Categories port keeps (ADR-0046), run against the in-memory port
//! here and, once Categories persist (#858), the `lib-database` adapter, so the fake the tests use
//! can be trusted.
//!
//! Each check is a function generic over a [`PortsFactory`]: a consumer runs it from its own test,
//! since an adapter's tests need their own set-up (a fresh database per test). Each writes through
//! the Categories Interactors, as a Client does, reloads as a restart would, and expects the same
//! tree back, in the same order. There is no delete check: delete is `lib-ledger`'s reassign
//! (#847, Q6), not a port write.

use lib_core::{CategoryId, CategoryTypes};

use crate::fake::InMemoryCategories;
use crate::interactor::{AddCategory, MoveCategory, RenameCategory};
use crate::{Category, CategoryError, CategoryRepository, default_categories};

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
    let mut ports = factory.create(default_categories());

    let dividends = add("Dividends", None, CategoryTypes::Income)
        .run(ports.categories())
        .expect("a top-level Category");
    factory.commit(&mut ports);
    let franked = add("Franked", Some(dividends.id), CategoryTypes::Income)
        .run(ports.categories())
        .expect("Dividends takes children");
    factory.commit(&mut ports);
    let gas = add("Gas", Some(UTILITIES), CategoryTypes::Expense)
        .run(ports.categories())
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
    let mut ports = factory.create(default_categories());
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
        default_categories().len()
    );
}

/// A move reloads under its new parent, its children still under it; a move to top level reloads
/// with no parent.
pub fn a_moved_category_reloads_under_its_new_parent<F: PortsFactory>(factory: &mut F) {
    let mut ports = factory.create(default_categories());

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
    let mut ports = factory.create(default_categories());

    add("Insurance", Some(HOUSING), CategoryTypes::Expense)
        .run(ports.categories())
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
    let mut ports = factory.create(default_categories());
    let stored = ports.categories().categories();

    assert_eq!(
        add("Peak", Some(ELECTRICITY), CategoryTypes::Expense).run(ports.categories()),
        Err(CategoryError::DepthExceeded)
    );
    assert_eq!(
        add("Bonus", Some(SALARY), CategoryTypes::Expense).run(ports.categories()),
        Err(CategoryError::TypeMismatch)
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
    let mut ports = factory.create(default_categories());
    let stored = ports.categories().categories();

    factory.break_writes(&mut ports);
    add("Dividends", None, CategoryTypes::Income)
        .run(ports.categories())
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
}
