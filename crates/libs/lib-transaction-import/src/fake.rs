//! The in-memory import port and reads, for tests and UI-first work. Each holds its rows in plain
//! fields so a test can set up or read state directly.

use std::cell::RefCell;
use std::rc::Rc;

use chrono::NaiveDate;
use lib_core::AccountId;
use lib_payees::fake::InMemoryPayees;
use lib_payees::{Payee, PayeeCategory, PayeeRepository};
use lib_toast::{Notice, Notify};
use lib_transactions::TransactionRepository;
use lib_transactions::fake::InMemoryTransactions;

use crate::error::Result;
use crate::plan::{ImportCommitted, ImportPlan};
use crate::protocols::{
    ImportAccount, ImportAccounts, ImportCategories, ImportPayees, ImportPorts, ImportRouter,
    TransactionImport,
};
use crate::statement::SeededStatement;

/// An Import's two stores in memory, the Payees and the Transactions, written together.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct InMemoryImport {
    pub payees: InMemoryPayees,
    pub transactions: InMemoryTransactions,
}

impl TransactionImport for InMemoryImport {
    fn commit(&mut self, plan: ImportPlan) -> Result<ImportCommitted> {
        let committed = plan.check(&self.payees.payees)?;
        for payee in plan.payees {
            self.payees.upsert_payee(payee);
        }
        for transaction in plan.transactions {
            self.transactions.upsert_transaction(transaction);
        }
        Ok(committed)
    }
}

impl ImportPayees for InMemoryImport {
    fn import_payees(&self) -> Vec<Payee> {
        self.payees.payees()
    }
}

/// The Accounts and Categories a test offers the step.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InMemoryImportReads {
    pub accounts: Vec<ImportAccount>,
    pub categories: Vec<PayeeCategory>,
}

impl ImportAccounts for InMemoryImportReads {
    fn import_account(&self, id: AccountId) -> Option<ImportAccount> {
        self.accounts
            .iter()
            .find(|account| account.id == id)
            .cloned()
    }
}

impl ImportCategories for InMemoryImportReads {
    fn import_categories(&self) -> Vec<PayeeCategory> {
        self.categories.clone()
    }
}

/// The stub Import's ports over one in-memory store, with the seeded statement dated back from
/// `today`, and that store, for a test to read what a commit wrote. The statement's Account is the
/// stub's active "ANZ Everyday".
pub fn stub_ports(today: NaiveDate) -> (ImportPorts, Rc<RefCell<InMemoryImport>>) {
    let store = Rc::new(RefCell::new(InMemoryImport {
        payees: InMemoryPayees {
            payees: lib_payees::default_payees(),
        },
        ..InMemoryImport::default()
    }));
    let reads = Rc::new(InMemoryImportReads {
        accounts: vec![ImportAccount {
            id: SeededStatement::STUB_ACCOUNT,
            name: "ANZ Everyday".to_string(),
            is_active: true,
        }],
        categories: lib_payees::fake::stub_categories(),
    });
    let ports = ImportPorts {
        import: store.clone(),
        payees: store.clone(),
        accounts: reads.clone(),
        categories: reads,
        statement: Rc::new(SeededStatement::stub(today)),
    };
    (ports, store)
}

/// A Router that records what the Presenter asked of it, in order.
#[derive(Debug, Default)]
pub struct RecordingRouter {
    pub notices: Vec<Notice>,
    /// One entry per `close_import`: what was committed, `None` for a cancel.
    pub closed: Vec<Option<ImportCommitted>>,
}

impl Notify for RecordingRouter {
    fn notify(&mut self, notice: Notice) {
        self.notices.push(notice);
    }
}

impl ImportRouter for RecordingRouter {
    fn close_import(&mut self, committed: Option<ImportCommitted>) {
        self.closed.push(committed);
    }
}

#[cfg(test)]
mod tests {
    use lib_core::PayeeId;

    use super::*;

    #[test]
    fn the_reads_answer_what_they_hold() {
        let everyday = ImportAccount {
            id: AccountId::seed(2),
            name: "ANZ Everyday".to_string(),
            is_active: true,
        };
        let reads = InMemoryImportReads {
            accounts: vec![everyday.clone()],
            categories: lib_payees::fake::stub_categories(),
        };
        assert_eq!(reads.import_account(AccountId::seed(2)), Some(everyday));
        assert_eq!(reads.import_account(AccountId::seed(9)), None);
        assert_eq!(
            reads.import_categories(),
            lib_payees::fake::stub_categories()
        );

        let import = InMemoryImport {
            payees: InMemoryPayees {
                payees: lib_payees::default_payees(),
            },
            ..InMemoryImport::default()
        };
        assert!(
            import
                .import_payees()
                .iter()
                .any(|payee| payee.id == PayeeId::seed(1))
        );
    }
}
