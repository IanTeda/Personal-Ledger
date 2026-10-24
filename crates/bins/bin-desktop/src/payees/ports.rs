//! The Payees ports as the Desktop holds them: the store `shell::router::build` is given, with
//! `PayeeCategories` copied from the Ledger read copy (#832's halfway shim), and the step that
//! keeps the Ledger's read copy of the Payees in line after a Payees write. The write is committed
//! by the input it was made in (`crate::input::send`), not here.
//!
//! The ports are one value shared three ways: the Payees module's Presenters read and write them
//! as [`PayeesPorts`], and the Ledger's forwarding shim ([`forward`]) and the Bills reads hold the
//! same value as a `PayeesStore`.
//!
//! The Categories are copied from the Ledger before each Payees event, since a port can't reach
//! the `LedgerStore` Entity without a gpui context. The copy goes when Categories answer it
//! through their own port (#808).

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use gpui::{App, Entity};
use lib_core::PayeeId;
use lib_database::PayeesStore;
use lib_ledger::{Ledger, LedgerChange};
use lib_payees::fake::{InMemoryPayeeCategories, InMemoryPayeeUsage};
use lib_payees::{PayeeCategories, PayeeRepository, PayeeUsage, PayeeUse, PayeesPorts, PortsMut};

use crate::ledger_store::LedgerStore;

/// The Payees ports the Payees module, the Ledger and the Bills reads share.
pub type DesktopPorts = Rc<RefCell<LedgerBackedPorts>>;

/// `store`'s Payees and their use, with the Categories a Payee may default to copied from the
/// Ledger.
pub struct LedgerBackedPorts {
    store: Box<dyn PayeesStore>,
    categories: InMemoryPayeeCategories,
}

impl LedgerBackedPorts {
    pub fn shared(store: Box<dyn PayeesStore>) -> DesktopPorts {
        Rc::new(RefCell::new(Self {
            store,
            categories: InMemoryPayeeCategories::default(),
        }))
    }

    /// `ledger`'s own Payees in memory, their use counted from its Splits and Bill Plans, for a
    /// test that needs no storage.
    pub fn in_memory(ledger: &Ledger) -> DesktopPorts {
        let ports = Self::shared(Box::new(lib_payees::fake::InMemoryPorts {
            payees: lib_payees::fake::InMemoryPayees {
                payees: ledger.payees().to_vec(),
            },
            usage: usage_in(ledger),
            ..Default::default()
        }));
        ports.borrow_mut().copy_categories(ledger);
        ports
    }

    fn copy_categories(&mut self, ledger: &Ledger) {
        self.categories.categories = ledger.payee_categories();
    }
}

/// How many of `ledger`'s Splits and Bill Plans name each Payee, as in-memory usage: what the
/// Payees adapter counts in the file.
fn usage_in(ledger: &Ledger) -> InMemoryPayeeUsage {
    let mut uses: HashMap<PayeeId, PayeeUse> = HashMap::new();
    let splits = ledger.transactions().iter().flat_map(|t| &t.splits);
    for id in splits.filter_map(|split| split.payee_id) {
        uses.entry(id).or_default().splits += 1;
    }
    for id in ledger.bill_plans().iter().filter_map(|plan| plan.payee_id) {
        uses.entry(id).or_default().bill_plans += 1;
    }
    InMemoryPayeeUsage { uses }
}

impl PayeesPorts for LedgerBackedPorts {
    fn payees(&self) -> &dyn PayeeRepository {
        self.store.payees()
    }

    fn usage(&self) -> &dyn PayeeUsage {
        self.store.usage()
    }

    fn categories(&self) -> &dyn PayeeCategories {
        &self.categories
    }

    fn split_mut(&mut self) -> PortsMut<'_> {
        self.store.split_mut()
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

    fn usage(&self) -> &dyn PayeeUsage {
        self.store.usage()
    }

    fn split_mut(&mut self) -> PortsMut<'_> {
        self.store.split_mut()
    }

    fn bill_payees(&self) -> Vec<lib_bills::BillPayee> {
        self.store.bill_payees()
    }

    fn reload(&mut self) -> lib_database::Result<()> {
        self.store.reload()
    }
}

/// The forwarding shim (#831): sends each Payee row in `changes` to `store`, in the order recorded,
/// returning whether it wrote anything. What it still carries are the writers not yet migrated:
/// Import commit (the Payees it creates and the match rules it remembers, #810) and Category
/// reassign (the Default categories it clears, #813). Every other change is another domain's.
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

/// Copies the Categories a Payee may default to from `ledger` into `ports`, for the next Payees
/// event and a view model drawn outside one.
pub fn copy_categories(ports: &DesktopPorts, ledger: &Ledger) {
    ports.borrow_mut().copy_categories(ledger);
}

/// Brings the Ledger read copy of the Payees in line with the ports, announcing `Domain::Payees`
/// when it changed, and copies the Categories back for the next Payees event. Run before and after
/// every Payees event and Dialog confirm, so Transactions, Bills, Import and Documents see a write
/// at once.
pub fn refresh(ports: &DesktopPorts, ledger: &Entity<LedgerStore>, cx: &mut App) {
    ledger.update(cx, |store, cx| store.refresh_payees(cx));
    copy_categories(ports, ledger.read(cx).ledger());
}

#[cfg(test)]
mod tests {
    use lib_core::{CategoryId, TransactionId};
    use lib_payees::fake::InMemoryPorts;

    use super::*;

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
            &[LedgerChange::DeleteTransaction(TransactionId::seed(1))]
        ));

        assert_eq!(store.payees.payee(coles.id), Some(coles));
    }

    #[test]
    fn in_memory_ports_count_the_ledgers_use_and_offer_its_leaves() {
        let ledger = Ledger::stub();
        let ports = LedgerBackedPorts::in_memory(&ledger);
        let ports = ports.borrow();

        let woolworths = PayeesPorts::usage(&*ports).payee_usage(PayeeId::seed(1));
        assert!(
            woolworths.splits > 0,
            "the stub's groceries name Woolworths"
        );
        let telstra = PayeesPorts::usage(&*ports).payee_usage(PayeeId::seed(20));
        assert_eq!(telstra.bill_plans, 1);
        let leaves = ports.categories().payee_categories();
        assert!(leaves.iter().any(|leaf| leaf.id == CategoryId::seed(7)));
        assert!(!leaves.iter().any(|leaf| leaf.id == CategoryId::seed(6)));
    }
}
