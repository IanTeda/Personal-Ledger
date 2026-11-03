//! Archive budget: makes a Budget read-only.

use chrono::NaiveDate;
use lib_core::BudgetId;

use super::support::{logged, write_set};
use crate::{BudgetRepository, Result};

/// Archives Budget `id`, which makes it read-only; never the default.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArchiveBudget {
    pub id: BudgetId,
    pub today: NaiveDate,
}

impl ArchiveBudget {
    pub fn run(self, repo: &mut dyn BudgetRepository) -> Result<()> {
        logged(
            "archive budget",
            write_set(repo, |budgets| budgets.archive(self.id, self.today)),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interactor::test_support::*;

    #[test]
    fn archiving_never_touches_the_default_and_dates_another_budget() {
        let mut repo = repo();
        let before = repo.clone();
        assert_eq!(
            ArchiveBudget {
                id: PERSONAL_SPENDING_ID,
                today: today()
            }
            .run(&mut repo),
            Err(Error::DefaultCannotBeArchived)
        );
        assert_eq!(repo, before);

        ArchiveBudget {
            id: JOINT,
            today: today(),
        }
        .run(&mut repo)
        .unwrap();
        assert_eq!(budget(&repo, JOINT).archived_at, Some(today()));
    }
}
