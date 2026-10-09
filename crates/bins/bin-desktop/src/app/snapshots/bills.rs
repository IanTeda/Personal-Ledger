//! What the integration tests read back of the Bills page, kept apart from `shell.rs` so the
//! snapshot shape and its accessor sit together.

use crate::app::Shell;
use crate::bills::{self, BillsDialog};

/// One Schedule row: the Plan it belongs to and the status it shows.
#[doc(hidden)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BillRowSnapshot {
    pub plan: String,
    pub status: String,
    pub actionable: bool,
}

/// The Bills page's state: plain values, so the private bill types stay private.
#[doc(hidden)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BillsSnapshot {
    /// `Schedule` or `Planner`.
    pub tab: String,
    /// The selected position in the active tab's rows, already clamped.
    pub selected: usize,
    pub period: (i32, u32),
    pub all: bool,
    /// The Schedule rows the period and filters leave visible, in table order.
    pub rows: Vec<BillRowSnapshot>,
    /// Every Bill Plan name in Planner order.
    pub plans: Vec<String>,
    /// The status chips switched on, e.g. `Paid`.
    pub statuses_on: Vec<String>,
    /// The focused Schedule filter select, e.g. `Plan`; `None` while none is.
    pub filter_focus: Option<String>,
    /// `Add`, `Edit`, `Pay` or `Skip`; `None` while no dialog is open.
    pub dialog: Option<String>,
    /// The open Add or Edit dialog's Name draft.
    pub plan_name: Option<String>,
    /// The open Pay dialog's panel, `Match` or `Pay`.
    pub pay_mode: Option<String>,
    /// How many Transactions the open Pay dialog offers to match.
    pub pay_candidates: Option<usize>,
}

impl Shell {
    /// The Bills page for a test, built the way the render builds its rows.
    #[doc(hidden)]
    pub fn bills_snapshot(&self, cx: &gpui::App) -> BillsSnapshot {
        let view = self.bills_view.read(cx);
        let state = view.state();
        let plans = self.bills_store.read(cx).plans();
        let rows = view.schedule_rows(cx);
        let plan_rows = bills::planner_order(plans);
        let len = match state.tab {
            bills::BillsTab::Schedule => rows.len(),
            bills::BillsTab::Planner => plans.len(),
        };
        BillsSnapshot {
            tab: format!("{:?}", state.tab),
            selected: state.selected.min(len.saturating_sub(1)),
            period: (state.period.year, state.period.month),
            all: state.all,
            rows: rows
                .iter()
                .map(|row| BillRowSnapshot {
                    plan: bills::get(plans, row.id.plan_id)
                        .map(|plan| plan.name.clone())
                        .unwrap_or_default(),
                    status: format!("{:?}", row.status),
                    actionable: row.is_actionable(),
                })
                .collect(),
            plans: plan_rows.iter().map(|plan| plan.name.clone()).collect(),
            statuses_on: state
                .filters
                .statuses
                .iter()
                .map(|status| format!("{status:?}"))
                .collect(),
            filter_focus: state
                .filter_focus
                .as_ref()
                .map(|(field, _)| format!("{field:?}")),
            dialog: self.bills_dialog().map(|dialog| {
                match dialog {
                    BillsDialog::Add(_) => "Add",
                    BillsDialog::Edit(..) => "Edit",
                    BillsDialog::Pay(_) => "Pay",
                    BillsDialog::Skip(_) => "Skip",
                }
                .to_string()
            }),
            plan_name: match self.bills_dialog() {
                Some(BillsDialog::Add(form) | BillsDialog::Edit(_, form)) => {
                    Some(form.name.text().to_string())
                }
                _ => None,
            },
            pay_mode: match self.bills_dialog() {
                Some(BillsDialog::Pay(form)) => Some(format!("{:?}", form.mode)),
                _ => None,
            },
            pay_candidates: match self.bills_dialog() {
                Some(BillsDialog::Pay(form)) => Some(form.candidates.len()),
                _ => None,
            },
        }
    }
}
