//! Stop new parent: Stops a Category that gained a child, in every active Budget.

use chrono::NaiveDate;
use lib_core::CategoryId;

use crate::{BudgetRepository, Budgets};

/// A budgeted leaf that gained a child is Stopped from the current month in every active Budget,
/// and loses any later record; returns how many Budgets it changed. Closed months keep their
/// amounts, archived Budgets are untouched, and a Category budgeted nowhere writes nothing. The
/// caller says which Category gained the child.
#[derive(Debug, Clone, PartialEq)]
pub struct StopNewParent {
    pub category_id: CategoryId,
    pub today: NaiveDate,
}

impl StopNewParent {
    pub fn run(self, repo: &mut dyn BudgetRepository) -> usize {
        let before = Budgets::from_stored(repo.budgets());
        let mut after = before.clone();
        after.stop_new_parent(self.category_id, self.today);
        let changed = after.changed_since(&before);
        for budget in &changed {
            repo.upsert_budget(budget);
        }
        tracing::info!(category_id = %self.category_id, budgets = changed.len(), "Stopped a new parent");
        changed.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interactor::test_support::*;

    const DINING: CategoryId = CategoryId::seed(8);

    fn stop(category_id: CategoryId) -> StopNewParent {
        StopNewParent {
            category_id,
            today: today(),
        }
    }

    fn chain(repo: &InMemoryBudgets, id: BudgetId, category: CategoryId) -> Vec<LimitRecord> {
        budget(repo, id).chain(category).to_vec()
    }

    #[test]
    fn a_new_parent_is_stopped_from_this_month_in_every_active_budget() {
        let mut repo = repo();
        let kept: Vec<_> = chain(&repo, PERSONAL_SPENDING_ID, GROCERIES)
            .into_iter()
            .filter(|record| record.month < current())
            .collect();

        assert_eq!(stop(GROCERIES).run(&mut repo), 2);

        for id in [PERSONAL_SPENDING_ID, JOINT] {
            assert_eq!(amount_in(&repo, id, current()), None);
            assert!(amount_in(&repo, id, current().prev()).is_some());
            assert_eq!(
                chain(&repo, id, GROCERIES).last(),
                Some(&LimitRecord {
                    month: current(),
                    limit: Limit::Stop
                })
            );
        }
        let after = chain(&repo, PERSONAL_SPENDING_ID, GROCERIES);
        assert_eq!(after[..after.len() - 1], kept[..]);
    }

    #[test]
    fn a_stop_drops_the_records_after_this_month() {
        let mut repo = repo();
        assert!(
            chain(&repo, PERSONAL_SPENDING_ID, DINING)
                .iter()
                .any(|record| record.month > current()),
            "Dining's seed changes next month"
        );

        stop(DINING).run(&mut repo);

        assert!(
            chain(&repo, PERSONAL_SPENDING_ID, DINING)
                .iter()
                .all(|record| record.month <= current())
        );
        assert_eq!(amount_in_for(&repo, DINING, current().next()), None);
    }

    fn amount_in_for(repo: &InMemoryBudgets, category: CategoryId, month: Period) -> Option<Money> {
        applied(&chain(repo, PERSONAL_SPENDING_ID, category), month).map(|found| found.amount)
    }

    #[test]
    fn an_archived_budget_and_an_unbudgeted_category_are_left_alone() {
        let mut repo = repo();
        ArchiveBudget {
            id: JOINT,
            today: today(),
        }
        .run(&mut repo)
        .unwrap();
        let joint = budget(&repo, JOINT);

        assert_eq!(stop(GROCERIES).run(&mut repo), 1);
        assert_eq!(budget(&repo, JOINT), joint);

        let stopped = repo.clone();
        assert_eq!(
            stop(CategoryId::seed(11)).run(&mut repo),
            0,
            "Salary is unbudgeted"
        );
        assert_eq!(stop(GROCERIES).run(&mut repo), 0, "already Stopped");
        assert_eq!(repo, stopped);
    }

    #[test]
    fn budgeting_counts_the_active_budgets_holding_an_amount_this_month() {
        let mut repo = repo();
        let count = |repo: &InMemoryBudgets| {
            Budgets::from_stored(repo.budgets()).budgeting(GROCERIES, today())
        };
        assert_eq!(count(&repo), 2);
        ArchiveBudget {
            id: JOINT,
            today: today(),
        }
        .run(&mut repo)
        .unwrap();
        assert_eq!(count(&repo), 1);
        stop(GROCERIES).run(&mut repo);
        assert_eq!(count(&repo), 0);
    }
}
