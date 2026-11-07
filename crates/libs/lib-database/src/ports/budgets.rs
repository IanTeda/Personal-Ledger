//! `lib-budgets`' ports answered over the stores: the Budgets from the Budgets store, and the
//! other domains' rows their figures count through [`StoredBudgetReads`] (#952, #1083).

use std::rc::Rc;

use lib_budgets::{Budget, BudgetReads, BudgetRepository, BudgetsPorts, PortsMut};
use lib_core::BudgetId;

use crate::adapter::store::{SharedBudgetsStore, StoredBudgetReads};

/// The Budgets store as the repository an Interactor writes through. It forwards rather than
/// lending the store's own: the store is shared behind a `RefCell` whose guard can't be lent out
/// of it.
pub struct StoredBudgetRepository {
    rows: SharedBudgetsStore,
}

impl BudgetRepository for StoredBudgetRepository {
    fn budgets(&self) -> Vec<Budget> {
        self.rows.borrow().budgets().budgets()
    }

    fn budget(&self, id: BudgetId) -> Option<Budget> {
        self.rows.borrow().budgets().budget(id)
    }

    fn upsert_budget(&mut self, budget: &Budget) {
        self.rows.borrow_mut().budgets_mut().upsert_budget(budget);
    }
}

/// [`BudgetsPorts`] over the shared Budgets store, and the reads over the stores of the domains
/// the figures count. A write reaches the store's recorder, so the commit names the Budgets.
pub struct StoredBudgetsPorts {
    budgets: StoredBudgetRepository,
    reads: Rc<StoredBudgetReads>,
}

impl StoredBudgetsPorts {
    pub fn new(store: SharedBudgetsStore, reads: Rc<StoredBudgetReads>) -> Self {
        Self {
            budgets: StoredBudgetRepository { rows: store },
            reads,
        }
    }
}

impl BudgetsPorts for StoredBudgetsPorts {
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

#[cfg(test)]
mod tests {
    use lib_budgets::{Budget, PERSONAL_SPENDING_ID};

    use crate::ports::test_support::{fakes, file};
    use crate::{Domain, LedgerPorts};

    #[test]
    fn a_budget_write_reports_the_budgets_written_on_either() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        for ledger in [fakes(), file(dir.path())] {
            let LedgerPorts {
                budgets, commit, ..
            } = ledger;
            let mut budget: Budget = budgets
                .borrow()
                .budgets()
                .budget(PERSONAL_SPENDING_ID)
                .expect("the Personal spending Budget");
            budget.name.push_str(" renamed");

            budgets
                .borrow_mut()
                .split_mut()
                .budgets
                .upsert_budget(&budget);

            assert_eq!(commit.commit().wrote, [Domain::Budgets]);
        }
    }

    #[test]
    fn a_budgets_read_reports_nothing_written() {
        let ledger = fakes();

        let _ = ledger.budgets.borrow().budgets().budgets();
        let _ = ledger.budgets.borrow().reads().budget_accounts();

        assert!(ledger.commit.commit().is_empty());
    }
}
