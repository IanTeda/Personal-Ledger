//! `lib-payees`' ports answered over the stores: the Payees from the Payees store, and what names a
//! Payee and the Categories it may default to from the Transactions, Bills and Categories stores
//! through [`StoredPayeeReads`] (#992, #1084).

use lib_core::PayeeId;
use lib_payees::{Payee, PayeeCategories, PayeeRepository, PayeeUsage, PayeesPorts, PortsMut};

use crate::adapter::store::{SharedPayeesStore, StoredPayeeReads};

/// The shared Payees store's Payees, borrowed for each call: the store's guard can't be lent out of
/// its `RefCell`.
struct Payees(SharedPayeesStore);

impl PayeeRepository for Payees {
    fn payees(&self) -> Vec<Payee> {
        self.0.borrow().payees().payees()
    }

    fn upsert_payee(&mut self, payee: Payee) {
        self.0.borrow_mut().payees_mut().upsert_payee(payee);
    }

    fn delete_payee(&mut self, id: PayeeId) {
        self.0.borrow_mut().payees_mut().delete_payee(id);
    }
}

/// [`lib_payees::PayeesPorts`] over the shared Payees store, and the reads over the Transactions,
/// Bills and Categories stores. A write reaches the store's recorder, so the commit names the
/// Payees in what the input wrote.
pub struct StoredPayeesPorts {
    payees: Payees,
    reads: StoredPayeeReads,
}

impl StoredPayeesPorts {
    pub fn new(payees: SharedPayeesStore, reads: StoredPayeeReads) -> Self {
        Self {
            payees: Payees(payees),
            reads,
        }
    }
}

impl PayeesPorts for StoredPayeesPorts {
    fn payees(&self) -> &dyn PayeeRepository {
        &self.payees
    }

    fn usage(&self) -> &dyn PayeeUsage {
        &self.reads
    }

    fn categories(&self) -> &dyn PayeeCategories {
        &self.reads
    }

    fn split_mut(&mut self) -> PortsMut<'_> {
        PortsMut {
            payees: &mut self.payees,
            usage: &self.reads,
        }
    }
}

#[cfg(test)]
mod tests {

    use crate::ports::test_support::{fakes, file};
    use crate::{Domain, LedgerPorts};

    #[test]
    fn a_payees_write_reports_the_payees_written_on_either() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        for ledger in [fakes(), file(dir.path())] {
            let LedgerPorts { payees, commit, .. } = ledger;
            let mut payee = payees
                .borrow()
                .payees()
                .payees()
                .into_iter()
                .next()
                .expect("a sample Payee");
            payee.name.push_str(" renamed");

            payees.borrow_mut().split_mut().payees.upsert_payee(payee);

            assert_eq!(commit.commit().wrote, [Domain::Payees]);
        }
    }
}
