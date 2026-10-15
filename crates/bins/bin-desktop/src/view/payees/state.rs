//! The Payees destination's Entity (ADR-0032, ADR-0037). [`PayeesView`] is Settings' Payees page:
//! it owns the selection, renders itself (`impl Render` in `settings::payees`), handles the
//! page's keys and movement, builds its dialogs and reaches `Shell` only through [`ViewEvent`]. The
//! Settings View forwards to it while the page is open; `Shell`'s Dialog host draws and confirms
//! the dialogs it builds.

use gpui::{App, Context, Entity, EventEmitter, Keystroke};
use lib_payees::Payee;

use crate::{
    categories::Category,
    chrome::{dialog_host::OpenDialog, statusline::PageStatus},
    navigation::key_router::Movement,
    payees::{
        self, PayeesStore,
        form::{DeleteAction, DeletePayeeForm, PayeeForm, PayeeOptions, PayeesDialog},
    },
    settings::{hints::settings_payees_hints, payees::scope_text},
    view::categories::state::CategoriesStore,
    view::event::{Reveal, ViewEvent},
};

/// The stores the Payees page reads: its rows, and the Categories its dialogs offer as a default.
pub struct PayeesSources {
    pub payees: Entity<PayeesStore>,
    pub categories: Entity<CategoriesStore>,
}

impl EventEmitter<ViewEvent> for PayeesView {}

/// Emitted when a click lands on the page's rows, so the Settings View, which owns the page's
/// focus, moves it from the index into the page.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RowClicked;

impl EventEmitter<RowClicked> for PayeesView {}

/// The Settings Payees page's own state: the Payee id under the cursor.
pub struct PayeesView {
    sources: PayeesSources,
    /// It may name a Payee that no longer exists, so read it through [`Self::selected_id`].
    selected: Option<u32>,
    /// Whether the page, not Settings' index, has keyboard focus, told by the Settings View as it
    /// draws the page.
    pub(crate) page_focused: bool,
}

impl PayeesView {
    pub fn new(sources: PayeesSources) -> Self {
        Self {
            sources,
            selected: None,
            page_focused: false,
        }
    }

    pub(crate) fn payees<'a>(&self, cx: &'a App) -> &'a [Payee] {
        self.sources.payees.read(cx).payees(cx)
    }

    pub(crate) fn categories<'a>(&self, cx: &'a App) -> &'a [Category] {
        self.sources.categories.read(cx).categories(cx)
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
        let ids: Vec<u32> = payees::sorted_by_name(self.payees(cx))
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
    pub fn selected_id(&self, cx: &App) -> Option<u32> {
        let sorted = payees::sorted_by_name(self.payees(cx));
        lib_list_cursor::resolve(sorted.iter().map(|payee| payee.id), self.selected)
    }

    /// Selects Payee `id`, if it still exists.
    pub fn select_id(&mut self, id: u32, cx: &mut Context<'_, Self>) {
        if payees::get(self.payees(cx), id).is_some() {
            self.selected = Some(id);
            cx.notify();
        }
    }

    // -----------------------------------------------------------------------------------------
    // Clicks
    // -----------------------------------------------------------------------------------------

    /// A click on a row: selects it and moves focus into the page.
    pub(crate) fn click_row(&mut self, id: u32, cx: &mut Context<'_, Self>) {
        self.select_id(id, cx);
        cx.emit(RowClicked);
    }

    /// **+ Add payee**: the same handler `n` reaches.
    pub(crate) fn click_add(&mut self, cx: &mut Context<'_, Self>) {
        self.open_add_dialog(cx);
    }

    pub(crate) fn click_edit(&mut self, id: u32, cx: &mut Context<'_, Self>) {
        self.select_id(id, cx);
        self.open_edit_dialog(id, cx);
    }

    pub(crate) fn click_delete(&mut self, id: u32, cx: &mut Context<'_, Self>) {
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
    fn open_edit_dialog(&mut self, id: u32, cx: &mut Context<'_, Self>) {
        let rows = self.payees(cx);
        let Some(payee) = payees::get(rows, id) else {
            return;
        };
        let form = PayeeForm::from_payee(payee, &self.dialog_options(cx), rows);
        open_dialog(PayeesDialog::Edit(id, form), cx);
    }

    /// Opens the Delete dialog on Payee `id`, copying its name and action in so the form validates
    /// without `Shell`. A no-op if the Payee is gone.
    fn open_delete_dialog(&mut self, id: u32, cx: &mut Context<'_, Self>) {
        let Some(payee) = payees::get(self.payees(cx), id) else {
            return;
        };
        let payee_use = self.sources.payees.read(cx).payee_use(id, cx);
        let action = DeleteAction::for_payee(payee, payee_use);
        let form = DeletePayeeForm::new(payee, action);
        open_dialog(PayeesDialog::Delete(id, form), cx);
    }
}

/// Asks `Shell`'s Dialog host to open a Payees dialog.
fn open_dialog(dialog: PayeesDialog, cx: &mut Context<'_, PayeesView>) {
    cx.emit(ViewEvent::OpenDialog(Box::new(OpenDialog::Payees(dialog))));
}
