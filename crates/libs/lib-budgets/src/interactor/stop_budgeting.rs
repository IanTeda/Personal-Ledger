//! Stop budgeting: writes a Stop on a Category from a month.

use chrono::NaiveDate;
use lib_core::{BudgetId, CategoryId, Period};

use super::support::{logged, write_budget};
use crate::{BudgetCategories, BudgetRepository, Result};

/// Stops budgeting a Category from `month` on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StopBudgeting {
    pub id: BudgetId,
    pub category_id: CategoryId,
    pub month: Period,
    pub today: NaiveDate,
}

impl StopBudgeting {
    pub fn run(
        self,
        categories: &dyn BudgetCategories,
        repo: &mut dyn BudgetRepository,
    ) -> Result<()> {
        logged(
            "stop budgeting",
            write_budget(repo, self.id, |budget| {
                budget.stop(categories, self.category_id, self.month, self.today)
            }),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interactor::test_support::*;

    #[test]
    fn stopping_writes_a_stop_and_an_unbudgeted_category_writes_nothing() {
        let mut repo = repo();
        StopBudgeting {
            id: PERSONAL_SPENDING_ID,
            category_id: GROCERIES,
            month: current().next(),
            today: today(),
        }
        .run(&reads(), &mut repo)
        .unwrap();
        assert_eq!(
            budget(&repo, PERSONAL_SPENDING_ID)
                .chain(GROCERIES)
                .last()
                .map(|record| &record.limit),
            Some(&Limit::Stop)
        );

        let before = repo.clone();
        StopBudgeting {
            id: JOINT,
            category_id: RENT,
            month: current(),
            today: today(),
        }
        .run(&reads(), &mut repo)
        .unwrap();
        assert_eq!(repo, before);
        assert_eq!(
            StopBudgeting {
                id: PERSONAL_SPENDING_ID,
                category_id: FOOD,
                month: current(),
                today: today(),
            }
            .run(&reads(), &mut repo),
            Err(Error::NotAnExpenseLeaf)
        );
    }
}
