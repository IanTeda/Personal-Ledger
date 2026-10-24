//! In-memory ports and the stub seed, for tests and UI-first work. Each holds its rows in a plain
//! field so a test can set up or read state directly.

use std::collections::HashMap;

use lib_core::PayeeId;

use crate::protocols::PortsMut;
use crate::{Payee, PayeeRepository, PayeeUsage, PayeeUse, default_payees};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InMemoryPayees {
    pub payees: Vec<Payee>,
}

impl PayeeRepository for InMemoryPayees {
    fn payees(&self) -> Vec<Payee> {
        self.payees.clone()
    }

    fn upsert_payee(&mut self, payee: Payee) {
        match self.payees.iter_mut().find(|row| row.id == payee.id) {
            Some(row) => *row = payee,
            None => self.payees.push(payee),
        }
    }

    fn delete_payee(&mut self, id: PayeeId) {
        self.payees.retain(|row| row.id != id);
    }
}

/// Usage counts a test sets per Payee; a Payee not listed is unused.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InMemoryPayeeUsage {
    pub uses: HashMap<PayeeId, PayeeUse>,
}

impl PayeeUsage for InMemoryPayeeUsage {
    fn payee_usage(&self, id: PayeeId) -> PayeeUse {
        self.uses.get(&id).copied().unwrap_or_default()
    }
}

/// Every Payees port in memory, together, so a test can hand a write the ones it takes.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InMemoryPorts {
    pub payees: InMemoryPayees,
    pub usage: InMemoryPayeeUsage,
}

impl InMemoryPorts {
    /// The stub seed: the stub Ledger's Payees (see [`default_payees`]) with nothing naming any.
    /// Ids match `lib-ledger`'s stub, so its Splits and Bill Plans still resolve their Payees.
    pub fn stub() -> Self {
        Self {
            payees: InMemoryPayees {
                payees: default_payees(),
            },
            usage: InMemoryPayeeUsage::default(),
        }
    }

    pub fn split_mut(&mut self) -> PortsMut<'_> {
        PortsMut {
            payees: &mut self.payees,
            usage: &self.usage,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn upserting_a_payee_replaces_the_row_with_its_id_or_appends_it() {
        let mut ports = InMemoryPorts::stub();
        let seeded = ports.payees.payees().len();
        let mut coles = ports.payees.payee(PayeeId::seed(2)).unwrap();
        coles.aliases.push("COLES EXPRESS".into());
        ports.payees.upsert_payee(coles.clone());
        let mut corner = coles.clone();
        corner.id = PayeeId::new();
        corner.name = "Corner Shop".into();
        corner.aliases = vec!["CORNER SHOP".into()];
        ports.payees.upsert_payee(corner.clone());

        assert_eq!(ports.payees.payees().len(), seeded + 1);
        assert_eq!(ports.payees.payee(PayeeId::seed(2)), Some(coles));
        assert_eq!(ports.payees.payee(corner.id), Some(corner));

        ports.payees.delete_payee(PayeeId::seed(2));
        assert_eq!(ports.payees.payee(PayeeId::seed(2)), None);
    }

    #[test]
    fn a_payee_no_test_names_is_unused() {
        let ports = InMemoryPorts::stub();
        assert!(!ports.usage.payee_usage(PayeeId::seed(1)).is_used());
    }
}
