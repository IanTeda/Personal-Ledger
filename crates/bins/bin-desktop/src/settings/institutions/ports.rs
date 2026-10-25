//! The Institutions port as the Desktop holds it (#849): the forwarding shim that saves the
//! Institutions page's adds through the store `shell::router::build` is given (`LedgerSetup`).
//!
//! The page is still today's code: its add runs as a `LedgerCommand` on the Ledger and records a
//! `LedgerChange`. [`forward`] sends the Institution ones to the store in the same commit as the
//! command, and the Ledger's Institutions become a read copy refreshed from the store after each
//! write and on rollback. The page leaves the shim with its module (#850).

use lib_database::InstitutionsStore;
use lib_ledger::LedgerChange;

/// Sends each Institution row in `changes` to `store`, in the order recorded, returning whether it
/// wrote anything. The one writer it carries is the Institutions page's add, so every row is new.
/// Every other change is another domain's.
pub fn forward(store: &mut dyn InstitutionsStore, changes: &[LedgerChange]) -> bool {
    let port = store.institutions_mut();
    let mut wrote = false;
    for change in changes {
        if let LedgerChange::UpsertInstitution(institution) = change {
            port.add_institution(institution.clone());
            wrote = true;
        }
    }
    wrote
}

#[cfg(test)]
mod tests {
    use lib_core::InstitutionId;
    use lib_institutions::fake::InMemoryInstitutions;
    use lib_institutions::{Institution, InstitutionRepository};

    use super::*;

    #[test]
    fn institution_changes_reach_the_store_and_others_are_left() {
        let mut store = InMemoryInstitutions::stub();
        let bank = Institution {
            id: InstitutionId::new(),
            name: "Bank X".into(),
            account_type: "savings".into(),
        };

        assert!(forward(
            &mut store,
            &[LedgerChange::UpsertInstitution(bank.clone())]
        ));
        assert!(!forward(
            &mut store,
            &[LedgerChange::DeleteTransaction(
                lib_core::TransactionId::seed(1)
            )]
        ));

        assert_eq!(store.institution(bank.id), Some(bank));
        assert_eq!(store.institutions.len(), 7);
    }
}
