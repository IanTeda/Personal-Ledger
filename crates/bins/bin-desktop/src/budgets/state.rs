//! The Budgets destination's own view state: which Budget, tab, month and cell it is showing.
//! The Budgets themselves stay on `Shell` (Categories and the Dashboard read them too).

use chrono::NaiveDate;

use super::{BudgetsTab, PlanEdit, default_plan_start};
use crate::period::Period;

#[derive(Debug)]
pub struct BudgetsState {
    /// The Budget the Budgets surface shows. The last one opened is a Client-scoped Preference, so
    /// this starts on the default Budget; the stub doesn't persist it.
    pub current: u32,
    pub tab: BudgetsTab,
    /// The Progress tab's calendar month; starts at today's.
    pub period: Period,
    /// The selected Progress row (a position in `PeriodFigures::rows`).
    pub selected: usize,
    /// The Plan grid's first month; it shows six from here.
    pub plan_start: Period,
    /// The Plan grid's cursor: a row across sections, and a column (the last is ROLLOVER).
    pub plan_cursor: (usize, usize),
    /// The Plan cell being typed into; `InputMode::Insert` is on for exactly as long as this is set.
    pub plan_edit: Option<PlanEdit>,
    /// The last month of the History tab's range (9c).
    pub history_end: Period,
    /// The History cursor: a row and a month column.
    pub history_cursor: (usize, usize),
    /// `x` on the History tab asked for the export; the key-down listener runs it, since the save
    /// dialog needs the `Context` a key handler doesn't have.
    pub export_pending: bool,
}

impl BudgetsState {
    pub fn new(current: u32, today: NaiveDate) -> Self {
        Self {
            current,
            tab: BudgetsTab::default(),
            period: Period::of(today),
            selected: 0,
            plan_start: default_plan_start(today),
            plan_cursor: (0, 0),
            plan_edit: None,
            history_end: Period::of(today),
            history_cursor: (0, 0),
            export_pending: false,
        }
    }
}
