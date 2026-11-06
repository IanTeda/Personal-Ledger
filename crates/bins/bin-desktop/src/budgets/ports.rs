//! The Budgets store as the Desktop holds it (#952): the store `shell::router::build` is given,
//! shared by the Open Ledger, Category delete, the Accounts, Units and Categories usage
//! counts and the Categories Monthly budget, and the Budgets read ports the Budgets View reads the
//! other domains through. The writes are committed by the input they were made in
//! (`crate::input::send`), not here.
//!
//! The Budgets page and its Dialogs write the store through `lib-budgets`' Interactors on
//! [`DesktopPorts`] (#954, #955), as does the Categories Monthly budget ([`set_monthly_limit`]).
//! Category delete drops the Category's Limits through `lib-category-deletion`'s port (#1005).
//! After a write through these ports, [`announce_written`] tells the Views reading the Budgets
//! (#1028); the Categories Monthly budget and Category delete announce their own.

use std::cell::RefCell;
use std::rc::Rc;

use chrono::NaiveDate;
use gpui::{App, Entity};
use lib_budgets::{
    Budget, BudgetCategories, BudgetReads, BudgetRepository, BudgetsPorts, PortsMut,
    SetMonthlyLimit, SharedPorts, StopNewParent,
};
use lib_core::{BudgetId, CategoryId, Money};
use lib_database::FakeStores;
use lib_database::{BudgetsStore, SharedBudgetsStore, StoredBudgetReads};

use crate::announcer::{Announcer, Domain};

/// The Budgets store as the repository an Interactor writes through. It forwards rather than
/// lending the store's own: the store is shared behind a `RefCell` whose guard can't be lent out
/// of it. It notes each write, so the Budgets are announced only after an event that wrote them
/// ([`announce_written`]).
pub struct Store {
    rows: SharedBudgetsStore,
    wrote: bool,
}

impl BudgetRepository for Store {
    fn budgets(&self) -> Vec<Budget> {
        self.rows.borrow().budgets().budgets()
    }

    fn budget(&self, id: BudgetId) -> Option<Budget> {
        self.rows.borrow().budgets().budget(id)
    }

    fn upsert_budget(&mut self, budget: &Budget) {
        self.wrote = true;
        self.rows.borrow_mut().budgets_mut().upsert_budget(budget);
    }
}

/// The Budgets ports as the Desktop holds them: the store the Open Ledger shares, and the other
/// domains read through their stores (#952).
pub struct StoredPorts {
    pub budgets: Store,
    pub reads: Rc<StoredBudgetReads>,
}

impl BudgetsPorts for StoredPorts {
    fn budgets(&self) -> &dyn BudgetRepository {
        &self.budgets
    }

    fn reads(&self) -> &dyn BudgetReads {
        &*self.reads
    }

    fn split_mut(&mut self) -> PortsMut<'_> {
        PortsMut {
            budgets: &mut self.budgets,
            reads: &*self.reads,
        }
    }
}

/// The Budgets ports the page and its Dialogs share.
#[derive(Clone)]
pub struct DesktopPorts {
    pub shared: Rc<RefCell<StoredPorts>>,
}

impl DesktopPorts {
    pub fn new(store: SharedBudgetsStore, reads: Rc<StoredBudgetReads>) -> Self {
        Self {
            shared: Rc::new(RefCell::new(StoredPorts {
                budgets: Store {
                    rows: store,
                    wrote: false,
                },
                reads,
            })),
        }
    }

    /// The ports as the Presenters hold them.
    pub fn presenter(&self) -> SharedPorts {
        self.shared.clone()
    }

    /// The other domains' reads, for the Dashboard's figures.
    pub fn reads(&self) -> Rc<StoredBudgetReads> {
        self.shared.borrow().reads.clone()
    }
}

/// Announces `Domain::Budgets` after a Budgets event or Dialog input, but only when it wrote
/// through `ports`, so the Dashboard and Budgets redraw on a write (#1028) and not on a cursor
/// move or a keystroke into a cell.
pub fn announce_written(ports: &DesktopPorts, announcer: &Entity<Announcer>, cx: &mut App) {
    let wrote = std::mem::take(&mut ports.shared.borrow_mut().budgets.wrote);
    if wrote {
        announcer.update(cx, |announcer, cx| announcer.announce(Domain::Budgets, cx));
    }
}

/// `stub`'s Budgets in memory, for a test that needs no storage.
pub fn in_memory(stub: &FakeStores) -> SharedBudgetsStore {
    Rc::new(RefCell::new(stub.budgets.clone()))
}

/// Budget `id`'s Monthly budget for Category `category`, as `lib-budgets`' `SetMonthlyLimit`
/// Interactor writes it, straight to `store`: the Categories Add and Edit Dialogs' save, which the input's
/// one commit joins with the Category's. Returns whether it wrote.
///
/// # Errors
/// Returns the rule's refusal; nothing is written then.
pub fn set_monthly_limit(
    store: &mut dyn BudgetsStore,
    categories: &dyn BudgetCategories,
    id: BudgetId,
    category: CategoryId,
    amount: Option<Money>,
    today: NaiveDate,
) -> Result<bool, lib_budgets::Error> {
    SetMonthlyLimit {
        id,
        category_id: category,
        amount,
        today,
    }
    .run(categories, store.budgets_mut())
}

/// Stops `category`, a leaf that just gained a child, from the current month in every active
/// Budget in `store`, as `lib-budgets`' `StopNewParent` Interactor writes it: the Categories Add
/// and Edit Dialogs' save, joined to the Category's one commit. Returns how many Budgets changed.
pub fn stop_new_parent(
    store: &mut dyn BudgetsStore,
    category: CategoryId,
    today: NaiveDate,
) -> usize {
    StopNewParent {
        category_id: category,
        today,
    }
    .run(store.budgets_mut())
}

#[cfg(test)]
mod tests {
    use lib_budgets::{PERSONAL_SPENDING_ID, applied};
    use lib_core::{CategoryId, Period};

    use super::*;

    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 9, 21).unwrap_or_default()
    }

    #[test]
    fn a_monthly_limit_is_written_to_the_store_by_the_budgets_rule() {
        let store = in_memory(&FakeStores::sample(today()));
        let categories = lib_database::budget_categories(&lib_categories::default_categories());
        let groceries = CategoryId::seed(7);

        let wrote = set_monthly_limit(
            &mut *store.borrow_mut(),
            &categories,
            PERSONAL_SPENDING_ID,
            groceries,
            Some(lib_budgets::cents_money(61_000)),
            today(),
        );

        assert_eq!(wrote, Ok(true));
        let personal = store
            .borrow()
            .budgets()
            .budget(PERSONAL_SPENDING_ID)
            .expect("the stub's default");
        assert_eq!(
            applied(personal.chain(groceries), Period::of(today())).map(|found| found.amount),
            Some(lib_budgets::cents_money(61_000))
        );
        let refused = set_monthly_limit(
            &mut *store.borrow_mut(),
            &categories,
            PERSONAL_SPENDING_ID,
            CategoryId::seed(6),
            Some(lib_budgets::cents_money(1)),
            today(),
        );
        assert_eq!(
            refused,
            Err(lib_budgets::Error::NotAnExpenseLeaf),
            "Food is a parent"
        );
    }
}
