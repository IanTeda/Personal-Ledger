//! What the integration tests read back of the Dashboard, the Toast layer and its history, and the
//! Import step, kept apart from `shell.rs` so the snapshot shapes and their accessors sit together.

use crate::bills;
use crate::shell::Shell;

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

/// One Import row: the raw statement text and its status.
#[doc(hidden)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportRowSnapshot {
    pub raw: String,
    /// `RuleMatch`, `NewPayee`, `Matched` or `NeedsReview`.
    pub status: String,
}

/// The Import step's state.
#[doc(hidden)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportSnapshot {
    pub rows: Vec<ImportRowSnapshot>,
    pub selected: usize,
    pub remember: bool,
    /// `Payee` or `Category` while a row's select is open.
    pub open_select: Option<String>,
    pub can_continue: bool,
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

    /// The Import step for a test; `None` while it is not showing.
    #[doc(hidden)]
    pub fn import_snapshot(&self) -> Option<ImportSnapshot> {
        let state = self.import.as_ref()?;
        Some(ImportSnapshot {
            rows: state
                .rows
                .iter()
                .map(|row| ImportRowSnapshot {
                    raw: row.raw.clone(),
                    status: format!("{:?}", row.status()),
                })
                .collect(),
            selected: state.selected,
            remember: state.remember,
            open_select: state
                .open_select
                .as_ref()
                .map(|(_, select, _)| format!("{select:?}")),
            can_continue: state.can_continue(),
        })
    }

    /// The Dashboard's Needs Attention Bill rows, in the order they draw.
    #[doc(hidden)]
    pub fn dashboard_bills_snapshot(&self, cx: &gpui::App) -> Vec<DashboardBillSnapshot> {
        let store = self.bills_store.read(cx);
        bills::attention_entries(store.plans(), store.entries(), self.today)
            .into_iter()
            .filter_map(|id| {
                let plan = bills::get(store.plans(), id.plan_id)?;
                Some(DashboardBillSnapshot {
                    plan: plan.name.clone(),
                    selector: format!("dashboard-bill-{}-{}", id.plan_id, id.due),
                })
            })
            .collect()
    }
}
