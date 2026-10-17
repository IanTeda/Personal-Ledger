//! The Budgets writes (ADR-0038): each a command value with its own refusal, checked before
//! anything is written so a refusal leaves the Ledger untouched, and recording an upsert per
//! Budget it changed (ADR-0039).
//!
//! Single-Budget rules stay in `lib_budgets`. What lives here reads other domains: creating a
//! Budget or filling a month reads the Accounts, Categories, Transactions and Bills, and every
//! Category Limit write is checked against the Categories.

use chrono::NaiveDate;
use lib_budgets::{BudgetError, BudgetSources, Budgets, FillSource, NewBudget, Rollover, Span};
use lib_core::{AccountId, BudgetId, CategoryId, Money, Period};

use crate::{Ledger, LedgerChange, LedgerCommand};

impl Ledger {
    /// Creates the Budget `new` describes and returns its id.
    pub fn add_budget(
        &mut self,
        new: &NewBudget,
        today: NaiveDate,
    ) -> Result<BudgetId, BudgetError> {
        self.write_budgets(|budgets, sources| budgets.create(new, sources, today))
    }

    /// Saves Budget `id`'s name and Accounts.
    pub fn edit_budget(
        &mut self,
        id: BudgetId,
        name: &str,
        account_ids: Vec<AccountId>,
    ) -> Result<(), BudgetError> {
        self.write_budgets(|budgets, sources| budgets.edit(id, name, account_ids, sources.accounts))
    }

    /// Copies Budget `id` and returns the copy's id.
    pub fn duplicate_budget(&mut self, id: BudgetId) -> Result<BudgetId, BudgetError> {
        self.write_budgets(|budgets, _| budgets.duplicate(id))
    }

    /// Archives Budget `id`, which makes it read-only.
    pub fn archive_budget(&mut self, id: BudgetId, today: NaiveDate) -> Result<(), BudgetError> {
        self.write_budgets(|budgets, _| budgets.archive(id, today))
    }

    /// Brings archived Budget `id` back.
    pub fn restore_budget(&mut self, id: BudgetId) -> Result<(), BudgetError> {
        self.write_budgets(|budgets, _| budgets.restore(id))
    }

    /// Makes Budget `id` the default, which unmarks the one that was.
    pub fn set_default_budget(&mut self, id: BudgetId) -> Result<(), BudgetError> {
        self.write_budgets(|budgets, _| budgets.set_default(id))
    }

    /// Writes a Category's Budget Amount from `month`, for 9e and the Plan grid.
    pub fn set_budget_amount(&mut self, set: &SetBudgetAmount) -> Result<(), BudgetError> {
        self.write_budgets(|budgets, sources| {
            budgets.set_amount(
                set.budget_id,
                sources.categories,
                set.category_id,
                set.month,
                set.span,
                set.amount.clone(),
                set.rollover,
                set.today,
            )
        })
    }

    /// Stops budgeting a Category from `month` on.
    pub fn stop_budgeting(
        &mut self,
        budget_id: BudgetId,
        category_id: CategoryId,
        month: Period,
        today: NaiveDate,
    ) -> Result<(), BudgetError> {
        self.write_budgets(|budgets, sources| {
            budgets.stop(budget_id, sources.categories, category_id, month, today)
        })
    }

    /// Saves the text typed into a Plan cell. Returns whether anything changed.
    pub fn save_budget_cell(&mut self, cell: &SaveBudgetCell) -> Result<bool, BudgetError> {
        self.write_budgets(|budgets, sources| {
            budgets.save_cell(
                cell.budget_id,
                sources.categories,
                cell.category_id,
                cell.month,
                cell.span,
                &cell.text,
                cell.today,
            )
        })
    }

    /// Moves a Category's Rollover on to the next choice and returns it.
    pub fn cycle_budget_rollover(
        &mut self,
        budget_id: BudgetId,
        category_id: CategoryId,
        today: NaiveDate,
    ) -> Result<Rollover, BudgetError> {
        self.write_budgets(|budgets, _| budgets.cycle_rollover(budget_id, category_id, today))
    }

    /// Fills `target`'s Month-only amounts from `source` and returns how many Categories changed.
    pub fn fill_budget(
        &mut self,
        budget_id: BudgetId,
        target: Period,
        source: FillSource,
        today: NaiveDate,
    ) -> Result<usize, BudgetError> {
        self.write_budgets(|budgets, sources| {
            budgets.fill(budget_id, sources, target, source, today)
        })
    }

    /// Runs `write` on a copy of the Budgets and takes the copy only when it lands, so a refusal
    /// leaves nothing half-written (some rules touch a chain before they refuse). Records an
    /// upsert for each Budget the write added or changed.
    fn write_budgets<R>(
        &mut self,
        write: impl FnOnce(&mut Budgets, &BudgetSources<'_>) -> Result<R, BudgetError>,
    ) -> Result<R, BudgetError> {
        let mut budgets = self.budgets.clone();
        let sources = BudgetSources {
            categories: &self.categories,
            accounts: &self.accounts,
            transactions: &self.transactions,
            plans: &self.bill_plans,
            entries: &self.bill_entries,
        };
        let result = write(&mut budgets, &sources)?;
        let changed: Vec<_> = budgets
            .all()
            .iter()
            .filter(|budget| self.budgets.get(budget.id) != Some(budget))
            .cloned()
            .collect();
        self.budgets = budgets;
        self.pending
            .extend(changed.into_iter().map(LedgerChange::UpsertBudget));
        Ok(result)
    }
}

/// What 9e and the Plan grid write for one Category and month.
#[derive(Debug, Clone, PartialEq)]
pub struct SetBudgetAmount {
    pub budget_id: BudgetId,
    pub category_id: CategoryId,
    pub month: Period,
    pub span: Span,
    pub amount: Money,
    /// `None` keeps the Rollover the month already inherits.
    pub rollover: Option<Rollover>,
    pub today: NaiveDate,
}

/// The text typed into one Plan cell.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SaveBudgetCell {
    pub budget_id: BudgetId,
    pub category_id: CategoryId,
    pub month: Period,
    pub span: Span,
    pub text: String,
    pub today: NaiveDate,
}

/// [`Ledger::add_budget`] as a value.
#[derive(Debug, Clone, PartialEq)]
pub struct AddBudget {
    pub new: NewBudget,
    pub today: NaiveDate,
}

impl LedgerCommand for AddBudget {
    type Outcome = BudgetId;
    type Refusal = BudgetError;
    fn run(self, ledger: &mut Ledger) -> Result<BudgetId, BudgetError> {
        ledger.add_budget(&self.new, self.today)
    }
}

/// [`Ledger::edit_budget`] as a value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditBudget {
    pub id: BudgetId,
    pub name: String,
    pub account_ids: Vec<AccountId>,
}

impl LedgerCommand for EditBudget {
    type Outcome = ();
    type Refusal = BudgetError;
    fn run(self, ledger: &mut Ledger) -> Result<(), BudgetError> {
        ledger.edit_budget(self.id, &self.name, self.account_ids)
    }
}

/// [`Ledger::duplicate_budget`] as a value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DuplicateBudget(pub BudgetId);

impl LedgerCommand for DuplicateBudget {
    type Outcome = BudgetId;
    type Refusal = BudgetError;
    fn run(self, ledger: &mut Ledger) -> Result<BudgetId, BudgetError> {
        ledger.duplicate_budget(self.0)
    }
}

/// [`Ledger::archive_budget`] as a value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArchiveBudget {
    pub id: BudgetId,
    pub today: NaiveDate,
}

impl LedgerCommand for ArchiveBudget {
    type Outcome = ();
    type Refusal = BudgetError;
    fn run(self, ledger: &mut Ledger) -> Result<(), BudgetError> {
        ledger.archive_budget(self.id, self.today)
    }
}

/// [`Ledger::restore_budget`] as a value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RestoreBudget(pub BudgetId);

impl LedgerCommand for RestoreBudget {
    type Outcome = ();
    type Refusal = BudgetError;
    fn run(self, ledger: &mut Ledger) -> Result<(), BudgetError> {
        ledger.restore_budget(self.0)
    }
}

/// [`Ledger::set_default_budget`] as a value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SetDefaultBudget(pub BudgetId);

impl LedgerCommand for SetDefaultBudget {
    type Outcome = ();
    type Refusal = BudgetError;
    fn run(self, ledger: &mut Ledger) -> Result<(), BudgetError> {
        ledger.set_default_budget(self.0)
    }
}

impl LedgerCommand for SetBudgetAmount {
    type Outcome = ();
    type Refusal = BudgetError;
    fn run(self, ledger: &mut Ledger) -> Result<(), BudgetError> {
        ledger.set_budget_amount(&self)
    }
}

/// [`Ledger::stop_budgeting`] as a value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StopBudgeting {
    pub budget_id: BudgetId,
    pub category_id: CategoryId,
    pub month: Period,
    pub today: NaiveDate,
}

impl LedgerCommand for StopBudgeting {
    type Outcome = ();
    type Refusal = BudgetError;
    fn run(self, ledger: &mut Ledger) -> Result<(), BudgetError> {
        ledger.stop_budgeting(self.budget_id, self.category_id, self.month, self.today)
    }
}

impl LedgerCommand for SaveBudgetCell {
    type Outcome = bool;
    type Refusal = BudgetError;
    fn run(self, ledger: &mut Ledger) -> Result<bool, BudgetError> {
        ledger.save_budget_cell(&self)
    }
}

/// [`Ledger::cycle_budget_rollover`] as a value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CycleBudgetRollover {
    pub budget_id: BudgetId,
    pub category_id: CategoryId,
    pub today: NaiveDate,
}

impl LedgerCommand for CycleBudgetRollover {
    type Outcome = Rollover;
    type Refusal = BudgetError;
    fn run(self, ledger: &mut Ledger) -> Result<Rollover, BudgetError> {
        ledger.cycle_budget_rollover(self.budget_id, self.category_id, self.today)
    }
}

/// [`Ledger::fill_budget`] as a value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FillBudget {
    pub budget_id: BudgetId,
    pub target: Period,
    pub source: FillSource,
    pub today: NaiveDate,
}

impl LedgerCommand for FillBudget {
    type Outcome = usize;
    type Refusal = BudgetError;
    fn run(self, ledger: &mut Ledger) -> Result<usize, BudgetError> {
        ledger.fill_budget(self.budget_id, self.target, self.source, self.today)
    }
}

#[cfg(test)]
mod tests {
    use lib_budgets::{Limit, PERSONAL_SPENDING_ID};

    use super::*;
    use crate::LedgerParts;

    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 9, 21).expect("a real date")
    }

    fn ledger() -> Ledger {
        Ledger::stub_on(today())
    }

    fn budget_ids(changes: &[LedgerChange]) -> Vec<BudgetId> {
        changes
            .iter()
            .map(|change| match change {
                LedgerChange::UpsertBudget(budget) => budget.id,
                other => panic!("a Budgets command wrote {other:?}"),
            })
            .collect()
    }

    fn new_budget(name: &str, ledger: &Ledger) -> NewBudget {
        let budget = ledger
            .budgets()
            .get(PERSONAL_SPENDING_ID)
            .expect("the seeded Budget");
        NewBudget {
            name: name.to_string(),
            unit: budget.unit,
            account_ids: budget.account_ids.clone(),
            start_from: lib_budgets::StartFrom::Empty,
        }
    }

    #[test]
    fn adding_a_budget_records_it() {
        let mut ledger = ledger();
        let id = ledger
            .add_budget(&new_budget("Holiday", &ledger.clone()), today())
            .expect("a new name on valid Accounts");
        assert!(ledger.budgets().get(id).is_some());
        assert_eq!(budget_ids(&ledger.drain_changes()), vec![id]);
    }

    #[test]
    fn a_refused_add_leaves_the_ledger_untouched() {
        let mut ledger = ledger();
        let before = ledger.budgets().clone();
        let mut new = new_budget("", &ledger);
        assert_eq!(
            ledger.add_budget(&new, today()),
            Err(BudgetError::NameRequired)
        );
        new.name = "personal spending".into();
        assert_eq!(
            ledger.add_budget(&new, today()),
            Err(BudgetError::NameTaken)
        );
        assert_eq!(ledger.budgets(), &before);
        assert!(ledger.drain_changes().is_empty());
    }

    #[test]
    fn editing_a_budget_records_it_and_a_refusal_does_not() {
        let mut ledger = ledger();
        let accounts = ledger
            .budgets()
            .get(PERSONAL_SPENDING_ID)
            .expect("seeded")
            .account_ids
            .clone();
        ledger
            .edit_budget(PERSONAL_SPENDING_ID, "Renamed", accounts.clone())
            .expect("a free name");
        assert_eq!(
            budget_ids(&ledger.drain_changes()),
            vec![PERSONAL_SPENDING_ID]
        );
        assert_eq!(
            ledger.edit_budget(PERSONAL_SPENDING_ID, "  ", accounts),
            Err(BudgetError::NameRequired)
        );
        assert!(ledger.drain_changes().is_empty());
    }

    #[test]
    fn duplicating_records_the_copy_only() {
        let mut ledger = ledger();
        let copy = ledger
            .duplicate_budget(PERSONAL_SPENDING_ID)
            .expect("the seeded Budget exists");
        assert_eq!(budget_ids(&ledger.drain_changes()), vec![copy]);
        assert_eq!(
            ledger.duplicate_budget(BudgetId::seed(u32::MAX)),
            Err(BudgetError::NotFound)
        );
        assert!(ledger.drain_changes().is_empty());
    }

    #[test]
    fn archiving_and_restoring_record_the_budget_and_the_default_cannot_be_archived() {
        let mut ledger = ledger();
        let copy = ledger.duplicate_budget(PERSONAL_SPENDING_ID).expect("copy");
        ledger.drain_changes();

        ledger
            .archive_budget(copy, today())
            .expect("not the default");
        assert_eq!(budget_ids(&ledger.drain_changes()), vec![copy]);
        ledger.restore_budget(copy).expect("exists");
        assert_eq!(budget_ids(&ledger.drain_changes()), vec![copy]);

        assert_eq!(
            ledger.archive_budget(PERSONAL_SPENDING_ID, today()),
            Err(BudgetError::DefaultCannotBeArchived)
        );
        assert!(ledger.drain_changes().is_empty());
    }

    #[test]
    fn making_a_budget_the_default_records_both_it_and_the_one_it_replaces() {
        let mut ledger = ledger();
        let copy = ledger.duplicate_budget(PERSONAL_SPENDING_ID).expect("copy");
        ledger.drain_changes();

        ledger.set_default_budget(copy).expect("active");
        let mut ids = budget_ids(&ledger.drain_changes());
        ids.sort_unstable();
        assert_eq!(ids, vec![PERSONAL_SPENDING_ID, copy]);
        assert_eq!(ledger.budgets().default_budget().map(|b| b.id), Some(copy));
    }

    #[test]
    fn setting_an_amount_records_the_budget_and_a_negative_one_is_refused() {
        let mut ledger = ledger();
        let category_id = budgetable_leaf(&ledger);
        let mut set = SetBudgetAmount {
            budget_id: PERSONAL_SPENDING_ID,
            category_id,
            month: Period::of(today()),
            span: Span::Onward,
            amount: "125.00".parse().expect("an amount"),
            rollover: None,
            today: today(),
        };
        ledger.set_budget_amount(&set).expect("an expense leaf");
        assert_eq!(
            budget_ids(&ledger.drain_changes()),
            vec![PERSONAL_SPENDING_ID]
        );

        let before = ledger.budgets().clone();
        set.amount = "-1".parse().expect("an amount");
        assert_eq!(
            ledger.set_budget_amount(&set),
            Err(BudgetError::NegativeAmount)
        );
        assert_eq!(ledger.budgets(), &before);
        assert!(ledger.drain_changes().is_empty());
    }

    fn budgetable_leaf(ledger: &Ledger) -> CategoryId {
        let budget = ledger.budgets().get(PERSONAL_SPENDING_ID).expect("seeded");
        *budget
            .limits
            .keys()
            .next()
            .expect("the seeded Budget budgets a Category")
    }

    #[test]
    fn stopping_an_unbudgeted_category_changes_nothing_and_a_budgeted_one_stops() {
        let mut ledger = ledger();
        let category_id = budgetable_leaf(&ledger);
        let month = Period::of(today());
        ledger
            .stop_budgeting(PERSONAL_SPENDING_ID, category_id, month, today())
            .expect("a budgeted leaf in an open month");
        assert_eq!(
            budget_ids(&ledger.drain_changes()),
            vec![PERSONAL_SPENDING_ID]
        );
        assert!(
            ledger
                .budgets()
                .get(PERSONAL_SPENDING_ID)
                .expect("seeded")
                .chain(category_id)
                .iter()
                .any(|record| record.month == month && matches!(record.limit, Limit::Stop))
        );

        assert_eq!(
            ledger.stop_budgeting(
                PERSONAL_SPENDING_ID,
                CategoryId::seed(u32::MAX),
                month,
                today()
            ),
            Err(BudgetError::NotAnExpenseLeaf)
        );
        assert!(ledger.drain_changes().is_empty());
    }

    #[test]
    fn saving_a_cell_records_a_change_and_an_unchanged_or_invalid_cell_records_nothing() {
        let mut ledger = ledger();
        let category_id = budgetable_leaf(&ledger);
        let cell = |text: &str| SaveBudgetCell {
            budget_id: PERSONAL_SPENDING_ID,
            category_id,
            month: Period::of(today()),
            span: Span::Onward,
            text: text.to_string(),
            today: today(),
        };

        assert_eq!(ledger.save_budget_cell(&cell("987.65")), Ok(true));
        assert_eq!(
            budget_ids(&ledger.drain_changes()),
            vec![PERSONAL_SPENDING_ID]
        );

        assert_eq!(ledger.save_budget_cell(&cell("987.65")), Ok(false));
        assert!(ledger.drain_changes().is_empty());

        assert_eq!(
            ledger.save_budget_cell(&cell("abc")),
            Err(BudgetError::InvalidAmount)
        );
        assert!(ledger.drain_changes().is_empty());
    }

    #[test]
    fn cycling_the_rollover_of_an_unbudgeted_category_is_refused_without_a_trace() {
        let mut ledger = ledger();
        let budgeted = budgetable_leaf(&ledger);
        let unbudgeted = ledger
            .categories()
            .iter()
            .map(|c| c.id)
            .find(|id| {
                ledger
                    .budgets()
                    .get(PERSONAL_SPENDING_ID)
                    .is_some_and(|b| b.chain(*id).is_empty())
            })
            .expect("a Category the seeded Budget doesn't budget");
        let before = ledger.budgets().clone();
        assert_eq!(
            ledger.cycle_budget_rollover(PERSONAL_SPENDING_ID, unbudgeted, today()),
            Err(BudgetError::NotBudgeted)
        );
        assert_eq!(ledger.budgets(), &before, "a refusal adds no empty chain");
        assert!(ledger.drain_changes().is_empty());

        ledger
            .cycle_budget_rollover(PERSONAL_SPENDING_ID, budgeted, today())
            .expect("budgeted this month");
        assert_eq!(
            budget_ids(&ledger.drain_changes()),
            vec![PERSONAL_SPENDING_ID]
        );
    }

    #[test]
    fn filling_a_closed_month_is_refused_and_an_open_one_records_the_budget() {
        let mut ledger = ledger();
        let open = Period::of(today());
        assert_eq!(
            ledger.fill_budget(
                PERSONAL_SPENDING_ID,
                open.prev(),
                FillSource::PreviousMonth,
                today()
            ),
            Err(BudgetError::ClosedMonth)
        );
        assert!(ledger.drain_changes().is_empty());

        let filled = ledger
            .fill_budget(
                PERSONAL_SPENDING_ID,
                open.next(),
                FillSource::PreviousMonth,
                today(),
            )
            .expect("an open month");
        assert_eq!(
            budget_ids(&ledger.drain_changes()).is_empty(),
            filled == 0,
            "the Budget is recorded exactly when a Category changed"
        );
    }

    #[test]
    fn commands_run_through_the_trait_and_a_refused_default_changes_nothing() {
        let mut ledger = ledger();
        let copy = DuplicateBudget(PERSONAL_SPENDING_ID)
            .run(&mut ledger)
            .expect("the seeded Budget exists");
        ArchiveBudget {
            id: copy,
            today: today(),
        }
        .run(&mut ledger)
        .expect("not the default");
        ledger.drain_changes();

        let before = ledger.budgets().clone();
        assert_eq!(
            SetDefaultBudget(copy).run(&mut ledger),
            Err(BudgetError::Archived)
        );
        assert_eq!(ledger.budgets(), &before);
        assert!(ledger.drain_changes().is_empty());
    }

    #[test]
    fn a_refusal_on_a_ledger_with_no_budgets_leaves_it_empty() {
        let mut ledger = Ledger::from_parts(LedgerParts::default());
        assert_eq!(
            ledger.restore_budget(BudgetId::seed(1)),
            Err(BudgetError::NotFound)
        );
        assert!(ledger.budgets().all().is_empty());
        assert!(ledger.drain_changes().is_empty());
    }
}
