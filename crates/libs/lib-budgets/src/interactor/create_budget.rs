//! Create budget: adds a Budget (11c).

use chrono::NaiveDate;
use lib_core::BudgetId;

use super::support::{logged, write_set};
use crate::{BudgetReads, BudgetRepository, NewBudget, Result};

/// Creates the Budget `new` describes (11c) and returns its id.
#[derive(Debug, Clone, PartialEq)]
pub struct CreateBudget {
    pub new: NewBudget,
    pub today: NaiveDate,
}

impl CreateBudget {
    pub fn run(self, reads: &dyn BudgetReads, repo: &mut dyn BudgetRepository) -> Result<BudgetId> {
        logged(
            "create budget",
            write_set(repo, |budgets| budgets.create(&self.new, reads, self.today)),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interactor::test_support::*;

    #[test]
    fn creating_a_budget_writes_it_and_a_refusal_writes_nothing() {
        let mut repo = repo();
        let id = CreateBudget {
            new: holiday(),
            today: today(),
        }
        .run(&reads(), &mut repo)
        .unwrap();
        assert_eq!(budget(&repo, id).name, "Holiday");
        assert!(!budget(&repo, id).is_default);

        let before = repo.clone();
        let taken = CreateBudget {
            new: holiday(),
            today: today(),
        }
        .run(&reads(), &mut repo);
        assert_eq!(taken, Err(Error::NameTaken));
        let blank = CreateBudget {
            new: NewBudget {
                name: " ".into(),
                ..holiday()
            },
            today: today(),
        }
        .run(&reads(), &mut repo);
        assert_eq!(blank, Err(Error::NameRequired));
        assert_eq!(repo, before);
    }
}
