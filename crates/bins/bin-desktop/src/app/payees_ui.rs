//! The Payees destination's wiring: keys, clicks, the Add, Edit and Delete dialogs, and the Settings Payees list. The pure rules live in `lib_payees`; the chrome in `view::payees` and `view::settings::payees`, and its View state in the `PayeesView` Entity.

use super::Shell;

use crate::{
    accounts,
    chrome::dialog_host::OpenDialog,
    navigation::{
        key_router::Movement,
        nav::{FocusZone, Noun},
    },
    payees::{self, Payee},
    settings::{SettingsFocus, SettingsSection},
};

use gpui::{App, Context, Keystroke};
use lib_toast::ToastKind;

/// `Ctrl-d`/`Ctrl-u` on the Settings Payees page: rows per half page.
const SETTINGS_PAYEES_HALF_PAGE: isize = 5;

impl Shell {
    /// The Payees rows, read through their store (ADR-0032).
    pub(super) fn payees_list<'a>(&self, cx: &'a App) -> &'a [Payee] {
        self.payees_store.read(cx).payees()
    }

    /// The Payees page's stored row position. Clamp it before use.
    fn payees_page_selected(&self, cx: &App) -> usize {
        self.payees_view.read(cx).selected()
    }

    fn set_payees_page_selected(&mut self, selected: usize, cx: &mut App) {
        self.payees_view
            .update(cx, |view, cx| view.set_selected(selected, cx));
    }

    /// The Settings Payees list's stored id. It may name a Payee that has since been removed.
    fn settings_payees_selected(&self, cx: &App) -> Option<u32> {
        self.payees_view.read(cx).settings_selected()
    }

    fn set_settings_payees_selected(&mut self, id: Option<u32>, cx: &mut App) {
        self.payees_view
            .update(cx, |view, cx| view.set_settings_selected(id, cx));
    }

    /// Whether Settings' Payees list owns the keyboard: the page, not the index, has focus.
    pub(super) fn settings_payees_page_has_focus(&self, cx: &App) -> bool {
        self.nav.noun() == Noun::Settings
            && self.nav.focus() == FocusZone::View
            && self.settings_view.read(cx).focus() == SettingsFocus::Page
            && self.settings_view.read(cx).selected_section() == SettingsSection::Payees
    }

    /// The Settings Payees page's selected Payee: the stored id while it still exists, else the
    /// first row, so a deleted Payee never leaves the page with nothing under the cursor.
    pub(super) fn settings_payees_selected_id(&self, cx: &App) -> Option<u32> {
        let sorted = payees::sorted_by_name(self.payees_list(cx));
        self.settings_payees_selected(cx)
            .filter(|id| sorted.iter().any(|payee| payee.id == *id))
            .or_else(|| sorted.first().map(|payee| payee.id))
    }

    /// `j`/`k`/`g`/`G`/`Ctrl-d`/`Ctrl-u` walk the Payees list A–Z. `enter` has no hand-off here.
    pub(super) fn apply_settings_payees_movement(&mut self, movement: Movement, cx: &mut App) {
        let current_id = self.settings_payees_selected_id(cx);
        let sorted = payees::sorted_by_name(self.payees_list(cx));
        let len = sorted.len();
        let current = current_id
            .and_then(|id| sorted.iter().position(|payee| payee.id == id))
            .unwrap_or(0);
        let next = match movement {
            Movement::Next => accounts::step_selection(current, len, 1),
            Movement::Prev => accounts::step_selection(current, len, -1),
            Movement::First => 0,
            Movement::Last => len.saturating_sub(1),
            Movement::HalfPageDown => {
                accounts::step_selection(current, len, SETTINGS_PAYEES_HALF_PAGE)
            }
            Movement::HalfPageUp => {
                accounts::step_selection(current, len, -SETTINGS_PAYEES_HALF_PAGE)
            }
            Movement::Enter => return,
        };
        let next_id = sorted.get(next).map(|payee| payee.id);
        self.set_settings_payees_selected(next_id, cx);
    }

    /// The selected Payee: the Payees page's row, or the Settings list's when that owns the
    /// keyboard.
    pub(super) fn selected_payee_id(&self, cx: &App) -> Option<u32> {
        if self.settings_payees_page_has_focus(cx) {
            return self.settings_payees_selected_id(cx);
        }
        let payees = self.payees_list(cx);
        payees
            .get(
                self.payees_page_selected(cx)
                    .min(payees.len().saturating_sub(1)),
            )
            .map(|payee| payee.id)
    }

    /// Selects the Payee with `id`, if it still exists.
    pub(super) fn select_payee(&mut self, id: u32, cx: &mut App) {
        self.set_settings_payees_selected(Some(id), cx);
        let index = self.payees_list(cx).iter().position(|payee| payee.id == id);
        if let Some(index) = index {
            self.set_payees_page_selected(index, cx);
        }
    }

    /// The Payees page's own `n`/`e`/`d` (only while it is the active noun and the view has focus,
    /// in `Normal` mode): the Add, Edit and Delete dialogs.
    pub(super) fn handle_payees_key(
        &mut self,
        keystroke: &Keystroke,
        cx: &mut Context<'_, Self>,
    ) -> bool {
        if !self.settings_payees_page_has_focus(cx) {
            return false;
        }
        let modifiers = &keystroke.modifiers;
        if modifiers.control || modifiers.alt || modifiers.platform || modifiers.shift {
            return false;
        }
        match keystroke.key.as_str() {
            "n" => self.open_add_payee_dialog(cx),
            "e" => {
                if let Some(id) = self.selected_payee_id(cx) {
                    self.open_edit_payee_dialog(id, cx);
                }
            }
            "d" => {
                if let Some(id) = self.selected_payee_id(cx) {
                    self.open_delete_payee_dialog(id, cx);
                }
            }
            _ => return false,
        }
        true
    }

    fn payee_dialog_options(&self, cx: &App) -> payees::form::PayeeOptions {
        payees::form::PayeeOptions::new(
            self.categories(cx),
            crate::msg::desktop_payees_category_none(),
        )
    }

    pub(super) fn open_add_payee_dialog(&mut self, cx: &App) {
        let form =
            payees::form::PayeeForm::new(&self.payee_dialog_options(cx), self.payees_list(cx));
        self.open_dialog(OpenDialog::Payees(payees::form::PayeesDialog::Add(form)));
    }

    /// Applies a confirmed Payees dialog (reached through [`Self::confirm_open_dialog`] from the
    /// **Add payee** / **Save** / **Delete** buttons or `Enter`). Add and Edit store the Payee and
    /// select it; a refused submit reopens the dialog with the error shown. Delete applies its
    /// [`payees::form::DeleteAction`], toasts the outcome and keeps the selection in range.
    pub(super) fn apply_payees_dialog(
        &mut self,
        dialog: payees::form::PayeesDialog,
        cx: &mut Context<'_, Self>,
    ) {
        match dialog {
            payees::form::PayeesDialog::Delete(id, form) => {
                // The dialog is only ever opened on a Payee that exists, so a miss means it was
                // removed while the dialog was open. There is nothing left to delete.
                let Some(name) = payees::get(self.payees_list(cx), id).map(|p| p.name.clone())
                else {
                    return;
                };
                let action = form.action();
                let transactions = self.transactions_store.read(cx).transactions().to_vec();
                let result = self.payees_store.update(cx, |store, cx| {
                    store.apply_delete(cx, id, action, &transactions)
                });
                let (kind, text) = match result {
                    Ok(()) => (
                        ToastKind::Success,
                        match action {
                            payees::form::DeleteAction::Delete => {
                                lib_locale::msg::toast_payee_deleted(&name)
                            }
                            payees::form::DeleteAction::Deactivate => {
                                lib_locale::msg::toast_payee_deactivated(&name)
                            }
                            payees::form::DeleteAction::Reactivate => {
                                lib_locale::msg::toast_payee_reactivated(&name)
                            }
                        },
                    ),
                    Err(error) => (
                        ToastKind::Error,
                        lib_locale::msg::toast_save_failed(
                            &lib_locale::msg::toast_entity_payee(),
                            &error.to_string(),
                        ),
                    ),
                };
                self.raise_toast(kind, text);
                let last = self.payees_list(cx).len().saturating_sub(1);
                let selected = self.payees_page_selected(cx).min(last);
                self.set_payees_page_selected(selected, cx);
            }
            payees::form::PayeesDialog::Add(mut form) => {
                let draft = form.draft();
                match self
                    .payees_store
                    .update(cx, |store, cx| store.insert(cx, &draft))
                {
                    Ok(id) => self.select_payee(id, cx),
                    Err(error) => {
                        form.error = Some(error);
                        self.open_dialog(OpenDialog::Payees(payees::form::PayeesDialog::Add(form)));
                    }
                }
            }
            payees::form::PayeesDialog::Edit(id, mut form) => {
                let draft = form.draft();
                match self
                    .payees_store
                    .update(cx, |store, cx| store.edit(cx, id, &draft))
                {
                    Ok(()) => self.select_payee(id, cx),
                    Err(error) => {
                        form.error = Some(error);
                        self.open_dialog(OpenDialog::Payees(payees::form::PayeesDialog::Edit(
                            id, form,
                        )));
                    }
                }
            }
        }
    }

    fn with_payee_form(
        &mut self,
        cx: &mut Context<'_, Self>,
        change: impl FnOnce(&mut payees::form::PayeeForm),
    ) {
        if let Some(form) = self
            .payees_dialog_mut()
            .and_then(payees::form::PayeesDialog::form_mut)
        {
            change(form);
        }
        cx.notify();
    }

    pub(super) fn handle_payees_dialog_field_click(
        &mut self,
        field: payees::form::PayeeField,
        cx: &mut Context<'_, Self>,
    ) {
        self.with_payee_form(cx, |form| {
            if field == payees::form::PayeeField::DefaultCategory {
                form.click_select();
            } else {
                form.focus(field);
            }
        });
    }

    pub(super) fn handle_payees_dialog_option_click(
        &mut self,
        index: usize,
        cx: &mut Context<'_, Self>,
    ) {
        self.with_payee_form(cx, |form| form.choose_category(index));
    }

    pub(super) fn handle_payees_dialog_add_rule(&mut self, cx: &mut Context<'_, Self>) {
        self.with_payee_form(cx, |form| {
            form.focus(payees::form::PayeeField::Rule);
            form.add_rule();
        });
    }

    pub(super) fn handle_payees_dialog_remove_rule(
        &mut self,
        index: usize,
        cx: &mut Context<'_, Self>,
    ) {
        self.with_payee_form(cx, |form| form.remove_rule(index));
    }

    pub(super) fn handle_payees_dialog_cancel(&mut self, cx: &mut Context<'_, Self>) {
        self.close_dialog();
        cx.notify();
    }

    pub(super) fn handle_payees_dialog_confirm(&mut self, cx: &mut Context<'_, Self>) {
        self.confirm_open_dialog(cx);
        cx.notify();
    }

    /// Opens the Edit dialog pre-filled from Payee `id`.
    pub(super) fn open_edit_payee_dialog(&mut self, id: u32, cx: &App) {
        let rows = self.payees_list(cx);
        let Some(payee) = payees::get(rows, id) else {
            return;
        };
        let form = payees::form::PayeeForm::from_payee(payee, &self.payee_dialog_options(cx), rows);
        self.open_dialog(OpenDialog::Payees(payees::form::PayeesDialog::Edit(
            id, form,
        )));
    }

    /// Opens the Delete dialog on Payee `id`, copying its name and action in so the form validates
    /// without `Shell`. A no-op if the Payee is gone.
    pub(super) fn open_delete_payee_dialog(&mut self, id: u32, cx: &App) {
        let Some(payee) = payees::get(self.payees_list(cx), id) else {
            return;
        };
        let action = payees::form::DeleteAction::for_payee(payee, self.transactions(cx));
        let form = payees::form::DeletePayeeForm::new(payee, action);
        self.open_dialog(OpenDialog::Payees(payees::form::PayeesDialog::Delete(
            id, form,
        )));
    }

    /// The Payee the Delete dialog is open on and what confirming it would do (#283).
    pub(super) fn delete_payee_target<'a>(
        &self,
        cx: &'a App,
    ) -> Option<(&'a Payee, payees::form::DeleteAction)> {
        let Some(payees::form::PayeesDialog::Delete(id, form)) = self.payees_dialog() else {
            return None;
        };
        Some((payees::get(self.payees_list(cx), *id)?, form.action()))
    }

    pub(super) fn handle_payees_add_click(&mut self, cx: &mut Context<'_, Self>) {
        self.open_add_payee_dialog(cx);
        cx.notify();
    }

    /// A click on a row of Settings' Payees list: selects it and moves focus into the page.
    pub(super) fn handle_settings_payees_row_click(&mut self, id: u32, cx: &mut Context<'_, Self>) {
        self.select_payee(id, cx);
        self.focus_settings_page(cx);
        cx.notify();
    }

    pub(super) fn handle_payees_edit_click(&mut self, id: u32, cx: &mut Context<'_, Self>) {
        self.select_payee(id, cx);
        self.open_edit_payee_dialog(id, cx);
        cx.notify();
    }

    pub(super) fn handle_payees_delete_click(&mut self, id: u32, cx: &mut Context<'_, Self>) {
        self.select_payee(id, cx);
        self.open_delete_payee_dialog(id, cx);
        cx.notify();
    }
}
