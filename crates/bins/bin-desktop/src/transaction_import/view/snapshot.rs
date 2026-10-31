//! What the integration tests read back of the Import step (ADR-0030), built by its Entity from
//! the Presenter's view model (ADR-0037).

use super::page::ImportPage;

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

impl ImportPage {
    /// The Import step for a test; `None` while it is not showing.
    #[doc(hidden)]
    pub fn snapshot(&self) -> Option<ImportSnapshot> {
        let view = self.view_model()?;
        Some(ImportSnapshot {
            rows: view
                .rows
                .iter()
                .map(|row| ImportRowSnapshot {
                    raw: row.raw.clone(),
                    status: format!("{:?}", row.status),
                })
                .collect(),
            selected: view.selected,
            remember: view.remember,
            open_select: view.open_select.map(|open| format!("{:?}", open.which)),
            can_continue: view.can_continue,
        })
    }
}
