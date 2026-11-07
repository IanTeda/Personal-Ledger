//! The Payees ports as the Desktop holds them: the store `shell::router::build` is given, with
//! `PayeeUsage` and `PayeeCategories` read from the Transactions, Bills and Categories stores
//! through `lib-database`'s [`StoredPayeeReads`] (#992). The write is committed and announced by
//! the input it was made in (`crate::input::send`), not here. The Ledger keeps no read copy of the Payees (#1005).
//!
//! The ports are one value shared three ways: the Payees module's Presenters read and write them
//! as [`PayeesPorts`], and Category delete, the Import and the Bills reads hold the same value as a
//! `PayeesStore`.
//!
//! The usage is counted in the stores on each ask, whether they are the adapters or
//! `Stores::fake`'s in-memory ones, so a Split written since, by a Category delete or the Import,
//! is counted with nothing copied from the Ledger.

use std::cell::RefCell;
use std::rc::Rc;

use lib_core::PayeeId;
use lib_database::{PayeesStore, SharedPayeesStore, StoredPayeeReads};
use lib_payees::{Payee, PayeeCategories, PayeeRepository, PayeeUsage, PayeesPorts, PortsMut};

/// The Payees ports the Payees module, Category delete, the Import and the Bills reads share.
pub type DesktopPorts = Rc<RefCell<LedgerBackedPorts>>;

/// The shared Payees store's Payees, borrowed for each call: the store is `LedgerPorts`' shared
/// one (#1082), whose guard can't be lent out of it.
struct SharedPayees {
    store: SharedPayeesStore,
}

impl PayeeRepository for SharedPayees {
    fn payees(&self) -> Vec<Payee> {
        self.store.borrow().payees().payees()
    }

    fn upsert_payee(&mut self, payee: Payee) {
        self.store.borrow_mut().payees_mut().upsert_payee(payee);
    }

    fn delete_payee(&mut self, id: PayeeId) {
        self.store.borrow_mut().payees_mut().delete_payee(id);
    }
}

/// `store`'s Payees, with their use and the Categories a Payee may default to read from the other
/// domains' stores.
pub struct LedgerBackedPorts {
    store: SharedPayeesStore,
    payees: SharedPayees,
    reads: StoredPayeeReads,
}

impl LedgerBackedPorts {
    pub fn shared(store: SharedPayeesStore, reads: StoredPayeeReads) -> DesktopPorts {
        Rc::new(RefCell::new(Self {
            payees: SharedPayees {
                store: store.clone(),
            },
            store,
            reads,
        }))
    }

    /// The sample Payees in memory, reading the other domains through `reads`, for a test that needs
    /// no storage.
    pub fn in_memory(reads: StoredPayeeReads) -> DesktopPorts {
        Self::shared(
            Rc::new(RefCell::new(lib_payees::fake::InMemoryPorts {
                payees: lib_payees::fake::InMemoryPayees {
                    payees: lib_payees::sample_payees(),
                },
                ..Default::default()
            })),
            reads,
        )
    }
}

impl PayeesPorts for LedgerBackedPorts {
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

/// The same ports as the store they wrap, for Category delete, the Import and the Bills reads.
impl PayeesStore for LedgerBackedPorts {
    fn payees(&self) -> &dyn PayeeRepository {
        &self.payees
    }

    fn payees_mut(&mut self) -> &mut dyn PayeeRepository {
        &mut self.payees
    }

    fn reload(&mut self) -> lib_database::Result<()> {
        self.store.borrow_mut().reload()
    }
}

#[cfg(test)]
mod tests {
    use lib_core::{CategoryId, PayeeId, TransactionId};
    use lib_database::{FakeStores, SharedTransactionsStore};

    use super::*;

    /// The ports on `sample`'s Payees, reading its Transactions, Bill Plans and Categories in
    /// memory, with the Transactions store a test writes a Split to.
    fn on(sample: &FakeStores) -> (DesktopPorts, SharedTransactionsStore) {
        let transactions: SharedTransactionsStore =
            Rc::new(RefCell::new(sample.transactions.clone()));
        let reads = StoredPayeeReads {
            transactions: transactions.clone(),
            bills: crate::bills::ports::in_memory_store(sample),
            categories: crate::categories::ports::in_memory(),
        };
        (LedgerBackedPorts::in_memory(reads), transactions)
    }

    #[test]
    fn in_memory_ports_count_the_stores_use_and_offer_its_leaves() {
        let (ports, _) = on(&FakeStores::sample(chrono::Local::now().date_naive()));
        let ports = ports.borrow();

        let woolworths = PayeesPorts::usage(&*ports)
            .payee_usage(PayeeId::seed(1))
            .expect("memory always counts");
        assert!(
            woolworths.splits > 0,
            "the sample's groceries name Woolworths"
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
        let sample = FakeStores::sample(chrono::Local::now().date_naive());
        let woolworths = PayeeId::seed(1);
        let (ports, transactions) = on(&sample);
        let before = PayeesPorts::usage(&*ports.borrow())
            .payee_usage(woolworths)
            .expect("memory always counts");

        let mut booked = sample.transactions.transactions[0].clone();
        booked.id = TransactionId::seed(9_999);
        booked.splits.truncate(1);
        booked.splits[0].payee_id = Some(woolworths);
        transactions.borrow_mut().upsert_transaction(booked);

        let uses = PayeesPorts::usage(&*ports.borrow()).payee_usage(woolworths);
        assert_eq!(uses.map(|uses| uses.splits), Some(before.splits + 1));
    }
}
