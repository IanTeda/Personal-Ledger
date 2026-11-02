//! The History tab (9c): a chart with one pair of bars per month (effective budget and Spent over
//! the budgeted leaves), then a table of Spent per Category and month with its AVG and OVER
//! columns, with the range shown and the cell cursor.
//!
//! An over-budget month or cell is flagged; the current month is "to date" and counts in neither
//! AVG nor OVER. **Export CSV** hands the Router the table shown as CSV ([`ExportHistory`]), which
//! the Client saves where the person chooses.

use bigdecimal::{BigDecimal, ToPrimitive, Zero};
use chrono::NaiveDate;
use lib_core::{Money, Period};
use lib_list_cursor::Step;
use lib_locale::format::format_month;

use super::budgets_page::BudgetsPagePresenter;
use super::{EMPTY_CELL, category_name, count, depth, money, range_label};
use crate::protocols::{BudgetReads, BudgetsRouter, ExportHistory};
use crate::{
    Budget, History, HistoryCsvLabels, HistoryRow, history, history_csv, history_range, msg,
};

/// What the History tab is told.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HistoryEvent {
    /// `[`/`]` and the nav's arrows: the range a month back or on.
    PreviousRange,
    NextRange,
    /// `h`/`l`: the month cursor, stopping at either end of the range.
    MonthLeft,
    MonthRight,
    /// `x` and **Export CSV**.
    Export,
    /// A click on a cell: moves the cursor there, and opens its detail when it was already there.
    ClickCell(usize, usize),
}

/// The tab's own state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct HistoryState {
    /// The range's last month.
    pub end: Period,
    /// A row and a month column.
    pub cursor: (usize, usize),
}

impl HistoryState {
    pub fn new(today: NaiveDate) -> Self {
        Self {
            end: Period::of(today),
            cursor: (0, 0),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct HistoryViewModel {
    pub bars: Vec<HistoryBarView>,
    /// `over in 2 of the last 5 closed months`.
    pub caption: String,
    pub columns: HistoryColumns,
    pub rows: Vec<HistoryRowView>,
    /// What shows in place of the chart and table while no Category has an amount.
    pub empty: Option<String>,
}

/// One month of the chart: the budget and Spent bars as shares of the range's largest figure,
/// 0 to 1, the pair's figures and the month's label.
#[derive(Debug, Clone, PartialEq)]
pub struct HistoryBarView {
    pub budget: f32,
    pub spent: f32,
    pub over: bool,
    /// The current month, still moving: drawn outlined.
    pub current: bool,
    pub pair: String,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct HistoryColumns {
    pub category: String,
    pub budget: String,
    pub months: Vec<String>,
    pub average: String,
    pub over: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoryRowView {
    /// A parent's name carries its `▾`.
    pub name: String,
    pub depth: usize,
    /// A parent, or the cursor's row.
    pub bold: bool,
    /// The effective budget of the latest month shown in which it had one.
    pub budget: String,
    pub cells: Vec<HistoryCellView>,
    pub average: String,
    pub over_count: String,
    /// Over in any closed month.
    pub over_alert: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoryCellView {
    pub text: String,
    pub over: bool,
    /// Unbudgeted that month.
    pub empty: bool,
    pub cursor: bool,
}

impl BudgetsPagePresenter {
    pub(super) fn history_event(&mut self, event: HistoryEvent, router: &mut dyn BudgetsRouter) {
        match event {
            HistoryEvent::PreviousRange => self.shift_history_range(false),
            HistoryEvent::NextRange => self.shift_history_range(true),
            HistoryEvent::MonthLeft => self.step_history_month(false),
            HistoryEvent::MonthRight => self.step_history_month(true),
            HistoryEvent::Export => self.export_history(router),
            HistoryEvent::ClickCell(row, column) => {
                let was_here = self.history.cursor == (row, column);
                self.history.cursor = (row, column);
                if was_here {
                    self.open_history_detail(router);
                }
            }
        }
    }

    pub(super) fn history_move(&mut self, step: Step) {
        let Some(history) = self.history_data() else {
            return;
        };
        let (row, column) = history_cursor(self.history.cursor, &history);
        self.history.cursor = (lib_list_cursor::step(row, history.rows.len(), step), column);
    }

    /// The chart and table for the range in view, `None` while the Budget has no amount yet.
    fn history_data(&self) -> Option<History> {
        let budget = self.budget()?;
        let ports = self.ports.borrow();
        history_of(&budget, ports.reads(), self.history.end, self.today())
    }

    /// Steps the range a month, inside the Budget's bounds.
    fn shift_history_range(&mut self, forward: bool) {
        let Some(budget) = self.budget() else {
            return;
        };
        let today = self.today();
        // Step from where the range really ends, not from a stale out-of-bounds end.
        let Some((_, last)) = history_range(&budget, self.history.end, today) else {
            return;
        };
        let target = if forward { last.next() } else { last.prev() };
        if let Some((_, held)) = history_range(&budget, target, today) {
            self.history.end = held;
        }
    }

    fn step_history_month(&mut self, forward: bool) {
        let Some(history) = self.history_data() else {
            return;
        };
        let (row, column) = history_cursor(self.history.cursor, &history);
        let column = if forward {
            (column + 1).min(history.months.len().saturating_sub(1))
        } else {
            column.saturating_sub(1)
        };
        self.history.cursor = (row, column);
    }

    /// `enter` on a cell: the Category detail for that Category and month (a parent's rollup).
    pub(super) fn open_history_detail(&mut self, router: &mut dyn BudgetsRouter) {
        let Some(history) = self.history_data() else {
            return;
        };
        let (row, column) = history_cursor(self.history.cursor, &history);
        self.history.cursor = (row, column);
        if let (Some(row), Some(month)) = (history.rows.get(row), history.months.get(column)) {
            router.open_detail(self.current, row.category_id, month.month);
        }
    }

    /// The table shown as CSV, for the Router to save.
    fn export_history(&self, router: &mut dyn BudgetsRouter) {
        let Some(history) = self.history_data() else {
            return;
        };
        let labels = HistoryCsvLabels {
            category: lib_locale::msg::column_category(),
            budget: msg::budgets_column_budget(),
            spent: msg::budgets_column_spent(),
            average: msg::budgets_history_column_avg(),
            over: msg::budgets_history_column_over(),
        };
        let csv = {
            let ports = self.ports.borrow();
            history_csv(&history, ports.reads(), &labels, super::period_label)
        };
        router.export_history(ExportHistory { csv });
    }

    /// The tab's view model, the range's label and the status line's right-hand text.
    pub(super) fn history_view(
        &self,
        budget: &Budget,
        reads: &dyn BudgetReads,
    ) -> (HistoryViewModel, String, String) {
        let Some(history) = history_of(budget, reads, self.history.end, self.today()) else {
            let view = HistoryViewModel {
                empty: Some(msg::budgets_history_empty()),
                ..HistoryViewModel::default()
            };
            return (view, String::new(), String::new());
        };
        let range = match (history.months.first(), history.months.last()) {
            (Some(first), Some(last)) => range_label(first.month, last.month),
            _ => String::new(),
        };
        let status = msg::budgets_status_history(
            count(history.leaves_ever_budgeted),
            &history.leaves_shown.to_string(),
        );
        let cursor = history_cursor(self.history.cursor, &history);
        let categories = reads.budget_categories();
        let view = HistoryViewModel {
            bars: bars(&history),
            caption: caption(&history),
            columns: HistoryColumns {
                category: lib_locale::msg::column_category(),
                budget: msg::budgets_column_budget(),
                months: history
                    .months
                    .iter()
                    .map(|month| format_month(month.month.month))
                    .collect(),
                average: msg::budgets_history_column_avg(),
                over: msg::budgets_history_column_over(),
            },
            rows: history
                .rows
                .iter()
                .enumerate()
                .map(|(index, row)| {
                    let name = category_name(&categories, row.category_id);
                    HistoryRowView {
                        name: if row.is_parent {
                            msg::budgets_row_parent(&name)
                        } else {
                            name
                        },
                        depth: depth(&categories, row.category_id),
                        bold: row.is_parent || cursor.0 == index,
                        budget: latest_budget(row).map_or_else(|| EMPTY_CELL.to_string(), money),
                        cells: row
                            .cells
                            .iter()
                            .enumerate()
                            .map(|(column, cell)| HistoryCellView {
                                text: cell.as_ref().map_or_else(
                                    || EMPTY_CELL.to_string(),
                                    |cell| {
                                        if cell.over {
                                            msg::budgets_history_cell_over(&money(&cell.spent))
                                        } else {
                                            money(&cell.spent)
                                        }
                                    },
                                ),
                                over: cell.as_ref().is_some_and(|cell| cell.over),
                                empty: cell.is_none(),
                                cursor: cursor == (index, column),
                            })
                            .collect(),
                        average: row
                            .average
                            .as_ref()
                            .map_or_else(|| EMPTY_CELL.to_string(), money),
                        over_count: if row.closed_months == 0 {
                            EMPTY_CELL.to_string()
                        } else {
                            msg::budgets_history_over_count(
                                &row.over_months.to_string(),
                                &row.closed_months.to_string(),
                            )
                        },
                        over_alert: row.over_months > 0,
                    }
                })
                .collect(),
            empty: history.rows.is_empty().then(msg::budgets_history_empty),
        };
        (view, range, status)
    }
}

fn history_of(
    budget: &Budget,
    reads: &dyn BudgetReads,
    end: Period,
    today: NaiveDate,
) -> Option<History> {
    let (first, last) = history_range(budget, end, today)?;
    Some(history(budget, reads, first, last, today))
}

/// The cursor held inside the table, so a shorter range can't leave it dangling.
fn history_cursor(cursor: (usize, usize), history: &History) -> (usize, usize) {
    (
        cursor.0.min(history.rows.len().saturating_sub(1)),
        cursor.1.min(history.months.len().saturating_sub(1)),
    )
}

/// The row's BUDGET column: the effective budget of the latest month shown in which it had one.
fn latest_budget(row: &HistoryRow) -> Option<&Money> {
    row.cells
        .iter()
        .rev()
        .find_map(|cell| cell.as_ref().map(|cell| &cell.budget))
}

/// Each month's pair of bars, scaled against the range's largest figure.
fn bars(history: &History) -> Vec<HistoryBarView> {
    let tallest = history
        .months
        .iter()
        .flat_map(|month| [&month.budget.0, &month.spent.0])
        .max()
        .cloned()
        .filter(|value| *value > BigDecimal::zero());
    let height = |value: &Money| {
        tallest.as_ref().map_or(0.0, |tallest| {
            (value.0.clone() / tallest.clone())
                .to_f32()
                .unwrap_or(0.0)
                .clamp(0.0, 1.0)
        })
    };
    history
        .months
        .iter()
        .map(|month| {
            let mut label = format_month(month.month.month);
            if month.is_current {
                label = msg::budgets_history_month_to_date(&label);
            } else if month.over {
                label = msg::budgets_history_month_over(&label);
            }
            HistoryBarView {
                budget: height(&month.budget),
                spent: height(&month.spent),
                over: month.over,
                current: month.is_current,
                pair: msg::budgets_history_pair(&money(&month.spent), &money(&month.budget)),
                label,
            }
        })
        .collect()
}

/// `over in 2 of the last 5 closed months`, or that there are none in range.
fn caption(history: &History) -> String {
    let closed: Vec<_> = history
        .months
        .iter()
        .filter(|month| !month.is_current)
        .collect();
    if closed.is_empty() {
        return msg::budgets_history_caption_none();
    }
    msg::budgets_history_caption(
        count(closed.len()),
        &closed.iter().filter(|month| month.over).count().to_string(),
    )
}
