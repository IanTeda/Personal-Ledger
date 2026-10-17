//! The **Payees** Settings page (ADR-0037, and its amendment's one folder per page): the Payees
//! table with its rules and Default category. [`PayeesPage`] reads its rows from [`LedgerStore`],
//! renders itself (`render.rs`), handles its keys and movement, builds its Dialogs (`dialogs/`)
//! and reaches `Shell` only through [`ViewEvent`]; `Shell`'s Dialog host draws them and confirms
//! them through `apply.rs`. The Settings View forwards to it while the page is open.

pub mod apply;
pub mod dialogs;
pub mod form;
mod render;

use gpui::{App, Context, Entity, EventEmitter, Keystroke, Subscription};
use lib_core::PayeeId;
use lib_ledger::Domain;
use lib_payees::Payee;

use crate::{
    chrome::{dialog_host::OpenDialog, statusline::PageStatus},
    ledger_store::{LedgerStore, notify_on},
    navigation::key_router::Movement,
    settings::hints::settings_payees_hints,
    view::event::{Reveal, ViewEvent},
};
use dialogs::PayeesDialog;
use form::{DeleteAction, DeletePayeeForm, PayeeForm, PayeeOptions};
use lib_categories::Category;
use render::scope_text;

/// The stores the Payees page reads: the Ledger, for its rows and each Payee's use, and the
/// Categories its dialogs offer as a default.
pub struct PayeesSources {
    pub ledger: Entity<LedgerStore>,
}

impl EventEmitter<ViewEvent> for PayeesPage {}

/// Emitted when a click lands on the page's rows, so the Settings View, which owns the page's
/// focus, moves it from the index into the page.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RowClicked;

impl EventEmitter<RowClicked> for PayeesPage {}

/// The Payees page's own state: the Payee id under the cursor.
pub struct PayeesPage {
    sources: PayeesSources,
    /// It may name a Payee that no longer exists, so read it through [`Self::selected_id`].
    selected: Option<PayeeId>,
    /// Whether the page, not Settings' index, has keyboard focus, told by the Settings View as it
    /// draws the page.
    pub(crate) page_focused: bool,
    /// Repaints the page when the Payees its rows and counts read change.
    _payees_changed: Subscription,
    _categories_changed: Subscription,
}

impl PayeesPage {
    pub fn new(sources: PayeesSources, cx: &mut Context<'_, Self>) -> Self {
        let _payees_changed = notify_on(&sources.ledger, Domain::Payees, cx);
        let _categories_changed = notify_on(&sources.ledger, Domain::Categories, cx);
        Self {
            sources,
            selected: None,
            page_focused: false,
            _payees_changed,
            _categories_changed,
        }
    }

    pub(crate) fn payees<'a>(&self, cx: &'a App) -> &'a [Payee] {
        self.sources.ledger.read(cx).ledger().payees()
    }

    pub(crate) fn categories<'a>(&self, cx: &'a App) -> &'a [Category] {
        self.sources.ledger.read(cx).ledger().categories()
    }

    pub fn set_page_focused(&mut self, focused: bool) {
        self.page_focused = focused;
    }

    // -----------------------------------------------------------------------------------------
    // The interface the Settings View forwards to while the page is open (ADR-0037)
    // -----------------------------------------------------------------------------------------

    /// The page claims no key ahead of the router: its `n`/`e`/`d` are router leftovers.
    pub fn key_ahead(&mut self, _keystroke: &Keystroke, _cx: &mut Context<'_, Self>) -> bool {
        false
    }

    /// The page's bare `n`, `e` and `d`, each through the same handler its button uses. Returns
    /// whether the key was one of them.
    pub fn handle_key(&mut self, keystroke: &Keystroke, cx: &mut Context<'_, Self>) -> bool {
        let modifiers = &keystroke.modifiers;
        if modifiers.control || modifiers.alt || modifiers.platform || modifiers.shift {
            return false;
        }
        match keystroke.key.as_str() {
            "n" => self.open_add_dialog(cx),
            "e" => {
                if let Some(id) = self.selected_id(cx) {
                    self.open_edit_dialog(id, cx);
                }
            }
            "d" => {
                if let Some(id) = self.selected_id(cx) {
                    self.open_delete_dialog(id, cx);
                }
            }
            _ => return false,
        }
        true
    }

    /// `j`/`k`/`g`/`G`/`Ctrl-d`/`Ctrl-u` walk the Payees list A–Z. `enter` has no hand-off here.
    pub fn apply_movement(&mut self, movement: Movement, cx: &mut Context<'_, Self>) {
        let Some(step) = movement.step() else {
            return;
        };
        let ids: Vec<PayeeId> = lib_payees::sorted_by_name(self.payees(cx))
            .iter()
            .map(|payee| payee.id)
            .collect();
        self.selected = lib_list_cursor::step_id(&ids, self.selected_id(cx), step);
        cx.notify();
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

    /// Opening the page keeps its selection, so returning finds the same row selected.
    pub fn on_enter(&mut self, _cx: &mut Context<'_, Self>) {}

    /// Selects a Payee another View handed off.
    pub fn reveal(&mut self, reveal: Reveal, cx: &mut Context<'_, Self>) {
        if let Reveal::Payee(id) = reveal {
            self.select_id(id, cx);
        }
    }

    /// The page's legend and its Payee counts.
    pub fn page_status(&self, cx: &App) -> Option<PageStatus> {
        Some(PageStatus {
            hints: settings_payees_hints(),
            right: scope_text(self.payees(cx)),
        })
    }

    // -----------------------------------------------------------------------------------------
    // Selection
    // -----------------------------------------------------------------------------------------

    /// The selected Payee's id: the stored id while it still exists, else the first row, so a
    /// deleted Payee never leaves the page with nothing under the cursor.
    pub fn selected_id(&self, cx: &App) -> Option<PayeeId> {
        let sorted = lib_payees::sorted_by_name(self.payees(cx));
        lib_list_cursor::resolve(sorted.iter().map(|payee| payee.id), self.selected)
    }

    /// Selects Payee `id`, if it still exists.
    pub fn select_id(&mut self, id: PayeeId, cx: &mut Context<'_, Self>) {
        if lib_payees::get(self.payees(cx), id).is_some() {
            self.selected = Some(id);
            cx.notify();
        }
    }

    // -----------------------------------------------------------------------------------------
    // Clicks
    // -----------------------------------------------------------------------------------------

    /// A click on a row: selects it and moves focus into the page.
    pub(crate) fn click_row(&mut self, id: PayeeId, cx: &mut Context<'_, Self>) {
        self.select_id(id, cx);
        cx.emit(RowClicked);
    }

    /// **+ Add payee**: the same handler `n` reaches.
    pub(crate) fn click_add(&mut self, cx: &mut Context<'_, Self>) {
        self.open_add_dialog(cx);
    }

    pub(crate) fn click_edit(&mut self, id: PayeeId, cx: &mut Context<'_, Self>) {
        self.select_id(id, cx);
        self.open_edit_dialog(id, cx);
    }

    pub(crate) fn click_delete(&mut self, id: PayeeId, cx: &mut Context<'_, Self>) {
        self.select_id(id, cx);
        self.open_delete_dialog(id, cx);
    }

    // -----------------------------------------------------------------------------------------
    // Dialogs
    // -----------------------------------------------------------------------------------------

    fn dialog_options(&self, cx: &App) -> PayeeOptions {
        PayeeOptions::new(
            self.categories(cx),
            crate::msg::desktop_payees_category_none(),
        )
    }

    fn open_add_dialog(&mut self, cx: &mut Context<'_, Self>) {
        let form = PayeeForm::new(&self.dialog_options(cx), self.payees(cx));
        open_dialog(PayeesDialog::Add(form), cx);
    }

    /// Opens the Edit dialog pre-filled from Payee `id`. A no-op if the Payee is gone.
    fn open_edit_dialog(&mut self, id: PayeeId, cx: &mut Context<'_, Self>) {
        let rows = self.payees(cx);
        let Some(payee) = lib_payees::get(rows, id) else {
            return;
        };
        let form = PayeeForm::from_payee(payee, &self.dialog_options(cx), rows);
        open_dialog(PayeesDialog::Edit(id, form), cx);
    }

    /// Opens the Delete dialog on Payee `id`, copying its name and action in so the form validates
    /// without `Shell`. A no-op if the Payee is gone.
    fn open_delete_dialog(&mut self, id: PayeeId, cx: &mut Context<'_, Self>) {
        let Some(payee) = lib_payees::get(self.payees(cx), id) else {
            return;
        };
        let payee_use = self.sources.ledger.read(cx).ledger().payee_use(id);
        let action = DeleteAction::for_payee(payee, payee_use);
        let form = DeletePayeeForm::new(payee, action);
        open_dialog(PayeesDialog::Delete(id, form), cx);
    }
}

/// Asks `Shell`'s Dialog host to open a Payees dialog.
fn open_dialog(dialog: PayeesDialog, cx: &mut Context<'_, PayeesPage>) {
    cx.emit(ViewEvent::OpenDialog(Box::new(OpenDialog::Payees(dialog))));
}

/// The Payees page: names A–Z, and the selected row's name, for the integration tests (ADR-0030).
#[doc(hidden)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingsPayeesSnapshot {
    pub payee_names: Vec<String>,
    pub selected_payee: Option<String>,
}

impl PayeesPage {
    #[doc(hidden)]
    pub fn snapshot(&self, cx: &App) -> SettingsPayeesSnapshot {
        let rows = self.payees(cx);
        SettingsPayeesSnapshot {
            payee_names: lib_payees::sorted_by_name(rows)
                .into_iter()
                .map(|payee| payee.name.clone())
                .collect(),
            selected_payee: self
                .selected_id(cx)
                .and_then(|id| lib_payees::get(rows, id))
                .map(|payee| payee.name.clone()),
        }
    }
}
