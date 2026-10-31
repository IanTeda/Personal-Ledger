//! The Budgets store as the Desktop holds it (#952): the store `shell::router::build` is given,
//! shared by the Ledger's forwarding shim and read copy, the Accounts, Units and Categories usage
//! counts and the Categories Monthly budget, and the Budgets read ports the Budgets View reads the
//! other domains through. The writes are committed by the input they were made in
//! (`crate::input::send`), not here.
//!
//! Every Budget writer but the Categories Monthly budget is still a `LedgerCommand` (#954 moves them onto the Interactors #953 added), so
//! [`forward`] carries each Budget a command wrote to the store. The Categories Monthly budget
//! writes the store itself ([`set_monthly_limit`]).

use std::cell::RefCell;
use std::rc::Rc;

use chrono::NaiveDate;
use lib_budgets::{BudgetCategories, BudgetError, SetMonthlyLimit, fake::InMemoryBudgets};
use lib_core::{BudgetId, CategoryId, Money};
use lib_database::{BudgetsStore, SharedBudgetsStore};
use lib_ledger::{Ledger, LedgerChange};

/// `ledger`'s own Budgets in memory, for a test that needs no storage.
pub fn in_memory(ledger: &Ledger) -> SharedBudgetsStore {
    Rc::new(RefCell::new(InMemoryBudgets {
        budgets: ledger.budgets().all().to_vec(),
    }))
}

/// The forwarding shim (#952): sends each Budget `changes` upserts to `store`, in the order
/// recorded, returning whether it wrote anything. Run before the Categories' deletes, since
/// Category reassign clears the chains naming the Category it deletes (their foreign key). Writers
/// carried: every Budgets `LedgerCommand`, until the View moves onto the Interactors (#954), and Category
/// reassign, until `lib-reassign` (#813).
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
