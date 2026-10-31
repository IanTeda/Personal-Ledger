//! The Budgets store and its operations: creating, editing, archiving and the Category Limit
//! chains' edits, each checked against the rules in `budgets`' module docs.

use std::collections::BTreeMap;

use bigdecimal::{BigDecimal, RoundingMode, Signed, Zero};
use chrono::NaiveDate;
use lib_core::{AccountId, BudgetId, CategoryId, Money, Period, UnitId};

use super::figures::expense_leaves;
use super::plan::{FillSource, fill_preview, three_month_averages};
use super::protocols::{
    BudgetAccount, BudgetAccounts, BudgetCategories, BudgetCategory, BudgetReads,
};
use super::{
    Budget, Limit, LimitRecord, Method, Rollover, applied, inherited_rollover, put, tree, zero,
};

/// Errors for Budget and Category Limit operations.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum BudgetError {
    #[error("budget not found")]
    NotFound,
    #[error("a budget needs a name")]
    NameRequired,
    #[error("another budget already has that name")]
    NameTaken,
    #[error("an archived budget is read-only")]
    Archived,
    #[error("the default budget can't be archived")]
    DefaultCannotBeArchived,
    #[error("only a leaf expense category can be budgeted")]
    NotAnExpenseLeaf,
    #[error("closed months are read-only")]
    ClosedMonth,
    #[error("a budget amount can't be negative")]
    NegativeAmount,
    #[error("account not found")]
    AccountNotFound,
    #[error("an on-budget account must take transactions in the budget's unit")]
    AccountNotInUnit,
    #[error("that category has no budget amount this month")]
    NotBudgeted,
    #[error("that isn't an amount")]
    InvalidAmount,
}

/// Where a new Budget's Category Limits come from (11c's Start from).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StartFrom {
    Empty,
    /// The source Budget's current-month amounts plus Rollover.
    Copy(BudgetId),
    /// The 3-month average on the chosen Accounts; a 0 average leaves the Category Unbudgeted.
    LastThreeMonths,
}

/// What 11c submits.
#[derive(Debug, Clone, PartialEq)]
pub struct NewBudget {
    pub name: String,
    pub unit: UnitId,
    pub account_ids: Vec<AccountId>,
    pub start_from: StartFrom,
}

/// Whether a Budget Amount runs onward or for its month alone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Span {
    Onward,
    MonthOnly,
}

/// Every Budget, in creation order.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Budgets {
    list: Vec<Budget>,
}

impl Budgets {
    /// Budgets as stored or seeded, taken as they are: each was checked against the rules when it
    /// was written, and the seed is authored to them, so this skips the checks `create` runs.
    pub fn from_stored(list: Vec<Budget>) -> Self {
        Self { list }
    }

    /// The Budgets added or changed since `before`, to write whole. A Budget that stopped being the
    /// default comes first, so storage, which holds one default, never sees two.
    pub fn changed_since(&self, before: &Budgets) -> Vec<Budget> {
        let mut changed: Vec<Budget> = self
            .list
            .iter()
            .filter(|budget| before.get(budget.id) != Some(*budget))
            .cloned()
            .collect();
        changed.sort_by_key(|budget| budget.is_default);
        changed
    }

    pub fn all(&self) -> &[Budget] {
        &self.list
    }

    pub fn get(&self, id: BudgetId) -> Option<&Budget> {
        self.list.iter().find(|budget| budget.id == id)
    }

    pub fn active(&self) -> impl Iterator<Item = &Budget> {
        self.list.iter().filter(|budget| !budget.is_archived())
    }

    pub fn archived(&self) -> impl Iterator<Item = &Budget> {
        self.list.iter().filter(|budget| budget.is_archived())
    }

    /// The Budget the rail badge, the Dashboard and first launch read.
    pub fn default_budget(&self) -> Option<&Budget> {
        self.list.iter().find(|budget| budget.is_default)
    }

    fn get_mut(&mut self, id: BudgetId) -> Result<&mut Budget, BudgetError> {
        self.list
            .iter_mut()
            .find(|budget| budget.id == id)
            .ok_or(BudgetError::NotFound)
    }

    fn editable_mut(&mut self, id: BudgetId) -> Result<&mut Budget, BudgetError> {
        let budget = self.get_mut(id)?;
        if budget.is_archived() {
            return Err(BudgetError::Archived);
        }
        Ok(budget)
    }

    fn name_taken(&self, name: &str, except: Option<BudgetId>) -> bool {
        self.list
            .iter()
            .any(|b| Some(b.id) != except && b.name.eq_ignore_ascii_case(name))
    }

    /// Creates a Budget (never the default unless it is the first) and returns its id.
    pub fn create(
        &mut self,
        new: &NewBudget,
        reads: &dyn BudgetReads,
        today: NaiveDate,
    ) -> Result<BudgetId, BudgetError> {
        let name = new.name.trim();
        if name.is_empty() {
            return Err(BudgetError::NameRequired);
        }
        if self.name_taken(name, None) {
            return Err(BudgetError::NameTaken);
        }
        check_accounts(&reads.budget_accounts(), new.unit, &new.account_ids)?;
        let copy_from = match &new.start_from {
            StartFrom::Copy(source) => {
                Some(self.get(*source).ok_or(BudgetError::NotFound)?.clone())
            }
            StartFrom::Empty | StartFrom::LastThreeMonths => None,
        };
        let current = Period::of(today);
        let mut budget = Budget {
            id: BudgetId::new(),
            name: name.to_string(),
            method: Method::Limits,
            unit: new.unit,
            account_ids: new.account_ids.clone(),
            is_default: self.list.is_empty(),
            archived_at: None,
            limits: BTreeMap::new(),
        };
        if let Some(source) = copy_from {
            for id in expense_leaves(&reads.budget_categories()) {
                if let Some(found) = applied(source.chain(id), current) {
                    put(
                        budget.limits.entry(id).or_default(),
                        LimitRecord {
                            month: current,
                            limit: Limit::Onward {
                                amount: found.amount,
                                rollover: found.rollover,
                            },
                        },
                    );
                }
            }
        } else if new.start_from == StartFrom::LastThreeMonths {
            for (id, average) in three_month_averages(&budget, reads, current) {
                if !average.0.is_zero() {
                    put(
                        budget.limits.entry(id).or_default(),
                        LimitRecord {
                            month: current,
                            limit: Limit::Onward {
                                amount: average,
                                rollover: Rollover::None,
                            },
                        },
                    );
                }
            }
        }
        let id = budget.id;
        self.list.push(budget);
        Ok(id)
    }

    /// 11c's edit mode: the one place to rename a Budget or change its Accounts (Unit and Method
    /// stay locked). An id with no Account, one deleted since the Budget named it, is dropped
    /// rather than refused (#854).
    pub fn edit(
        &mut self,
        id: BudgetId,
        name: &str,
        account_ids: Vec<AccountId>,
        accounts: &dyn BudgetAccounts,
    ) -> Result<(), BudgetError> {
        let accounts = accounts.budget_accounts();
        let name = name.trim();
        if name.is_empty() {
            return Err(BudgetError::NameRequired);
        }
        if self.name_taken(name, Some(id)) {
            return Err(BudgetError::NameTaken);
        }
        let budget = self.editable_mut(id)?;
        let account_ids: Vec<AccountId> = account_ids
            .into_iter()
            .filter(|id| accounts.iter().any(|account| account.id == *id))
            .collect();
        check_accounts(&accounts, budget.unit, &account_ids)?;
        budget.name = name.to_string();
        budget.account_ids = account_ids;
        Ok(())
    }

    /// Copies Unit, Method, Accounts and the full chains as "<name> copy" (numbered when taken).
    /// The copy is never the default and is never archived.
    pub fn duplicate(&mut self, id: BudgetId) -> Result<BudgetId, BudgetError> {
        let source = self.get(id).ok_or(BudgetError::NotFound)?.clone();
        let mut name = format!("{} copy", source.name);
        let mut number = 2;
        while self.name_taken(&name, None) {
            name = format!("{} copy {number}", source.name);
            number += 1;
        }
        let copy = Budget {
            id: BudgetId::new(),
            name,
            is_default: false,
            archived_at: None,
            ..source
        };
        let new_id = copy.id;
        self.list.push(copy);
        Ok(new_id)
    }

    /// Archiving never deletes, and never the default.
    pub fn archive(&mut self, id: BudgetId, today: NaiveDate) -> Result<(), BudgetError> {
        let budget = self.get_mut(id)?;
        if budget.is_default {
            return Err(BudgetError::DefaultCannotBeArchived);
        }
        budget.archived_at.get_or_insert(today);
        Ok(())
    }

    pub fn restore(&mut self, id: BudgetId) -> Result<(), BudgetError> {
        self.get_mut(id)?.archived_at = None;
        Ok(())
    }

    /// Makes an active Budget the one default.
    pub fn set_default(&mut self, id: BudgetId) -> Result<(), BudgetError> {
        self.editable_mut(id)?;
        for budget in &mut self.list {
            budget.is_default = budget.id == id;
        }
        Ok(())
    }

    /// Writes an Onward or Month-only Budget Amount starting in `month`, replacing whatever record
    /// starts there. `rollover` of `None` inherits the record it follows.
    #[expect(
        clippy::too_many_arguments,
        reason = "one call per Plan-grid or Edit budget save"
    )]
    pub fn set_amount(
        &mut self,
        id: BudgetId,
        categories: &dyn BudgetCategories,
        category_id: CategoryId,
        month: Period,
        span: Span,
        amount: Money,
        rollover: Option<Rollover>,
        today: NaiveDate,
    ) -> Result<(), BudgetError> {
        check_leaf(&categories.budget_categories(), category_id)?;
        check_open_month(month, today)?;
        if amount.0.is_negative() {
            return Err(BudgetError::NegativeAmount);
        }
        let budget = self.editable_mut(id)?;
        let chain = budget.limits.entry(category_id).or_default();
        let rollover = rollover.unwrap_or_else(|| inherited_rollover(chain, month));
        let limit = match span {
            Span::Onward => Limit::Onward { amount, rollover },
            Span::MonthOnly => Limit::MonthOnly { amount, rollover },
        };
        put(chain, LimitRecord { month, limit });
        Ok(())
    }

    /// Writes a Stop from `month`. A Category with no amount there and no record starting there
    /// has nothing to stop, so nothing is written.
    pub fn stop(
        &mut self,
        id: BudgetId,
        categories: &dyn BudgetCategories,
        category_id: CategoryId,
        month: Period,
        today: NaiveDate,
    ) -> Result<(), BudgetError> {
        check_leaf(&categories.budget_categories(), category_id)?;
        check_open_month(month, today)?;
        let budget = self.editable_mut(id)?;
        let chain = budget.limits.entry(category_id).or_default();
        let has_record = chain.iter().any(|record| record.month == month);
        if has_record || applied(chain, month).is_some() {
            put(
                chain,
                LimitRecord {
                    month,
                    limit: Limit::Stop,
                },
            );
        }
        if chain.is_empty() {
            budget.limits.remove(&category_id);
        }
        Ok(())
    }

    /// Removes the Month-only record starting in `month`, so the month falls back to the carried
    /// Onward value. Returns whether one was removed.
    pub fn clear_month_only(
        &mut self,
        id: BudgetId,
        category_id: CategoryId,
        month: Period,
        today: NaiveDate,
    ) -> Result<bool, BudgetError> {
        check_open_month(month, today)?;
        let budget = self.editable_mut(id)?;
        let Some(chain) = budget.limits.get_mut(&category_id) else {
            return Ok(false);
        };
        let before = chain.len();
        chain.retain(|r| !(r.month == month && matches!(r.limit, Limit::MonthOnly { .. })));
        let removed = chain.len() != before;
        if chain.is_empty() {
            budget.limits.remove(&category_id);
        }
        Ok(removed)
    }

    /// The Plan grid's `r`: cycles the current month's Rollover, writing an Onward record from the
    /// current month with the same amount.
    pub fn cycle_rollover(
        &mut self,
        id: BudgetId,
        category_id: CategoryId,
        today: NaiveDate,
    ) -> Result<Rollover, BudgetError> {
        let current = Period::of(today);
        let budget = self.editable_mut(id)?;
        let chain = budget.limits.entry(category_id).or_default();
        let Some(found) = applied(chain, current) else {
            return Err(BudgetError::NotBudgeted);
        };
        let next = found.rollover.next();
        put(
            chain,
            LimitRecord {
                month: current,
                limit: Limit::Onward {
                    amount: found.amount,
                    rollover: next,
                },
            },
        );
        Ok(next)
    }

    /// Saves what was typed into a Plan cell. An amount writes an Onward or Month-only record
    /// (`0` is a real 0.00). Empty text clears: Onward writes a Stop from `month`, Month-only
    /// removes that month's own Month-only record. Returns whether anything changed.
    #[expect(
        clippy::too_many_arguments,
        reason = "one call per Plan-grid cell save"
    )]
    pub fn save_cell(
        &mut self,
        id: BudgetId,
        categories: &dyn BudgetCategories,
        category_id: CategoryId,
        month: Period,
        span: Span,
        text: &str,
        today: NaiveDate,
    ) -> Result<bool, BudgetError> {
        let text = text.trim();
        if text.is_empty() {
            return match span {
                Span::Onward => {
                    let before = self.get(id).ok_or(BudgetError::NotFound)?.clone();
                    self.stop(id, categories, category_id, month, today)?;
                    Ok(self.get(id) != Some(&before))
                }
                Span::MonthOnly => self.clear_month_only(id, category_id, month, today),
            };
        }
        let amount: Money = text.parse().map_err(|_| BudgetError::InvalidAmount)?;
        let amount = Money(amount.0.with_scale_round(2, RoundingMode::HalfUp));
        let chain = self
            .get(id)
            .ok_or(BudgetError::NotFound)?
            .chain(category_id);
        let unchanged = applied(chain, month).is_some_and(|found| found.amount == amount)
            && !chain
                .iter()
                .any(|r| r.month == month && matches!(r.limit, Limit::MonthOnly { .. }));
        if span == Span::Onward && unchanged {
            return Ok(false);
        }
        self.set_amount(
            id,
            categories,
            category_id,
            month,
            span,
            amount,
            None,
            today,
        )?;
        Ok(true)
    }

    /// Fill (9f): writes `source`'s Month-only amounts into `target`, as [`fill_preview`] lists
    /// them. Returns how many Categories it wrote.
    pub fn fill(
        &mut self,
        id: BudgetId,
        reads: &dyn BudgetReads,
        target: Period,
        source: FillSource,
        today: NaiveDate,
    ) -> Result<usize, BudgetError> {
        check_open_month(target, today)?;
        let budget = self.editable_mut(id)?;
        let preview = fill_preview(budget, reads, target, source);
        for line in &preview.changes {
            let chain = budget.limits.entry(line.category_id).or_default();
            let rollover = inherited_rollover(chain, target);
            put(
                chain,
                LimitRecord {
                    month: target,
                    limit: Limit::MonthOnly {
                        amount: line.after.clone(),
                        rollover,
                    },
                },
            );
        }
        Ok(preview.changes.len())
    }

    /// Categories 5c's write: a changed value is an Onward from the current month, clearing is a
    /// Stop from it, and an unchanged value writes nothing. Returns whether it wrote.
    pub fn set_monthly_limit(
        &mut self,
        id: BudgetId,
        categories: &dyn BudgetCategories,
        category_id: CategoryId,
        amount: Option<Money>,
        today: NaiveDate,
    ) -> Result<bool, BudgetError> {
        let current = Period::of(today);
        let existing = self
            .get(id)
            .ok_or(BudgetError::NotFound)?
            .chain(category_id);
        let existing = applied(existing, current).map(|found| found.amount);
        if existing == amount {
            return Ok(false);
        }
        match amount {
            Some(amount) => self.set_amount(
                id,
                categories,
                category_id,
                current,
                Span::Onward,
                amount,
                None,
                today,
            )?,
            None => self.stop(id, categories, category_id, current, today)?,
        }
        Ok(true)
    }

    /// The current month's Budget Amount as 5c shows it (Month-only included). A parent is a
    /// read-only rollup: the sum of its budgeted leaves, `None` when none is.
    pub fn monthly_limit(
        &self,
        id: BudgetId,
        categories: &dyn BudgetCategories,
        category_id: CategoryId,
        today: NaiveDate,
    ) -> Option<Money> {
        let budget = self.get(id)?;
        let current = Period::of(today);
        let categories = categories.budget_categories();
        let amounts: Vec<BigDecimal> = tree::descendants_inclusive(&categories, category_id)
            .into_iter()
            .filter(|below| tree::is_leaf(&categories, *below))
            .filter_map(|below| applied(budget.chain(below), current))
            .map(|found| found.amount.0)
            .collect();
        (!amounts.is_empty()).then(|| Money(amounts.into_iter().fold(zero(), |a, b| a + b)))
    }

    /// How many Budgets hold a chain for the Category: the Delete Category dialog's count.
    pub fn limit_count(&self, category_id: CategoryId) -> usize {
        self.list
            .iter()
            .filter(|budget| budget.limits.contains_key(&category_id))
            .count()
    }

    /// A deleted Category's records go with it, in every Budget (archived ones too).
    pub fn remove_category(&mut self, category_id: CategoryId) {
        for budget in &mut self.list {
            budget.limits.remove(&category_id);
        }
    }

    /// A budgeted leaf that gained a child gets an automatic Stop from the current month and loses
    /// any later record, in every active Budget; a parent only ever rolls up. Returns the Categories
    /// stopped.
    pub fn stop_new_parents(
        &mut self,
        categories: &dyn BudgetCategories,
        today: NaiveDate,
    ) -> Vec<CategoryId> {
        let categories = categories.budget_categories();
        let current = Period::of(today);
        let mut stopped = Vec::new();
        for budget in self.list.iter_mut().filter(|b| !b.is_archived()) {
            for (id, chain) in &mut budget.limits {
                if tree::is_leaf(&categories, *id) {
                    continue;
                }
                let was_budgeted = applied(chain, current).is_some();
                chain.retain(|record| record.month < current);
                if was_budgeted {
                    put(
                        chain,
                        LimitRecord {
                            month: current,
                            limit: Limit::Stop,
                        },
                    );
                    if !stopped.contains(id) {
                        stopped.push(*id);
                    }
                }
            }
        }
        stopped
    }
}

pub fn check_leaf(
    categories: &[BudgetCategory],
    category_id: CategoryId,
) -> Result<(), BudgetError> {
    if tree::is_expense(categories, category_id) && tree::is_leaf(categories, category_id) {
        Ok(())
    } else {
        Err(BudgetError::NotAnExpenseLeaf)
    }
}

fn check_open_month(month: Period, today: NaiveDate) -> Result<(), BudgetError> {
    if month < Period::of(today) {
        Err(BudgetError::ClosedMonth)
    } else {
        Ok(())
    }
}

pub fn check_accounts(
    accounts: &[BudgetAccount],
    unit: UnitId,
    ids: &[AccountId],
) -> Result<(), BudgetError> {
    for id in ids {
        let account = accounts
            .iter()
            .find(|a| a.id == *id)
            .ok_or(BudgetError::AccountNotFound)?;
        if !account.takes_transactions || account.unit != unit {
            return Err(BudgetError::AccountNotInUnit);
        }
    }
    Ok(())
}

/// Whether spending (an expense amount, negative) exceeds the budget for this category.
/// Categories 5c's row bar reads it against the month-to-date sum, which keeps the ledger's sign.
pub fn is_over_budget(spent: &Money, budget: &Money) -> bool {
    spent.0 < budget.0.clone() * -1 // spent is negative; over-budget when |spent| > budget
}
