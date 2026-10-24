//! Match existing transaction: ties an open entry to an unmatched Expense Split dated near its due
//! date. Writes only the entry and reads the Splits through their port, so it is a Bills
//! Interactor rather than a cross-domain one.

use chrono::Duration;

use super::Rows;
use crate::protocols::{BillAccounts, BillCategories, BillTransactions, BillsRepository};
use crate::{
    BillError, BillSplit, EntryId, MATCH_WINDOW_DAYS, SplitRef, match_candidates, match_split,
    preselected_candidate,
};

/// The Splits a Match on `entry` may name: those dated within the window of its due date.
fn window(transactions: &dyn BillTransactions, entry: EntryId) -> Vec<BillSplit> {
    let reach = Duration::days(MATCH_WINDOW_DAYS);
    transactions.splits_between(entry.due - reach, entry.due + reach)
}

/// The Splits an entry may be Matched to, read when Pay is pressed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MatchCandidates {
    pub entry: EntryId,
}

impl MatchCandidates {
    /// The candidates in the Match list's order (Payee match, Account match, then closeness of
    /// amount and of date), and the one it pre-selects: the first, only when its Payee is the
    /// Plan's. None for an entry that isn't open.
    pub fn run(
        self,
        bills: &dyn BillsRepository,
        transactions: &dyn BillTransactions,
        accounts: &dyn BillAccounts,
        categories: &dyn BillCategories,
    ) -> (Vec<SplitRef>, Option<SplitRef>) {
        let rows = Rows::read(bills);
        let nearby = window(transactions, self.entry);
        let candidates = match_candidates(
            &rows.plans,
            &rows.entries,
            &nearby,
            &accounts.bill_accounts(),
            &categories.bill_categories(),
            self.entry,
        );
        let preselected = crate::get(&rows.plans, self.entry.plan_id)
            .and_then(|plan| preselected_candidate(plan, &candidates, &nearby));
        (candidates, preselected)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MatchBill {
    pub entry: EntryId,
    pub split: SplitRef,
}

impl MatchBill {
    /// Writes the entry Paid by `split`.
    ///
    /// # Errors
    /// `EntryNotFound` or `EntryNotOpen` for an entry that is gone or settled, and `NotACandidate`
    /// for a Split that isn't one of its [`MatchCandidates`].
    pub fn run(
        self,
        bills: &mut dyn BillsRepository,
        transactions: &dyn BillTransactions,
        accounts: &dyn BillAccounts,
        categories: &dyn BillCategories,
    ) -> Result<(), BillError> {
        let outcome = self.apply(bills, transactions, accounts, categories);
        let (plan, due) = (self.entry.plan_id, self.entry.due);
        let (transaction_id, split) = (self.split.transaction_id, self.split.split_index);
        match &outcome {
            Ok(()) => tracing::info!(
                bill_plan_id = %plan,
                %due,
                %transaction_id,
                split_index = split,
                "Matched a Bill to a Split"
            ),
            Err(refusal) => tracing::info!(
                bill_plan_id = %plan,
                %due,
                %transaction_id,
                split_index = split,
                %refusal,
                "Refused to match a Bill to a Split"
            ),
        }
        outcome
    }

    fn apply(
        self,
        bills: &mut dyn BillsRepository,
        transactions: &dyn BillTransactions,
        accounts: &dyn BillAccounts,
        categories: &dyn BillCategories,
    ) -> Result<(), BillError> {
        let mut rows = Rows::read(bills);
        match_split(
            &rows.plans,
            &mut rows.entries,
            &window(transactions, self.entry),
            &accounts.bill_accounts(),
            &categories.bill_categories(),
            self.entry,
            self.split,
        )?;
        rows.write_back(bills);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use lib_core::{AccountId, CategoryId, PayeeId, TransactionId};

    use super::*;
    use crate::fake::InMemoryPorts;
    use crate::interactor::test_support::{cents, date, entry_of, stub};
    use crate::{Resolution, entry};

    /// The one Split of an expense on `day` of September in ANZ Everyday.
    fn spend(id: u32, day: u32, payee: Option<PayeeId>, amount: i64) -> BillSplit {
        BillSplit {
            split: SplitRef {
                transaction_id: TransactionId::seed(90_000 + id),
                split_index: 0,
            },
            date: date(2026, 9, day),
            account_id: AccountId::seed(2),
            description: None,
            amount: cents(-amount),
            category_id: CategoryId::seed(5),
            payee_id: payee,
        }
    }

    /// The stub with two expenses near Telstra Internet's 22 Sep entry: one to its Payee.
    fn with_candidates() -> (InMemoryPorts, EntryId, PayeeId) {
        let mut ports = stub();
        let telstra = entry_of(&ports, "Telstra Internet", date(2026, 9, 22));
        let payee = ports
            .bills
            .plans
            .iter()
            .find(|p| p.id == telstra.plan_id)
            .and_then(|p| p.payee_id)
            .expect("Telstra Internet has a Payee");
        ports.reads.splits.push(spend(1, 20, None, 8_000));
        ports.reads.splits.push(spend(2, 21, Some(payee), 7_999));
        // Outside the window.
        ports.reads.splits.push(BillSplit {
            date: date(2026, 7, 1),
            ..spend(3, 1, Some(payee), 7_999)
        });
        (ports, telstra, payee)
    }

    fn candidates(ports: &InMemoryPorts, entry: EntryId) -> (Vec<SplitRef>, Option<SplitRef>) {
        MatchCandidates { entry }.run(&ports.bills, &ports.reads, &ports.reads, &ports.reads)
    }

    #[test]
    fn the_payee_match_comes_first_and_is_preselected() {
        let (ports, telstra, _) = with_candidates();

        let (found, preselected) = candidates(&ports, telstra);

        let ids: Vec<u32> = found
            .iter()
            .map(|split| split.transaction_id.seed_number())
            .collect();
        assert_eq!(ids.first(), Some(&90_002));
        assert!(ids.contains(&90_001));
        assert!(!ids.contains(&90_003), "outside the window");
        assert_eq!(preselected, found.first().copied());
    }

    #[test]
    fn without_a_payee_match_nothing_is_preselected() {
        let mut ports = stub();
        let origin = entry_of(&ports, "Origin Energy", date(2026, 9, 24));
        let before = candidates(&ports, origin).0.len();
        ports.reads.splits.push(spend(1, 20, None, 15_000));

        let (found, preselected) = candidates(&ports, origin);

        assert_eq!(found.len(), before + 1);
        assert_eq!(preselected, None);
    }

    #[test]
    fn matching_pays_the_entry_and_takes_the_split_off_every_list() {
        let (mut ports, telstra, _) = with_candidates();
        let (found, _) = candidates(&ports, telstra);
        let split = found[0];

        MatchBill {
            entry: telstra,
            split,
        }
        .run(&mut ports.bills, &ports.reads, &ports.reads, &ports.reads)
        .expect("a candidate");

        assert_eq!(
            entry(&ports.bills.entries, telstra).map(|e| e.resolution.clone()),
            Some(Resolution::Paid(split))
        );
        let netflix = entry_of(&ports, "Netflix", date(2026, 9, 18));
        assert!(!candidates(&ports, netflix).0.contains(&split));
    }

    #[test]
    fn a_split_that_is_no_candidate_is_refused_and_nothing_is_written() {
        let (mut ports, telstra, _) = with_candidates();
        let before = ports.bills.clone();

        let refused = MatchBill {
            entry: telstra,
            split: SplitRef {
                transaction_id: TransactionId::seed(90_003),
                split_index: 0,
            },
        }
        .run(&mut ports.bills, &ports.reads, &ports.reads, &ports.reads);

        assert_eq!(refused, Err(BillError::NotACandidate));
        assert_eq!(ports.bills, before);
    }
}
