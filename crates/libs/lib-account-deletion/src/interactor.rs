//! Account delete, the module's one Interactor: it removes an Account, the Transactions booked to
//! it and the Paid links those Transactions held, three domains, so it lives here rather than in
//! `lib-accounts`. It checks everything before writing, so a refusal writes nothing.
//!
//! A Budget that counted the Account keeps its id: `lib-budgets` ignores an id with no Account
//! until Budgets persist with a cascading link (#811).

use lib_bills::unmatch_transaction;
use lib_core::AccountId;

use crate::error::{Error, Result};
use crate::protocols::{AccountDeleted, AccountDeletion, AccountRemoval};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeleteAccount {
    pub id: AccountId,
}

impl DeleteAccount {
    /// Deletes the Account and the Transactions booked to it, and Unmatches the Bill Schedule
    /// entries they paid, so those entries return to Needs Attention.
    ///
    /// # Errors
    /// `NotFound` for an Account that is gone, and `PaidFromByBillPlans` while a Bill Plan pays
    /// from it.
    pub fn run(self, deletion: &mut dyn AccountDeletion) -> Result<AccountDeleted> {
        let outcome = self.apply(deletion);
        match &outcome {
            Ok(deleted) => tracing::info!(
                account_id = %self.id,
                transactions = deleted.transactions,
                unmatched = deleted.unmatched,
                "Deleted an Account"
            ),
            Err(refusal) => {
                tracing::info!(account_id = %self.id, %refusal, "Refused to delete an Account");
            }
        }
        outcome
    }

    fn apply(self, deletion: &mut dyn AccountDeletion) -> Result<AccountDeleted> {
        let account = deletion.account(self.id).ok_or(Error::NotFound)?;
        let plans = deletion.bill_plans_paying_from(self.id);
        if plans > 0 {
            return Err(Error::PaidFromByBillPlans { plans });
        }
        let transactions = deletion.transactions_booked_to(self.id);
        let before = deletion.bill_entries();
        let mut after = before.clone();
        for transaction in &transactions {
            unmatch_transaction(&mut after, *transaction);
        }
        let unmatched: Vec<_> = after
            .into_iter()
            .zip(before)
            .filter(|(after, before)| after != before)
            .map(|(after, _)| after)
            .collect();
        let deleted = AccountDeleted {
            id: self.id,
            name: account.name,
            transactions: transactions.len(),
            unmatched: unmatched.len(),
        };
        deletion.remove_account(AccountRemoval {
            account: self.id,
            transactions,
            unmatched,
        });
        Ok(deleted)
    }
}

#[cfg(test)]
mod tests {
    use lib_accounts::AccountRepository as _;
    use lib_bills::{BillPlanRepository as _, Resolution, entry};
    use lib_transactions::TransactionRepository as _;

    use super::*;
    use crate::fake::InMemoryDeletion;
    use crate::fake::world::{EVERYDAY, OFFSET, WALLET, paid_by_everyday, paid_by_wallet, world};

    #[test]
    fn deleting_takes_the_account_its_transactions_and_their_paid_links() {
        let mut deletion = world();
        let booked = deletion.transactions_booked_to(WALLET);
        assert_eq!(booked.len(), 2, "the world books two to the Wallet");

        let deleted = DeleteAccount { id: WALLET }
            .run(&mut deletion)
            .expect("nothing pays from the Wallet");

        assert_eq!(
            deleted,
            AccountDeleted {
                id: WALLET,
                name: "Wallet".into(),
                transactions: 2,
                unmatched: 1,
            }
        );
        assert_eq!(deletion.accounts.account(WALLET), None);
        assert!(
            booked
                .iter()
                .all(|id| deletion.transactions.transaction(*id).is_none())
        );
        assert_eq!(
            entry(&deletion.bills.entries, paid_by_wallet()).map(|e| e.resolution.clone()),
            Some(Resolution::Unresolved)
        );
    }

    #[test]
    fn what_another_account_paid_and_holds_is_left_alone() {
        let mut deletion = world();
        let before = deletion.clone();

        DeleteAccount { id: WALLET }
            .run(&mut deletion)
            .expect("nothing pays from the Wallet");

        assert_eq!(
            deletion.accounts.account(EVERYDAY),
            before.accounts.account(EVERYDAY)
        );
        let kept = |d: &InMemoryDeletion| d.transactions_booked_to(EVERYDAY);
        assert_eq!(kept(&deletion), kept(&before));
        assert_eq!(
            entry(&deletion.bills.entries, paid_by_everyday()).map(|e| e.resolution.clone()),
            entry(&before.bills.entries, paid_by_everyday()).map(|e| e.resolution.clone()),
        );
        assert_eq!(deletion.bills.bill_plans(), before.bills.bill_plans());
    }

    #[test]
    fn an_account_a_bill_plan_pays_from_is_refused_and_nothing_is_written() {
        let mut deletion = world();
        let before = deletion.clone();

        assert_eq!(
            DeleteAccount { id: EVERYDAY }.run(&mut deletion),
            Err(Error::PaidFromByBillPlans { plans: 1 })
        );
        assert_eq!(deletion, before);
    }

    #[test]
    fn an_account_gone_is_refused_and_one_with_nothing_booked_goes_alone() {
        let mut deletion = world();
        let before = deletion.clone();
        assert_eq!(
            DeleteAccount {
                id: AccountId::seed(9_999)
            }
            .run(&mut deletion),
            Err(Error::NotFound)
        );
        assert_eq!(deletion, before);

        let deleted = DeleteAccount { id: OFFSET }
            .run(&mut deletion)
            .expect("nothing pays from the Offset");

        assert_eq!((deleted.transactions, deleted.unmatched), (0, 0));
        assert_eq!(deletion.transactions, before.transactions);
        assert_eq!(deletion.bills, before.bills);
    }
}
