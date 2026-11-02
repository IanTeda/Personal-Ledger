//! The stub Documents, over the rows the Ledger's own records give Documents' read ports.
//!
//! The Documents and Document Types aren't the Ledger's: they live in the Documents store, read
//! and written through `lib-documents`' Presenters (#968). The stub's Documents link the stub
//! Ledger's records, so they are built here, the way each owning domain's store fills its rows in
//! `lib-database`; the seed tests hold the two together.

use chrono::NaiveDate;
use lib_accounts::Account;
use lib_bills::BillPlan;
use lib_documents::{
    Document, DocumentAccount, DocumentBillPlan, DocumentItem, DocumentPayee, DocumentTransaction,
};
use lib_institutions::Institution;
use lib_inventory::Inventory;
use lib_payees::Payee;
use lib_transactions::Transaction;

use crate::Ledger;

impl Ledger {
    /// The Ledger's rows as Documents reads them.
    pub(crate) fn link_records(&self) -> lib_documents::LinkRecords {
        lib_documents::LinkRecords {
            transactions: document_transactions(&self.transactions),
            payees: document_payees(&self.payees),
            accounts: document_accounts(&self.accounts, &self.institutions),
            plans: document_bill_plans(&self.bill_plans),
            items: document_items(&self.inventory),
        }
    }

    /// `lib-documents`' stub Documents as of `today`, linking this Ledger's records: the stub
    /// Ledger's own, for a Client or test that seeds a Documents store beside it.
    pub fn stub_documents(&self, today: NaiveDate) -> Vec<Document> {
        lib_documents::default_documents(&self.link_records(), today).documents
    }
}

/// `transactions` as Documents reads them, for the stub's own Documents seed.
pub(crate) fn document_transactions(transactions: &[Transaction]) -> Vec<DocumentTransaction> {
    transactions
        .iter()
        .map(|transaction| DocumentTransaction {
            id: transaction.id,
            date: transaction.date,
            amount: transaction.total(),
            account_id: transaction.account_id,
            payee_ids: transaction
                .splits
                .iter()
                .map(|split| split.payee_id)
                .collect(),
            memo: transaction.description.clone(),
        })
        .collect()
}

pub(crate) fn document_payees(payees: &[Payee]) -> Vec<DocumentPayee> {
    payees
        .iter()
        .map(|payee| DocumentPayee {
            id: payee.id,
            name: payee.name.clone(),
            aliases: payee.aliases.clone(),
            is_active: payee.is_active,
        })
        .collect()
}

pub(crate) fn document_accounts(
    accounts: &[Account],
    institutions: &[Institution],
) -> Vec<DocumentAccount> {
    accounts
        .iter()
        .map(|account| DocumentAccount {
            id: account.id,
            name: account.name.clone(),
            institution: lib_institutions::get(institutions, account.institution)
                .map_or_else(String::new, |institution| institution.name.clone()),
        })
        .collect()
}

pub(crate) fn document_bill_plans(plans: &[BillPlan]) -> Vec<DocumentBillPlan> {
    plans
        .iter()
        .map(|plan| DocumentBillPlan {
            id: plan.id,
            name: plan.name.clone(),
            cadence: lib_bills::presenter::recurrence_label(plan.recurrence),
            payee_id: plan.payee_id,
        })
        .collect()
}

pub(crate) fn document_items(inventory: &Inventory) -> Vec<DocumentItem> {
    inventory
        .items
        .iter()
        .map(|item| DocumentItem {
            id: item.id,
            name: item.name.clone(),
        })
        .collect()
}
