//! Cycle rollover: the Plan grid's `r`.

use chrono::NaiveDate;
use lib_core::{BudgetId, CategoryId};

use super::support::{logged, write_budget};
use crate::{BudgetRepository, Result, Rollover};

/// Moves a Category's current-month Rollover on to the next choice and returns it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CycleRollover {
    pub id: BudgetId,
    pub category_id: CategoryId,
    pub today: NaiveDate,
}

impl CycleRollover {
    pub fn run(self, repo: &mut dyn BudgetRepository) -> Result<Rollover> {
        logged(
            "cycle rollover",
            write_budget(repo, self.id, |budget| {
                budget.cycle_rollover(self.category_id, self.today)
            }),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interactor::test_support::*;

    #[test]
    fn cycling_the_rollover_walks_the_choices_and_refuses_an_unbudgeted_category() {
        let mut repo = repo();
        let cycle = CycleRollover {
            id: PERSONAL_SPENDING_ID,
            category_id: GROCERIES,
            today: today(),
        };
        let first = cycle.run(&mut repo).unwrap();
        let second = cycle.run(&mut repo).unwrap();
        assert_eq!(second, first.next());
        assert_eq!(
            applied(
                budget(&repo, PERSONAL_SPENDING_ID).chain(GROCERIES),
                current()
            )
            .map(|found| found.rollover),
            Some(second)
        );

        let before = repo.clone();
        let unbudgeted = CycleRollover {
            id: JOINT,
            category_id: RENT,
            ..cycle
        }
        .run(&mut repo);
        assert_eq!(unbudgeted, Err(Error::NotBudgeted));
        assert_eq!(repo, before, "a refusal adds no empty chain");
    }
}
