//! Duplicate budget: copies a Budget and its chains.

use lib_core::BudgetId;

use super::support::{logged, write_set};
use crate::{BudgetRepository, Result};

/// Copies Budget `id` and returns the copy's id.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DuplicateBudget(pub BudgetId);

impl DuplicateBudget {
    pub fn run(self, repo: &mut dyn BudgetRepository) -> Result<BudgetId> {
        logged(
            "duplicate budget",
            write_set(repo, |budgets| budgets.duplicate(self.0)),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interactor::test_support::*;

    #[test]
    fn duplicating_copies_the_chains_as_a_numbered_non_default_budget() {
        let mut repo = repo();
        let first = DuplicateBudget(PERSONAL_SPENDING_ID)
            .run(&mut repo)
            .unwrap();
        let second = DuplicateBudget(PERSONAL_SPENDING_ID)
            .run(&mut repo)
            .unwrap();

        assert_eq!(budget(&repo, first).name, "Personal spending copy");
        assert_eq!(budget(&repo, second).name, "Personal spending copy 2");
        assert!(!budget(&repo, first).is_default);
        assert_eq!(
            budget(&repo, first).limits,
            budget(&repo, PERSONAL_SPENDING_ID).limits
        );
        assert_eq!(
            DuplicateBudget(BudgetId::new()).run(&mut repo),
            Err(Error::NotFound)
        );
    }
}
