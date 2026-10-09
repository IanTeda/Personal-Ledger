//! The View on show, as Chrome reads it: its title for the top bar and its status-line content.
//! Both are typed data built from the View's own Messages (ADR-0032), so Chrome renders them and
//! holds no destination-specific text.

use super::statusline::PageStatus;

#[derive(Debug, Clone)]
pub struct ViewChrome {
    /// The active View's title, from `ActiveView::title`.
    pub title: String,
    /// The page's status-line content, or `None` when the View has none.
    pub status: Option<PageStatus>,
}
