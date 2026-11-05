//! The Payees ports as the Desktop holds them: the store `shell::router::build` is given, with
//! `PayeeUsage` and `PayeeCategories` read from the Transactions, Bills and Categories stores
//! through `lib-database`'s [`StoredPayeeReads`] (#992), and the step that keeps the Ledger's read
//! copy of the Payees in line after a Payees write. The write is committed by the input it was
//! made in (`crate::input::send`), not here.
//!
//! The ports are one value shared three ways: the Payees module's Presenters read and write them
//! as [`PayeesPorts`], and the Ledger's forwarding shim ([`forward`]) and the Bills reads hold the
//! same value as a `PayeesStore`.
//!
//! The usage is counted in the stores on each ask, whether they are the adapters or
//! `Stores::fake`'s in-memory ones, so a Split written since, by a Category reassign or the Import,
//! is counted with nothing copied from the Ledger.

use std::cell::RefCell;
use std::rc::Rc;

use gpui::{App, Entity};
use lib_database::{PayeesStore, StoredPayeeReads};
use lib_ledger::{Ledger, LedgerChange};
use lib_payees::{PayeeCategories, PayeeRepository, PayeeUsage, PayeesPorts, PortsMut};

use crate::ledger_store::LedgerStore;

/// The Payees ports the Payees module, the Ledger and the Bills reads share.
pub type DesktopPorts = Rc<RefCell<LedgerBackedPorts>>;

/// `store`'s Payees, with their use and the Categories a Payee may default to read from the other
/// domains' stores.
pub struct LedgerBackedPorts {
    store: Box<dyn PayeesStore>,
    reads: StoredPayeeReads,
}

impl LedgerBackedPorts {
    pub fn shared(store: Box<dyn PayeesStore>, reads: StoredPayeeReads) -> DesktopPorts {
        Rc::new(RefCell::new(Self { store, reads }))
    }

    /// `ledger`'s own Payees in memory, reading the other domains through `reads`, for a test that
    /// needs no storage.
    pub fn in_memory(ledger: &Ledger, reads: StoredPayeeReads) -> DesktopPorts {
        Self::shared(
            Box::new(lib_payees::fake::InMemoryPorts {
                payees: lib_payees::fake::InMemoryPayees {
                    payees: ledger.payees().to_vec(),
                },
                ..Default::default()
            }),
            reads,
        )
    }
}

impl PayeesPorts for LedgerBackedPorts {
    fn payees(&self) -> &dyn PayeeRepository {
        self.store.payees()
    }

    fn usage(&self) -> &dyn PayeeUsage {
        &self.reads
    }

    fn categories(&self) -> &dyn PayeeCategories {
        &self.reads
    }

    fn split_mut(&mut self) -> PortsMut<'_> {
        PortsMut {
            payees: self.store.payees_mut(),
            usage: &self.reads,
        }
    }
}

/// The same ports as the store they wrap, for the Ledger's shim and the Bills reads.
impl PayeesStore for LedgerBackedPorts {
    fn payees(&self) -> &dyn PayeeRepository {
        self.store.payees()
    }

    fn payees_mut(&mut self) -> &mut dyn PayeeRepository {
        self.store.payees_mut()
    }

    fn reload(&mut self) -> lib_database::Result<()> {
        self.store.reload()
    }
}

/// The forwarding shim (#831): sends each Payee row in `changes` to `store`, in the order recorded,
/// returning whether it wrote anything. What it still carries is the one writer not yet migrated:
/// Category reassign (the Default categories it clears, #813); the Import writes through
/// `lib-transaction-import`'s port (#943). Every other change is another domain's.
pub fn forward(store: &mut dyn PayeesStore, changes: &[LedgerChange]) -> bool {
    let port = store.payees_mut();
    let mut wrote = false;
    for change in changes {
        if let LedgerChange::UpsertPayee(payee) = change {
            port.upsert_payee(payee.clone());
            wrote = true;
        }
    }
    wrote
}

/// Brings the Ledger read copy of the Payees in line with the ports, announcing `Domain::Payees`
/// when it changed. Run around every Payees page event and when a Payees Dialog closes, so
/// Transactions, Bills, Import and Documents see a write at once.
pub fn refresh(ledger: &Entity<LedgerStore>, cx: &mut App) {
    ledger.update(cx, |store, cx| store.refresh_payees(cx));
}

#[cfg(test)]
mod tests {
    use lib_core::{CategoryId, PayeeId, TransactionId};
    use lib_database::SharedTransactionsStore;
    use lib_payees::fake::InMemoryPorts;

    use super::*;

    /// The ports on `ledger`'s Payees, reading its Transactions, Bill Plans and Categories in
    /// memory, with the Transactions store a test writes a Split to.
    fn on(ledger: &Ledger) -> (DesktopPorts, SharedTransactionsStore) {
        let transactions: SharedTransactionsStore =
            Rc::new(RefCell::new(lib_transactions::fake::InMemoryTransactions {
                transactions: ledger.transactions().to_vec(),
            }));
        let reads = StoredPayeeReads {
            transactions: transactions.clone(),
            bills: crate::bills::ports::in_memory_store(ledger),
            categories: crate::categories::ports::in_memory(ledger),
        };
        (LedgerBackedPorts::in_memory(ledger, reads), transactions)
    }

    #[test]
    fn payee_upserts_reach_the_store_and_others_are_left() {
        let mut store = InMemoryPorts::stub();
        let mut coles = store.payees.payee(PayeeId::seed(2)).expect("a stub Payee");
        coles.default_category = None;

        assert!(forward(
            &mut store,
            &[LedgerChange::UpsertPayee(coles.clone())]
        ));
        assert!(!forward(
            &mut store,
            &[LedgerChange::DeleteCategory(CategoryId::seed(1))]
        ));

        assert_eq!(store.payees.payee(coles.id), Some(coles));
    }

    #[test]
    fn in_memory_ports_count_the_stores_use_and_offer_its_leaves() {
        let ledger = Ledger::stub();
        let (ports, _) = on(&ledger);
        let ports = ports.borrow();

        let woolworths = PayeesPorts::usage(&*ports)
            .payee_usage(PayeeId::seed(1))
            .expect("memory always counts");
        assert!(
            woolworths.splits > 0,
            "the stub's groceries name Woolworths"
        );
        let telstra = PayeesPorts::usage(&*ports)
            .payee_usage(PayeeId::seed(20))
            .expect("memory always counts");
        assert_eq!(telstra.bill_plans, 1);
        let leaves = ports.categories().payee_categories();
        assert!(leaves.iter().any(|leaf| leaf.id == CategoryId::seed(7)));
        assert!(!leaves.iter().any(|leaf| leaf.id == CategoryId::seed(6)));
    }

    #[test]
    fn a_split_written_to_the_store_after_the_ports_are_built_is_counted() {
        let ledger = Ledger::stub();
        let woolworths = PayeeId::seed(1);
        let (ports, transactions) = on(&ledger);
        let before = PayeesPorts::usage(&*ports.borrow())
            .payee_usage(woolworths)
            .expect("memory always counts");

        let mut booked = ledger.transactions()[0].clone();
        booked.id = TransactionId::seed(9_999);
        booked.splits.truncate(1);
        booked.splits[0].payee_id = Some(woolworths);
        transactions.borrow_mut().upsert_transaction(booked);

        let uses = PayeesPorts::usage(&*ports.borrow()).payee_usage(woolworths);
        assert_eq!(uses.map(|uses| uses.splits), Some(before.splits + 1));
    }
}
