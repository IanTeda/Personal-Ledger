//! The Payees port as the Desktop holds it (#831): the forwarding shim that saves today's writers'
//! Payee changes through the store `shell::router::build` is given (`LedgerSetup`).
//!
//! The Payees page and every writer of Payees are still today's code: each runs as a
//! `LedgerCommand` on the Ledger and records a `LedgerChange`. [`forward`] sends the Payee ones to
//! the store in the same commit as the command, and the Ledger's Payees become a read copy
//! refreshed from the store after each write and on rollback. The page leaves the shim with its
//! module (#832), Import commit with `lib-transaction-import` (#810) and Category reassign with
//! `lib-reassign` (#813).

use lib_database::PayeesStore;
use lib_ledger::LedgerChange;

/// Sends each Payee row in `changes` to `store`, in the order recorded, returning whether it wrote
/// anything. The writers it carries: the Payees page, Import commit (the Payees it creates and the
/// match rules it remembers) and Category reassign (the Default categories it clears). Every other
/// change is another domain's.
pub fn forward(store: &mut dyn PayeesStore, changes: &[LedgerChange]) -> bool {
    let port = store.payees_mut();
    let mut wrote = false;
    for change in changes {
        match change {
            LedgerChange::UpsertPayee(payee) => port.upsert_payee(payee.clone()),
            LedgerChange::DeletePayee(id) => port.delete_payee(*id),
            _ => continue,
        }
        wrote = true;
    }
    wrote
}

#[cfg(test)]
mod tests {
    use lib_core::PayeeId;
    use lib_payees::PayeeRepository;
    use lib_payees::fake::InMemoryPorts;

    use super::*;

    #[test]
    fn payee_changes_reach_the_store_and_others_are_left() {
        let mut store = InMemoryPorts::stub();
        let mut coles = store.payees.payee(PayeeId::seed(2)).expect("a stub Payee");
        coles.default_category = None;

        assert!(forward(
            &mut store,
            &[
                LedgerChange::UpsertPayee(coles.clone()),
                LedgerChange::DeletePayee(PayeeId::seed(8)),
            ]
        ));
        assert!(!forward(
            &mut store,
            &[LedgerChange::DeleteTransaction(
                lib_core::TransactionId::seed(1)
            )]
        ));

        assert_eq!(store.payees.payee(coles.id), Some(coles));
        assert_eq!(store.payees.payee(PayeeId::seed(8)), None);
    }
}
