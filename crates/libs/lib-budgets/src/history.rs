//! The History tab: Budget against Spent month by month, its range and its CSV export.

use std::collections::BTreeMap;

use bigdecimal::{BigDecimal, RoundingMode};
use chrono::NaiveDate;
use lib_core::{CategoryId, CategoryTypes, Money, Period};

use super::figures::{SpentIndex, effective, expense_leaves};
use super::protocols::{BudgetCategories, BudgetReads};
use super::{Budget, Limit, tree, zero};

/// One month of one History row: what applied then against what was spent.
#[derive(Debug, Clone, PartialEq)]
pub struct HistoryCell {
    pub budget: Money,
    pub spent: Money,
    pub over: bool,
}

/// A History row: a budgeted leaf or a parent's rollup, one cell per visible month (`None` is the
/// `—` of an unbudgeted month).
#[derive(Debug, Clone, PartialEq)]
pub struct HistoryRow {
    pub category_id: CategoryId,
    pub is_parent: bool,
    pub cells: Vec<Option<HistoryCell>>,
    /// Mean Spent over the closed months shown in which the row was budgeted.
    pub average: Option<Money>,
    /// Closed budgeted months that were over.
    pub over_months: u32,
    /// Closed months shown in which the row was budgeted: OVER's denominator.
    pub closed_months: u32,
}

/// The chart's pair for one month: totals over budgeted leaves only.
#[derive(Debug, Clone, PartialEq)]
pub struct HistoryMonth {
    pub month: Period,
    pub budget: Money,
    pub spent: Money,
    /// Total Spent exceeds the total effective budget.
    pub over: bool,
    /// The current month ("to date"), counted in neither AVG nor OVER.
    pub is_current: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct History {
    pub months: Vec<HistoryMonth>,
    pub rows: Vec<HistoryRow>,
    /// Leaves shown out of leaves ever budgeted in this Budget: the status line.
    pub leaves_shown: usize,
    pub leaves_ever_budgeted: usize,
}

/// The range nav's bounds: the first Budget Amount month to the current month, which is the
/// rightmost column (future months belong to Plan).
pub fn history_bounds(budget: &Budget, today: NaiveDate) -> Option<(Period, Period)> {
    budget
        .first_amount_month()
        .map(|first| (first, Period::of(today)))
}

/// Budget against Spent for every month in `first..=last` (both inclusive), each recomputed from
/// the Budget's current Accounts and the current Category tree. Known Costs are left out.
pub fn history(
    budget: &Budget,
    reads: &dyn BudgetReads,
    first: Period,
    last: Period,
    today: NaiveDate,
) -> History {
    let categories = reads.budget_categories();
    let accounts = reads.budget_accounts();
    let index = SpentIndex::load(budget, reads, &accounts, first, last);
    let current = Period::of(today);
    let mut months = Vec::new();
    let mut walking = first;
    while walking <= last {
        months.push(walking);
        walking = walking.next();
    }

    // month index -> leaf -> cell
    let leaf_cells: BTreeMap<CategoryId, Vec<Option<HistoryCell>>> = expense_leaves(&categories)
        .into_iter()
        .map(|id| {
            let cells = months
                .iter()
                .map(|&month| {
                    effective(budget, &index, id, month, today).map(|found| {
                        let total = found.total();
                        let spent = index.spent(id, month);
                        HistoryCell {
                            over: spent > total.0,
                            budget: total,
                            spent: Money(spent),
                        }
                    })
                })
                .collect();
            (id, cells)
        })
        .collect();

    let month_totals = months
        .iter()
        .enumerate()
        .map(|(at, &month)| {
            let (budgeted, spent) = leaf_cells
                .values()
                .filter_map(|cells| cells[at].as_ref())
                .fold((zero(), zero()), |(b, s), cell| {
                    (b + cell.budget.0.clone(), s + cell.spent.0.clone())
                });
            HistoryMonth {
                month,
                over: spent > budgeted,
                budget: Money(budgeted),
                spent: Money(spent),
                is_current: month == current,
            }
        })
        .collect();

    let shown = |cells: &[Option<HistoryCell>]| cells.iter().any(Option::is_some);
    let mut rows = Vec::new();
    for category in &categories {
        let id = category.id;
        if let Some(cells) = leaf_cells.get(&id) {
            if shown(cells) {
                rows.push(history_row(id, false, cells.clone(), &months, current));
            }
            continue;
        }
        if category.category_type != CategoryTypes::Expense {
            continue;
        }
        let below: Vec<&Vec<Option<HistoryCell>>> = tree::descendants_inclusive(&categories, id)
            .iter()
            .filter_map(|leaf| leaf_cells.get(leaf))
            .filter(|cells| shown(cells))
            .collect();
        if below.is_empty() {
            continue;
        }
        let rolled: Vec<Option<HistoryCell>> = (0..months.len())
            .map(|at| {
                let cells: Vec<&HistoryCell> = below
                    .iter()
                    .filter_map(|cells| cells[at].as_ref())
                    .collect();
                (!cells.is_empty()).then(|| {
                    let budget = cells.iter().fold(zero(), |a, c| a + c.budget.0.clone());
                    let spent = cells.iter().fold(zero(), |a, c| a + c.spent.0.clone());
                    HistoryCell {
                        over: spent > budget,
                        budget: Money(budget),
                        spent: Money(spent),
                    }
                })
            })
            .collect();
        rows.push(history_row(id, true, rolled, &months, current));
    }

    let leaves_shown = rows.iter().filter(|row| !row.is_parent).count();
    let leaves_ever_budgeted = budget
        .limits
        .iter()
        .filter(|(id, chain)| {
            leaf_cells.contains_key(id) && chain.iter().any(|r| !matches!(r.limit, Limit::Stop))
        })
        .count();
    History {
        months: month_totals,
        rows,
        leaves_shown,
        leaves_ever_budgeted,
    }
}

fn history_row(
    category_id: CategoryId,
    is_parent: bool,
    cells: Vec<Option<HistoryCell>>,
    months: &[Period],
    current: Period,
) -> HistoryRow {
    let closed: Vec<&HistoryCell> = months
        .iter()
        .zip(&cells)
        .filter(|(month, _)| **month < current)
        .filter_map(|(_, cell)| cell.as_ref())
        .collect();
    let closed_months = u32::try_from(closed.len()).unwrap_or(u32::MAX);
    let average = (!closed.is_empty()).then(|| {
        let total = closed.iter().fold(zero(), |a, c| a + c.spent.0.clone());
        Money((total / BigDecimal::from(closed_months)).with_scale_round(2, RoundingMode::HalfUp))
    });
    let over_months = u32::try_from(closed.iter().filter(|c| c.over).count()).unwrap_or(u32::MAX);
    HistoryRow {
        category_id,
        is_parent,
        cells,
        average,
        over_months,
        closed_months,
    }
}

/// How many months the History tab shows at once.
pub const HISTORY_MONTHS: usize = 6;

/// The History window ending at `end`: up to [`HISTORY_MONTHS`] months, held inside
/// [`history_bounds`], or `None` while the Budget has no Budget Amount yet.
pub fn history_range(budget: &Budget, end: Period, today: NaiveDate) -> Option<(Period, Period)> {
    let (earliest, latest) = history_bounds(budget, today)?;
    // A window never ends before it could be full, unless the Budget is younger than that.
    let first_full = (1..HISTORY_MONTHS).fold(earliest, |month, _| month.next());
    let last = end.max(first_full).min(latest);
    let first = (1..HISTORY_MONTHS)
        .fold(last, |month, _| month.prev())
        .max(earliest);
    Some((first, last))
}

/// The History table's column headings for the export, already worded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoryCsvLabels {
    pub category: String,
    pub budget: String,
    pub spent: String,
    pub average: String,
    pub over: String,
}

fn csv_field(text: &str) -> String {
    if text.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", text.replace('"', "\"\""))
    } else {
        text.to_string()
    }
}

/// The visible History table as CSV: a row per Category (by path), a budget and a spent column
/// per month, then AVG and OVER. Plain numbers, an unbudgeted month left empty.
pub fn history_csv(
    history: &History,
    categories: &dyn BudgetCategories,
    labels: &HistoryCsvLabels,
    month_label: impl Fn(Period) -> String,
) -> String {
    let mut lines = Vec::with_capacity(history.rows.len() + 1);
    let mut header = vec![csv_field(&labels.category)];
    for month in &history.months {
        let month = month_label(month.month);
        header.push(csv_field(&format!("{month} {}", labels.budget)));
        header.push(csv_field(&format!("{month} {}", labels.spent)));
    }
    header.push(csv_field(&labels.average));
    header.push(csv_field(&labels.over));
    lines.push(header.join(","));
    let categories = categories.budget_categories();
    for row in &history.rows {
        let name = tree::get(&categories, row.category_id)
            .map(|category| category.path.clone())
            .unwrap_or_default();
        let mut fields = vec![csv_field(&name)];
        for cell in &row.cells {
            match cell {
                Some(cell) => {
                    fields.push(cell.budget.0.with_scale(2).to_string());
                    fields.push(cell.spent.0.with_scale(2).to_string());
                }
                None => fields.extend([String::new(), String::new()]),
            }
        }
        fields.push(
            row.average
                .as_ref()
                .map(|average| average.0.with_scale(2).to_string())
                .unwrap_or_default(),
        );
        fields.push(if row.closed_months == 0 {
            String::new()
        } else {
            format!("{}/{}", row.over_months, row.closed_months)
        });
        lines.push(fields.join(","));
    }
    lines.join("\n") + "\n"
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::*;

    #[test]
    fn history_average_and_over_use_closed_months_only() {
        let (world, budget) = history_world();
        let first = Period::of(date(2026, 6, 1));
        let history = history(&budget, world.ledger(), first, sep(), today());
        let dining = history
            .rows
            .iter()
            .find(|r| r.category_id == world.category("Dining"));
        // June 100, July 350, August 200: mean 216.67; over in July only; September counts in neither.
        assert_eq!(
            dining.and_then(|r| r.average.clone()),
            Some(money("216.67"))
        );
        assert_eq!(
            dining.map(|r| (r.over_months, r.closed_months)),
            Some((1, 3))
        );
        assert_eq!(dining.map(|r| r.cells.len()), Some(4));
        let september = dining.and_then(|r| r.cells.last().cloned()).flatten();
        assert_eq!(september.map(|c| c.over), Some(true));
    }

    #[test]
    fn history_shows_a_dash_for_an_unbudgeted_month_and_parents_as_rollups() {
        let (world, budget) = history_world();
        let earlier = Period::of(date(2026, 5, 1));
        let history = history(&budget, world.ledger(), earlier, sep(), today());
        let dining = history
            .rows
            .iter()
            .find(|r| r.category_id == world.category("Dining"));
        assert_eq!(dining.map(|r| r.cells[0].is_none()), Some(true));
        let food = history
            .rows
            .iter()
            .find(|r| r.category_id == world.category("Food"));
        assert_eq!(food.map(|r| r.is_parent), Some(true));
        assert_eq!(
            food.and_then(|r| r.cells[2].clone()).map(|c| c.spent),
            Some(money("350.00"))
        );
        assert_eq!((history.leaves_shown, history.leaves_ever_budgeted), (1, 1));
    }

    #[test]
    fn history_month_totals_count_budgeted_leaves_only_and_mark_the_current_month() {
        let (mut world, budget) = history_world();
        world.post("ANZ Everyday", "Household", date(2026, 7, 9), "-999.00");
        let first = Period::of(date(2026, 6, 1));
        let history = history(&budget, world.ledger(), first, sep(), today());
        let july = &history.months[1];
        assert_eq!(
            (july.budget.clone(), july.spent.clone(), july.over),
            (money("300.00"), money("350.00"), true)
        );
        assert_eq!(
            history
                .months
                .iter()
                .map(|m| m.is_current)
                .collect::<Vec<_>>(),
            [false, false, false, true]
        );
    }

    #[test]
    fn history_bounds_run_from_the_first_amount_to_the_current_month() {
        let (_, budget) = history_world();
        assert_eq!(
            history_bounds(&budget, today()),
            Some((Period::of(date(2026, 6, 1)), sep()))
        );
        assert_eq!(history_bounds(&unreachable_budget(), today()), None);
    }

    #[test]
    fn the_history_range_is_six_months_inside_the_bounds() {
        let (_, budget) = history_world();
        let jun = Period::of(date(2026, 6, 1));
        // The Budget starts in June, so only four months exist yet.
        assert_eq!(history_range(&budget, sep(), today()), Some((jun, sep())));
        assert_eq!(
            history_range(&budget, sep().next(), today()),
            Some((jun, sep())),
            "never past the current month"
        );
        assert_eq!(
            history_range(&budget, jun, today()),
            Some((jun, sep())),
            "never a shorter window than the Budget allows"
        );
        // A later today gives a full window that can step back.
        let later = date(2027, 3, 10);
        let mar = Period::of(later);
        assert_eq!(
            history_range(&budget, mar, later),
            Some((Period::of(date(2026, 10, 1)), mar))
        );
        assert_eq!(
            history_range(&budget, Period::of(date(2026, 12, 1)), later),
            Some((Period::of(date(2026, 7, 1)), Period::of(date(2026, 12, 1))))
        );
        assert_eq!(history_range(&unreachable_budget(), sep(), today()), None);
    }

    #[test]
    fn the_history_csv_writes_plain_numbers_and_quotes_awkward_names() {
        let (mut world, budget) = history_world();
        let dining = world.category("Dining");
        if let Some(category) = world.reads.categories.iter_mut().find(|c| c.id == dining) {
            category.name = "Dining, \"out\"".to_string();
            category.path = "Food \u{203a} Dining, \"out\"".to_string();
        }
        let jun = Period::of(date(2026, 6, 1));
        let shown = history(&budget, world.ledger(), jun, sep(), today());
        let labels = HistoryCsvLabels {
            category: "Category".into(),
            budget: "budget".into(),
            spent: "spent".into(),
            average: "Avg".into(),
            over: "Over".into(),
        };
        let csv = history_csv(&shown, world.ledger(), &labels, |month| {
            format!("{}-{:02}", month.year, month.month)
        });
        let lines: Vec<&str> = csv.lines().collect();
        assert_eq!(lines.len(), shown.rows.len() + 1);
        assert!(lines[0].starts_with("Category,2026-06 budget,2026-06 spent,"));
        assert!(lines[0].ends_with(",2026-09 budget,2026-09 spent,Avg,Over"));
        let row = shown
            .rows
            .iter()
            .position(|row| row.category_id == dining)
            .unwrap_or_else(|| unreachable!("Dining is budgeted"));
        let line = lines[row + 1];
        assert!(
            line.contains("Dining, \"\"out\"\"\",300.00,100.00,300.00,350.00,"),
            "{line}"
        );
        assert!(line.starts_with('"'));
        assert!(!line.contains('\u{25b2}'));
        let dining_row = &shown.rows[row];
        assert!(line.ends_with(&format!(
            ",{}/{}",
            dining_row.over_months, dining_row.closed_months
        )));
        // Every line has the same number of unquoted separators as the header has columns.
        assert_eq!(lines[0].split(',').count(), 1 + shown.months.len() * 2 + 2);
    }
}
