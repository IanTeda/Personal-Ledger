//! The Budgets store as the Desktop holds it (#952): the store `shell::router::build` is given,
//! shared by the Ledger's forwarding shim and read copy, the Accounts, Units and Categories usage
//! counts and the Categories Monthly budget, and the Budgets read ports the Budgets View reads the
//! other domains through. The writes are committed by the input they were made in
//! (`crate::input::send`), not here.
//!
//! The Budgets page and its Dialogs write the store through `lib-budgets`' Interactors on
//! [`DesktopPorts`] (#954, #955), as does the Categories Monthly budget ([`set_monthly_limit`]).
//! The one writer left on the Ledger's forwarding shim is Category reassign, so [`forward`] carries
//! the Budgets it writes to the store. After either, [`refresh`] brings the Ledger's read copy in
//! line, for the Dashboard and the rail badge.

use std::cell::RefCell;
use std::rc::Rc;

use chrono::NaiveDate;
use gpui::{App, Entity};
use lib_budgets::{
    Budget, BudgetCategories, BudgetError, BudgetReads, BudgetRepository, BudgetsPorts, PortsMut,
    SetMonthlyLimit, SharedPorts, StopNewParent, fake::InMemoryBudgets,
};
use lib_core::{BudgetId, CategoryId, Money};
use lib_database::{BudgetsStore, SharedBudgetsStore, StoredBudgetReads};
use lib_ledger::{Ledger, LedgerChange};

use crate::ledger_store::LedgerStore;

/// The Budgets store as the repository an Interactor writes through. It forwards rather than
/// lending the store's own: the store is shared behind a `RefCell` whose guard can't be lent out
/// of it.
pub struct Store(pub SharedBudgetsStore);

impl BudgetRepository for Store {
    fn budgets(&self) -> Vec<Budget> {
        self.0.borrow().budgets().budgets()
    }

    fn budget(&self, id: BudgetId) -> Option<Budget> {
        self.0.borrow().budgets().budget(id)
    }

    fn upsert_budget(&mut self, budget: &Budget) {
        self.0.borrow_mut().budgets_mut().upsert_budget(budget);
    }
}

/// The Budgets ports as the Desktop holds them: the store `LedgerStore` shares, and the other
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
                budgets: Store(store),
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

/// Brings the Ledger's Budgets read copy in line with the store after a Budgets write, announcing
/// `Domain::Budgets` when it changed.
pub fn refresh(ledger: &Entity<LedgerStore>, cx: &mut App) {
    ledger.update(cx, |store, cx| store.refresh_budgets(cx));
}

/// `ledger`'s own Budgets in memory, for a test that needs no storage.
pub fn in_memory(ledger: &Ledger) -> SharedBudgetsStore {
    Rc::new(RefCell::new(InMemoryBudgets {
        budgets: ledger.budgets().all().to_vec(),
    }))
}

/// The forwarding shim (#952): sends each Budget `changes` upserts to `store`, in the order
/// recorded, returning whether it wrote anything. Run before the Categories' deletes, since
/// Category reassign clears the chains naming the Category it deletes (their foreign key). The one
/// writer carried is Category reassign, until `lib-reassign` (#813).
pub fn forward(store: &mut dyn BudgetsStore, changes: &[LedgerChange]) -> bool {
    let port = store.budgets_mut();
    let mut wrote = false;
    for change in changes {
        if let LedgerChange::UpsertBudget(budget) = change {
            port.upsert_budget(budget);
            wrote = true;
        }
    }
    wrote
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
) -> Result<bool, BudgetError> {
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
    use lib_core::{Period, TagId};

    use super::*;

    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 9, 21).unwrap_or_default()
    }

    #[test]
    fn budget_upserts_reach_the_store_and_nothing_else_does() {
        let ledger = Ledger::stub_on(today());
        let store = in_memory(&ledger);
        let mut joint = ledger.budgets().all()[1].clone();
        joint.name = "Shared house".into();
        let changes = [
            LedgerChange::UpsertBudget(joint.clone()),
            LedgerChange::DeleteTag(TagId::seed(1)),
        ];

        assert!(forward(&mut *store.borrow_mut(), &changes));
        assert_eq!(store.borrow().budgets().budget(joint.id), Some(joint));
        assert!(!forward(
            &mut *store.borrow_mut(),
            &[LedgerChange::DeleteTag(TagId::seed(1))]
        ));
    }

    #[test]
    fn a_monthly_limit_is_written_to_the_store_by_the_budgets_rule() {
        let ledger = Ledger::stub_on(today());
        let store = in_memory(&ledger);
        let categories = lib_ledger::budget_categories(ledger.categories());
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
            Err(BudgetError::NotAnExpenseLeaf),
            "Food is a parent"
        );
    }
}
