//! The Payees ports as the Desktop holds them: the store `shell::router::build` is given, with
//! `PayeeCategories` read from the Categories store (#858) and an in-memory store's usage counted
//! from the Ledger read copy (#841), and the step that keeps the Ledger's read copy of the Payees
//! in line after a Payees write. The write is committed by the input it was made in
//! (`crate::input::send`), not here.
//!
//! The ports are one value shared three ways: the Payees module's Presenters read and write them
//! as [`PayeesPorts`], and the Ledger's forwarding shim ([`forward`]) and the Bills reads hold the
//! same value as a `PayeesStore`.
//!
//! An in-memory store's usage is copied from the Ledger before each Payees event, since a port
//! can't reach the `LedgerStore` Entity without a gpui context. So a Split written since, by an
//! Import commit, is counted. The stored Desktop's usage is the adapter's, counted in the file.

use std::cell::RefCell;
use std::rc::Rc;

use gpui::{App, Entity};
use lib_database::{PayeesStore, SharedCategoriesStore};
use lib_ledger::{Ledger, LedgerChange};
use lib_payees::fake::InMemoryPayeeUsage;
use lib_payees::{
    PayeeCategories, PayeeCategory, PayeeRepository, PayeeUsage, PayeesPorts, PortsMut,
};

use crate::ledger_store::LedgerStore;

/// The Payees ports the Payees module, the Ledger and the Bills reads share.
pub type DesktopPorts = Rc<RefCell<LedgerBackedPorts>>;

/// The Categories store as the Default category select reads it, borrowed for each read: the store
/// is shared behind a `RefCell` whose guard can't be lent out of it.
struct Categories(SharedCategoriesStore);

impl PayeeCategories for Categories {
    fn payee_categories(&self) -> Vec<PayeeCategory> {
        self.0.borrow().payee_categories()
    }
}

/// `store`'s Payees and their use, with the Categories a Payee may default to read from
/// `categories`, and an in-memory store's use counted from the Ledger.
pub struct LedgerBackedPorts {
    store: Box<dyn PayeesStore>,
    categories: Categories,
}

impl LedgerBackedPorts {
    pub fn shared(store: Box<dyn PayeesStore>, categories: SharedCategoriesStore) -> DesktopPorts {
        Rc::new(RefCell::new(Self {
            store,
            categories: Categories(categories),
        }))
    }

    /// `ledger`'s own Payees in memory, their use counted from its Splits and Bill Plans, reading
    /// `categories`, for a test that needs no storage.
    pub fn in_memory(ledger: &Ledger, categories: SharedCategoriesStore) -> DesktopPorts {
        let ports = Self::shared(
            Box::new(lib_payees::fake::InMemoryPorts {
                payees: lib_payees::fake::InMemoryPayees {
                    payees: ledger.payees().to_vec(),
                },
                ..Default::default()
            }),
            categories,
        );
        ports.borrow_mut().copy_from_ledger(ledger);
        ports
    }

    fn copy_from_ledger(&mut self, ledger: &Ledger) {
        if let Some(usage) = self.store.usage_to_count() {
            usage.uses = ledger.payee_uses();
        }
    }
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

    fn usage_to_count(&mut self) -> Option<&mut InMemoryPayeeUsage> {
        self.store.usage_to_count()
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

/// Recounts an in-memory store's usage from `ledger`'s Splits and Bill Plans, for the next Payees
/// event and a view model drawn outside one.
pub fn copy_from_ledger(ports: &DesktopPorts, ledger: &Ledger) {
    ports.borrow_mut().copy_from_ledger(ledger);
}

/// Brings the Ledger read copy of the Payees in line with the ports, announcing `Domain::Payees`
/// when it changed, and recounts the usage for the next Payees event. Run before and after
/// every Payees event and Dialog confirm, so Transactions, Bills, Import and Documents see a write
/// at once.
pub fn refresh(ports: &DesktopPorts, ledger: &Entity<LedgerStore>, cx: &mut App) {
    ledger.update(cx, |store, cx| store.refresh_payees(cx));
    copy_from_ledger(ports, ledger.read(cx).ledger());
}

#[cfg(test)]
mod tests {
    use lib_core::{CategoryId, PayeeId, TagId, TransactionId};
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
            &[LedgerChange::DeleteTag(TagId::seed(1))]
        ));

        assert_eq!(store.payees.payee(coles.id), Some(coles));
    }

    #[test]
    fn in_memory_ports_count_the_ledgers_use_and_offer_its_leaves() {
        let ledger = Ledger::stub();
        let ports =
            LedgerBackedPorts::in_memory(&ledger, crate::categories::ports::in_memory(&ledger));
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
    fn a_split_written_after_the_ports_are_built_is_counted_at_the_next_copy() {
        let mut ledger = Ledger::stub();
        let woolworths = PayeeId::seed(1);
        let ports =
            LedgerBackedPorts::in_memory(&ledger, crate::categories::ports::in_memory(&ledger));
        let before = PayeesPorts::usage(&*ports.borrow())
            .payee_usage(woolworths)
            .expect("memory always counts");

        let mut transactions = ledger.transactions().to_vec();
        let mut booked = transactions[0].clone();
        booked.id = TransactionId::seed(9_999);
        booked.splits.truncate(1);
        booked.splits[0].payee_id = Some(woolworths);
        transactions.push(booked);
        ledger.refresh_transactions(transactions);
        copy_from_ledger(&ports, &ledger);

        let uses = PayeesPorts::usage(&*ports.borrow()).payee_usage(woolworths);
        assert_eq!(uses.map(|uses| uses.splits), Some(before.splits + 1));
    }
}
