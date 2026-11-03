//! Clear month only: removes a Category's Month-only record.

use chrono::NaiveDate;
use lib_core::{BudgetId, CategoryId, Period};

use super::support::{logged, write_budget};
use crate::{BudgetRepository, Result};

/// Removes a Category's Month-only record in `month`; returns whether there was one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClearMonthOnly {
    pub id: BudgetId,
    pub category_id: CategoryId,
    pub month: Period,
    pub today: NaiveDate,
}

impl ClearMonthOnly {
    pub fn run(self, repo: &mut dyn BudgetRepository) -> Result<bool> {
        logged(
            "clear month only",
            write_budget(repo, self.id, |budget| {
                budget.clear_month_only(self.category_id, self.month, self.today)
            }),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interactor::test_support::*;

    #[test]
    fn clearing_a_month_only_removes_it_and_says_whether_there_was_one() {
        let mut repo = repo();
        let clear = ClearMonthOnly {
            id: PERSONAL_SPENDING_ID,
            category_id: GROCERIES,
            month: current().next(),
            today: today(),
        };
        assert_eq!(clear.run(&mut repo), Ok(false));

        set(Span::MonthOnly, current().next(), 5_000)
            .run(&reads(), &mut repo)
            .unwrap();
        assert_eq!(clear.run(&mut repo), Ok(true));
        assert_eq!(
            amount_in(&repo, PERSONAL_SPENDING_ID, current().next()),
            amount_in(&repo, PERSONAL_SPENDING_ID, current()),
            "the month falls back to the carried Onward"
        );
        let closed = ClearMonthOnly {
            month: current().prev(),
            ..clear
        }
        .run(&mut repo);
        assert_eq!(closed, Err(Error::ClosedMonth));
    }
}
