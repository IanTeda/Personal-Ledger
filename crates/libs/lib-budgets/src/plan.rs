//! The Plan tab: the figures behind Fill and Unallocated, and the Plan grid (9b).

use bigdecimal::{BigDecimal, RoundingMode, Signed};
use chrono::NaiveDate;
use lib_core::{CategoryTypes, Money, Period};

use lib_categories::{self as categories, Category};

use super::figures::{Ledger, SpentIndex, expense_leaves};
use super::{AVERAGE_MONTHS, Budget, Rollover, applied, zero};

/// A month's Total budgeted: Budget Amounts only, no carry.
pub fn total_budgeted(budget: &Budget, categories: &[Category], month: Period) -> Money {
    Money(
        expense_leaves(categories)
            .into_iter()
            .filter_map(|id| applied(budget.chain(id), month))
            .fold(zero(), |sum, found| sum + found.amount.0),
    )
}

/// The three months immediately before `target`.
fn months_before(target: Period) -> impl Iterator<Item = Period> {
    (1..=AVERAGE_MONTHS).map(move |back| (0..back).fold(target, |month, _| month.prev()))
}

fn average_of(index: &SpentIndex, category_id: u32, target: Period) -> Money {
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
    ledger: &Ledger<'_>,
    category_id: u32,
    target: Period,
) -> Money {
    average_of(&SpentIndex::build(budget, ledger), category_id, target)
}

/// Every leaf Expense Category's 3-month average for `target`, tree order.
pub fn three_month_averages(
    budget: &Budget,
    ledger: &Ledger<'_>,
    target: Period,
) -> Vec<(u32, Money)> {
    let index = SpentIndex::build(budget, ledger);
    expense_leaves(ledger.categories)
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
    pub category_id: u32,
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
    pub kept: Vec<u32>,
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
    ledger: &Ledger<'_>,
    target: Period,
    source: FillSource,
) -> FillPreview {
    let index = SpentIndex::build(budget, ledger);
    let previous = target.prev();
    let mut preview = FillPreview {
        target,
        source,
        changes: Vec::new(),
        kept: Vec::new(),
        unchanged: 0,
        total: Money(zero()),
    };
    for id in expense_leaves(ledger.categories) {
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
pub fn average_income(budget: &Budget, ledger: &Ledger<'_>, today: NaiveDate) -> Money {
    let last_three: Vec<Period> = months_before(Period::of(today)).collect();
    let income: Vec<u32> = ledger
        .categories
        .iter()
        .filter(|c| c.category_type == CategoryTypes::Income)
        .map(|c| c.id)
        .collect();
    let mut total = zero();
    for transaction in ledger
        .transactions
        .iter()
        .filter(|t| budget.account_ids.contains(&t.account_id))
        .filter(|t| last_three.contains(&Period::of(t.date)))
    {
        for split in transaction
            .splits
            .iter()
            .filter(|s| income.contains(&s.category_id))
        {
            total += split.amount.0.clone();
        }
    }
    Money((total / BigDecimal::from(AVERAGE_MONTHS)).with_scale_round(2, RoundingMode::HalfUp))
}

/// The Plan grid's footer for `month`: average income minus Total budgeted. Negative means
/// over-allocated.
pub fn unallocated(budget: &Budget, ledger: &Ledger<'_>, month: Period, today: NaiveDate) -> Money {
    Money(
        average_income(budget, ledger, today).0
            - total_budgeted(budget, ledger.categories, month).0,
    )
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
    pub category_id: u32,
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
pub fn plan(budget: &Budget, ledger: &Ledger<'_>, start: Period, today: NaiveDate) -> Plan {
    let current = Period::of(today);
    let months: Vec<Period> = std::iter::successors(Some(start), |month| Some(month.next()))
        .take(PLAN_MONTHS)
        .collect();
    let mut sections: Vec<PlanSection> = Vec::new();
    for (id, _) in categories::paths_in_tree_order(ledger.categories) {
        if !expense_leaves(ledger.categories).contains(&id) {
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
        let Some(category) = ledger.categories.iter().find(|c| c.id == id) else {
            continue;
        };
        if !shown {
            continue;
        }
        let parent = category
            .parent
            .and_then(|parent| ledger.categories.iter().find(|c| c.id == parent))
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
    let average_income = average_income(budget, ledger, today);
    let totals: Vec<Money> = months
        .iter()
        .map(|month| total_budgeted(budget, ledger.categories, *month))
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
    pub category_id: u32,
    pub month: Period,
    pub text: String,
    fresh: bool,
}

impl PlanEdit {
    pub fn new(category_id: u32, month: Period, current: Option<&Money>) -> Self {
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
