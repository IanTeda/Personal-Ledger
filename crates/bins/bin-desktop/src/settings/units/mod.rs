//! The **Units** Settings page (ADR-0037, and its amendment's one folder per page): the Units
//! table and its Price Sources. [`UnitsPage`] reads both straight from [`LedgerStore`], renders
//! itself (`render.rs`), builds the Add, Edit and Delete unit Dialogs (`dialogs/`) and reaches
//! `Shell` only through [`ViewEvent`]; `Shell`'s Dialog host draws them and confirms them through
//! `apply.rs`. The Settings View forwards the page's keys and status to it while the page has
//! focus.

pub mod apply;
pub mod dialogs;
pub mod form;
mod render;

use gpui::{App, Context, Entity, EventEmitter, Keystroke, Subscription};
use lib_ledger::Domain;
use lib_units::{PriceSourceRow, UnitRow};

use crate::{
    chrome::{dialog_host::OpenDialog, statusline::PageStatus},
    ledger_store::{LedgerStore, notify_on},
    navigation::key_router::Movement,
    settings::{SettingsPage, hints::settings_plain_page_hints},
    view::event::ViewEvent,
};
use dialogs::UnitsDialog;
use form::{DeleteUnitForm, UnitForm};

impl EventEmitter<ViewEvent> for UnitsPage {}

/// The Units page. It has no selection of its own: every row is reached by its buttons.
pub struct UnitsPage {
    ledger: Entity<LedgerStore>,
    _units_changed: Subscription,
}

impl UnitsPage {
    pub fn new(ledger: Entity<LedgerStore>, cx: &mut Context<'_, Self>) -> Self {
        let _units_changed = notify_on(&ledger, Domain::Units, cx);
        Self {
            ledger,
            _units_changed,
        }
    }

    fn units<'a>(&self, cx: &'a App) -> &'a [UnitRow] {
        self.ledger.read(cx).ledger().units()
    }

    fn price_sources<'a>(&self, cx: &'a App) -> &'a [PriceSourceRow] {
        self.ledger.read(cx).ledger().price_sources()
    }

    // -----------------------------------------------------------------------------------------
    // The interface the Settings View forwards to while the page has focus (ADR-0037)
    // -----------------------------------------------------------------------------------------

    /// The page has no keys of its own: each Dialog opens from its button.
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
            right: SettingsPage::Units.scope_note(),
        })
    }

    /// The heading's meta: how many Units the table holds.
    pub fn scope_note(&self, cx: &App) -> String {
        let count = i64::try_from(self.units(cx).len()).unwrap_or(i64::MAX);
        crate::msg::desktop_settings_scope_units(count)
    }

    // -----------------------------------------------------------------------------------------
    // Clicks
    // -----------------------------------------------------------------------------------------

    /// **+ Add unit** opens the Add unit dialog.
    fn click_add(&mut self, cx: &mut Context<'_, Self>) {
        open_dialog(UnitsDialog::Add(UnitForm::default()), cx);
    }

    /// A Units row's **edit** opens the Edit unit dialog filled from the row.
    fn click_edit(&mut self, index: usize, cx: &mut Context<'_, Self>) {
        let Some(row) = self.units(cx).get(index) else {
            return;
        };
        let form = UnitForm::from_row(row);
        open_dialog(UnitsDialog::Edit(index, form), cx);
    }

    /// A Units row's **delete** opens the destructive Delete unit confirm dialog.
    fn click_delete(&mut self, index: usize, cx: &mut Context<'_, Self>) {
        let Some(row) = self.units(cx).get(index) else {
            return;
        };
        let form = DeleteUnitForm::new(row.code.as_str());
        open_dialog(UnitsDialog::Delete(index, form), cx);
    }

    /// Price Sources' test, edit, delete and add have no Dialog yet, so they say so on the status
    /// line.
    fn not_built(&mut self, message: String, cx: &mut Context<'_, Self>) {
        cx.emit(ViewEvent::SetStatus(Some(message)));
    }

    // -----------------------------------------------------------------------------------------
    // Tests
    // -----------------------------------------------------------------------------------------

    /// The Units rows. The open Dialog's fields are `Shell`'s, which fills them in.
    #[doc(hidden)]
    pub fn snapshot(&self, cx: &App) -> SettingsUnitsSnapshot {
        let units = self.units(cx);
        SettingsUnitsSnapshot {
            unit_codes: units.iter().map(|unit| unit.code.clone()).collect(),
            unit_rows: units
                .iter()
                .map(|unit| {
                    (
                        unit.code.clone(),
                        unit.name.clone(),
                        unit.kind.as_str().to_string(),
                    )
                })
                .collect(),
            unit_dialog_name: None,
            unit_dialog_kind: None,
            unit_dialog_field: None,
        }
    }
}

/// The Units page: codes in table order, `(code, name, kind)` rows, and the Add/Edit unit dialog's
/// Name draft, Type (`UnitKind` variant name) and focused field (`Code` or `Name`).
#[doc(hidden)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingsUnitsSnapshot {
    pub unit_codes: Vec<String>,
    pub unit_rows: Vec<(String, String, String)>,
    pub unit_dialog_name: Option<String>,
    pub unit_dialog_kind: Option<String>,
    pub unit_dialog_field: Option<String>,
}

/// What the open Units Dialog shows, for the Settings snapshots.
#[doc(hidden)]
pub struct UnitsDialogSnapshot {
    /// `AddUnit`, `EditUnit` or `DeleteUnit`.
    pub label: &'static str,
    /// The Code draft, on Add and Edit.
    pub code: Option<String>,
    /// The typed-back confirmation, on Delete.
    pub confirm: Option<String>,
    /// The Add/Edit form's Name draft, Type and focused field.
    pub form: Option<(String, String, String)>,
}

impl UnitsDialog {
    #[doc(hidden)]
    pub fn snapshot(&self) -> UnitsDialogSnapshot {
        let (label, form) = match self {
            Self::Add(form) => ("AddUnit", form),
            Self::Edit(_, form) => ("EditUnit", form),
            Self::Delete(_, form) => {
                return UnitsDialogSnapshot {
                    label: "DeleteUnit",
                    code: None,
                    confirm: Some(form.confirm_input.text().to_string()),
                    form: None,
                };
            }
        };
        UnitsDialogSnapshot {
            label,
            code: Some(form.code.text().to_string()),
            confirm: None,
            form: Some((
                form.name.text().to_string(),
                format!("{:?}", form.kind),
                format!("{:?}", form.focused_field),
            )),
        }
    }
}

/// Asks `Shell`'s Dialog host to open a Units Dialog.
fn open_dialog(dialog: UnitsDialog, cx: &mut Context<'_, UnitsPage>) {
    cx.emit(ViewEvent::OpenDialog(Box::new(OpenDialog::Units(dialog))));
}
