//! Set default budget: moves the default to another Budget.

use lib_core::BudgetId;

use super::support::{logged, write_set};
use crate::{BudgetRepository, Result};

/// Makes active Budget `id` the default, which unmarks the one that was.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SetDefaultBudget(pub BudgetId);

impl SetDefaultBudget {
    pub fn run(self, repo: &mut dyn BudgetRepository) -> Result<()> {
        logged(
            "set default budget",
            write_set(repo, |budgets| budgets.set_default(self.0)),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interactor::test_support::*;

    #[test]
    fn setting_the_default_moves_it_and_refuses_an_archived_budget() {
        let mut repo = repo();
        SetDefaultBudget(JOINT).run(&mut repo).unwrap();
        let defaults: Vec<BudgetId> = repo
            .budgets()
            .iter()
            .filter(|budget| budget.is_default)
            .map(|budget| budget.id)
            .collect();
        assert_eq!(defaults, [JOINT]);

        ArchiveBudget {
            id: PERSONAL_SPENDING_ID,
            today: today(),
        }
        .run(&mut repo)
        .unwrap();
        let before = repo.clone();
        assert_eq!(
            SetDefaultBudget(PERSONAL_SPENDING_ID).run(&mut repo),
            Err(Error::Archived)
        );
        assert_eq!(repo, before);
    }
}
