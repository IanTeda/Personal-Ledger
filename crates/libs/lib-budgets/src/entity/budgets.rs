//! The Budgets set: every Budget, and the rules that hold across them -- a unique name, one
//! default that can't be archived, and creating, editing, duplicating and archiving a Budget.

use std::collections::BTreeMap;

use bigdecimal::{BigDecimal, Zero};
use chrono::NaiveDate;
use lib_core::{AccountId, BudgetId, CategoryId, Money, Period, UnitId};

use crate::figures::expense_leaves;
use crate::plan::{FillSource, three_month_averages};
use crate::protocols::{BudgetAccount, BudgetAccounts, BudgetCategories, BudgetReads};
use crate::{
    Budget, Error, Limit, LimitRecord, Method, Result, Rollover, Span, applied, put, tree, zero,
};

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

    fn get_mut(&mut self, id: BudgetId) -> Result<&mut Budget> {
        self.list
            .iter_mut()
            .find(|budget| budget.id == id)
            .ok_or(Error::NotFound)
    }

    fn editable_mut(&mut self, id: BudgetId) -> Result<&mut Budget> {
        let budget = self.get_mut(id)?;
        if budget.is_archived() {
            return Err(Error::Archived);
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
    ) -> Result<BudgetId> {
        let name = new.name.trim();
        if name.is_empty() {
            return Err(Error::NameRequired);
        }
        if self.name_taken(name, None) {
            return Err(Error::NameTaken);
        }
        check_accounts(&reads.budget_accounts(), new.unit, &new.account_ids)?;
        let copy_from = match &new.start_from {
            StartFrom::Copy(source) => Some(self.get(*source).ok_or(Error::NotFound)?.clone()),
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
    ) -> Result<()> {
        let accounts = accounts.budget_accounts();
        let name = name.trim();
        if name.is_empty() {
            return Err(Error::NameRequired);
        }
        if self.name_taken(name, Some(id)) {
            return Err(Error::NameTaken);
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
    pub fn duplicate(&mut self, id: BudgetId) -> Result<BudgetId> {
        let source = self.get(id).ok_or(Error::NotFound)?.clone();
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
    pub fn archive(&mut self, id: BudgetId, today: NaiveDate) -> Result<()> {
        let budget = self.get_mut(id)?;
        if budget.is_default {
            return Err(Error::DefaultCannotBeArchived);
        }
        budget.archived_at.get_or_insert(today);
        Ok(())
    }

    pub fn restore(&mut self, id: BudgetId) -> Result<()> {
        self.get_mut(id)?.archived_at = None;
        Ok(())
    }

    /// Makes an active Budget the one default.
    pub fn set_default(&mut self, id: BudgetId) -> Result<()> {
        self.editable_mut(id)?;
        for budget in &mut self.list {
            budget.is_default = budget.id == id;
        }
        Ok(())
    }

    /// Budget `id`'s [`Budget::set_amount`].
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
    ) -> Result<()> {
        self.get_mut(id)?.set_amount(
            categories,
            category_id,
            month,
            span,
            amount,
            rollover,
            today,
        )
    }

    /// Budget `id`'s [`Budget::stop`].
    pub fn stop(
        &mut self,
        id: BudgetId,
        categories: &dyn BudgetCategories,
        category_id: CategoryId,
        month: Period,
        today: NaiveDate,
    ) -> Result<()> {
        self.get_mut(id)?
            .stop(categories, category_id, month, today)
    }

    /// Budget `id`'s [`Budget::clear_month_only`].
    pub fn clear_month_only(
        &mut self,
        id: BudgetId,
        category_id: CategoryId,
        month: Period,
        today: NaiveDate,
    ) -> Result<bool> {
        self.get_mut(id)?
            .clear_month_only(category_id, month, today)
    }

    /// Budget `id`'s [`Budget::cycle_rollover`].
    pub fn cycle_rollover(
        &mut self,
        id: BudgetId,
        category_id: CategoryId,
        today: NaiveDate,
    ) -> Result<Rollover> {
        self.get_mut(id)?.cycle_rollover(category_id, today)
    }

    /// Budget `id`'s [`Budget::save_cell`].
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
    ) -> Result<bool> {
        self.get_mut(id)?
            .save_cell(categories, category_id, month, span, text, today)
    }

    /// Budget `id`'s [`Budget::fill`].
    pub fn fill(
        &mut self,
        id: BudgetId,
        reads: &dyn BudgetReads,
        target: Period,
        source: FillSource,
        today: NaiveDate,
    ) -> Result<usize> {
        self.get_mut(id)?.fill(reads, target, source, today)
    }

    /// Budget `id`'s [`Budget::set_monthly_limit`].
    pub fn set_monthly_limit(
        &mut self,
        id: BudgetId,
        categories: &dyn BudgetCategories,
        category_id: CategoryId,
        amount: Option<Money>,
        today: NaiveDate,
    ) -> Result<bool> {
        self.get_mut(id)?
            .set_monthly_limit(categories, category_id, amount, today)
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
    /// any later record, in every active Budget; a parent only ever rolls up. Returns how many
    /// Budgets changed. The caller says which Category gained the child.
    pub fn stop_new_parent(&mut self, category_id: CategoryId, today: NaiveDate) -> usize {
        self.list
            .iter_mut()
            .map(|budget| budget.stop_new_parent(category_id, today))
            .filter(|changed| *changed)
            .count()
    }

    /// How many active Budgets hold a Budget Amount for the Category this month: the Budgets a new
    /// child would Stop it in.
    pub fn budgeting(&self, category_id: CategoryId, today: NaiveDate) -> usize {
        let current = Period::of(today);
        self.active()
            .filter(|budget| applied(budget.chain(category_id), current).is_some())
            .count()
    }

    /// How many active Budgets use any of `categories` this month or later: a Budget Amount in
    /// effect this month, or a record starting after it that isn't a Stop. A chain Stopped before
    /// this month, and an archived Budget, don't count. Those Budgets are what a type change to
    /// the Categories would leave budgeting the wrong kind.
    pub fn using(&self, categories: &[CategoryId], today: NaiveDate) -> usize {
        let current = Period::of(today);
        self.active()
            .filter(|budget| {
                categories.iter().any(|id| {
                    let chain = budget.chain(*id);
                    applied(chain, current).is_some()
                        || chain.iter().any(|record| {
                            record.month > current && !matches!(record.limit, Limit::Stop)
                        })
                })
            })
            .count()
    }
}

pub fn check_accounts(accounts: &[BudgetAccount], unit: UnitId, ids: &[AccountId]) -> Result<()> {
    for id in ids {
        let account = accounts
            .iter()
            .find(|a| a.id == *id)
            .ok_or(Error::AccountNotFound)?;
        if !account.takes_transactions || account.unit != unit {
            return Err(Error::AccountNotInUnit);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::*;

    /// Editing a Budget that names a deleted Account drops the id rather than refusing the edit.
    #[test]
    fn editing_a_budget_drops_an_account_since_deleted() {
        let (mut world, mut budgets, id) = september();
        let offset = world.account("ANZ Offset");
        world.reads.accounts.retain(|account| account.id != offset);
        let account_ids = budgets
            .get(id)
            .map(|budget| budget.account_ids.clone())
            .unwrap_or_default();

        assert_eq!(
            budgets.edit(id, "Renamed", account_ids, world.ledger()),
            Ok(())
        );

        assert_eq!(
            budgets.get(id).map(|budget| budget.account_ids.clone()),
            Some(vec![world.account("ANZ Everyday")])
        );
    }

    #[test]
    fn a_parents_monthly_limit_is_the_sum_of_its_budgeted_leaves() {
        let world = World::new();
        let (mut budgets, id) = world.budgets();
        set(
            &mut budgets,
            &world,
            id,
            "Groceries",
            sep(),
            Span::Onward,
            "500.00",
            None,
        );
        set(
            &mut budgets,
            &world,
            id,
            "Dining",
            sep(),
            Span::MonthOnly,
            "300.00",
            None,
        );
        let food = world.category("Food");
        assert_eq!(
            budgets.monthly_limit(id, world.ledger(), food, today()),
            Some(money("800.00"))
        );
        assert_eq!(
            budgets.monthly_limit(id, world.ledger(), world.category("Housing"), today()),
            None
        );
    }

    #[test]
    fn a_deleted_categorys_records_go_with_it_in_every_budget() {
        let world = World::new();
        let (mut budgets, id) = world.budgets();
        set(
            &mut budgets,
            &world,
            id,
            "Dining",
            sep(),
            Span::Onward,
            "250.00",
            None,
        );
        let dining = world.category("Dining");
        assert_eq!(budgets.limit_count(dining), 1);
        budgets.remove_category(dining);
        assert_eq!(budgets.limit_count(dining), 0);
    }

    #[test]
    fn the_first_budget_is_the_default_and_later_ones_are_not() {
        let world = World::new();
        let (mut budgets, first) = world.budgets();
        let second = budgets.create(
            &new_budget(&world, "Second", StartFrom::Empty),
            world.ledger(),
            today(),
        );
        assert_eq!(budgets.default_budget().map(|b| b.id), Some(first));
        assert_eq!(
            second
                .ok()
                .and_then(|id| budgets.get(id))
                .map(|b| b.is_default),
            Some(false)
        );
    }

    #[test]
    fn a_budget_needs_a_unique_name_and_same_unit_accounts_that_take_transactions() {
        let world = World::new();
        let (mut budgets, _) = world.budgets();
        let ledger = world.ledger();
        let create = |budgets: &mut Budgets, new: NewBudget| budgets.create(&new, ledger, today());
        assert_eq!(
            create(&mut budgets, new_budget(&world, "  ", StartFrom::Empty)),
            Err(Error::NameRequired)
        );
        assert_eq!(
            create(&mut budgets, new_budget(&world, "test", StartFrom::Empty)),
            Err(Error::NameTaken)
        );
        let mut wrong_unit = new_budget(&world, "Crypto", StartFrom::Empty);
        wrong_unit.account_ids = vec![world.account("Bitcoin")];
        assert_eq!(
            create(&mut budgets, wrong_unit),
            Err(Error::AccountNotInUnit)
        );
        let mut loan = new_budget(&world, "Loans", StartFrom::Empty);
        loan.account_ids = vec![world.account("Home Loan")];
        assert_eq!(create(&mut budgets, loan), Err(Error::AccountNotInUnit));
        let mut missing = new_budget(&world, "Missing", StartFrom::Empty);
        missing.account_ids = vec![AccountId::seed(9_999)];
        assert_eq!(create(&mut budgets, missing), Err(Error::AccountNotFound));
    }

    #[test]
    fn start_from_copy_takes_the_sources_current_amounts_and_rollover_as_onward() {
        let world = World::new();
        let (mut budgets, source) = world.budgets();
        set(
            &mut budgets,
            &world,
            source,
            "Dining",
            sep(),
            Span::MonthOnly,
            "250.00",
            Some(Rollover::CarryBoth),
        );
        set(
            &mut budgets,
            &world,
            source,
            "Water",
            sep().next(),
            Span::Onward,
            "60.00",
            None,
        );
        let copy = budgets.create(
            &new_budget(&world, "Copy", StartFrom::Copy(source)),
            world.ledger(),
            today(),
        );
        let copied = copy
            .ok()
            .and_then(|id| budgets.get(id))
            .cloned()
            .unwrap_or_else(unreachable_budget);
        let dining = copied.chain(world.category("Dining"));
        assert_eq!(amount_in(dining, sep()), Some(money("250.00")));
        assert_eq!(amount_in(dining, sep().next()), Some(money("250.00")));
        assert_eq!(
            applied(dining, sep()).map(|a| a.rollover),
            Some(Rollover::CarryBoth)
        );
        // A future-only amount isn't in the source's current month.
        assert!(copied.chain(world.category("Water")).is_empty());
    }

    #[test]
    fn start_from_last_three_months_leaves_a_zero_average_unbudgeted() {
        let mut world = World::new();
        world.post("ANZ Everyday", "Dining", date(2026, 8, 4), "-303.00");
        world.post("ANZ Everyday", "Dining", date(2026, 7, 4), "-300.00");
        let (mut budgets, _) = world.budgets();
        let created = budgets.create(
            &new_budget(&world, "Averages", StartFrom::LastThreeMonths),
            world.ledger(),
            today(),
        );
        let made = created
            .ok()
            .and_then(|id| budgets.get(id))
            .cloned()
            .unwrap_or_else(unreachable_budget);
        // 603 over 3 months is 201, rounded to 200.
        assert_eq!(
            amount_in(made.chain(world.category("Dining")), sep()),
            Some(money("200.00"))
        );
        assert!(made.chain(world.category("Water")).is_empty());
        assert_eq!(
            applied(made.chain(world.category("Dining")), sep()).map(|a| a.rollover),
            Some(Rollover::None)
        );
    }

    #[test]
    fn edit_renames_and_changes_accounts_but_keeps_the_unit() {
        let world = World::new();
        let (mut budgets, id) = world.budgets();
        let accounts = vec![world.account("Wallet")];
        assert_eq!(
            budgets.edit(id, " Renamed ", accounts.clone(), world.ledger()),
            Ok(())
        );
        let edited = budgets.get(id);
        assert_eq!(edited.map(|b| b.name.as_str()), Some("Renamed"));
        assert_eq!(edited.map(|b| b.account_ids.clone()), Some(accounts));
        assert_eq!(
            budgets.edit(id, "", vec![], world.ledger()),
            Err(Error::NameRequired)
        );
        assert_eq!(
            budgets.edit(id, "X", vec![world.account("Bitcoin")], world.ledger()),
            Err(Error::AccountNotInUnit)
        );
    }

    #[test]
    fn duplicating_copies_the_chains_and_numbers_a_taken_name() {
        let world = World::new();
        let (mut budgets, id) = world.budgets();
        set(
            &mut budgets,
            &world,
            id,
            "Dining",
            sep(),
            Span::Onward,
            "250.00",
            Some(Rollover::CarryUnspent),
        );
        let first = budgets.duplicate(id).expect("the id exists");
        let second = budgets.duplicate(id).expect("the id exists");
        assert_eq!(
            budgets.get(first).map(|b| b.name.as_str()),
            Some("Test copy")
        );
        assert_eq!(
            budgets.get(second).map(|b| b.name.as_str()),
            Some("Test copy 2")
        );
        let copy = budgets
            .get(first)
            .cloned()
            .unwrap_or_else(unreachable_budget);
        assert!(!copy.is_default && !copy.is_archived());
        assert_eq!(copy.unit, UnitId::SEED_AUD);
        assert_eq!(
            copy.chain(world.category("Dining")),
            budgets
                .get(id)
                .map(|b| b.chain(world.category("Dining")))
                .unwrap_or_default()
        );
        assert_eq!(budgets.duplicate(BudgetId::seed(99)), Err(Error::NotFound));
    }

    #[test]
    fn the_default_cant_be_archived_but_another_can_be_archived_and_restored() {
        let world = World::new();
        let (mut budgets, first) = world.budgets();
        let second = budgets
            .create(
                &new_budget(&world, "Second", StartFrom::Empty),
                world.ledger(),
                today(),
            )
            .expect("the id exists");
        assert_eq!(
            budgets.archive(first, today()),
            Err(Error::DefaultCannotBeArchived)
        );
        assert_eq!(budgets.archive(second, today()), Ok(()));
        assert_eq!(
            budgets.get(second).and_then(|b| b.archived_at),
            Some(today())
        );
        assert_eq!(budgets.active().count(), 1);
        assert_eq!(budgets.archived().count(), 1);
        assert_eq!(budgets.restore(second), Ok(()));
        assert_eq!(budgets.active().count(), 2);
    }

    #[test]
    fn set_default_moves_the_flag_and_refuses_an_archived_budget() {
        let world = World::new();
        let (mut budgets, first) = world.budgets();
        let second = budgets
            .create(
                &new_budget(&world, "Second", StartFrom::Empty),
                world.ledger(),
                today(),
            )
            .expect("the id exists");
        let third = budgets
            .create(
                &new_budget(&world, "Third", StartFrom::Empty),
                world.ledger(),
                today(),
            )
            .expect("the id exists");
        assert_eq!(budgets.archive(third, today()), Ok(()));
        assert_eq!(budgets.set_default(third), Err(Error::Archived));
        assert_eq!(budgets.set_default(second), Ok(()));
        assert_eq!(budgets.all().iter().filter(|b| b.is_default).count(), 1);
        assert_eq!(budgets.default_budget().map(|b| b.id), Some(second));
        // The old default can now be archived.
        assert_eq!(budgets.archive(first, today()), Ok(()));
    }

    #[test]
    fn an_archived_budget_is_read_only_but_still_computed() {
        let mut world = World::new();
        world.post("ANZ Everyday", "Dining", date(2026, 9, 4), "-40.00");
        let (mut budgets, first) = world.budgets();
        set(
            &mut budgets,
            &world,
            first,
            "Dining",
            sep(),
            Span::Onward,
            "250.00",
            None,
        );
        let second = budgets
            .create(
                &new_budget(&world, "Second", StartFrom::Empty),
                world.ledger(),
                today(),
            )
            .expect("the id exists");
        set(
            &mut budgets,
            &world,
            second,
            "Dining",
            sep(),
            Span::Onward,
            "100.00",
            None,
        );
        assert_eq!(budgets.archive(second, today()), Ok(()));
        let refused = budgets.set_amount(
            second,
            world.ledger(),
            world.category("Dining"),
            sep(),
            Span::Onward,
            money("1.00"),
            None,
            today(),
        );
        assert_eq!(refused, Err(Error::Archived));
        assert_eq!(
            budgets.edit(second, "New name", vec![], world.ledger()),
            Err(Error::Archived)
        );
        let archived = budgets
            .get(second)
            .cloned()
            .unwrap_or_else(unreachable_budget);
        let figures = period_figures(&archived, world.ledger(), sep(), today());
        assert_eq!(figures.budgeted, money("100.00"));
    }

    /// A Budget uses a Category while an amount is in effect this month or one starts later; a
    /// chain Stopped before this month and an archived Budget don't use it.
    #[test]
    fn using_counts_the_active_budgets_with_an_amount_this_month_or_later() {
        let (world, mut budgets, id) = september();
        let dining = world.category("Dining");
        let groceries = world.category("Groceries");
        let household = world.category("Household");
        let october = Period::of(date(2026, 10, 1));

        assert_eq!(budgets.using(&[dining, groceries], today()), 1);
        assert_eq!(budgets.using(&[household], today()), 0, "never budgeted");

        // Household starts next month.
        set(
            &mut budgets,
            &world,
            id,
            "Household",
            october,
            Span::Onward,
            "50.00",
            None,
        );
        assert_eq!(budgets.using(&[household], today()), 1, "starts later");

        // Stopped from this month, then nothing is left to use.
        for name in ["Dining", "Groceries"] {
            let stopped = budgets.stop(id, world.ledger(), world.category(name), sep(), today());
            assert_eq!(stopped, Ok(()));
        }
        assert_eq!(budgets.using(&[dining, groceries], today()), 0);

        // An archived Budget doesn't block, even with a later record.
        let copy = budgets.duplicate(id).expect("the id exists");
        assert_eq!(
            budgets.using(&[household], today()),
            2,
            "the copy uses it too"
        );
        assert_eq!(budgets.archive(copy, today()), Ok(()));
        assert_eq!(budgets.using(&[household], today()), 1);
    }
}
