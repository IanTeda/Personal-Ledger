//! Committing the rows: checking them and the Account, resolving the plan and sending it through
//! the one atomic port.

use lib_core::AccountId;
use lib_payees::Payee;

use crate::entity::ImportRow;
use crate::error::{Error, Result};
use crate::plan::{ImportCommitted, ImportPlan};
use crate::protocols::{ImportAccounts, TransactionImport};

/// Commits the rows to `account_id` through the port. Remembers a new Payee's cleaned token as its
/// alias when `remember`.
#[derive(Debug, Clone, PartialEq)]
pub struct CommitImport {
    pub account_id: AccountId,
    pub rows: Vec<ImportRow>,
    pub remember: bool,
}

impl CommitImport {
    /// `payees` is every stored Payee, read before the call: the port that writes may be the store
    /// that holds them, so it can't be borrowed for the read as well.
    ///
    /// # Errors
    /// `NeedsReview` while any row does, `AccountGone` or `AccountInactive` for an Account that
    /// can't take Transactions, and `PayeeNameTaken` or `AliasTaken` when the port refuses the plan (a new name or alias
    /// since taken). Each writes nothing.
    pub fn run(
        self,
        accounts: &dyn ImportAccounts,
        payees: &[Payee],
        import: &mut dyn TransactionImport,
    ) -> Result<ImportCommitted> {
        let outcome = self.apply(accounts, payees, import);
        match &outcome {
            Ok(committed) => tracing::info!(
                transactions = committed.transactions,
                new_payees = committed.new_payees,
                "Committed an Import"
            ),
            Err(refusal) => tracing::info!(%refusal, "Refused to commit an Import"),
        }
        outcome
    }

    fn apply(
        self,
        accounts: &dyn ImportAccounts,
        payees: &[Payee],
        import: &mut dyn TransactionImport,
    ) -> Result<ImportCommitted> {
        let pending = self.rows.iter().filter(|row| row.needs_review()).count();
        if pending > 0 {
            return Err(Error::NeedsReview(pending));
        }
        let account = accounts
            .import_account(self.account_id)
            .ok_or(Error::AccountGone)?;
        if !account.is_active {
            return Err(Error::AccountInactive);
        }
        let plan = ImportPlan::resolve(self.account_id, &self.rows, self.remember, payees)?;
        import.commit(plan)
    }
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;
    use lib_core::CategoryId;

    use super::*;
    use crate::fake::stub_ports;
    use crate::interactor::MatchStatement;
    use crate::statement::SeededStatement;

    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 9, 28).expect("a September day")
    }

    /// The seeded rows with every unresolved Category given Household.
    fn resolved(rows: Vec<ImportRow>) -> Vec<ImportRow> {
        rows.into_iter()
            .map(|row| ImportRow {
                category: row.category.or(Some(CategoryId::seed(10))),
                ..row
            })
            .collect()
    }

    #[test]
    fn committing_writes_every_row_and_new_payee_once() {
        let (ports, store) = stub_ports(today());
        let rows = resolved(MatchStatement::run(&*ports.statement, &*ports.payees.borrow()).rows);

        let stored = ports.payees.borrow().import_payees();
        let committed = CommitImport {
            account_id: SeededStatement::STUB_ACCOUNT,
            rows,
            remember: true,
        }
        .run(&*ports.accounts, &stored, &mut *ports.import.borrow_mut())
        .expect("every row is resolved");

        assert_eq!(
            committed,
            ImportCommitted {
                transactions: 18,
                new_payees: 3
            }
        );
        assert_eq!(store.borrow().transactions.transactions.len(), 18);
    }

    #[test]
    fn a_commit_is_refused_while_rows_need_review_and_writes_nothing() {
        let (ports, store) = stub_ports(today());
        let rows = MatchStatement::run(&*ports.statement, &*ports.payees.borrow()).rows;
        let before = store.borrow().clone();

        let stored = ports.payees.borrow().import_payees();
        let refused = CommitImport {
            account_id: SeededStatement::STUB_ACCOUNT,
            rows,
            remember: true,
        }
        .run(&*ports.accounts, &stored, &mut *ports.import.borrow_mut());

        assert_eq!(refused, Err(Error::NeedsReview(4)));
        assert_eq!(*store.borrow(), before);
    }
}
