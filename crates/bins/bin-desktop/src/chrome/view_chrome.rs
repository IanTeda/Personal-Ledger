//! The View on show, as Chrome reads it: its status-line content. Typed data built from the
//! View's own Messages (ADR-0032), so Chrome renders it and holds no destination-specific text.
//! The top bar takes nothing from the View: its wordmark stands alone, as the View's own heading
//! names the screen.

use super::statusline::PageStatus;

#[derive(Debug, Clone)]
pub struct ViewChrome {
    /// The page's status-line content, or `None` when the View has none.
    pub status: Option<PageStatus>,
}
