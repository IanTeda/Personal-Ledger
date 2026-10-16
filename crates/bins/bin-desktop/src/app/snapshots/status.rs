//! What the integration tests read back of the status line, kept apart from `status.rs` so the
//! snapshot shape and its accessor sit together.

use crate::app::Shell;

/// The status line's per-page content: the key legend and the right-hand text.
#[doc(hidden)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusSnapshot {
    /// `(key, action)` pairs, in display order.
    pub hints: Vec<(String, String)>,
    pub right: String,
}

impl Shell {
    /// The status line's page content for a test; `None` when the View on show has none.
    #[doc(hidden)]
    pub fn status_snapshot(&self, cx: &gpui::App) -> Option<StatusSnapshot> {
        self.page_status(cx).map(|status| StatusSnapshot {
            hints: status
                .hints
                .into_iter()
                .map(|(key, action)| (key.to_string(), action))
                .collect(),
            right: status.right,
        })
    }
}
