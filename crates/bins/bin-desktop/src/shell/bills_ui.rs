//! What the integration tests read back of the Bills page, kept apart from `shell.rs` so the
//! snapshot shape and its accessor sit together.

use super::Shell;
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
    pub fn bills_snapshot(&self) -> BillsSnapshot {
        let rows = self.bills_schedule_rows();
        let plan_rows = bills::planner_order(&self.bill_plans);
        let len = match self.bills_tab {
            bills::BillsTab::Schedule => rows.len(),
            bills::BillsTab::Planner => self.bill_plans.len(),
        };
        BillsSnapshot {
            tab: format!("{:?}", self.bills_tab),
            selected: self.bills_selected.min(len.saturating_sub(1)),
            period: (self.bills_period.year, self.bills_period.month),
            all: self.bills_all,
            rows: rows
                .iter()
                .map(|row| BillRowSnapshot {
                    plan: bills::get(&self.bill_plans, row.id.plan_id)
                        .map(|plan| plan.name.clone())
                        .unwrap_or_default(),
                    status: format!("{:?}", row.status),
                    actionable: row.is_actionable(),
                })
                .collect(),
            plans: plan_rows.iter().map(|plan| plan.name.clone()).collect(),
            statuses_on: self
                .bills_filters
                .statuses
                .iter()
                .map(|status| format!("{status:?}"))
                .collect(),
            filter_focus: self
                .bills_filter_focus
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
