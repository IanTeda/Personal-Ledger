//! What the integration tests read back of the Budgets page (ADR-0030), built from the page
//! Presenter's state and the figures its ports read. The open Dialog's fields are the Dialog
//! host's, so `Root`'s accessor fills them in.

use lib_budgets::{BudgetReads, period_figures};
use lib_core::CategoryId;

use super::page::BudgetsPage;
use crate::budgets::legacy::Legacy;

/// One Progress row: the Category it shows and whether it carries a Budget Amount.
#[doc(hidden)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BudgetRowSnapshot {
    pub category: String,
    pub is_parent: bool,
    pub budgeted: bool,
}

/// The Budgets page's state: plain values, so the private budget types stay private.
#[doc(hidden)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BudgetsSnapshot {
    /// `Progress`, `Plan` or `History`.
    pub tab: String,
    /// The Budget on show.
    pub budget: String,
    pub period: (i32, u32),
    /// The selected Progress row.
    pub selected: usize,
    pub rows: Vec<BudgetRowSnapshot>,
    /// The Plan's first month, and its cursor as (row, column).
    pub plan_start: (i32, u32),
    pub plan_cursor: (usize, usize),
    /// The Plan cell being typed into, as its draft text.
    pub plan_edit: Option<String>,
    pub history_end: (i32, u32),
    /// `Switcher`, `Budget`, `Manage`, `Detail`, `EditLimit`, `Fill` or `Stop`.
    pub dialog: Option<String>,
    /// The open Edit budget form's Amount draft.
    pub limit_amount: Option<String>,
    /// The Budget names the Switcher lists for its query.
    pub switcher_names: Option<Vec<String>>,
    /// The Switcher's highlighted row.
    pub switcher_selected: Option<usize>,
    /// The open New budget form's Name draft.
    pub budget_name: Option<String>,
    /// The Fill dialog's chosen source, `PreviousMonth` or `Average`.
    pub fill_source: Option<String>,
    /// The Category the open Stop dialog is for.
    pub stop_category: Option<CategoryId>,
    /// The status line's right-hand text for the tab shown.
    pub status: String,
}

impl BudgetsPage {
    /// The page's part of the snapshot. Every Dialog field is left `None` for `Root` to fill.
    #[doc(hidden)]
    pub fn snapshot(&self) -> BudgetsSnapshot {
        let presenter = self.presenter();
        let legacy = Legacy {
            ports: self.ports(),
            today: self.today(),
        };
        let reads = self.ports().reads();
        let reads: &dyn BudgetReads = &*reads;
        let categories = reads.budget_categories();
        let (budget, rows) = legacy
            .budget(presenter.current())
            .map(|budget| {
                let figures = period_figures(&budget, reads, presenter.period(), self.today());
                let rows = figures
                    .rows
                    .iter()
                    .map(|row| BudgetRowSnapshot {
                        category: categories
                            .iter()
                            .find(|category| category.id == row.category_id)
                            .map(|category| category.name.clone())
                            .unwrap_or_default(),
                        is_parent: row.is_parent,
                        budgeted: row.budget.is_some(),
                    })
                    .collect::<Vec<_>>();
                (budget.name, rows)
            })
            .unwrap_or_default();
        let (period, plan_start, history_end) = (
            presenter.period(),
            presenter.plan_start(),
            presenter.history_end(),
        );
        BudgetsSnapshot {
            tab: format!("{:?}", presenter.tab()),
            budget,
            period: (period.year, period.month),
            selected: presenter.selected().min(rows.len().saturating_sub(1)),
            rows,
            plan_start: (plan_start.year, plan_start.month),
            plan_cursor: presenter.plan_cursor(),
            plan_edit: presenter.plan_draft().map(str::to_string),
            history_end: (history_end.year, history_end.month),
            dialog: None,
            limit_amount: None,
            switcher_names: None,
            switcher_selected: None,
            budget_name: None,
            fill_source: None,
            stop_category: None,
            status: self.view_model().status,
        }
    }
}
