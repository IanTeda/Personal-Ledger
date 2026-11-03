//! Set amount: writes a Category's Budget Amount (9e and the Plan grid).

use chrono::NaiveDate;
use lib_core::{BudgetId, CategoryId, Money, Period};

use super::support::{logged, write_budget};
use crate::{BudgetCategories, BudgetRepository, Result, Rollover, Span};

/// Writes a Category's Budget Amount from `month` (9e and the Plan grid).
#[derive(Debug, Clone, PartialEq)]
pub struct SetAmount {
    pub id: BudgetId,
    pub category_id: CategoryId,
    pub month: Period,
    pub span: Span,
    pub amount: Money,
    /// `None` keeps the Rollover the month already inherits.
    pub rollover: Option<Rollover>,
    pub today: NaiveDate,
}

impl SetAmount {
    pub fn run(
        self,
        categories: &dyn BudgetCategories,
        repo: &mut dyn BudgetRepository,
    ) -> Result<()> {
        logged(
            "set amount",
            write_budget(repo, self.id, |budget| {
                budget.set_amount(
                    categories,
                    self.category_id,
                    self.month,
                    self.span,
                    self.amount,
                    self.rollover,
                    self.today,
                )
            }),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interactor::test_support::*;

    #[test]
    fn setting_an_amount_writes_the_record_and_refuses_what_the_rules_do() {
        let mut repo = repo();
        set(Span::MonthOnly, current().next(), 12_345)
            .run(&reads(), &mut repo)
            .unwrap();
        assert_eq!(
            amount_in(&repo, PERSONAL_SPENDING_ID, current().next()),
            Some(cents_money(12_345))
        );

        let before = repo.clone();
        let closed = set(Span::Onward, current().prev(), 100).run(&reads(), &mut repo);
        assert_eq!(closed, Err(Error::ClosedMonth));
        let negative = set(Span::Onward, current(), -1).run(&reads(), &mut repo);
        assert_eq!(negative, Err(Error::NegativeAmount));
        let parent = SetAmount {
            category_id: FOOD,
            ..set(Span::Onward, current(), 100)
        }
        .run(&reads(), &mut repo);
        assert_eq!(parent, Err(Error::NotAnExpenseLeaf));
        let unknown = SetAmount {
            id: BudgetId::new(),
            ..set(Span::Onward, current(), 100)
        }
        .run(&reads(), &mut repo);
        assert_eq!(unknown, Err(Error::NotFound));
        assert_eq!(repo, before);
    }

    #[test]
    fn an_archived_budget_refuses_every_chain_edit() {
        let mut repo = repo();
        ArchiveBudget {
            id: JOINT,
            today: today(),
        }
        .run(&mut repo)
        .unwrap();
        let before = repo.clone();

        let set = SetAmount {
            id: JOINT,
            ..set(Span::Onward, current(), 100)
        }
        .run(&reads(), &mut repo);
        let stop = StopBudgeting {
            id: JOINT,
            category_id: GROCERIES,
            month: current(),
            today: today(),
        }
        .run(&reads(), &mut repo);
        let cycle = CycleRollover {
            id: JOINT,
            category_id: GROCERIES,
            today: today(),
        }
        .run(&mut repo);

        assert_eq!(set, Err(Error::Archived));
        assert_eq!(stop, Err(Error::Archived));
        assert_eq!(cycle, Err(Error::Archived));
        assert_eq!(repo, before);
    }
}
