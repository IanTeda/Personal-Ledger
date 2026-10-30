//! What the integration tests read back of the Import View (ADR-0030), built by its Entity from
//! its own state (ADR-0037).

use super::state::ImportView;

/// One Import row: the raw statement text and its status.
#[doc(hidden)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportRowSnapshot {
    pub raw: String,
    /// `RuleMatch`, `NewPayee`, `Matched` or `NeedsReview`.
    pub status: String,
}

/// The Import View's state.
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

impl ImportView {
    /// The Import View for a test; `None` while it is not showing.
    #[doc(hidden)]
    pub fn snapshot(&self) -> Option<ImportSnapshot> {
        let state = self.state.as_ref()?;
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
}
