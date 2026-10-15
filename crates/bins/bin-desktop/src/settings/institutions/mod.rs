//! The **Institutions** Settings page (ADR-0037, and its amendment's one folder per page): the
//! Institutions table. [`InstitutionsPage`] reads it straight from [`LedgerStore`], renders itself
//! (`render.rs`), builds the Add institution Dialog (`dialogs/`) and reaches `Shell` only through
//! [`ViewEvent`]; `Shell`'s Dialog host draws it and confirms it through `apply.rs`. The Settings
//! View forwards the page's keys and status to it while the page has focus.

pub mod apply;
pub mod dialogs;
pub mod form;
mod render;

use gpui::{App, Context, Entity, EventEmitter, Keystroke, Subscription};
use lib_institutions::InstitutionRow;
use lib_ledger::Domain;

use crate::{
    chrome::{dialog_host::OpenDialog, statusline::PageStatus},
    ledger_store::{LedgerStore, notify_on},
    navigation::key_router::Movement,
    settings::{SettingsPage, hints::settings_plain_page_hints},
    view::event::ViewEvent,
};
use dialogs::InstitutionsDialog;
use form::AddInstitutionForm;

impl EventEmitter<ViewEvent> for InstitutionsPage {}

/// The Institutions page. It has no selection of its own: every row is reached by its buttons.
pub struct InstitutionsPage {
    ledger: Entity<LedgerStore>,
    _institutions_changed: Subscription,
}

impl InstitutionsPage {
    pub fn new(ledger: Entity<LedgerStore>, cx: &mut Context<'_, Self>) -> Self {
        let _institutions_changed = notify_on(&ledger, Domain::Institutions, cx);
        Self {
            ledger,
            _institutions_changed,
        }
    }

    fn institutions<'a>(&self, cx: &'a App) -> &'a [InstitutionRow] {
        self.ledger.read(cx).ledger().institutions()
    }

    // -----------------------------------------------------------------------------------------
    // The interface the Settings View forwards to while the page has focus (ADR-0037)
    // -----------------------------------------------------------------------------------------

    /// The page has no keys of its own: its Dialog opens from its button.
    pub fn key_ahead(&mut self, _keystroke: &Keystroke, _cx: &mut Context<'_, Self>) -> bool {
        false
    }

    pub fn handle_key(&mut self, _keystroke: &Keystroke, _cx: &mut Context<'_, Self>) -> bool {
        false
    }

    /// With no selection to move, the page takes no movement and the Settings View scrolls its
    /// body instead.
    pub fn apply_movement(&mut self, _movement: Movement, _cx: &mut Context<'_, Self>) -> bool {
        false
    }

    /// The page has nothing of its own to close: `Esc` is left to Settings, which steps back to
    /// the index.
    pub fn escape(&mut self, _cx: &mut Context<'_, Self>) -> bool {
        false
    }

    /// The page has no Search: `/` is inert there.
    pub fn start_search(&mut self, _cx: &mut Context<'_, Self>) -> bool {
        false
    }

    pub fn search_key(&mut self, _keystroke: &Keystroke, _cx: &mut Context<'_, Self>) -> bool {
        false
    }

    /// The plain page legend and the page's fixed note.
    pub fn page_status(&self, _cx: &App) -> Option<PageStatus> {
        Some(PageStatus {
            hints: settings_plain_page_hints(),
            right: SettingsPage::Institutions.scope_note(),
        })
    }

    /// The heading's meta: how many Institutions the table holds.
    pub fn scope_note(&self, cx: &App) -> String {
        let count = i64::try_from(self.institutions(cx).len()).unwrap_or(i64::MAX);
        crate::msg::desktop_settings_scope_institutions(count)
    }

    // -----------------------------------------------------------------------------------------
    // Clicks
    // -----------------------------------------------------------------------------------------

    /// **+ Add institution** opens the Add institution dialog, its Default unit seeded from the
    /// Units table's first row.
    fn click_add(&mut self, cx: &mut Context<'_, Self>) {
        let form = AddInstitutionForm::new(self.ledger.read(cx).ledger().units());
        open_dialog(InstitutionsDialog::Add(form), cx);
    }

    /// Institutions' edit and delete have no Dialog yet, so they say so on the status line.
    fn not_built(&mut self, message: String, cx: &mut Context<'_, Self>) {
        cx.emit(ViewEvent::SetStatus(Some(message)));
    }

    // -----------------------------------------------------------------------------------------
    // Tests
    // -----------------------------------------------------------------------------------------

    /// The Institutions rows. The open Dialog's fields are `Shell`'s, which fills them in.
    #[doc(hidden)]
    pub fn snapshot(&self, cx: &App) -> SettingsInstitutionsSnapshot {
        SettingsInstitutionsSnapshot {
            institutions: self
                .institutions(cx)
                .iter()
                .map(|row| (row.name.clone(), row.account_type.clone()))
                .collect(),
            institution_dialog_types: None,
            institution_dialog_unit: None,
        }
    }
}

/// The Institutions page: `(name, account type)` rows in table order, and the Add institution
/// dialog's checked products (`InstitutionProduct` variant names) and chosen Default unit code.
#[doc(hidden)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingsInstitutionsSnapshot {
    pub institutions: Vec<(String, String)>,
    pub institution_dialog_types: Option<Vec<String>>,
    pub institution_dialog_unit: Option<String>,
}

/// What the open Institutions Dialog shows, for the Settings snapshots.
#[doc(hidden)]
pub struct InstitutionsDialogSnapshot {
    /// `AddInstitution`.
    pub label: &'static str,
    /// The Name draft.
    pub name: String,
    /// The checked products (`InstitutionProduct` variant names) and the chosen Default unit code.
    pub form: (Vec<String>, Option<String>),
}

impl InstitutionsDialog {
    #[doc(hidden)]
    pub fn snapshot(&self) -> InstitutionsDialogSnapshot {
        let Self::Add(form) = self;
        InstitutionsDialogSnapshot {
            label: "AddInstitution",
            name: form.name.text().to_string(),
            form: (
                form.account_types
                    .iter()
                    .map(|product| format!("{product:?}"))
                    .collect(),
                form.default_unit_code.clone(),
            ),
        }
    }
}

/// Asks `Shell`'s Dialog host to open an Institutions Dialog.
fn open_dialog(dialog: InstitutionsDialog, cx: &mut Context<'_, InstitutionsPage>) {
    cx.emit(ViewEvent::OpenDialog(Box::new(OpenDialog::Institutions(
        dialog,
    ))));
}
