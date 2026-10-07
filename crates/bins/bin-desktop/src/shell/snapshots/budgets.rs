//! What the integration tests read back of the Budgets page, kept apart from `shell.rs` so the
//! snapshot shape and its accessor sit together.

use crate::budgets::{self, BudgetsDialog};
use crate::shell::Shell;

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
    pub stop_category: Option<u32>,
}

impl Shell {
    /// The Budgets page for a test, built the way the render builds its rows.
    #[doc(hidden)]
    pub fn budgets_snapshot(&self) -> BudgetsSnapshot {
        let (budget, rows) = self
            .budgets_figures()
            .map(|(budget, figures)| {
                let rows = figures
                    .rows
                    .iter()
                    .map(|row| BudgetRowSnapshot {
                        category: self
                            .categories
                            .iter()
                            .find(|category| category.id == row.category_id)
                            .map(|category| category.name.clone())
                            .unwrap_or_default(),
                        is_parent: row.is_parent,
                        budgeted: row.budget.is_some(),
                    })
                    .collect::<Vec<_>>();
                (budget.name.clone(), rows)
            })
            .unwrap_or_default();
        let switcher = match self.budgets_dialog() {
            Some(BudgetsDialog::Switcher(switcher)) => Some(switcher),
            _ => None,
        };
        BudgetsSnapshot {
            tab: format!("{:?}", self.budgets_tab),
            budget,
            period: (self.budgets_period.year, self.budgets_period.month),
            selected: self.budgets_selected.min(rows.len().saturating_sub(1)),
            rows,
            plan_start: (self.budgets_plan_start.year, self.budgets_plan_start.month),
            plan_cursor: self.budgets_plan_cursor,
            plan_edit: self
                .budgets_plan_edit
                .as_ref()
                .map(|edit| edit.text.clone()),
            history_end: (
                self.budgets_history_end.year,
                self.budgets_history_end.month,
            ),
            dialog: self.budgets_dialog().map(|dialog| {
                match dialog {
                    BudgetsDialog::Switcher(_) => "Switcher",
                    BudgetsDialog::Budget(_) => "Budget",
                    BudgetsDialog::Manage(_) => "Manage",
                    BudgetsDialog::CategoryDetail(_) => "Detail",
                    BudgetsDialog::EditLimit(_) => "EditLimit",
                    BudgetsDialog::Fill { .. } => "Fill",
                    BudgetsDialog::Stop(_) => "Stop",
                }
                .to_string()
            }),
            limit_amount: match self.budgets_dialog() {
                Some(BudgetsDialog::EditLimit(form)) => Some(form.amount.text().to_string()),
                _ => None,
            },
            switcher_names: switcher.map(|switcher| {
                budgets::switcher_ids(&self.budgets, switcher.query.text())
                    .into_iter()
                    .filter_map(|id| self.budgets.get(id).map(|budget| budget.name.clone()))
                    .collect()
            }),
            switcher_selected: switcher.map(|switcher| switcher.selected),
            budget_name: match self.budgets_dialog() {
                Some(BudgetsDialog::Budget(form)) => Some(form.name.text().to_string()),
                _ => None,
            },
            fill_source: match self.budgets_dialog() {
                Some(BudgetsDialog::Fill { source, .. }) => Some(format!("{source:?}")),
                _ => None,
            },
            stop_category: match self.budgets_dialog() {
                Some(BudgetsDialog::Stop(form)) => Some(form.category_id),
                _ => None,
            },
        }
    }
}
