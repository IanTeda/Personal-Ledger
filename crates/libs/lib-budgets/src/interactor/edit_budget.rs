//! Edit budget: renames a Budget and changes its Accounts (11c's edit mode).

use lib_core::{AccountId, BudgetId};

use super::support::{logged, write_set};
use crate::{BudgetAccounts, BudgetRepository, Result};

/// Saves Budget `id`'s name and Accounts (11c's edit mode).
#[derive(Debug, Clone, PartialEq)]
pub struct EditBudget {
    pub id: BudgetId,
    pub name: String,
    pub account_ids: Vec<AccountId>,
}

impl EditBudget {
    pub fn run(self, accounts: &dyn BudgetAccounts, repo: &mut dyn BudgetRepository) -> Result<()> {
        logged(
            "edit budget",
            write_set(repo, |budgets| {
                budgets.edit(self.id, &self.name, self.account_ids, accounts)
            }),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interactor::test_support::*;

    #[test]
    fn editing_a_budget_renames_it_and_refuses_a_taken_name_or_an_archived_budget() {
        let mut repo = repo();
        EditBudget {
            id: JOINT,
            name: "Shared house".into(),
            account_ids: vec![ANZ_EVERYDAY],
        }
        .run(&reads(), &mut repo)
        .unwrap();
        assert_eq!(budget(&repo, JOINT).name, "Shared house");
        assert_eq!(budget(&repo, JOINT).account_ids, [ANZ_EVERYDAY]);

        let before = repo.clone();
        let taken = EditBudget {
            id: JOINT,
            name: "personal spending".into(),
            account_ids: vec![],
        }
        .run(&reads(), &mut repo);
        assert_eq!(taken, Err(Error::NameTaken));
        ArchiveBudget {
            id: JOINT,
            today: today(),
        }
        .run(&mut repo)
        .unwrap();
        let archived = EditBudget {
            id: JOINT,
            name: "Again".into(),
            account_ids: vec![],
        }
        .run(&reads(), &mut repo);
        assert_eq!(archived, Err(Error::Archived));
        assert_ne!(repo, before, "only the archive landed");
        assert_eq!(budget(&repo, JOINT).name, "Shared house");
    }

    #[test]
    fn an_edit_drops_the_id_of_an_account_deleted_since_the_budget_named_it() {
        let mut repo = repo();
        let deleted = AccountId::new();
        EditBudget {
            id: JOINT,
            name: "Joint".into(),
            account_ids: vec![ANZ_EVERYDAY, deleted],
        }
        .run(&reads(), &mut repo)
        .unwrap();
        assert_eq!(budget(&repo, JOINT).account_ids, [ANZ_EVERYDAY]);
    }
}
