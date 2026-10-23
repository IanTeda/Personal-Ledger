//! The stub Bills joined to the other domains' stubs: `lib-bills`' seed laid out against the stub
//! Accounts, Categories and Payees as Bills reads them, and its payments booked through
//! `lib-bill-payments`' `PayBill`, so the stub's Paid entries are written as a Client pays one.

use lib_bill_payments::BillPayments;
use lib_bill_payments::interactor::PayBill;
use lib_bills::{BillPlan, BillScheduleEntry, BillsSeed, EntryId, StubBills};
use lib_core::{BillPlanId, TransactionId};
use lib_transactions::Transaction;

/// The seed's Bills and the Ledger's Transactions as the payments port, so `PayBill` books into
/// them.
struct SeedPayments<'a> {
    plans: &'a [BillPlan],
    entries: &'a mut [BillScheduleEntry],
    transactions: &'a mut Vec<Transaction>,
}

impl BillPayments for SeedPayments<'_> {
    fn bill_plan(&self, id: BillPlanId) -> Option<BillPlan> {
        lib_bills::get(self.plans, id).cloned()
    }

    fn entry(&self, id: EntryId) -> Option<BillScheduleEntry> {
        lib_bills::entry(self.entries, id).cloned()
    }

    fn record_payment(&mut self, transaction: Transaction, entry: BillScheduleEntry) {
        // Newest first, as the Transactions stub keeps them.
        let at = self
            .transactions
            .iter()
            .position(|t| t.date <= transaction.date)
            .unwrap_or(self.transactions.len());
        self.transactions.insert(at, transaction);
        if let Some(found) = lib_bills::entry_mut(self.entries, entry.id()) {
            *found = entry;
        }
    }
}

/// The next seed Transaction id: the stub numbers its Transactions in the order they are written.
fn next_id(transactions: &[Transaction]) -> TransactionId {
    TransactionId::seed(u32::try_from(transactions.len() + 1).unwrap_or_default())
}

/// Books `stub`'s payments into `transactions` through `PayBill`, each at its status, then its
/// Match candidate, and returns the Bills they settle.
pub(crate) fn book_stub_bills(stub: StubBills, transactions: &mut Vec<Transaction>) -> BillsSeed {
    let StubBills {
        seed: BillsSeed { plans, mut entries },
        payments,
        match_candidate,
    } = stub;
    for payment in payments {
        let id = next_id(transactions);
        let paid = PayBill {
            entry: payment.entry,
            amount: payment.amount,
            date: payment.date,
        }
        .run_as(
            id,
            &mut SeedPayments {
                plans: &plans,
                entries: &mut entries,
                transactions,
            },
        );
        if paid.is_ok()
            && let Some(t) = transactions.iter_mut().find(|t| t.id == id)
        {
            t.status = payment.status;
        }
    }
    if let Some(debit) = match_candidate {
        let transaction = debit.transaction(next_id(transactions));
        let at = transactions
            .iter()
            .position(|t| t.date <= transaction.date)
            .unwrap_or(transactions.len());
        transactions.insert(at, transaction);
    }
    BillsSeed { plans, entries }
}
