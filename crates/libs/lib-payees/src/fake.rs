//! In-memory ports and the stub seed, for tests and UI-first work. Each holds its rows in a plain
//! field so a test can set up or read state directly. [`RecordingRouter`] is a Router that only
//! records what it was asked.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use lib_core::{CategoryId, PayeeId};
use lib_toast::{Notice, Notify};

use crate::protocols::{
    PayeeCategories, PayeeCategory, PayeesDialog, PayeesOutcome, PayeesPorts, PayeesRouter,
    PortsMut, SharedPorts,
};
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

/// Usage counts a test sets per Payee; a Payee not listed is unused. With `unreadable`, every
/// count fails, as a Ledger file that can't be read would.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InMemoryPayeeUsage {
    pub uses: HashMap<PayeeId, PayeeUse>,
    pub unreadable: bool,
}

impl PayeeUsage for InMemoryPayeeUsage {
    fn payee_usage(&self, id: PayeeId) -> Option<PayeeUse> {
        (!self.unreadable).then(|| self.uses.get(&id).copied().unwrap_or_default())
    }
}

/// The Categories a test offers as defaults, in order.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InMemoryPayeeCategories {
    pub categories: Vec<PayeeCategory>,
}

impl PayeeCategories for InMemoryPayeeCategories {
    fn payee_categories(&self) -> Vec<PayeeCategory> {
        self.categories.clone()
    }
}

/// The stub Categories' leaves, with their ids, as a Payee's Default category reads them.
pub fn stub_categories() -> Vec<PayeeCategory> {
    [
        (2, "Rent"),
        (4, "Electricity"),
        (5, "Water"),
        (7, "Groceries"),
        (8, "Dining"),
        (9, "Transport"),
        (10, "Household"),
        (11, "Salary"),
        (12, "Interest"),
    ]
    .into_iter()
    .map(|(n, label)| PayeeCategory {
        id: CategoryId::seed(n),
        label: label.to_string(),
    })
    .collect()
}

/// Every Payees port in memory, together, so a test can hand a write the ones it takes.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InMemoryPorts {
    pub payees: InMemoryPayees,
    pub usage: InMemoryPayeeUsage,
    pub categories: InMemoryPayeeCategories,
}

impl InMemoryPorts {
    /// The stub seed: the stub Ledger's Payees (see [`default_payees`]) with nothing naming any,
    /// and the stub Categories' leaves. Ids match `lib-ledger`'s stub, so its Splits and Bill
    /// Plans still resolve their Payees.
    pub fn stub() -> Self {
        Self {
            payees: InMemoryPayees {
                payees: default_payees(),
            },
            usage: InMemoryPayeeUsage::default(),
            categories: InMemoryPayeeCategories {
                categories: stub_categories(),
            },
        }
    }

    pub fn split_mut(&mut self) -> PortsMut<'_> {
        PortsMut {
            payees: &mut self.payees,
            usage: &self.usage,
        }
    }

    /// These ports shared, as the Presenters hold them, with a handle a test reads them back
    /// through.
    pub fn shared(self) -> (Rc<RefCell<Self>>, SharedPorts) {
        let ports = Rc::new(RefCell::new(self));
        let shared: SharedPorts = ports.clone();
        (ports, shared)
    }
}

impl PayeesPorts for InMemoryPorts {
    fn payees(&self) -> &dyn PayeeRepository {
        &self.payees
    }

    fn usage(&self) -> &dyn PayeeUsage {
        &self.usage
    }

    fn categories(&self) -> &dyn PayeeCategories {
        &self.categories
    }

    fn split_mut(&mut self) -> PortsMut<'_> {
        InMemoryPorts::split_mut(self)
    }
}

/// What a Presenter asked its Router for, in order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RouterCall {
    OpenDialog(PayeesDialog),
    CloseDialog(Option<PayeesOutcome>),
    Notice(Notice),
}

/// A Router that does nothing but record each call, in order.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RecordingRouter {
    pub calls: Vec<RouterCall>,
}

impl RecordingRouter {
    /// The calls made since the last `take`.
    pub fn take(&mut self) -> Vec<RouterCall> {
        std::mem::take(&mut self.calls)
    }
}

impl PayeesRouter for RecordingRouter {
    /// Always opens: the recording Router builds no Presenter that could fail.
    fn open_dialog(&mut self, dialog: PayeesDialog) -> bool {
        self.calls.push(RouterCall::OpenDialog(dialog));
        true
    }

    fn close_dialog(&mut self, outcome: Option<PayeesOutcome>) {
        self.calls.push(RouterCall::CloseDialog(outcome));
    }
}

impl Notify for RecordingRouter {
    fn notify(&mut self, notice: Notice) {
        self.calls.push(RouterCall::Notice(notice));
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
        assert_eq!(
            ports.usage.payee_usage(PayeeId::seed(1)),
            Some(PayeeUse::default())
        );
    }
}
