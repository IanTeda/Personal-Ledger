//! The in-memory ports and the sample seed, for tests and UI-first work. Each holds its rows in a
//! plain field, in the order they were added, so a test can set up or read state directly.
//! [`RecordingRouter`] is a Router that only records what it was asked.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

use lib_core::{CategoryId, CategoryTypes, Money};
use lib_toast::{Notice, Notify};
use lib_viper::dialog::DialogRouter;

use crate::protocols::{
    CategoriesDialog, CategoriesOutcome, CategoriesRouter, CategoryBudget, CategoryHandover,
    CategoryUsage, CategoryUse, Holdings, MonthlyBudget, MonthlyBudgetRefusal, ParentStop,
    SharedCategories, TypeChangeBlockers,
};
use crate::{
    Category, CategoryRepository, descendants_inclusive, get, is_leaf, sample_categories,
    tree_order,
};

#[derive(Debug, Clone, Default, PartialEq)]
pub struct InMemoryCategories {
    /// The rows in the order they were added; [`CategoryRepository::categories`] lists them in
    /// tree order.
    pub categories: Vec<Category>,
}

impl InMemoryCategories {
    /// The sample seed: the Sample Ledger's Categories (see [`sample_categories`]). Ids match
    /// `lib-database`'s joined Sample Ledger, so its Splits, Bill Plans, Payees and Budgets still resolve theirs.
    pub fn sample() -> Self {
        Self {
            categories: sample_categories(),
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
    /// The Budgets and Bill Plans in use per Category, for [`CategoryUsage::type_change_blockers`].
    pub blockers: BTreeMap<CategoryId, TypeChangeBlockers>,
}

impl CategoryUsage for InMemoryCategoryUsage {
    fn category_usage(&self, id: CategoryId) -> CategoryUse {
        self.uses.get(&id).copied().unwrap_or_default()
    }

    /// The sum over the subtree's Categories.
    fn type_change_blockers(&self, subtree: &[CategoryId]) -> TypeChangeBlockers {
        sum_blockers(&self.blockers, subtree)
    }
}

/// One Budget's amounts this month, by Category, keeping the Budgets' rule that only a leaf
/// Expense Category is budgeted. With `archived` set, it is read-only under that name. A Stop
/// removes the Category's amount and is recorded in `stopped`, as November, in `1 + other_budgets`
/// Budgets.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InMemoryCategoryBudget {
    pub amounts: BTreeMap<CategoryId, Money>,
    pub archived: Option<String>,
    /// Active Budgets besides this one that budget the same Categories.
    pub other_budgets: usize,
    /// The Categories [`CategoryBudget::stop_new_parent`] was asked to stop, in order.
    pub stopped: Vec<CategoryId>,
    /// What each Category holds, for [`CategoryHandover`]: this fake is the one set of ports the
    /// Add and Edit tests lend, so it answers the handover too.
    pub held: BTreeMap<CategoryId, Holdings>,
    /// Each `(from, to)` [`CategoryHandover::hand_over`] was asked for, in order.
    pub handed_over: Vec<(CategoryId, CategoryId)>,
    /// The Budgets and Bill Plans in use per Category, for [`CategoryUsage::type_change_blockers`].
    pub blockers: BTreeMap<CategoryId, TypeChangeBlockers>,
}

impl CategoryUsage for InMemoryCategoryBudget {
    fn category_usage(&self, _id: CategoryId) -> CategoryUse {
        CategoryUse::default()
    }

    /// The sum over the subtree's Categories.
    fn type_change_blockers(&self, subtree: &[CategoryId]) -> TypeChangeBlockers {
        sum_blockers(&self.blockers, subtree)
    }
}

/// The blockers `blockers` holds for the Categories in `subtree`, summed.
fn sum_blockers(
    blockers: &BTreeMap<CategoryId, TypeChangeBlockers>,
    subtree: &[CategoryId],
) -> TypeChangeBlockers {
    subtree.iter().filter_map(|id| blockers.get(id)).fold(
        TypeChangeBlockers::default(),
        |sum, used| TypeChangeBlockers {
            budgets: sum.budgets + used.budgets,
            bill_plans: sum.bill_plans + used.bill_plans,
        },
    )
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

    fn parent_stop(&self, category: CategoryId) -> Option<ParentStop> {
        (self.archived.is_none() && self.amounts.contains_key(&category)).then_some(ParentStop {
            budgets: 1 + self.other_budgets,
            month: 11,
        })
    }

    fn stop_new_parent(&mut self, parent: CategoryId) {
        self.stopped.push(parent);
        self.amounts.remove(&parent);
    }
}

impl CategoryHandover for InMemoryCategoryBudget {
    fn holdings(&self, category: CategoryId) -> Holdings {
        self.held.get(&category).copied().unwrap_or_default()
    }

    /// Moves `from`'s holdings to `to`, recording the call only when there was something to move.
    fn hand_over(&mut self, from: CategoryId, to: CategoryId) {
        let Some(held) = self.held.remove(&from).filter(|held| !held.is_empty()) else {
            return;
        };
        let to_held = self.held.entry(to).or_default();
        to_held.splits += held.splits;
        to_held.bill_plans += held.bill_plans;
        to_held.payee_defaults += held.payee_defaults;
        self.handed_over.push((from, to));
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
}

impl DialogRouter<CategoriesOutcome> for RecordingRouter {
    fn close(&mut self, outcome: Option<CategoriesOutcome>) {
        self.calls.push(RouterCall::CloseDialog(outcome));
    }
}

impl Notify for RecordingRouter {
    fn notify(&mut self, notice: Notice) {
        self.calls.push(RouterCall::Notice(notice));
    }
}
