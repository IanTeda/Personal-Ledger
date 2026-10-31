//! The Ledger's Payees are a read copy of the Payees port (#831): a Client refreshes it from the
//! port after each Payees write ([`Ledger::refresh_payees`]), for the Views that still read
//! Payees from the Ledger. The Payees writes are `lib-payees`' Interactors (#832) and
//! `lib-transaction-import`'s commit (#943), except Category reassign's, which records
//! `UpsertPayee` changes a Client forwards to the port. The copy goes when the last reader has its own port.
//!
//! [`Ledger::payee_uses`] counts what names each Payee from the Ledger's Splits and Bill Plans,
//! for a Client whose Payees port is in memory and can't count them itself. The Payees adapter
//! counts the same rows in the file.

use std::collections::HashMap;

use lib_core::PayeeId;
use lib_payees::PayeeUse;

use crate::Ledger;

impl Ledger {
    /// Replaces the read copy of the Payees with what the Payees port holds, in the port's order.
    /// Returns whether anything changed, so a Client announces only a real change.
    pub fn refresh_payees(&mut self, payees: Vec<lib_payees::Payee>) -> bool {
        if self.payees == payees {
            return false;
        }
        self.payees = payees;
        true
    }

    /// How many Splits and Bill Plans name each Payee; a Payee nothing names is left out.
    pub fn payee_uses(&self) -> HashMap<PayeeId, PayeeUse> {
        let mut uses: HashMap<PayeeId, PayeeUse> = HashMap::new();
        let splits = self.transactions.iter().flat_map(|t| &t.splits);
        for id in splits.filter_map(|split| split.payee_id) {
            uses.entry(id).or_default().splits += 1;
        }
        for id in self.bill_plans.iter().filter_map(|plan| plan.payee_id) {
            uses.entry(id).or_default().bill_plans += 1;
        }
        uses
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_refresh_takes_the_ports_rows_and_reports_only_a_change() {
        let mut ledger = Ledger::stub();
        let mut payees = ledger.payees().to_vec();

        assert!(!ledger.refresh_payees(payees.clone()));

        payees.remove(0);
        assert!(ledger.refresh_payees(payees.clone()));
        assert_eq!(ledger.payees(), payees);
    }

    #[test]
    fn payee_uses_count_the_splits_and_bill_plans_naming_each_payee() {
        let mut ledger = Ledger::stub();
        let woolworths = PayeeId::seed(1);
        let before = ledger.payee_uses()[&woolworths];
        assert!(before.splits > 0, "the stub's groceries name Woolworths");
        assert_eq!(ledger.payee_uses()[&PayeeId::seed(20)].bill_plans, 1);

        let mut transactions = ledger.transactions().to_vec();
        let mut extra = transactions[0].clone();
        extra.id = lib_core::TransactionId::seed(9_999);
        for split in &mut extra.splits {
            split.payee_id = Some(woolworths);
        }
        let added = extra.splits.len();
        transactions.push(extra);
        ledger.refresh_transactions(transactions);

        assert_eq!(
            ledger.payee_uses()[&woolworths].splits,
            before.splits + added,
            "a Split written after the first count is counted"
        );
    }
}
