//! The **Institutions** Settings page (ADR-0037, and its amendment's one folder per page): the
//! Institutions table. [`InstitutionsPage`] reads it straight from [`LedgerStore`], renders itself
//! (`render.rs`), builds the Add institution Dialog (`dialogs/`) and reaches `Root` only through
//! [`ViewEvent`]. The Dialog host holds it, and it draws, reads back and confirms itself
//! (`dialogs/`, `apply.rs`). The Settings View forwards the page's keys and status to it while the page has focus.

pub mod apply;
pub mod dialogs;
pub mod form;
mod render;

use crate::settings::pages::Page;
use gpui::{AnyElement, IntoElement as _};
use gpui::{App, Context, Entity, EventEmitter, Subscription};
use lib_institutions::InstitutionRow;
use lib_ledger::Domain;

use crate::{
    ledger_store::{LedgerStore, notify_on},
    settings::SettingsPage,
    shell::view::dialog_host::open_dialog,
    view::event::ViewEvent,
};
use apply::InstitutionsOutcome;
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

    /// The heading's meta: how many Institutions the table holds.
    pub(crate) fn scope_note(&self, cx: &App) -> String {
        let count = i64::try_from(self.institutions(cx).len()).unwrap_or(i64::MAX);
        crate::msg::desktop_settings_scope_institutions(count)
    }
}

impl Page for InstitutionsPage {
    const PAGE: SettingsPage = SettingsPage::Institutions;

    fn heading_meta(&self, cx: &App) -> AnyElement {
        self.scope_note(cx).into_any_element()
    }
}

impl InstitutionsPage {
    // -----------------------------------------------------------------------------------------
    // Clicks
    // -----------------------------------------------------------------------------------------

    /// **+ Add institution** opens the Add institution dialog, its Default unit seeded from the
    /// Units table's first row.
    fn click_add(&mut self, cx: &mut Context<'_, Self>) {
        let form = AddInstitutionForm::new(self.ledger.read(cx).ledger().units());
        open_dialog(InstitutionsDialog::Add(form), Self::land, cx);
    }

    /// A confirmed Add institution needs no Toast and the table repaints from the Ledger, so there
    /// is nothing to land.
    fn land(&mut self, _outcome: InstitutionsOutcome, _cx: &mut Context<'_, Self>) {}

    /// Institutions' edit and delete have no Dialog yet, so they say so on the status line.
    fn not_built(&mut self, message: String, cx: &mut Context<'_, Self>) {
        cx.emit(ViewEvent::SetStatus(Some(message)));
    }

    // -----------------------------------------------------------------------------------------
    // Tests
    // -----------------------------------------------------------------------------------------

    /// The Institutions rows. The open Dialog reads itself back (`Dialog::readback`).
    #[doc(hidden)]
    pub fn snapshot(&self, cx: &App) -> SettingsInstitutionsSnapshot {
        SettingsInstitutionsSnapshot {
            institutions: self
                .institutions(cx)
                .iter()
                .map(|row| (row.name.clone(), row.account_type.clone()))
                .collect(),
        }
    }
}

/// The Institutions page: `(name, account type)` rows in table order.
#[doc(hidden)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingsInstitutionsSnapshot {
    pub institutions: Vec<(String, String)>,
}
