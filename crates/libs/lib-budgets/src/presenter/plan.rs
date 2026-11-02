//! The Plan tab (9b): a six-month grid of Budget Amounts per leaf Category under its parent's
//! label, a ROLLOVER column, and the Total budgeted and Unallocated footers, with the range shown,
//! the cell cursor and the cell being typed into.
//!
//! Typing into a cell is the Client's Insert mode: the Client hands each typed character in as
//! [`PlanEvent::Type`] and reads [`BudgetsPagePresenter::editing`] to enter and leave the mode.
//! The cell saves through [`SaveCell`], and text that isn't an amount keeps it open.

use bigdecimal::Signed;
use chrono::NaiveDate;
use lib_core::{CategoryId, Period};
use lib_list_cursor::Step;
use lib_locale::format::format_month;
use lib_viper::view::count;

use super::budgets_page::BudgetsPagePresenter;
use super::{EMPTY_CELL, money, range_label};
use crate::interactor::{CycleRollover, SaveCell};
use crate::protocols::{BudgetCategories, BudgetReads, BudgetsRouter};
use crate::{
    Budget, PLAN_MONTHS, PLAN_ROLLOVER_COLUMN, Plan, PlanEdit, Rollover, Span, default_plan_start,
    fill_target, msg, plan, plan_start_bounds,
};

/// What the Plan tab is told.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlanEvent {
    /// `[`/`]` and the nav's arrows: the range a month back or on.
    PreviousRange,
    NextRange,
    /// `h`/`l`: the cursor's column.
    ColumnLeft,
    ColumnRight,
    /// `i`: types into the cursor's cell, or cycles the ROLLOVER cell.
    StartEdit,
    /// `r`: cycles the cursor row's Rollover.
    CycleRollover,
    /// `f` and the Fill button: Fill on the month the cursor targets.
    Fill,
    /// `x`/`backspace`: clears the cursor's cell (a Stop from that month).
    Clear,
    /// `0`: an explicit 0.00 onward.
    Zero,
    /// `c` and **+ Budget a category**.
    BudgetCategory,
    /// A click on a cell (`PLAN_ROLLOVER_COLUMN` for the ROLLOVER cell): moves the cursor there,
    /// and starts typing into an open month or cycles the Rollover.
    ClickCell(usize, usize),
    /// A character typed into the cell.
    Type(char),
    Backspace,
    /// `enter` (Onward), `shift+enter` (Month-only) or `tab`/`shift+tab`, which save and move on
    /// to the next (`step` 1) or previous (-1) month's cell.
    Save {
        span: Span,
        step: i32,
    },
    /// `esc`: drops the cell being typed into.
    Abandon,
}

/// The tab's own state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct PlanState {
    /// The grid's first month; it shows six from here.
    pub start: Period,
    /// A row across sections, and a column (the last is ROLLOVER).
    pub cursor: (usize, usize),
    /// The cell being typed into.
    pub edit: Option<PlanEdit>,
}

impl PlanState {
    pub fn new(today: NaiveDate) -> Self {
        Self {
            start: default_plan_start(today),
            cursor: (0, 0),
            edit: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanViewModel {
    pub category_column: String,
    pub months: Vec<PlanMonthView>,
    pub rollover_column: String,
    pub sections: Vec<PlanSectionView>,
    pub total: PlanFooterView,
    pub unallocated: PlanFooterView,
    /// What shows in place of the grid when no Category has an amount in the range.
    pub empty: Option<String>,
}

/// A month's column heading: `Jun · now` on the current month, which the View fills.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanMonthView {
    pub label: String,
    pub now: bool,
}

/// A parent's label, or `Other`, over its leaves.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanSectionView {
    pub label: String,
    pub rows: Vec<PlanRowView>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanRowView {
    /// The row's position across sections: what a click sends.
    pub index: usize,
    pub name: String,
    /// The cursor is on this row.
    pub cursor: bool,
    pub cells: Vec<PlanCellView>,
    pub rollover: PlanCellView,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanCellView {
    /// The amount, the text being typed while `editing`, or the Rollover mode.
    pub text: String,
    /// A record starts in this month: drawn bold.
    pub own_record: bool,
    /// A closed month: muted and read-only.
    pub closed: bool,
    pub cursor: bool,
    pub editing: bool,
}

/// A footer line: its label and one figure per month, each flagged when it is over.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanFooterView {
    pub label: String,
    pub figures: Vec<(String, bool)>,
}

fn rollover_label(rollover: Rollover) -> String {
    match rollover {
        Rollover::None => EMPTY_CELL.to_string(),
        Rollover::CarryUnspent => msg::budgets_plan_rollover_unspent(),
        Rollover::CarryBoth => msg::budgets_plan_rollover_both(),
    }
}

impl BudgetsPagePresenter {
    pub(super) fn plan_event(&mut self, event: PlanEvent, router: &mut dyn BudgetsRouter) {
        match event {
            PlanEvent::PreviousRange => {
                self.shift_plan_range(false);
            }
            PlanEvent::NextRange => {
                self.shift_plan_range(true);
            }
            PlanEvent::ColumnLeft => self.plan.cursor.1 = self.plan.cursor.1.saturating_sub(1),
            PlanEvent::ColumnRight => {
                self.plan.cursor.1 = (self.plan.cursor.1 + 1).min(PLAN_ROLLOVER_COLUMN);
            }
            PlanEvent::StartEdit => self.start_plan_edit(),
            PlanEvent::CycleRollover => {
                if let Some(category) = self.plan_row_category(self.held_plan_cursor().0) {
                    self.cycle_rollover(category);
                }
            }
            PlanEvent::Fill => {
                if self.editable().is_some() {
                    router.open_fill(self.current, self.fill_month());
                }
            }
            PlanEvent::Clear => self.write_plan_cell(""),
            PlanEvent::Zero => self.write_plan_cell("0"),
            PlanEvent::BudgetCategory => {
                router.open_limit_picker(self.current);
            }
            PlanEvent::ClickCell(row, column) => {
                self.plan.cursor = (row, column.min(PLAN_ROLLOVER_COLUMN));
                self.start_plan_edit();
            }
            PlanEvent::Type(c) => {
                if let Some(edit) = self.plan.edit.as_mut() {
                    edit.type_char(c);
                }
            }
            PlanEvent::Backspace => {
                if let Some(edit) = self.plan.edit.as_mut() {
                    edit.backspace();
                }
            }
            PlanEvent::Save { span, step } => self.commit_plan_edit(span, step),
            // The page dropped the cell before this reached the tab.
            PlanEvent::Abandon => {}
        }
    }

    pub(super) fn plan_move(&mut self, step: Step) {
        let rows = self.plan_data().map_or(0, |plan| plan.row_count());
        let selected = self.held_plan_cursor().0;
        self.plan.cursor.0 = lib_list_cursor::step(selected, rows, step);
    }

    /// The grid for the range in view.
    fn plan_data(&self) -> Option<Plan> {
        let budget = self.budget()?;
        let ports = self.ports.borrow();
        Some(plan(&budget, ports.reads(), self.plan.start, self.today()))
    }

    /// The cursor held inside the grid, so a row that vanished can't leave it dangling.
    fn held_plan_cursor(&self) -> (usize, usize) {
        let rows = self.plan_data().map_or(0, |plan| plan.row_count());
        (
            self.plan.cursor.0.min(rows.saturating_sub(1)),
            self.plan.cursor.1.min(PLAN_ROLLOVER_COLUMN),
        )
    }

    /// Row `index`'s Category.
    pub(super) fn plan_row_category(&self, index: usize) -> Option<CategoryId> {
        self.plan_data()?.row(index).map(|row| row.category_id)
    }

    /// The month Fill would target from the cursor: the first open month at or after its column.
    fn fill_month(&self) -> Period {
        let column = self.held_plan_cursor().1;
        let cursor = self
            .plan_data()
            .and_then(|plan| plan.months.get(column).copied());
        fill_target(cursor, self.today())
    }

    /// Steps the range a month, inside the Budget's bounds. Returns whether it moved.
    fn shift_plan_range(&mut self, forward: bool) -> bool {
        let Some(budget) = self.budget() else {
            return false;
        };
        let (earliest, latest) = plan_start_bounds(&budget, self.today());
        let target = if forward {
            self.plan.start.next()
        } else {
            self.plan.start.prev()
        };
        if target < earliest || target > latest {
            return false;
        }
        self.plan.start = target;
        true
    }

    fn cycle_rollover(&mut self, category: CategoryId) {
        let today = self.today();
        let _ = self.with_repo(|repo| {
            CycleRollover {
                id: self.current,
                category_id: category,
                today,
            }
            .run(repo)
        });
    }

    /// `enter`/`i`/a click on the cursor's cell: types into an open month, or cycles Rollover.
    pub(super) fn start_plan_edit(&mut self) {
        let Some(plan) = self.plan_data() else {
            return;
        };
        let (row_index, column) = self.held_plan_cursor();
        self.plan.cursor = (row_index, column);
        let Some(row) = plan.row(row_index) else {
            return;
        };
        if column == PLAN_ROLLOVER_COLUMN {
            self.cycle_rollover(row.category_id);
            return;
        }
        let Some(cell) = row.cells.get(column) else {
            return;
        };
        if cell.closed || self.editable().is_none() {
            return;
        }
        self.plan.edit = Some(PlanEdit::new(
            row.category_id,
            cell.month,
            cell.amount.as_ref(),
        ));
    }

    /// Saves the typed cell; `step` moves on to the next (1) or previous (-1) month's cell and
    /// types into it. Text that isn't an amount keeps the cell open.
    fn commit_plan_edit(&mut self, span: Span, step: i32) {
        let Some(edit) = self.plan.edit.take() else {
            return;
        };
        let saved = self.save_cell(edit.category_id, edit.month, span, &edit.text);
        if saved == Err(crate::Error::InvalidAmount) {
            self.plan.edit = Some(edit);
            return;
        }
        if step == 0 {
            return;
        }
        let column = self.plan.cursor.1;
        if step > 0 {
            if column + 1 < PLAN_MONTHS {
                self.plan.cursor.1 += 1;
            } else if !self.shift_plan_range(true) {
                return;
            }
        } else if column > 0 {
            self.plan.cursor.1 -= 1;
        } else if !self.shift_plan_range(false) {
            return;
        }
        self.start_plan_edit();
    }

    /// `x`/`backspace` clear the cursor's month cell and `0` writes an explicit 0.00 onward.
    fn write_plan_cell(&mut self, text: &str) {
        let Some(plan) = self.plan_data() else {
            return;
        };
        let (row_index, column) = self.held_plan_cursor();
        let Some(row) = plan.row(row_index) else {
            return;
        };
        let Some(cell) = row.cells.get(column).filter(|cell| !cell.closed) else {
            return;
        };
        let _ = self.save_cell(row.category_id, cell.month, Span::Onward, text);
    }

    fn save_cell(
        &mut self,
        category: CategoryId,
        month: Period,
        span: Span,
        text: &str,
    ) -> Result<bool, crate::Error> {
        let today = self.today();
        let mut ports = self.ports.borrow_mut();
        let ports = ports.split_mut();
        let categories: &dyn BudgetCategories = ports.reads;
        SaveCell {
            id: self.current,
            category_id: category,
            month,
            span,
            text: text.to_string(),
            today,
        }
        .run(categories, ports.budgets)
    }

    /// The tab's view model, the range's label, the Fill button's text and the status line's
    /// right-hand text.
    pub(super) fn plan_view(
        &self,
        budget: &Budget,
        reads: &dyn BudgetReads,
    ) -> (PlanViewModel, String, String, String) {
        let today = self.today();
        let current = Period::of(today);
        let grid = plan(budget, reads, self.plan.start, today);
        let rows = grid.row_count();
        let cursor = (
            self.plan.cursor.0.min(rows.saturating_sub(1)),
            self.plan.cursor.1.min(PLAN_ROLLOVER_COLUMN),
        );
        let range = match (grid.months.first(), grid.months.last()) {
            (Some(first), Some(last)) => range_label(*first, *last),
            _ => String::new(),
        };
        let fill = msg::budgets_fill_button(&format_month(
            fill_target(grid.months.get(cursor.1).copied(), today).month,
        ));
        let status = msg::budgets_status_plan(&range, count(rows));
        let mut index = 0;
        let sections = grid
            .sections
            .iter()
            .map(|section| PlanSectionView {
                label: section
                    .parent
                    .clone()
                    .unwrap_or_else(msg::budgets_plan_other),
                rows: section
                    .rows
                    .iter()
                    .map(|row| {
                        let at = index;
                        index += 1;
                        PlanRowView {
                            index: at,
                            name: row.name.clone(),
                            cursor: cursor.0 == at,
                            cells: row
                                .cells
                                .iter()
                                .enumerate()
                                .map(|(column, cell)| {
                                    let editing = self.plan.edit.as_ref().filter(|edit| {
                                        edit.category_id == row.category_id
                                            && edit.month == cell.month
                                    });
                                    PlanCellView {
                                        text: editing.map_or_else(
                                            || {
                                                cell.amount
                                                    .as_ref()
                                                    .map_or_else(|| EMPTY_CELL.to_string(), money)
                                            },
                                            |edit| edit.text.clone(),
                                        ),
                                        own_record: cell.own_record,
                                        closed: cell.closed,
                                        cursor: cursor == (at, column),
                                        editing: editing.is_some(),
                                    }
                                })
                                .collect(),
                            rollover: PlanCellView {
                                text: row.rollover.map_or_else(String::new, rollover_label),
                                own_record: false,
                                closed: false,
                                cursor: cursor == (at, PLAN_ROLLOVER_COLUMN),
                                editing: false,
                            },
                        }
                    })
                    .collect(),
            })
            .collect();
        let view = PlanViewModel {
            category_column: lib_locale::msg::column_category(),
            months: grid
                .months
                .iter()
                .map(|month| {
                    let now = *month == current;
                    let label = format_month(month.month);
                    PlanMonthView {
                        label: if now {
                            msg::budgets_plan_month_now(&label)
                        } else {
                            label
                        },
                        now,
                    }
                })
                .collect(),
            rollover_column: msg::budgets_plan_column_rollover(),
            sections,
            total: PlanFooterView {
                label: msg::budgets_plan_total(),
                figures: grid
                    .totals
                    .iter()
                    .map(|total| (money(total), false))
                    .collect(),
            },
            unallocated: PlanFooterView {
                label: msg::budgets_plan_unallocated(&money(&grid.average_income)),
                figures: grid
                    .unallocated
                    .iter()
                    .map(|left| (money(left), left.0.is_negative()))
                    .collect(),
            },
            empty: (rows == 0).then(msg::budgets_plan_empty),
        };
        (view, range, fill, status)
    }
}
