//! The Plan tab: the figures behind Fill and Unallocated, and the Plan grid (9b).

use bigdecimal::{BigDecimal, RoundingMode, Signed};
use chrono::NaiveDate;
use lib_core::{CategoryId, CategoryTypes, Money, Period};

use super::figures::{SpentIndex, expense_leaves};
use super::protocols::{BudgetCategories, BudgetCategory, BudgetReads};
use super::{AVERAGE_MONTHS, Budget, Rollover, applied, tree, zero};

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
