//! The Budgets intents, one Interactor each: it loads the Budget from the [`BudgetRepository`],
//! applies the rule the entity or the set holds, and writes each Budget the rule changed whole
//! through `upsert_budget`. A refusal is a [`BudgetError`] and writes nothing: a chain edit runs on a
//! copy of the Budget, kept only when the rule lands, since some rules touch a chain before they
//! refuse.
//!
//! The rules that read the other domains take the read port they need ([`BudgetCategories`],
//! [`BudgetAccounts`] or [`BudgetReads`]); the repository is a separate argument, so a caller
//! whose store holds both can pass the reads it keeps beside it.

use chrono::NaiveDate;
use lib_core::{AccountId, BudgetId, CategoryId, Money, Period};

use crate::{
    Budget, BudgetAccounts, BudgetCategories, BudgetError, BudgetReads, BudgetRepository, Budgets,
    FillSource, NewBudget, Rollover, Span,
};

/// Logs how an intent ended and hands the outcome back.
fn logged<R>(intent: &str, outcome: Result<R, BudgetError>) -> Result<R, BudgetError> {
    match &outcome {
        Ok(_) => tracing::info!(intent, "Budgets intent done"),
        Err(refusal) => tracing::info!(intent, %refusal, "Budgets intent refused"),
    }
    outcome
}

/// Runs `rule` on a copy of the Budget set and writes each Budget it added or changed, a Budget
/// losing the default first (storage holds one).
fn write_set<R>(
    repo: &mut dyn BudgetRepository,
    rule: impl FnOnce(&mut Budgets) -> Result<R, BudgetError>,
) -> Result<R, BudgetError> {
    let before = Budgets::from_stored(repo.budgets());
    let mut after = before.clone();
    let result = rule(&mut after)?;
    for budget in after.changed_since(&before) {
        repo.upsert_budget(&budget);
    }
    Ok(result)
}

/// Runs `rule` on a copy of Budget `id` and writes it when it changed.
fn write_budget<R>(
    repo: &mut dyn BudgetRepository,
    id: BudgetId,
    rule: impl FnOnce(&mut Budget) -> Result<R, BudgetError>,
) -> Result<R, BudgetError> {
    let before = repo.budget(id).ok_or(BudgetError::NotFound)?;
    let mut after = before.clone();
    let result = rule(&mut after)?;
    if after != before {
        repo.upsert_budget(&after);
    }
    Ok(result)
}

/// Creates the Budget `new` describes (11c) and returns its id.
#[derive(Debug, Clone, PartialEq)]
pub struct CreateBudget {
    pub new: NewBudget,
    pub today: NaiveDate,
}

impl CreateBudget {
    pub fn run(
        self,
        reads: &dyn BudgetReads,
        repo: &mut dyn BudgetRepository,
    ) -> Result<BudgetId, BudgetError> {
        logged(
            "create budget",
            write_set(repo, |budgets| budgets.create(&self.new, reads, self.today)),
        )
    }
}

/// Saves Budget `id`'s name and Accounts (11c's edit mode).
#[derive(Debug, Clone, PartialEq)]
pub struct EditBudget {
    pub id: BudgetId,
    pub name: String,
    pub account_ids: Vec<AccountId>,
}

impl EditBudget {
    pub fn run(
        self,
        accounts: &dyn BudgetAccounts,
        repo: &mut dyn BudgetRepository,
    ) -> Result<(), BudgetError> {
        logged(
            "edit budget",
            write_set(repo, |budgets| {
                budgets.edit(self.id, &self.name, self.account_ids, accounts)
            }),
        )
    }
}

/// Copies Budget `id` and returns the copy's id.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DuplicateBudget(pub BudgetId);

impl DuplicateBudget {
    pub fn run(self, repo: &mut dyn BudgetRepository) -> Result<BudgetId, BudgetError> {
        logged(
            "duplicate budget",
            write_set(repo, |budgets| budgets.duplicate(self.0)),
        )
    }
}

/// Archives Budget `id`, which makes it read-only; never the default.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArchiveBudget {
    pub id: BudgetId,
    pub today: NaiveDate,
}

impl ArchiveBudget {
    pub fn run(self, repo: &mut dyn BudgetRepository) -> Result<(), BudgetError> {
        logged(
            "archive budget",
            write_set(repo, |budgets| budgets.archive(self.id, self.today)),
        )
    }
}

/// Brings archived Budget `id` back.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RestoreBudget(pub BudgetId);

impl RestoreBudget {
    pub fn run(self, repo: &mut dyn BudgetRepository) -> Result<(), BudgetError> {
        logged(
            "restore budget",
            write_set(repo, |budgets| budgets.restore(self.0)),
        )
    }
}

/// Makes active Budget `id` the default, which unmarks the one that was.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SetDefaultBudget(pub BudgetId);

impl SetDefaultBudget {
    pub fn run(self, repo: &mut dyn BudgetRepository) -> Result<(), BudgetError> {
        logged(
            "set default budget",
            write_set(repo, |budgets| budgets.set_default(self.0)),
        )
    }
}

/// Writes a Category's Budget Amount from `month` (9e and the Plan grid).
#[derive(Debug, Clone, PartialEq)]
pub struct SetAmount {
    pub id: BudgetId,
    pub category_id: CategoryId,
    pub month: Period,
    pub span: Span,
    pub amount: Money,
    /// `None` keeps the Rollover the month already inherits.
    pub rollover: Option<Rollover>,
    pub today: NaiveDate,
}

impl SetAmount {
    pub fn run(
        self,
        categories: &dyn BudgetCategories,
        repo: &mut dyn BudgetRepository,
    ) -> Result<(), BudgetError> {
        logged(
            "set amount",
            write_budget(repo, self.id, |budget| {
                budget.set_amount(
                    categories,
                    self.category_id,
                    self.month,
                    self.span,
                    self.amount,
                    self.rollover,
                    self.today,
                )
            }),
        )
    }
}

/// Stops budgeting a Category from `month` on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StopBudgeting {
    pub id: BudgetId,
    pub category_id: CategoryId,
    pub month: Period,
    pub today: NaiveDate,
}

impl StopBudgeting {
    pub fn run(
        self,
        categories: &dyn BudgetCategories,
        repo: &mut dyn BudgetRepository,
    ) -> Result<(), BudgetError> {
        logged(
            "stop budgeting",
            write_budget(repo, self.id, |budget| {
                budget.stop(categories, self.category_id, self.month, self.today)
            }),
        )
    }
}

/// Removes a Category's Month-only record in `month`; returns whether there was one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClearMonthOnly {
    pub id: BudgetId,
    pub category_id: CategoryId,
    pub month: Period,
    pub today: NaiveDate,
}

impl ClearMonthOnly {
    pub fn run(self, repo: &mut dyn BudgetRepository) -> Result<bool, BudgetError> {
        logged(
            "clear month only",
            write_budget(repo, self.id, |budget| {
                budget.clear_month_only(self.category_id, self.month, self.today)
            }),
        )
    }
}

/// Moves a Category's current-month Rollover on to the next choice and returns it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CycleRollover {
    pub id: BudgetId,
    pub category_id: CategoryId,
    pub today: NaiveDate,
}

impl CycleRollover {
    pub fn run(self, repo: &mut dyn BudgetRepository) -> Result<Rollover, BudgetError> {
        logged(
            "cycle rollover",
            write_budget(repo, self.id, |budget| {
                budget.cycle_rollover(self.category_id, self.today)
            }),
        )
    }
}

/// Saves the text typed into a Plan cell; returns whether anything changed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SaveCell {
    pub id: BudgetId,
    pub category_id: CategoryId,
    pub month: Period,
    pub span: Span,
    pub text: String,
    pub today: NaiveDate,
}

impl SaveCell {
    pub fn run(
        self,
        categories: &dyn BudgetCategories,
        repo: &mut dyn BudgetRepository,
    ) -> Result<bool, BudgetError> {
        logged(
            "save cell",
            write_budget(repo, self.id, |budget| {
                budget.save_cell(
                    categories,
                    self.category_id,
                    self.month,
                    self.span,
                    &self.text,
                    self.today,
                )
            }),
        )
    }
}

/// Fills `target`'s Month-only amounts from `source` (9f); returns how many Categories changed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Fill {
    pub id: BudgetId,
    pub target: Period,
    pub source: FillSource,
    pub today: NaiveDate,
}

impl Fill {
    pub fn run(
        self,
        reads: &dyn BudgetReads,
        repo: &mut dyn BudgetRepository,
    ) -> Result<usize, BudgetError> {
        logged(
            "fill",
            write_budget(repo, self.id, |budget| {
                budget.fill(reads, self.target, self.source, self.today)
            }),
        )
    }
}

/// Categories 5c's write: sets (or, with `None`, clears) a Category's Monthly budget from the
/// current month; returns whether it wrote.
#[derive(Debug, Clone, PartialEq)]
pub struct SetMonthlyLimit {
    pub id: BudgetId,
    pub category_id: CategoryId,
    pub amount: Option<Money>,
    pub today: NaiveDate,
}

impl SetMonthlyLimit {
    pub fn run(
        self,
        categories: &dyn BudgetCategories,
        repo: &mut dyn BudgetRepository,
    ) -> Result<bool, BudgetError> {
        logged(
            "set monthly limit",
            write_budget(repo, self.id, |budget| {
                budget.set_monthly_limit(categories, self.category_id, self.amount, self.today)
            }),
        )
    }
}

#[cfg(test)]
mod tests {
    use lib_core::UnitId;

    use super::*;
    use crate::fake::{InMemoryBudgetReads, InMemoryBudgets};
    use crate::{Limit, PERSONAL_SPENDING_ID, StartFrom, applied, cents_money};

    const JOINT: BudgetId = BudgetId::seed(2);
    const GROCERIES: CategoryId = CategoryId::seed(7);
    const RENT: CategoryId = CategoryId::seed(2);
    const FOOD: CategoryId = CategoryId::seed(6);
    const ANZ_EVERYDAY: AccountId = AccountId::seed(2);

    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 9, 21).unwrap_or_default()
    }

    fn current() -> Period {
        Period::of(today())
    }

    fn repo() -> InMemoryBudgets {
        InMemoryBudgets::stub(today())
    }

    fn reads() -> InMemoryBudgetReads {
        InMemoryBudgetReads::stub()
    }

    fn budget(repo: &InMemoryBudgets, id: BudgetId) -> Budget {
        repo.budget(id).unwrap()
    }

    fn amount_in(repo: &InMemoryBudgets, id: BudgetId, month: Period) -> Option<Money> {
        applied(budget(repo, id).chain(GROCERIES), month).map(|found| found.amount)
    }

    fn holiday() -> NewBudget {
        NewBudget {
            name: "Holiday".into(),
            unit: UnitId::SEED_AUD,
            account_ids: vec![AccountId::seed(1)],
            start_from: StartFrom::Empty,
        }
    }

    #[test]
    fn creating_a_budget_writes_it_and_a_refusal_writes_nothing() {
        let mut repo = repo();
        let id = CreateBudget {
            new: holiday(),
            today: today(),
        }
        .run(&reads(), &mut repo)
        .unwrap();
        assert_eq!(budget(&repo, id).name, "Holiday");
        assert!(!budget(&repo, id).is_default);

        let before = repo.clone();
        let taken = CreateBudget {
            new: holiday(),
            today: today(),
        }
        .run(&reads(), &mut repo);
        assert_eq!(taken, Err(BudgetError::NameTaken));
        let blank = CreateBudget {
            new: NewBudget {
                name: " ".into(),
                ..holiday()
            },
            today: today(),
        }
        .run(&reads(), &mut repo);
        assert_eq!(blank, Err(BudgetError::NameRequired));
        assert_eq!(repo, before);
    }

    #[test]
    fn editing_a_budget_renames_it_and_refuses_a_taken_name_or_an_archived_budget() {
        let mut repo = repo();
        EditBudget {
            id: JOINT,
            name: "Shared house".into(),
            account_ids: vec![ANZ_EVERYDAY],
        }
        .run(&reads(), &mut repo)
        .unwrap();
        assert_eq!(budget(&repo, JOINT).name, "Shared house");
        assert_eq!(budget(&repo, JOINT).account_ids, [ANZ_EVERYDAY]);

        let before = repo.clone();
        let taken = EditBudget {
            id: JOINT,
            name: "personal spending".into(),
            account_ids: vec![],
        }
        .run(&reads(), &mut repo);
        assert_eq!(taken, Err(BudgetError::NameTaken));
        ArchiveBudget {
            id: JOINT,
            today: today(),
        }
        .run(&mut repo)
        .unwrap();
        let archived = EditBudget {
            id: JOINT,
            name: "Again".into(),
            account_ids: vec![],
        }
        .run(&reads(), &mut repo);
        assert_eq!(archived, Err(BudgetError::Archived));
        assert_ne!(repo, before, "only the archive landed");
        assert_eq!(budget(&repo, JOINT).name, "Shared house");
    }

    #[test]
    fn duplicating_copies_the_chains_as_a_numbered_non_default_budget() {
        let mut repo = repo();
        let first = DuplicateBudget(PERSONAL_SPENDING_ID)
            .run(&mut repo)
            .unwrap();
        let second = DuplicateBudget(PERSONAL_SPENDING_ID)
            .run(&mut repo)
            .unwrap();

        assert_eq!(budget(&repo, first).name, "Personal spending copy");
        assert_eq!(budget(&repo, second).name, "Personal spending copy 2");
        assert!(!budget(&repo, first).is_default);
        assert_eq!(
            budget(&repo, first).limits,
            budget(&repo, PERSONAL_SPENDING_ID).limits
        );
        assert_eq!(
            DuplicateBudget(BudgetId::new()).run(&mut repo),
            Err(BudgetError::NotFound)
        );
    }

    #[test]
    fn archiving_and_restoring_never_touch_the_default() {
        let mut repo = repo();
        let before = repo.clone();
        assert_eq!(
            ArchiveBudget {
                id: PERSONAL_SPENDING_ID,
                today: today()
            }
            .run(&mut repo),
            Err(BudgetError::DefaultCannotBeArchived)
        );
        assert_eq!(repo, before);

        ArchiveBudget {
            id: JOINT,
            today: today(),
        }
        .run(&mut repo)
        .unwrap();
        assert_eq!(budget(&repo, JOINT).archived_at, Some(today()));
        RestoreBudget(JOINT).run(&mut repo).unwrap();
        assert_eq!(budget(&repo, JOINT).archived_at, None);
        assert_eq!(
            RestoreBudget(BudgetId::new()).run(&mut repo),
            Err(BudgetError::NotFound)
        );
    }

    #[test]
    fn setting_the_default_moves_it_and_refuses_an_archived_budget() {
        let mut repo = repo();
        SetDefaultBudget(JOINT).run(&mut repo).unwrap();
        let defaults: Vec<BudgetId> = repo
            .budgets()
            .iter()
            .filter(|budget| budget.is_default)
            .map(|budget| budget.id)
            .collect();
        assert_eq!(defaults, [JOINT]);

        ArchiveBudget {
            id: PERSONAL_SPENDING_ID,
            today: today(),
        }
        .run(&mut repo)
        .unwrap();
        let before = repo.clone();
        assert_eq!(
            SetDefaultBudget(PERSONAL_SPENDING_ID).run(&mut repo),
            Err(BudgetError::Archived)
        );
        assert_eq!(repo, before);
    }

    fn set(span: Span, month: Period, amount: i64) -> SetAmount {
        SetAmount {
            id: PERSONAL_SPENDING_ID,
            category_id: GROCERIES,
            month,
            span,
            amount: cents_money(amount),
            rollover: None,
            today: today(),
        }
    }

    #[test]
    fn setting_an_amount_writes_the_record_and_refuses_what_the_rules_do() {
        let mut repo = repo();
        set(Span::MonthOnly, current().next(), 12_345)
            .run(&reads(), &mut repo)
            .unwrap();
        assert_eq!(
            amount_in(&repo, PERSONAL_SPENDING_ID, current().next()),
            Some(cents_money(12_345))
        );

        let before = repo.clone();
        let closed = set(Span::Onward, current().prev(), 100).run(&reads(), &mut repo);
        assert_eq!(closed, Err(BudgetError::ClosedMonth));
        let negative = set(Span::Onward, current(), -1).run(&reads(), &mut repo);
        assert_eq!(negative, Err(BudgetError::NegativeAmount));
        let parent = SetAmount {
            category_id: FOOD,
            ..set(Span::Onward, current(), 100)
        }
        .run(&reads(), &mut repo);
        assert_eq!(parent, Err(BudgetError::NotAnExpenseLeaf));
        let unknown = SetAmount {
            id: BudgetId::new(),
            ..set(Span::Onward, current(), 100)
        }
        .run(&reads(), &mut repo);
        assert_eq!(unknown, Err(BudgetError::NotFound));
        assert_eq!(repo, before);
    }

    #[test]
    fn an_archived_budget_refuses_every_chain_edit() {
        let mut repo = repo();
        ArchiveBudget {
            id: JOINT,
            today: today(),
        }
        .run(&mut repo)
        .unwrap();
        let before = repo.clone();

        let set = SetAmount {
            id: JOINT,
            ..set(Span::Onward, current(), 100)
        }
        .run(&reads(), &mut repo);
        let stop = StopBudgeting {
            id: JOINT,
            category_id: GROCERIES,
            month: current(),
            today: today(),
        }
        .run(&reads(), &mut repo);
        let cycle = CycleRollover {
            id: JOINT,
            category_id: GROCERIES,
            today: today(),
        }
        .run(&mut repo);

        assert_eq!(set, Err(BudgetError::Archived));
        assert_eq!(stop, Err(BudgetError::Archived));
        assert_eq!(cycle, Err(BudgetError::Archived));
        assert_eq!(repo, before);
    }

    #[test]
    fn stopping_writes_a_stop_and_an_unbudgeted_category_writes_nothing() {
        let mut repo = repo();
        StopBudgeting {
            id: PERSONAL_SPENDING_ID,
            category_id: GROCERIES,
            month: current().next(),
            today: today(),
        }
        .run(&reads(), &mut repo)
        .unwrap();
        assert_eq!(
            budget(&repo, PERSONAL_SPENDING_ID)
                .chain(GROCERIES)
                .last()
                .map(|record| &record.limit),
            Some(&Limit::Stop)
        );

        let before = repo.clone();
        StopBudgeting {
            id: JOINT,
            category_id: RENT,
            month: current(),
            today: today(),
        }
        .run(&reads(), &mut repo)
        .unwrap();
        assert_eq!(repo, before);
        assert_eq!(
            StopBudgeting {
                id: PERSONAL_SPENDING_ID,
                category_id: FOOD,
                month: current(),
                today: today(),
            }
            .run(&reads(), &mut repo),
            Err(BudgetError::NotAnExpenseLeaf)
        );
    }

    #[test]
    fn clearing_a_month_only_removes_it_and_says_whether_there_was_one() {
        let mut repo = repo();
        let clear = ClearMonthOnly {
            id: PERSONAL_SPENDING_ID,
            category_id: GROCERIES,
            month: current().next(),
            today: today(),
        };
        assert_eq!(clear.run(&mut repo), Ok(false));

        set(Span::MonthOnly, current().next(), 5_000)
            .run(&reads(), &mut repo)
            .unwrap();
        assert_eq!(clear.run(&mut repo), Ok(true));
        assert_eq!(
            amount_in(&repo, PERSONAL_SPENDING_ID, current().next()),
            amount_in(&repo, PERSONAL_SPENDING_ID, current()),
            "the month falls back to the carried Onward"
        );
        let closed = ClearMonthOnly {
            month: current().prev(),
            ..clear
        }
        .run(&mut repo);
        assert_eq!(closed, Err(BudgetError::ClosedMonth));
    }

    #[test]
    fn cycling_the_rollover_walks_the_choices_and_refuses_an_unbudgeted_category() {
        let mut repo = repo();
        let cycle = CycleRollover {
            id: PERSONAL_SPENDING_ID,
            category_id: GROCERIES,
            today: today(),
        };
        let first = cycle.run(&mut repo).unwrap();
        let second = cycle.run(&mut repo).unwrap();
        assert_eq!(second, first.next());
        assert_eq!(
            applied(
                budget(&repo, PERSONAL_SPENDING_ID).chain(GROCERIES),
                current()
            )
            .map(|found| found.rollover),
            Some(second)
        );

        let before = repo.clone();
        let unbudgeted = CycleRollover {
            id: JOINT,
            category_id: RENT,
            ..cycle
        }
        .run(&mut repo);
        assert_eq!(unbudgeted, Err(BudgetError::NotBudgeted));
        assert_eq!(repo, before, "a refusal adds no empty chain");
    }

    #[test]
    fn saving_a_cell_writes_an_amount_clears_on_empty_text_and_refuses_bad_text() {
        let mut repo = repo();
        let save = |text: &str, span| SaveCell {
            id: PERSONAL_SPENDING_ID,
            category_id: GROCERIES,
            month: current().next(),
            span,
            text: text.into(),
            today: today(),
        };

        assert_eq!(
            save("612.505", Span::MonthOnly).run(&reads(), &mut repo),
            Ok(true)
        );
        assert_eq!(
            amount_in(&repo, PERSONAL_SPENDING_ID, current().next()),
            Some(cents_money(61_251)),
            "rounded half up to cents"
        );
        assert_eq!(save("", Span::MonthOnly).run(&reads(), &mut repo), Ok(true));
        assert_eq!(
            save("", Span::MonthOnly).run(&reads(), &mut repo),
            Ok(false)
        );
        let before = repo.clone();
        assert_eq!(
            save("lots", Span::Onward).run(&reads(), &mut repo),
            Err(BudgetError::InvalidAmount)
        );
        assert_eq!(repo, before);
    }

    #[test]
    fn filling_writes_the_months_amounts_and_says_how_many() {
        let mut repo = repo();
        let target = current().next();
        let fill = Fill {
            id: PERSONAL_SPENDING_ID,
            target,
            source: FillSource::PreviousMonth,
            today: today(),
        };

        let wrote = fill.run(&reads(), &mut repo).unwrap();

        let personal = budget(&repo, PERSONAL_SPENDING_ID);
        let month_only = personal
            .limits
            .values()
            .filter(|chain| {
                chain.iter().any(|record| {
                    record.month == target && matches!(record.limit, Limit::MonthOnly { .. })
                })
            })
            .count();
        assert_eq!(wrote, month_only);
        let closed = Fill {
            target: current().prev(),
            ..fill
        }
        .run(&reads(), &mut repo);
        assert_eq!(closed, Err(BudgetError::ClosedMonth));
    }

    #[test]
    fn setting_a_monthly_limit_writes_an_onward_and_an_unchanged_value_writes_nothing() {
        let mut repo = repo();
        let limit = |amount| SetMonthlyLimit {
            id: PERSONAL_SPENDING_ID,
            category_id: GROCERIES,
            amount,
            today: today(),
        };

        assert_eq!(
            limit(Some(cents_money(61_000))).run(&reads(), &mut repo),
            Ok(true)
        );
        assert_eq!(
            amount_in(&repo, PERSONAL_SPENDING_ID, current()),
            Some(cents_money(61_000))
        );
        let before = repo.clone();
        assert_eq!(
            limit(Some(cents_money(61_000))).run(&reads(), &mut repo),
            Ok(false)
        );
        assert_eq!(repo, before);
        assert_eq!(limit(None).run(&reads(), &mut repo), Ok(true));
        assert_eq!(amount_in(&repo, PERSONAL_SPENDING_ID, current()), None);
        let parent = SetMonthlyLimit {
            category_id: FOOD,
            ..limit(Some(cents_money(1)))
        }
        .run(&reads(), &mut repo);
        assert_eq!(parent, Err(BudgetError::NotAnExpenseLeaf));
    }
}
