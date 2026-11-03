//! Restore budget: brings an archived Budget back.

use lib_core::BudgetId;

use super::support::{logged, write_set};
use crate::{BudgetRepository, Result};

/// Brings archived Budget `id` back.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RestoreBudget(pub BudgetId);

impl RestoreBudget {
    pub fn run(self, repo: &mut dyn BudgetRepository) -> Result<()> {
        logged(
            "restore budget",
            write_set(repo, |budgets| budgets.restore(self.0)),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interactor::test_support::*;

    #[test]
    fn restoring_brings_an_archived_budget_back_and_refuses_an_unknown_one() {
        let mut repo = repo();
        ArchiveBudget {
            id: JOINT,
            today: today(),
        }
        .run(&mut repo)
        .unwrap();

        RestoreBudget(JOINT).run(&mut repo).unwrap();

        assert_eq!(budget(&repo, JOINT).archived_at, None);
        assert_eq!(
            RestoreBudget(BudgetId::new()).run(&mut repo),
            Err(Error::NotFound)
        );
    }
}
