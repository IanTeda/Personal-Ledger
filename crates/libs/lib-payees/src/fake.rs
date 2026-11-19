//! In-memory ports and the sample seed, for tests and UI-first work. Each holds its rows in a plain
//! field so a test can set up or read state directly. [`RecordingRouter`] is a Router that only
//! records what it was asked.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use lib_core::{CategoryId, CategoryRef, CategoryTypes, PayeeId};
use lib_toast::{Notice, Notify};
use lib_viper::dialog::DialogRouter;

use crate::protocols::{
    PayeeCategories, PayeeCategory, PayeesDialog, PayeesOutcome, PayeesPorts, PayeesRouter,
    PortsMut, SharedPorts,
};
use crate::{Payee, PayeeRepository, PayeeUsage, PayeeUse, sample_payees};

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

/// The sample Categories' leaves, with their ids, as a Payee's Default category reads them: in Tree
/// order, worked out from the sample's tree (`lib_core::tree_order`) rather than listed by hand, as
/// `lib-database`'s `payee_categories` lists the Categories store's leaves. No two sample leaves
/// share a name, so each label is the leaf's own name.
pub fn sample_categories() -> Vec<PayeeCategory> {
    let category = |n: u32, name: &str, parent: Option<u32>, category_type, is_leaf| CategoryRef {
        id: CategoryId::seed(n),
        name: name.to_string(),
        path: name.to_string(),
        parent: parent.map(CategoryId::seed),
        category_type,
        is_leaf,
    };
    let (expense, income) = (CategoryTypes::Expense, CategoryTypes::Income);
    let rows = vec![
        category(1, "Housing", None, expense.clone(), false),
        category(2, "Rent", Some(1), expense.clone(), true),
        category(3, "Utilities", Some(1), expense.clone(), false),
        category(4, "Electricity", Some(3), expense.clone(), true),
        category(5, "Water", Some(3), expense.clone(), true),
        category(6, "Food", None, expense.clone(), false),
        category(7, "Groceries", Some(6), expense.clone(), true),
        category(8, "Dining", Some(6), expense.clone(), true),
        category(9, "Transport", None, expense.clone(), true),
        category(10, "Household", None, expense, true),
        category(11, "Salary", None, income.clone(), true),
        category(12, "Interest", None, income, true),
    ];
    lib_core::tree_order(&rows)
        .into_iter()
        .filter(|row| row.is_leaf)
        .map(|row| PayeeCategory {
            id: row.id,
            label: row.name,
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
    /// The sample seed: the Sample Ledger's Payees (see [`sample_payees`]) with nothing naming any,
    /// and the sample Categories' leaves. Ids match `lib-database`'s joined Sample Ledger, so its Splits and Bill
    /// Plans still resolve their Payees.
    pub fn sample() -> Self {
        Self {
            payees: InMemoryPayees {
                payees: sample_payees(),
            },
            usage: InMemoryPayeeUsage::default(),
            categories: InMemoryPayeeCategories {
                categories: sample_categories(),
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
    OpenTransactions(PayeeId),
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

    fn open_transactions(&mut self, id: PayeeId) {
        self.calls.push(RouterCall::OpenTransactions(id));
    }
}

/// A Dialog's close records as the page Router's does, so one recording reads either.
impl DialogRouter<PayeesOutcome> for RecordingRouter {
    fn close(&mut self, outcome: Option<PayeesOutcome>) {
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
        let mut ports = InMemoryPorts::sample();
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
        let ports = InMemoryPorts::sample();
        assert_eq!(
            ports.usage.payee_usage(PayeeId::seed(1)),
            PayeeUse::default()
        );
    }
}
