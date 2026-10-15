//! The temporary write hatch: a `&mut` borrow of every collection, so the Desktop's writes keep
//! working while the Ledger owns the data and before each domain's writes move onto commands.
//!
//! ADR-0038 says there is no `edit(FnOnce(&mut Vec<_>))` hatch on `Ledger`. This one exists only
//! for the migration decided on the Ledger prototype (#593): each domain's commands ticket deletes
//! its fields, and the last one (#648) deletes the hatch. A hatch write records no
//! [`LedgerChange`](crate::LedgerChange), so it never reaches the repository.

use lib_accounts::Account;
use lib_bills::{BillPlan, BillScheduleEntry};
use lib_budgets::Budgets;
use lib_categories::Category;
use lib_documents::{Document, types::DocumentTypeRow};
use lib_payees::Payee;
use lib_tags::Tag;
use lib_transactions::Transaction;

use crate::Ledger;

/// Every collection, borrowed at once so a write that crosses domains (paying a Bill writes
/// Transactions) can reach them all through one borrow of the Ledger.
pub struct LedgerHatch<'a> {
    pub accounts: &'a mut Vec<Account>,
    pub categories: &'a mut Vec<Category>,
    pub payees: &'a mut Vec<Payee>,
    pub tags: &'a mut Vec<Tag>,
    pub transactions: &'a mut Vec<Transaction>,
    pub bill_plans: &'a mut Vec<BillPlan>,
    pub bill_entries: &'a mut Vec<BillScheduleEntry>,
    pub budgets: &'a mut Budgets,
    pub documents: &'a mut Vec<Document>,
    pub document_types: &'a mut Vec<DocumentTypeRow>,
}

impl Ledger {
    /// Borrows every collection for a write that hasn't moved onto a command yet.
    #[deprecated(
        note = "a write that skips the change list; move it onto a Ledger command (#638 to #647)"
    )]
    pub fn hatch(&mut self) -> LedgerHatch<'_> {
        LedgerHatch {
            accounts: &mut self.accounts,
            categories: &mut self.categories,
            payees: &mut self.payees,
            tags: &mut self.tags,
            transactions: &mut self.transactions,
            bill_plans: &mut self.bill_plans,
            bill_entries: &mut self.bill_entries,
            budgets: &mut self.budgets,
            documents: &mut self.documents,
            document_types: &mut self.document_types,
        }
    }
}
