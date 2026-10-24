//! The Ledger's Payees are a read copy of the Payees port (#831): a Client refreshes it from the
//! port after each Payees write ([`Ledger::refresh_payees`]), for the Views that still read
//! Payees from the Ledger. The Payees writes are `lib-payees`' Interactors (#832), except Import
//! commit's and Category reassign's, which record `UpsertPayee` changes a Client forwards to the
//! port. The copy goes when the last reader has its own port.

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
}
