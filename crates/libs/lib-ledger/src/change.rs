//! What a command records when it writes (ADR-0039), and the command trait every write goes
//! through (ADR-0038).
//!
//! A command checks everything first and writes only once nothing can refuse, so a refusal leaves
//! the Ledger untouched. On success it records a [`LedgerChange`] per row it wrote, and the store
//! drains them. The domains a store announces are derived from those changes rather than declared
//! by each command, so a command can't name one domain and touch another.

use std::collections::BTreeSet;

use lib_accounts::Account;
use lib_bills::{BillPlan, BillScheduleEntry};
use lib_budgets::Budget;
use lib_categories::Category;
use lib_documents::{Document, types::DocumentTypeRow};
use lib_payees::Payee;
use lib_tags::Tag;
use lib_transactions::Transaction;

use crate::Ledger;

/// A part of the Ledger a write can touch: what the Desktop's `LedgerChanged { what }` carries,
/// so a reader can skip a change that isn't its own.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Domain {
    Accounts,
    Categories,
    Payees,
    Tags,
    Transactions,
    Bills,
    Budgets,
    Documents,
    DocumentTypes,
    Inventory,
    Institutions,
    Units,
}

/// One row a command wrote, as the repository will receive it (ADR-0039). Only the rows the Ledger
/// prototype's commands write so far; each domain adds its own as its writes move onto commands.
#[derive(Debug, Clone, PartialEq)]
pub enum LedgerChange {
    UpsertAccount(Account),
    DeleteAccount(u32),
    UpsertPayee(Payee),
    DeletePayee(u32),
    UpsertCategory(Category),
    DeleteCategory(u32),
    UpsertTag(Tag),
    DeleteTag(u32),
    UpsertTransaction(Transaction),
    DeleteTransaction(u32),
    UpsertBillPlan(BillPlan),
    UpsertBillEntry(BillScheduleEntry),
    UpsertBudget(Budget),
    UpsertDocument(Document),
    UpsertDocumentType(DocumentTypeRow),
    DeleteDocumentType(u32),
}

impl LedgerChange {
    pub fn domain(&self) -> Domain {
        match self {
            Self::UpsertPayee(_) | Self::DeletePayee(_) => Domain::Payees,
            Self::UpsertAccount(_) | Self::DeleteAccount(_) => Domain::Accounts,
            Self::UpsertCategory(_) | Self::DeleteCategory(_) => Domain::Categories,
            Self::UpsertTag(_) | Self::DeleteTag(_) => Domain::Tags,
            Self::UpsertTransaction(_) | Self::DeleteTransaction(_) => Domain::Transactions,
            Self::UpsertBillPlan(_) | Self::UpsertBillEntry(_) => Domain::Bills,
            Self::UpsertBudget(_) => Domain::Budgets,
            Self::UpsertDocument(_) => Domain::Documents,
            Self::UpsertDocumentType(_) | Self::DeleteDocumentType(_) => Domain::DocumentTypes,
        }
    }
}

/// The domains a batch of changes touched, each once.
pub fn domains(changes: &[LedgerChange]) -> BTreeSet<Domain> {
    changes.iter().map(LedgerChange::domain).collect()
}

/// A command as a value, so a Dialog's `command()` can return one and a single `apply` can run any
/// of them (ADR-0037). Each runs through its `Ledger` method, and refuses with its own type so a
/// Client maps every refusal to one Message.
pub trait LedgerCommand {
    type Outcome;
    type Refusal;
    fn run(self, ledger: &mut Ledger) -> Result<Self::Outcome, Self::Refusal>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn domains_names_each_touched_domain_once() {
        let changes = [
            LedgerChange::DeletePayee(1),
            LedgerChange::DeletePayee(2),
            LedgerChange::DeleteCategory(3),
        ];
        assert_eq!(
            domains(&changes),
            BTreeSet::from([Domain::Payees, Domain::Categories])
        );
    }

    #[test]
    fn no_changes_touch_no_domain() {
        assert!(domains(&[]).is_empty());
    }
}
