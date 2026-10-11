//! What the integration tests read back of the Bills page (ADR-0030), built by the Bills Entity
//! from its own state (ADR-0037). The open dialog's fields are the Dialog host's, so `Shell`'s
//! accessor fills them in.

use gpui::App;

use super::state::BillsView;
use crate::bills::{self, BillsTab};

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

impl BillsView {
    /// The page for a test, built the way the render builds its rows. The dialog fields are left
    /// empty for `Shell` to fill.
    #[doc(hidden)]
    pub fn snapshot(&self, cx: &App) -> BillsSnapshot {
        let state = &self.state;
        let plans = self.sources.bills.read(cx).plans(cx);
        let rows = self.schedule_rows(cx);
        let len = match state.tab {
            BillsTab::Schedule => rows.len(),
            BillsTab::Planner => plans.len(),
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
            plans: bills::planner_order(plans)
                .iter()
                .map(|plan| plan.name.clone())
                .collect(),
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
            dialog: None,
            plan_name: None,
            pay_mode: None,
            pay_candidates: None,
        }
    }
}
