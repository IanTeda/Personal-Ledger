//! The Plan tab: the figures behind Fill and Unallocated, and the Plan grid (9b).

use bigdecimal::{BigDecimal, RoundingMode, Signed};
use chrono::NaiveDate;
use lib_core::{CategoryId, CategoryTypes, Money, Period};

use super::figures::{SpentIndex, expense_leaves};
use super::protocols::{BudgetCategories, BudgetCategory, BudgetReads};
use super::{Budget, Rollover, applied, tree, zero};

/// How far back "the last 3 months" reaches, for the average and Unallocated figures.
pub const AVERAGE_MONTHS: i32 = 3;

/// A month's Total budgeted: Budget Amounts only, no carry.
pub fn total_budgeted(budget: &Budget, categories: &dyn BudgetCategories, month: Period) -> Money {
    total_over(budget, &categories.budget_categories(), month)
}

fn total_over(budget: &Budget, categories: &[BudgetCategory], month: Period) -> Money {
    Money(
        expense_leaves(categories)
            .into_iter()
            .filter_map(|id| applied(budget.chain(id), month))
            .fold(zero(), |sum, found| sum + found.amount.0),
    )
}

/// What Budget `budget` spent in the three months before `target`, indexed.
fn spent_before(budget: &Budget, reads: &dyn BudgetReads, target: Period) -> SpentIndex {
    let accounts = reads.budget_accounts();
    SpentIndex::load(
        budget,
        reads,
        &accounts,
        target.shift(-AVERAGE_MONTHS),
        target.prev(),
    )
}

/// The three months immediately before `target`.
fn months_before(target: Period) -> impl Iterator<Item = Period> {
    (1..=AVERAGE_MONTHS).map(move |back| (0..back).fold(target, |month, _| month.prev()))
}

fn average_of(index: &SpentIndex, category_id: CategoryId, target: Period) -> Money {
    let total =
        months_before(target).fold(zero(), |sum, month| sum + index.spent(category_id, month));
    let mean = total / BigDecimal::from(AVERAGE_MONTHS);
    // Nearest 10, ties away from zero, and never below zero.
    let rounded = (mean / BigDecimal::from(10)).with_scale_round(0, RoundingMode::HalfUp)
        * BigDecimal::from(10);
    Money(
        if rounded.is_negative() {
            zero()
        } else {
            rounded
        }
        .with_scale(2),
    )
}

/// A leaf's 3-month average Spent on the Budget's Accounts, from the three months before
/// `target`: Fill's source and 11c's "Last 3 months' spending".
pub fn three_month_average(
    budget: &Budget,
    reads: &dyn BudgetReads,
    category_id: CategoryId,
    target: Period,
) -> Money {
    average_of(&spent_before(budget, reads, target), category_id, target)
}

/// Every leaf Expense Category's 3-month average for `target`, tree order.
pub fn three_month_averages(
    budget: &Budget,
    reads: &dyn BudgetReads,
    target: Period,
) -> Vec<(CategoryId, Money)> {
    let index = spent_before(budget, reads, target);
    expense_leaves(&reads.budget_categories())
        .into_iter()
        .map(|id| (id, average_of(&index, id, target)))
        .collect()
}

/// Where Fill (9f) takes a month's amounts from. "The plan" is not a source: it is what the month
/// already resolves to, and so the baseline the preview diffs against.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FillSource {
    /// The month before, as budgeted: its Budget Amounts, never its carry.
    #[default]
    PreviousMonth,
    /// The 3-month average Spent before the target.
    Average,
}

impl FillSource {
    pub const ALL: [FillSource; 2] = [FillSource::PreviousMonth, FillSource::Average];
}

/// One Category Fill would change: what the plan gives the month now, and what Fill writes.
#[derive(Debug, Clone, PartialEq)]
pub struct FillLine {
    pub category_id: CategoryId,
    /// `None` when the plan leaves the month Unbudgeted.
    pub before: Option<Money>,
    pub after: Money,
}

/// What one source would do to the target month.
#[derive(Debug, Clone, PartialEq)]
pub struct FillPreview {
    pub target: Period,
    pub source: FillSource,
    /// The Categories whose amount changes, tree order.
    pub changes: Vec<FillLine>,
    /// Categories with their own record in the target month: never overwritten.
    pub kept: Vec<CategoryId>,
    /// Eligible Categories the source leaves at the plan's amount.
    pub unchanged: usize,
    /// The target month's Total budgeted once filled.
    pub total: Money,
}

/// Fill's target: the first open month at or after `cursor`, which is next month when the cursor
/// sits on a closed month (or on no month at all).
pub fn fill_target(cursor: Option<Period>, today: NaiveDate) -> Period {
    let current = Period::of(today);
    match cursor {
        Some(month) if month >= current => month,
        _ => current.next(),
    }
}

/// What filling `target` from `source` would write: a Month-only amount for each leaf budgeted in
/// the month before, skipping any with its own record in `target` and any the source leaves
/// where the plan already has it.
pub fn fill_preview(
    budget: &Budget,
    reads: &dyn BudgetReads,
    target: Period,
    source: FillSource,
) -> FillPreview {
    let index = spent_before(budget, reads, target);
    let previous = target.prev();
    let mut preview = FillPreview {
        target,
        source,
        changes: Vec::new(),
        kept: Vec::new(),
        unchanged: 0,
        total: Money(zero()),
    };
    for id in expense_leaves(&reads.budget_categories()) {
        let chain = budget.chain(id);
        let planned = applied(chain, target).map(|found| found.amount);
        let filled = applied(chain, previous).and_then(|last| {
            if chain.iter().any(|record| record.month == target) {
                preview.kept.push(id);
                return None;
            }
            let after = match source {
                FillSource::PreviousMonth => last.amount,
                FillSource::Average => average_of(&index, id, target),
            };
            if planned.as_ref() == Some(&after) {
                preview.unchanged += 1;
                return None;
            }
            Some(after)
        });
        let ends_at = filled.clone().or_else(|| planned.clone());
        if let Some(amount) = ends_at {
            preview.total.0 += amount.0;
        }
        if let Some(after) = filled {
            preview.changes.push(FillLine {
                category_id: id,
                before: planned,
                after,
            });
        }
    }
    preview
}

/// The average signed Income Splits on the Budget's Accounts over the last 3 closed months.
pub fn average_income(budget: &Budget, reads: &dyn BudgetReads, today: NaiveDate) -> Money {
    let current = Period::of(today);
    let income: Vec<CategoryId> = reads
        .budget_categories()
        .iter()
        .filter(|c| c.category_type == CategoryTypes::Income)
        .map(|c| c.id)
        .collect();
    let accounts = reads.budget_accounts();
    let mut total = zero();
    for split in reads
        .budget_splits(
            current.shift(-AVERAGE_MONTHS).first_day(),
            current.prev().last_day(),
        )
        .iter()
        .filter(|s| budget.counts(&accounts, s.account_id) && income.contains(&s.category_id))
    {
        total += split.amount.0.clone();
    }
    Money((total / BigDecimal::from(AVERAGE_MONTHS)).with_scale_round(2, RoundingMode::HalfUp))
}

/// The Plan grid's footer for `month`: average income minus Total budgeted. Negative means
/// over-allocated.
pub fn unallocated(
    budget: &Budget,
    reads: &dyn BudgetReads,
    month: Period,
    today: NaiveDate,
) -> Money {
    Money(average_income(budget, reads, today).0 - total_budgeted(budget, reads, month).0)
}

// ---------------------------------------------------------------------------------------------
// The Plan grid (9b)
// ---------------------------------------------------------------------------------------------

/// The months the Plan grid shows at once.
pub const PLAN_MONTHS: usize = 6;

/// The cursor column past the last month: the ROLLOVER cell.
pub const PLAN_ROLLOVER_COLUMN: usize = PLAN_MONTHS;

/// How far ahead of the current month the Plan range can start.
const PLAN_MAX_LEAD: i32 = 12;

/// One month of a Category's row.
#[derive(Debug, Clone, PartialEq)]
pub struct PlanCell {
    pub month: Period,
    /// `None` is Unbudgeted (never budgeted, or Stopped).
    pub amount: Option<Money>,
    /// A record starts in this month: shown bold.
    pub own_record: bool,
    /// Before the current month: read-only.
    pub closed: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PlanRow {
    pub category_id: CategoryId,
    pub name: String,
    pub cells: Vec<PlanCell>,
    /// The current month's Rollover, `None` while that month is Unbudgeted.
    pub rollover: Option<Rollover>,
}

/// Rows under their Category's direct parent, or under no label for a top-level leaf.
#[derive(Debug, Clone, PartialEq)]
pub struct PlanSection {
    pub parent: Option<String>,
    pub rows: Vec<PlanRow>,
}

/// Everything the Plan tab draws for one range.
#[derive(Debug, Clone, PartialEq)]
pub struct Plan {
    pub months: Vec<Period>,
    pub sections: Vec<PlanSection>,
    /// Total budgeted per month: Budget Amounts only, no carry.
    pub totals: Vec<Money>,
    /// Average income minus each month's total; negative is over-allocated.
    pub unallocated: Vec<Money>,
    pub average_income: Money,
}

impl Plan {
    pub fn row_count(&self) -> usize {
        self.sections.iter().map(|section| section.rows.len()).sum()
    }

    /// The row at a position counted across sections.
    pub fn row(&self, index: usize) -> Option<&PlanRow> {
        self.sections
            .iter()
            .flat_map(|section| section.rows.iter())
            .nth(index)
    }
}

/// The range the Plan opens on: two months of history, the current month, then three ahead.
pub fn default_plan_start(today: NaiveDate) -> Period {
    Period::of(today).prev().prev()
}

/// The earliest and latest range start: the Budget's first Budget Amount (else the default start)
/// to a year ahead of the current month.
pub fn plan_start_bounds(budget: &Budget, today: NaiveDate) -> (Period, Period) {
    let earliest = budget.first_amount_month().map_or_else(
        || default_plan_start(today),
        |first| first.min(default_plan_start(today)),
    );
    let latest = (0..PLAN_MAX_LEAD).fold(default_plan_start(today), |month, _| month.next());
    (earliest, latest)
}

/// The Plan grid's figures for the six months from `start`. A row is any leaf with an amount or
/// a record in those months, so a cell just cleared keeps its row.
pub fn plan(budget: &Budget, reads: &dyn BudgetReads, start: Period, today: NaiveDate) -> Plan {
    let current = Period::of(today);
    let categories = reads.budget_categories();
    let leaves = expense_leaves(&categories);
    let months: Vec<Period> = std::iter::successors(Some(start), |month| Some(month.next()))
        .take(PLAN_MONTHS)
        .collect();
    let mut sections: Vec<PlanSection> = Vec::new();
    for category in &categories {
        let id = category.id;
        if !leaves.contains(&id) {
            continue;
        }
        let chain = budget.chain(id);
        let cells: Vec<PlanCell> = months
            .iter()
            .map(|month| {
                let found = applied(chain, *month);
                PlanCell {
                    month: *month,
                    own_record: found.as_ref().is_some_and(|f| f.own_record),
                    amount: found.map(|f| f.amount),
                    closed: *month < current,
                }
            })
            .collect();
        let shown = months.iter().zip(&cells).any(|(month, cell)| {
            cell.amount.is_some() || chain.iter().any(|record| record.month == *month)
        });
        if !shown {
            continue;
        }
        let parent = category
            .parent
            .and_then(|parent| tree::get(&categories, parent))
            .map(|parent| parent.name.clone());
        let row = PlanRow {
            category_id: id,
            name: category.name.clone(),
            cells,
            rollover: applied(chain, current).map(|found| found.rollover),
        };
        match sections.iter_mut().find(|section| section.parent == parent) {
            Some(section) => section.rows.push(row),
            None => sections.push(PlanSection {
                parent,
                rows: vec![row],
            }),
        }
    }
    // Top-level leaves have no label, so they close the grid as its "Other" group.
    sections.sort_by_key(|section| section.parent.is_none());
    let average_income = average_income(budget, reads, today);
    let totals: Vec<Money> = months
        .iter()
        .map(|month| total_over(budget, &categories, *month))
        .collect();
    let unallocated = totals
        .iter()
        .map(|total| Money(average_income.0.clone() - total.0.clone()))
        .collect();
    Plan {
        months,
        sections,
        totals,
        unallocated,
        average_income,
    }
}

/// The cell being typed into. The first key replaces the prefilled amount, as in a spreadsheet;
/// after that keys edit it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanEdit {
    pub category_id: CategoryId,
    pub month: Period,
    pub text: String,
    fresh: bool,
}

impl PlanEdit {
    pub fn new(category_id: CategoryId, month: Period, current: Option<&Money>) -> Self {
        Self {
            category_id,
            month,
            text: current.map(|money| money.0.to_string()).unwrap_or_default(),
            fresh: true,
        }
    }

    /// Digits and one decimal point; anything else is ignored.
    pub fn type_char(&mut self, c: char) {
        if !(c.is_ascii_digit() || c == '.') {
            return;
        }
        if self.fresh {
            self.text.clear();
            self.fresh = false;
        }
        if c == '.' && self.text.contains('.') {
            return;
        }
        self.text.push(c);
    }

    pub fn backspace(&mut self) {
        if self.fresh {
            self.text.clear();
            self.fresh = false;
        } else {
            self.text.pop();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::*;

    #[test]
    fn the_three_month_average_rounds_to_the_nearest_ten_and_floors_at_zero() {
        let mut world = World::new();
        world.post("ANZ Everyday", "Dining", date(2026, 6, 4), "-100.00");
        world.post("ANZ Everyday", "Dining", date(2026, 7, 4), "-100.00");
        world.post("ANZ Everyday", "Dining", date(2026, 8, 4), "-113.00");
        // Mean 104.33 rounds to 100.00. September itself is excluded.
        world.post("ANZ Everyday", "Dining", date(2026, 9, 4), "-900.00");
        world.post("ANZ Everyday", "Groceries", date(2026, 8, 4), "20.00");
        let (budgets, id) = world.budgets();
        let budget = budgets.get(id).cloned().unwrap_or_else(unreachable_budget);
        let ledger = world.ledger();
        assert_eq!(
            three_month_average(&budget, ledger, world.category("Dining"), sep()),
            money("100.00")
        );
        // A net refund averages below zero and is floored.
        assert_eq!(
            three_month_average(&budget, ledger, world.category("Groceries"), sep()),
            money("0.00")
        );
    }

    #[test]
    fn fill_targets_the_first_open_month_at_or_after_the_cursor() {
        assert_eq!(fill_target(Some(sep()), today()), sep());
        assert_eq!(
            fill_target(Some(sep().next().next()), today()),
            sep().next().next()
        );
        assert_eq!(fill_target(Some(sep().prev()), today()), sep().next());
        assert_eq!(fill_target(None, today()), sep().next());
    }

    #[test]
    fn fill_from_the_previous_month_copies_amounts_and_keeps_edited_cells() {
        let (world, mut budgets, id) = fill_world();
        let october = sep().next();
        let budget = budgets.get(id).cloned().unwrap_or_else(unreachable_budget);
        let preview = fill_preview(&budget, world.ledger(), october, FillSource::PreviousMonth);
        // Dining's September Month-only differs from the plan; Groceries matches it; Rent has its
        // own October record; Transport wasn't budgeted in September.
        assert_eq!(
            preview.changes,
            vec![FillLine {
                category_id: world.category("Dining"),
                before: Some(money("250.00")),
                after: money("400.00"),
            }]
        );
        assert_eq!(preview.kept, vec![world.category("Rent")]);
        assert_eq!(preview.unchanged, 1);
        assert_eq!(preview.total, money("2060.00"));

        let wrote = budgets.fill(
            id,
            world.ledger(),
            october,
            FillSource::PreviousMonth,
            today(),
        );
        assert_eq!(wrote, Ok(1));
        let budget = budgets.get(id).cloned().unwrap_or_else(unreachable_budget);
        let dining = budget.chain(world.category("Dining"));
        let filled = applied(dining, october).unwrap_or_else(|| unreachable!("filled"));
        assert_eq!(filled.amount, money("400.00"));
        assert_eq!(filled.rollover, Rollover::CarryUnspent);
        // Month-only: November falls back to the Onward amount.
        assert_eq!(
            applied(dining, october.next()).map(|found| found.amount),
            Some(money("250.00"))
        );
        assert_eq!(
            total_budgeted(&budget, world.ledger(), october),
            preview.total
        );
        // A second Fill finds every Category kept or unchanged.
        assert_eq!(
            budgets.fill(
                id,
                world.ledger(),
                october,
                FillSource::PreviousMonth,
                today()
            ),
            Ok(0)
        );
    }

    #[test]
    fn fill_from_the_average_rounds_spent_and_refuses_a_closed_month() {
        let (world, mut budgets, id) = fill_world();
        let budget = budgets.get(id).cloned().unwrap_or_else(unreachable_budget);
        // The current month: the three months before it are June to August.
        let preview = fill_preview(&budget, world.ledger(), sep(), FillSource::Average);
        // Dining has its own September record and Rent has no spend: 1000.00 becomes 0.00.
        assert_eq!(preview.kept, vec![world.category("Dining")]);
        assert_eq!(
            preview.changes,
            vec![FillLine {
                category_id: world.category("Rent"),
                before: Some(money("1000.00")),
                after: money("0.00"),
            }]
        );
        assert_eq!(preview.unchanged, 1, "Groceries averages its 500.00");
        assert_eq!(
            budgets.fill(
                id,
                world.ledger(),
                sep().prev(),
                FillSource::Average,
                today()
            ),
            Err(Error::ClosedMonth)
        );
    }

    #[test]
    fn an_average_exactly_on_a_five_rounds_away_from_zero() {
        let mut world = World::new();
        world.post("ANZ Everyday", "Dining", date(2026, 8, 4), "-45.00");
        // Total 45 over 3 months is 15.00, which rounds up to 20.
        let (budgets, id) = world.budgets();
        let budget = budgets.get(id).cloned().unwrap_or_else(unreachable_budget);
        assert_eq!(
            three_month_average(&budget, world.ledger(), world.category("Dining"), sep()),
            money("20.00")
        );
    }

    #[test]
    fn unallocated_is_average_income_minus_total_budgeted() {
        let mut world = World::new();
        world.post("ANZ Everyday", "Salary", date(2026, 6, 25), "6000.00");
        world.post("ANZ Everyday", "Salary", date(2026, 7, 25), "6000.00");
        world.post("ANZ Everyday", "Salary", date(2026, 8, 25), "5550.00");
        // Off-budget income doesn't count.
        world.post("Amex Platinum", "Salary", date(2026, 8, 26), "3000.00");
        let (mut budgets, id) = world.budgets();
        set(
            &mut budgets,
            &world,
            id,
            "Rent",
            sep(),
            Span::Onward,
            "1000.00",
            None,
        );
        set(
            &mut budgets,
            &world,
            id,
            "Groceries",
            sep(),
            Span::Onward,
            "5000.00",
            None,
        );
        let budget = budgets.get(id).cloned().unwrap_or_else(unreachable_budget);
        let ledger = world.ledger();
        assert_eq!(average_income(&budget, ledger, today()), money("5850.00"));
        assert_eq!(
            total_budgeted(&budget, world.ledger(), sep()),
            money("6000.00")
        );
        assert_eq!(
            unallocated(&budget, ledger, sep(), today()),
            money("-150.00")
        );
    }

    #[test]
    fn the_first_key_replaces_the_prefilled_amount_and_later_keys_edit_it() {
        let mut edit = PlanEdit::new(CategoryId::seed(1), sep(), Some(&money("250.00")));
        assert_eq!(edit.text, "250.00");
        edit.type_char('3');
        edit.type_char('x');
        edit.type_char('0');
        edit.type_char('.');
        edit.type_char('.');
        edit.type_char('5');
        assert_eq!(edit.text, "30.5");
        edit.backspace();
        assert_eq!(edit.text, "30.");
    }

    #[test]
    fn backspace_on_a_prefilled_cell_clears_it_at_once() {
        let mut edit = PlanEdit::new(CategoryId::seed(1), sep(), Some(&money("250.00")));
        edit.backspace();
        assert_eq!(edit.text, "");
    }

    #[test]
    fn the_plan_groups_leaves_under_their_parent_and_totals_each_month() {
        let world = World::new();
        let (mut budgets, id) = world.budgets();
        save(
            &mut budgets,
            &world,
            id,
            "Rent",
            sep(),
            Span::Onward,
            "1000",
        )
        .ok();
        save(
            &mut budgets,
            &world,
            id,
            "Electricity",
            sep(),
            Span::Onward,
            "90",
        )
        .ok();
        save(&mut budgets, &world, id, "Water", sep(), Span::Onward, "60").ok();
        save(
            &mut budgets,
            &world,
            id,
            "Transport",
            sep().next(),
            Span::Onward,
            "40",
        )
        .ok();
        let Some(budget) = budgets.get(id) else {
            panic!("the Budget was created");
        };
        let start = default_plan_start(today());
        let grid = plan(budget, world.ledger(), start, today());
        assert_eq!(grid.months.len(), PLAN_MONTHS);
        assert_eq!(grid.months.get(2), Some(&sep()));
        let labels: Vec<Option<&str>> = grid
            .sections
            .iter()
            .map(|section| section.parent.as_deref())
            .collect();
        // A top-level leaf (Transport) closes the grid as the unlabelled group.
        assert_eq!(labels, vec![Some("Housing"), Some("Utilities"), None]);
        assert_eq!(grid.row_count(), 4);
        // Sep is the third month; Oct the fourth.
        assert_eq!(grid.totals.get(2), Some(&money("1150.00")));
        assert_eq!(grid.totals.get(3), Some(&money("1190.00")));
        assert_eq!(grid.totals.first(), Some(&money("0")));
        let cell = grid
            .row(0)
            .and_then(|row| row.cells.get(2))
            .expect("Rent's September cell");
        assert!(cell.own_record && !cell.closed);
        assert!(
            grid.row(0)
                .is_some_and(|row| row.cells.first().is_some_and(|c| c.closed))
        );
        // Unallocated is average income less the month's total.
        assert_eq!(
            grid.unallocated.get(2).map(|left| left.0.clone()),
            Some(grid.average_income.0.clone() - money("1150.00").0)
        );
    }

    #[test]
    fn a_cleared_row_stays_in_the_grid_while_a_record_starts_in_the_range() {
        let world = World::new();
        let (mut budgets, id) = world.budgets();
        save(
            &mut budgets,
            &world,
            id,
            "Rent",
            sep().next(),
            Span::Onward,
            "1000",
        )
        .ok();
        save(
            &mut budgets,
            &world,
            id,
            "Rent",
            sep().next(),
            Span::Onward,
            "",
        )
        .ok();
        let Some(budget) = budgets.get(id) else {
            panic!("the Budget was created");
        };
        let grid = plan(budget, world.ledger(), default_plan_start(today()), today());
        assert_eq!(grid.row_count(), 1);
        assert!(
            grid.row(0)
                .is_some_and(|row| row.cells.iter().all(|c| c.amount.is_none()))
        );
    }

    #[test]
    fn the_plan_range_is_bounded_by_the_first_amount_and_a_year_ahead() {
        let world = World::new();
        let (budgets, id) = world.budgets();
        let Some(budget) = budgets.get(id) else {
            panic!("the Budget was created");
        };
        let (earliest, latest) = plan_start_bounds(budget, today());
        assert_eq!(earliest, default_plan_start(today()));
        assert_eq!(latest, (0..12).fold(earliest, |month, _| month.next()));
    }
}
