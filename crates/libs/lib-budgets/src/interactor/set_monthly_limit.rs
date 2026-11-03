//! Set monthly limit: Categories 5c's write.

use chrono::NaiveDate;
use lib_core::{BudgetId, CategoryId, Money};

use super::support::{logged, write_budget};
use crate::{BudgetCategories, BudgetRepository, Result};

/// Categories 5c's write: sets (or, with `None`, clears) a Category's Monthly budget from the
/// current month; returns whether it wrote.
#[derive(Debug, Clone, PartialEq)]
pub struct SetMonthlyLimit {
    pub id: BudgetId,
    pub category_id: CategoryId,
    pub amount: Option<Money>,
    pub today: NaiveDate,
}

impl SetMonthlyLimit {
    pub fn run(
        self,
        categories: &dyn BudgetCategories,
        repo: &mut dyn BudgetRepository,
    ) -> Result<bool> {
        logged(
            "set monthly limit",
            write_budget(repo, self.id, |budget| {
                budget.set_monthly_limit(categories, self.category_id, self.amount, self.today)
            }),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interactor::test_support::*;

    #[test]
    fn setting_a_monthly_limit_writes_an_onward_and_an_unchanged_value_writes_nothing() {
        let mut repo = repo();
        let limit = |amount| SetMonthlyLimit {
            id: PERSONAL_SPENDING_ID,
            category_id: GROCERIES,
            amount,
            today: today(),
        };

        assert_eq!(
            limit(Some(cents_money(61_000))).run(&reads(), &mut repo),
            Ok(true)
        );
        assert_eq!(
            amount_in(&repo, PERSONAL_SPENDING_ID, current()),
            Some(cents_money(61_000))
        );
        let before = repo.clone();
        assert_eq!(
            limit(Some(cents_money(61_000))).run(&reads(), &mut repo),
            Ok(false)
        );
        assert_eq!(repo, before);
        assert_eq!(limit(None).run(&reads(), &mut repo), Ok(true));
        assert_eq!(amount_in(&repo, PERSONAL_SPENDING_ID, current()), None);
        let parent = SetMonthlyLimit {
            category_id: FOOD,
            ..limit(Some(cents_money(1)))
        }
        .run(&reads(), &mut repo);
        assert_eq!(parent, Err(Error::NotAnExpenseLeaf));
    }
}
