//! What the integration tests read back of the Dashboard and the Toast layer and its history, kept
//! apart from `shell.rs` so the snapshot shapes and their accessors sit together.

use crate::dashboard::DashboardBillSnapshot;
use crate::shell::router::Root;

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

impl Root {
    /// The Toast layer for a test.
    #[doc(hidden)]
    pub fn toasts_snapshot(&self) -> ToastsSnapshot {
        ToastsSnapshot {
            visible: self
                .presenter
                .toasts()
                .visible()
                .iter()
                .map(|toast| ToastSnapshot {
                    kind: format!("{:?}", toast.kind()),
                    text: toast.text().to_string(),
                    count: toast.count(),
                })
                .collect(),
            more: self.presenter.toasts().more_count(),
            history: self
                .presenter
                .toasts()
                .history()
                .iter()
                .map(|entry| ToastSnapshot {
                    kind: format!("{:?}", entry.kind()),
                    text: entry.text().to_string(),
                    count: entry.count(),
                })
                .collect(),
            history_open: self.toast_history_open(),
            on: self.presenter.toasts().display().toasts_on,
        }
    }

    /// The Dashboard's Needs Attention Bill rows, in the order they draw.
    #[doc(hidden)]
    pub fn dashboard_bills_snapshot(&self, cx: &gpui::App) -> Vec<DashboardBillSnapshot> {
        self.open().views.dashboard.read(cx).bills_snapshot()
    }

    /// How many times the Dashboard has asked to redraw from now on, for a test that checks a
    /// write elsewhere reaches it.
    #[doc(hidden)]
    pub fn count_dashboard_redraws(
        &self,
        cx: &mut gpui::App,
    ) -> std::rc::Rc<std::cell::Cell<usize>> {
        let count = std::rc::Rc::new(std::cell::Cell::new(0));
        let seen = count.clone();
        cx.observe(&self.open().views.dashboard, move |_, _| {
            seen.set(seen.get() + 1);
        })
        .detach();
        count
    }
}
