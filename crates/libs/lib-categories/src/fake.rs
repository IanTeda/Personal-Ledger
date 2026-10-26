//! The in-memory ports and the stub seed, for tests and UI-first work. Each holds its rows in a
//! plain field, in the order they were added, so a test can set up or read state directly.
//! [`RecordingRouter`] is a Router that only records what it was asked.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

use lib_core::{CategoryId, CategoryTypes, Money};
use lib_toast::{Notice, Notify};

use crate::protocols::{
    CategoriesDialog, CategoriesOutcome, CategoriesRouter, CategoryBudget, CategoryReassign,
    CategoryReassigned, CategoryUsage, CategoryUse, MonthlyBudget, MonthlyBudgetRefusal,
    ReassignRefusal, SharedCategories,
};
use crate::{
    Category, CategoryRepository, default_categories, descendants_inclusive, get, is_leaf,
    tree_order,
};

#[derive(Debug, Clone, Default, PartialEq)]
pub struct InMemoryCategories {
    /// The rows in the order they were added; [`CategoryRepository::categories`] lists them in
    /// tree order.
    pub categories: Vec<Category>,
}

impl InMemoryCategories {
    /// The stub seed: the stub Ledger's Categories (see [`default_categories`]). Ids match
    /// `lib-ledger`'s stub, so its Splits, Bill Plans, Payees and Budgets still resolve theirs.
    pub fn stub() -> Self {
        Self {
            categories: default_categories(),
        }
    }

    /// These Categories shared, as the Presenters hold them, with a handle a test reads them back
    /// through.
    pub fn shared(self) -> (Rc<RefCell<Self>>, SharedCategories) {
        let ports = Rc::new(RefCell::new(self));
        let shared: SharedCategories = ports.clone();
        (ports, shared)
    }
}

impl CategoryRepository for InMemoryCategories {
    fn categories(&self) -> Vec<Category> {
        tree_order(&self.categories)
    }

    fn upsert_category(&mut self, category: Category) {
        match self.categories.iter_mut().find(|row| row.id == category.id) {
            Some(row) => *row = category,
            None => self.categories.push(category),
        }
    }
}

/// Usage counts a test sets per Category; a Category not listed is unused.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InMemoryCategoryUsage {
    pub uses: BTreeMap<CategoryId, CategoryUse>,
}

impl CategoryUsage for InMemoryCategoryUsage {
    fn category_usage(&self, id: CategoryId) -> CategoryUse {
        self.uses.get(&id).copied().unwrap_or_default()
    }
}

/// One Budget's amounts this month, by Category, keeping the Budgets' rule that only a leaf
/// Expense Category is budgeted. With `archived` set, it is read-only under that name.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InMemoryCategoryBudget {
    pub amounts: BTreeMap<CategoryId, Money>,
    pub archived: Option<String>,
}

impl CategoryBudget for InMemoryCategoryBudget {
    fn monthly_budget(
        &self,
        categories: &[Category],
        category: Option<CategoryId>,
    ) -> MonthlyBudget {
        let amounts: Vec<&Money> = category
            .map(|id| descendants_inclusive(categories, id))
            .unwrap_or_default()
            .iter()
            .filter_map(|id| self.amounts.get(id))
            .collect();
        let amount = (!amounts.is_empty())
            .then(|| Money(amounts.into_iter().map(|amount| amount.0.clone()).sum()));
        MonthlyBudget {
            amount,
            archived: self.archived.clone(),
        }
    }

    fn check_monthly_budget(
        &self,
        categories: &[Category],
        category: CategoryId,
        amount: Option<&Money>,
    ) -> Result<(), MonthlyBudgetRefusal> {
        if self.archived.is_some() {
            return Err(MonthlyBudgetRefusal::ReadOnly);
        }
        if self.amounts.get(&category) == amount {
            return Ok(());
        }
        let expense_leaf = get(categories, category).is_some_and(|row| {
            row.category_type == CategoryTypes::Expense && is_leaf(categories, category)
        });
        if expense_leaf {
            Ok(())
        } else {
            Err(MonthlyBudgetRefusal::NotAnExpenseLeaf)
        }
    }

    /// Takes the check as made: a test's Category is written before its budget, as a Client's is.
    fn set_monthly_budget(
        &mut self,
        category: CategoryId,
        amount: Option<Money>,
    ) -> Result<(), MonthlyBudgetRefusal> {
        if self.archived.is_some() {
            return Err(MonthlyBudgetRefusal::ReadOnly);
        }
        match amount {
            Some(amount) => self.amounts.insert(category, amount),
            None => self.amounts.remove(&category),
        };
        Ok(())
    }
}

/// Category delete over shared in-memory Categories, with nothing in other domains to re-point:
/// it removes a leaf and reports `splits_moved`. With `refusal` set, every delete is refused so.
#[derive(Debug, Clone)]
pub struct InMemoryReassign {
    pub categories: Rc<RefCell<InMemoryCategories>>,
    pub splits_moved: usize,
    pub refusal: Option<ReassignRefusal>,
}

impl InMemoryReassign {
    pub fn new(categories: Rc<RefCell<InMemoryCategories>>) -> Self {
        Self {
            categories,
            splits_moved: 0,
            refusal: None,
        }
    }
}

impl CategoryReassign for InMemoryReassign {
    fn delete_category(&mut self, id: CategoryId) -> Result<CategoryReassigned, ReassignRefusal> {
        if let Some(refusal) = self.refusal {
            return Err(refusal);
        }
        let mut categories = self.categories.borrow_mut();
        let rows = &mut categories.categories;
        let name = get(rows, id).ok_or(ReassignRefusal::NotFound)?.name.clone();
        if !is_leaf(rows, id) {
            return Err(ReassignRefusal::HasChildren);
        }
        rows.retain(|row| row.id != id);
        Ok(CategoryReassigned {
            name,
            splits_moved: self.splits_moved,
        })
    }
}

/// What a Presenter asked its Router, in order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RouterCall {
    OpenDialog(CategoriesDialog),
    CloseDialog(Option<CategoriesOutcome>),
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

impl CategoriesRouter for RecordingRouter {
    /// Always opens: the recording Router builds no Presenter that could fail.
    fn open_dialog(&mut self, dialog: CategoriesDialog) -> bool {
        self.calls.push(RouterCall::OpenDialog(dialog));
        true
    }

    fn close_dialog(&mut self, outcome: Option<CategoriesOutcome>) {
        self.calls.push(RouterCall::CloseDialog(outcome));
    }
}

impl Notify for RecordingRouter {
    fn notify(&mut self, notice: Notice) {
        self.calls.push(RouterCall::Notice(notice));
    }
}
