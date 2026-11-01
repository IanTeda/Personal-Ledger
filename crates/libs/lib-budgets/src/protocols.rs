//! The read ports the Budgets domain reads the other domains through, each returning
//! `lib-budgets`' own rows, so the crate depends on no other feature crate. They return rows, not
//! sums: Spent, the Rollover carry and Known Costs are worked out once, in [`crate::period_figures`]
//! and its helpers. `fake` implements each in memory.
//!
//! The figures read the ports together, through [`BudgetReads`].
//!
//! [`BudgetRepository`] is where the Budgets themselves are kept, synchronous as `lib-payees`' is
//! (ADR-0046): an adapter serves reads from memory and queues each write, so a write never fails
//! here; a failed save is the adapter's to report. `lib-database`'s Budgets adapter implements it
//! (#952).
//!
//! The Presenters reach the repository and the read ports together, through [`BudgetsPorts`], and
//! ask for navigation through [`BudgetsRouter`], which each Client implements.

use std::cell::RefCell;
use std::rc::Rc;

use chrono::NaiveDate;
use lib_core::{AccountId, BudgetId, CategoryId, CategoryTypes, Money, PayeeId, Period, UnitId};
use lib_toast::Notify;

use crate::Budget;

/// The Budgets on the Ledger, each with its on-budget Accounts and Category Limit chains. A Budget
/// is archived, never deleted, so there is no delete.
pub trait BudgetRepository {
    /// Every Budget, in creation order.
    fn budgets(&self) -> Vec<Budget>;

    fn budget(&self, id: BudgetId) -> Option<Budget> {
        self.budgets().into_iter().find(|budget| budget.id == id)
    }

    /// Writes `budget` whole, replacing the Budget with its id or adding it: its row, its Accounts
    /// and every chain, so a chain it no longer holds is gone.
    fn upsert_budget(&mut self, budget: &Budget);
}

/// A Category as Budgets reads it. `path` (ancestors then `name`) and `is_leaf` are the
/// Categories' rules, filled by the side that answers [`BudgetCategories`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BudgetCategory {
    pub id: CategoryId,
    pub name: String,
    pub path: String,
    pub parent: Option<CategoryId>,
    pub category_type: CategoryTypes,
    pub is_leaf: bool,
}

/// An Account as Budgets reads it. `takes_transactions` is the Accounts' rule, filled by the side
/// that answers [`BudgetAccounts`], so Budgets checks the flag rather than knowing Account types.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BudgetAccount {
    pub id: AccountId,
    pub name: String,
    pub unit: UnitId,
    pub takes_transactions: bool,
}

/// One Split of a Transaction as Budgets reads it, flattened with its Transaction's date and
/// Account.
#[derive(Debug, Clone, PartialEq)]
pub struct BudgetSplit {
    /// The Transaction's date.
    pub date: NaiveDate,
    /// The Account the Transaction is posted against.
    pub account_id: AccountId,
    pub category_id: CategoryId,
    /// The Ledger's sign: expenses are negative, so Spent negates it.
    pub amount: Money,
    pub payee_id: Option<PayeeId>,
}

/// An open Bill Schedule entry as Known Costs read it, with its Bill Plan's Category, Account and
/// estimated Amount.
#[derive(Debug, Clone, PartialEq)]
pub struct BudgetBill {
    pub due: NaiveDate,
    pub category_id: CategoryId,
    pub account_id: AccountId,
    /// The Plan's estimated Amount, positive.
    pub amount: Money,
}

/// An active Bill Plan's name, Category and Account, which the Category detail counts and Stop
/// budgeting names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BudgetBillPlan {
    pub name: String,
    pub category_id: CategoryId,
    pub account_id: AccountId,
}

/// The Category tree a Budget limits.
pub trait BudgetCategories {
    /// Every Category in tree order: depth-first from each top-level Category, each parent before
    /// its children. A row the tree can't reach is left out.
    fn budget_categories(&self) -> Vec<BudgetCategory>;
}

/// Rows already read answer the port as themselves, for a caller holding a tree the port's side
/// hasn't saved yet.
impl BudgetCategories for Vec<BudgetCategory> {
    fn budget_categories(&self) -> Vec<BudgetCategory> {
        self.clone()
    }
}

/// The Accounts a Budget may count: every Account, flagged, not a filtered list.
pub trait BudgetAccounts {
    fn budget_accounts(&self) -> Vec<BudgetAccount>;
}

/// The Splits Spent is counted from.
pub trait BudgetSplits {
    /// The Splits of the Transactions dated from `from` to `to`, both included, in the
    /// Transactions' order.
    fn budget_splits(&self, from: NaiveDate, to: NaiveDate) -> Vec<BudgetSplit>;
}

/// The Bills Known Costs are counted from.
pub trait BudgetBills {
    /// Every open Bill Schedule entry, whatever its due date.
    fn open_bills(&self) -> Vec<BudgetBill>;

    /// Every active Bill Plan.
    fn active_bill_plans(&self) -> Vec<BudgetBillPlan>;
}

/// The Units a Budget's amounts are named in. A read port, so `lib-budgets` needn't depend on the
/// Units crate to write `aud`.
pub trait BudgetUnits {
    /// Unit `id`'s code, or `None` when it is gone.
    fn unit_code(&self, id: UnitId) -> Option<String>;
}

/// The Payees the Category detail names its Splits by.
pub trait BudgetPayees {
    /// Payee `id`'s name, or `None` when it is gone.
    fn payee_name(&self, id: PayeeId) -> Option<String>;
}

/// Every Budgets read port, as the one value the figures take.
pub trait BudgetReads:
    BudgetCategories + BudgetAccounts + BudgetSplits + BudgetBills + BudgetUnits + BudgetPayees
{
}

impl<T> BudgetReads for T where
    T: BudgetCategories
        + BudgetAccounts
        + BudgetSplits
        + BudgetBills
        + BudgetUnits
        + BudgetPayees
        + ?Sized
{
}

/// The repository and the read ports, as one value the Presenters share. An Interactor writes the
/// Budgets while reading the other domains, so [`Self::split_mut`] lends both at once.
pub trait BudgetsPorts {
    fn budgets(&self) -> &dyn BudgetRepository;

    fn reads(&self) -> &dyn BudgetReads;

    fn split_mut(&mut self) -> PortsMut<'_>;
}

/// The ports borrowed together for one Interactor call.
pub struct PortsMut<'a> {
    pub budgets: &'a mut dyn BudgetRepository,
    pub reads: &'a dyn BudgetReads,
}

/// The ports as the page and its Dialogs hold them: one set, shared, so a Budget Manage archives is
/// on the page as soon as the Dialog closes. Single-threaded, as both Clients' UI threads are.
pub type SharedPorts = Rc<RefCell<dyn BudgetsPorts>>;

/// What a Manage budgets row's action does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManageAction {
    Open,
    Edit,
    Duplicate,
    SetDefault,
    Archive,
    Restore,
}

/// What a closed Dialog hands the page: a Budget to show, or the Dialog the page opens next, since a
/// Dialog's Router can't open another.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BudgetsOutcome {
    /// Show this Budget.
    Show(BudgetId),
    /// Show this Budget and open its edit mode: the copy a duplicate made.
    ShowAndEdit(BudgetId),
    /// Open New budget.
    NewBudget,
    /// Open this Budget's edit mode.
    EditBudget(BudgetId),
    /// Open Manage budgets.
    Manage,
    /// Open Edit budget on this Category, starting in this month: the Category detail's **Edit
    /// budget**.
    Limit(CategoryId, Period),
    /// Open Stop budgeting on this Category from this month: Edit budget's footer link.
    Stop(CategoryId, Period),
    /// Show Transactions filtered to this Category and month: the Category detail's hand-off.
    Transactions(CategoryId, Period),
}

/// The History table as CSV, which the Router saves where the person chooses and reports with a
/// Toast (#811 Q15).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportHistory {
    pub csv: String,
}

/// Navigation out of the Budgets Presenters, implemented by each Client and passed into each
/// `handle` call, since the Desktop can only open a Dialog with the gpui context that call runs in.
/// A Toast or status-line message goes to the Shell through [`Notify`] (ADR-0048).
///
/// Each `open_*` builds the Dialog's Presenter from the Budget and month it is given and answers
/// whether it opened one: a Category with nothing to edit or stop opens nothing.
pub trait BudgetsRouter: Notify {
    /// The Switcher popover, with Budget `current` highlighted.
    fn open_switcher(&mut self, current: BudgetId) -> bool;

    /// Manage budgets, with Budget `current` under the cursor.
    fn open_manage(&mut self, current: BudgetId) -> bool;

    /// New budget, naming Budget `current` as the one Copy categories copies.
    fn open_new_budget(&mut self, current: BudgetId) -> bool;

    /// Budget `id`'s edit mode: rename it or change its Accounts.
    fn open_edit_budget(&mut self, id: BudgetId) -> bool;

    /// Edit budget on Category `category` of Budget `budget`, starting in `month`, or its Category
    /// picker when the Category has no amount this month.
    fn open_limit(&mut self, budget: BudgetId, category: CategoryId, month: Period) -> bool;

    /// Budget a category: Edit budget with its Category picker.
    fn open_limit_picker(&mut self, budget: BudgetId) -> bool;

    /// Stop budgeting Category `category` of Budget `budget` from `month`.
    fn open_stop(&mut self, budget: BudgetId, category: CategoryId, month: Period) -> bool;

    /// Fill `month` of Budget `budget`.
    fn open_fill(&mut self, budget: BudgetId, month: Period) -> bool;

    /// The Category detail for Category `category` of Budget `budget` in `month`.
    fn open_detail(&mut self, budget: BudgetId, category: CategoryId, month: Period) -> bool;

    /// Saves the History export where the person chooses, and reports how it went.
    fn export_history(&mut self, export: ExportHistory);

    /// Shows the Bills Schedule on `period`.
    fn show_bills_period(&mut self, period: Period);

    /// Shows Transactions filtered to Category `category` in `month` and Budget `budget`'s
    /// on-budget Accounts.
    fn show_transactions(&mut self, budget: BudgetId, category: CategoryId, month: Period);

    /// Closes the open Dialog. `outcome` is what it hands the page, `None` when it closed with
    /// nothing to show.
    fn close_dialog(&mut self, outcome: Option<BudgetsOutcome>);
}
