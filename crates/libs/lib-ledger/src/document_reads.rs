//! Halfway shim (#962) between `lib-documents`' read ports and the domains that still live on the
//! Ledger: the Ledger answers every Documents read port from its own collections, each row's text
//! filled by its owning domain (`docs/development/viper-architecture.md`, Rules > Dependencies). It
//! goes when `lib-database`'s stores answer them (#963).

use lib_accounts::Account;
use lib_bills::BillPlan;
use lib_documents::{
    DocumentAccount, DocumentAccounts, DocumentBillPlan, DocumentBillPlans, DocumentItem,
    DocumentItems, DocumentPayee, DocumentPayees, DocumentTransaction, DocumentTransactions,
};
use lib_institutions::Institution;
use lib_inventory::Inventory;
use lib_payees::Payee;
use lib_transactions::Transaction;

use crate::Ledger;

impl DocumentTransactions for Ledger {
    fn document_transactions(&self) -> Vec<DocumentTransaction> {
        document_transactions(&self.transactions)
    }
}

impl DocumentPayees for Ledger {
    fn document_payees(&self) -> Vec<DocumentPayee> {
        document_payees(&self.payees)
    }
}

impl DocumentAccounts for Ledger {
    fn document_accounts(&self) -> Vec<DocumentAccount> {
        document_accounts(&self.accounts, &self.institutions)
    }
}

impl DocumentBillPlans for Ledger {
    fn document_bill_plans(&self) -> Vec<DocumentBillPlan> {
        document_bill_plans(&self.bill_plans)
    }
}

impl DocumentItems for Ledger {
    fn document_items(&self) -> Vec<DocumentItem> {
        document_items(&self.inventory)
    }
}

/// `transactions` as Documents reads them: the stub's own Documents seed's, and each command
/// that matches against them.
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
