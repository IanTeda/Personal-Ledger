//! What the integration tests read back of the Dashboard and the Toast layer and its history, kept
//! apart from `shell.rs` so the snapshot shapes and their accessors sit together.

use crate::app::Shell;
use crate::bills;

/// One Toast as the stack or the history shows it.
#[doc(hidden)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToastSnapshot {
    /// `Info`, `Success`, `Warning` or `Error`.
    pub kind: String,
    pub text: String,
    pub count: u32,
}

/// The Toast layer: the visible stack (oldest first), what it holds back, and the history.
#[doc(hidden)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToastsSnapshot {
    pub visible: Vec<ToastSnapshot>,
    pub more: usize,
    /// The session history, newest first.
    pub history: Vec<ToastSnapshot>,
    pub history_open: bool,
    /// The Toasts Preference.
    pub on: bool,
}

/// A Dashboard Needs Attention Bill row: its Plan and the `debug_selector` of its click target.
#[doc(hidden)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DashboardBillSnapshot {
    pub plan: String,
    pub selector: String,
}

impl Shell {
    /// The Toast layer for a test.
    #[doc(hidden)]
    pub fn toasts_snapshot(&self) -> ToastsSnapshot {
        ToastsSnapshot {
            visible: self
                .chrome
                .toasts
                .visible()
                .iter()
                .map(|toast| ToastSnapshot {
                    kind: format!("{:?}", toast.kind()),
                    text: toast.text().to_string(),
                    count: toast.count(),
                })
                .collect(),
            more: self.chrome.toasts.more_count(),
            history: self
                .chrome
                .toasts
                .history()
                .iter()
                .map(|entry| ToastSnapshot {
                    kind: format!("{:?}", entry.kind()),
                    text: entry.text().to_string(),
                    count: entry.count(),
                })
                .collect(),
            history_open: self.toast_history_open(),
            on: self.chrome.toasts.display().toasts_on,
        }
    }

    /// The Dashboard's Needs Attention Bill rows, in the order they draw.
    #[doc(hidden)]
    pub fn dashboard_bills_snapshot(&self, cx: &gpui::App) -> Vec<DashboardBillSnapshot> {
        let ledger = self.ledger.read(cx).ledger();
        bills::attention_entries(ledger.bill_plans(), ledger.bill_entries(), self.today)
            .into_iter()
            .filter_map(|id| {
                let plan = bills::get(ledger.bill_plans(), id.plan_id)?;
                Some(DashboardBillSnapshot {
                    plan: plan.name.clone(),
                    selector: format!("dashboard-bill-{}-{}", id.plan_id, id.due),
                })
            })
            .collect()
    }
}
