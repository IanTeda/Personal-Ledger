//! The Documents read copy and the rows the stub's own Documents seed reads.
//!
//! The Ledger's Documents and Document Types are a read copy of the Documents store, refreshed after
//! each Documents write and on rollback ([`Ledger::refresh_documents`], #963), for the Documents
//! View and Settings › Documents until they run on `lib-documents`' Presenters (#966 to #968).
//!
//! The mappings fill each row the way the owning domain's store does in `lib-database`, so the
//! stub's Documents seed links what the stored stub holds; the seed tests hold the two together.

use lib_accounts::Account;
use lib_bills::BillPlan;
use lib_documents::types::DocumentTypeRow;
use lib_documents::{
    Document, DocumentAccount, DocumentBillPlan, DocumentItem, DocumentPayee, DocumentTransaction,
};
use lib_institutions::Institution;
use lib_inventory::Inventory;
use lib_payees::Payee;
use lib_transactions::Transaction;

use crate::Ledger;

impl Ledger {
    /// The Ledger's rows as Documents reads them, for the seed tests to match the stub's Documents
    /// against the records they name.
    #[cfg(test)]
    pub(crate) fn link_records(&self) -> lib_documents::LinkRecords {
        lib_documents::LinkRecords {
            transactions: document_transactions(&self.transactions),
            payees: document_payees(&self.payees),
            accounts: document_accounts(&self.accounts, &self.institutions),
            plans: document_bill_plans(&self.bill_plans),
            items: document_items(&self.inventory),
        }
    }

    /// Replaces the Documents and Document Types read copy with the store's rows, returning whether
    /// either changed.
    pub fn refresh_documents(
        &mut self,
        documents: Vec<Document>,
        types: Vec<DocumentTypeRow>,
    ) -> bool {
        if self.documents == documents && self.document_types == types {
            return false;
        }
        self.documents = documents;
        self.document_types = types;
        true
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refreshing_the_documents_reports_only_a_change() {
        let mut ledger = Ledger::stub();
        let mut documents = ledger.documents().to_vec();
        let types = ledger.document_types().to_vec();
        assert!(!ledger.refresh_documents(documents.clone(), types.clone()));

        documents[0].title = "Renamed".into();
        assert!(ledger.refresh_documents(documents.clone(), types));
        assert_eq!(ledger.documents(), documents);
    }
}
