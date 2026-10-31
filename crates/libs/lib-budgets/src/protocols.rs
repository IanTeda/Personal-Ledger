//! The read ports the Budgets domain reads the other domains through, each returning
//! `lib-budgets`' own rows, so the crate depends on no other feature crate. They return rows, not
//! sums: Spent, the Rollover carry and Known Costs are worked out once, in [`crate::period_figures`]
//! and its helpers. `fake` implements each in memory.
//!
//! The figures read the ports together, through [`BudgetReads`].

use chrono::NaiveDate;
use lib_core::{AccountId, CategoryId, CategoryTypes, Money, PayeeId, UnitId};

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

/// An active Bill Plan's Category and Account, which the Category detail counts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BudgetBillPlan {
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
