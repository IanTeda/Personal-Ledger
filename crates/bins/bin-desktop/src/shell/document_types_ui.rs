//! The Settings › Documents page's Add, Edit and Remove dialogs (#481): opening them, their keys
//! and clicks, applying the change to the in-memory types, and building their elements. The pure
//! rules live in `document_types`; the chrome in `view::settings::document_type_dialogs`.

use std::rc::Rc;

use gpui::{AnyElement, Context, Keystroke};

use super::Shell;
use crate::{
    document_types::{
        self, DocumentTypeForm, DocumentTypesDialog, FormField, RemindLead, TracksDate,
    },
    nav::InputMode,
    view::settings::document_type_dialogs as view,
};

/// The dialog's status-line legend: Remove and the notice have no fields to `tab` through.
pub(super) fn dialog_hints(dialog: &DocumentTypesDialog) -> Vec<(&'static str, String)> {
    let mut hints = vec![
        ("esc", crate::msg::desktop_hint_cancel()),
        ("enter", crate::msg::desktop_hint_confirm()),
    ];
    if matches!(
        dialog,
        DocumentTypesDialog::Add(_) | DocumentTypesDialog::Edit(..)
    ) {
        hints.push(("tab", crate::msg::desktop_hint_next_field()));
    }
    hints
}

impl Shell {
    pub(super) fn open_add_document_type_dialog(&mut self) {
        self.document_types_dialog = Some(DocumentTypesDialog::Add(DocumentTypeForm::default()));
        self.nav.enter_mode(InputMode::Dialog);
    }

    pub(super) fn open_edit_document_type_dialog(&mut self, id: u32) {
        let Some(position) = document_types::position(&self.document_types, id) else {
            return;
        };
        let form = DocumentTypeForm::from_row(&self.document_types[position]);
        self.document_types_dialog = Some(DocumentTypesDialog::Edit(id, form));
        self.nav.enter_mode(InputMode::Dialog);
    }

    /// Other, the Default, has no remove action and gets a notice instead.
    pub(super) fn open_remove_document_type_dialog(&mut self, id: u32) {
        let Some(position) = document_types::position(&self.document_types, id) else {
            return;
        };
        self.document_types_dialog = Some(if self.document_types[position].is_default {
            DocumentTypesDialog::DefaultNotice
        } else {
            DocumentTypesDialog::Remove(
                id,
                document_types::destination_select(&self.document_types, id),
            )
        });
        self.nav.enter_mode(InputMode::Dialog);
    }

    fn close_document_types_dialog(&mut self) {
        self.document_types_dialog = None;
        self.nav.exit_mode();
    }

    /// The first `Esc` on an open destination list closes the list only.
    pub(super) fn close_open_document_type_select(&mut self) -> bool {
        if let Some(DocumentTypesDialog::Remove(_, select)) = self.document_types_dialog.as_mut() {
            let was_open = select.is_open();
            select.cancel();
            return was_open;
        }
        false
    }

    /// Keys while a Document type dialog is open. `Esc` never reaches here.
    pub(super) fn handle_document_types_dialog_key(&mut self, keystroke: &Keystroke) -> bool {
        let modifiers = &keystroke.modifiers;
        let key = keystroke.key.as_str();
        match self.document_types_dialog.as_mut() {
            Some(DocumentTypesDialog::Remove(id, select)) => {
                let list = document_types::destination_names(&self.document_types, *id);
                match key {
                    "up" | "down" => {
                        let delta = if key == "up" { -1 } else { 1 };
                        if select.is_open() {
                            select.move_highlight(&list, delta);
                        } else {
                            select.step(&list, delta);
                        }
                    }
                    "space" if !select.is_open() => select.open(&list),
                    "space" => select.commit(&list),
                    "enter" if select.is_open() => select.commit(&list),
                    "enter" => self.confirm_document_types_dialog(),
                    _ => return false,
                }
                true
            }
            Some(DocumentTypesDialog::DefaultNotice) => {
                if key == "enter" {
                    self.close_document_types_dialog();
                    return true;
                }
                false
            }
            Some(DocumentTypesDialog::Add(form) | DocumentTypesDialog::Edit(_, form)) => {
                match key {
                    "tab" => form.cycle_focus(modifiers.shift),
                    "left" => form.step_focused(-1),
                    "right" => form.step_focused(1),
                    "space" if form.focused == FormField::FinancialYear => {
                        form.toggle_financial_year();
                    }
                    "enter" => self.confirm_document_types_dialog(),
                    "backspace" => form.backspace(),
                    _ => {
                        if modifiers.control
                            || modifiers.alt
                            || modifiers.platform
                            || modifiers.function
                        {
                            return false;
                        }
                        if let Some(text) = keystroke.key_char.as_deref()
                            && text.chars().count() == 1
                            && let Some(ch) = text.chars().next()
                        {
                            form.push_char(ch);
                        }
                    }
                }
                true
            }
            None => false,
        }
    }

    /// **Add type** / **Save** / **Remove** and `enter`: a no-op while the name is invalid;
    /// otherwise applies the change, selects the type and closes the dialog.
    pub(super) fn confirm_document_types_dialog(&mut self) {
        let Some(dialog) = self.document_types_dialog.clone() else {
            return;
        };
        match dialog {
            DocumentTypesDialog::Add(form) => {
                if !form.is_valid(&self.document_types, None) {
                    return;
                }
                let id = document_types::add_type(&mut self.document_types, &form);
                self.settings_documents_selected = Some(id);
            }
            DocumentTypesDialog::Edit(id, form) => {
                if !form.is_valid(&self.document_types, Some(id)) {
                    return;
                }
                document_types::edit_type(&mut self.document_types, id, &form);
                self.settings_documents_selected = Some(id);
            }
            DocumentTypesDialog::Remove(id, select) => {
                // Keep the cursor on the neighbour that takes the removed row's place.
                let position = document_types::position(&self.document_types, id).unwrap_or(0);
                if document_types::remove_type(&mut self.document_types, id, select.value())
                    .is_err()
                {
                    return;
                }
                self.settings_documents_selected = self
                    .document_types
                    .get(position.min(self.document_types.len().saturating_sub(1)))
                    .map(|row| row.id);
            }
            DocumentTypesDialog::DefaultNotice => {}
        }
        self.close_document_types_dialog();
    }

    fn with_document_type_form(
        &mut self,
        cx: &mut Context<'_, Self>,
        change: impl FnOnce(&mut DocumentTypeForm),
    ) {
        if let Some(form) = self
            .document_types_dialog
            .as_mut()
            .and_then(DocumentTypesDialog::form_mut)
        {
            change(form);
        }
        cx.notify();
    }

    fn handle_document_types_dialog_cancel(&mut self, cx: &mut Context<'_, Self>) {
        self.close_document_types_dialog();
        cx.notify();
    }

    fn handle_document_types_dialog_confirm(&mut self, cx: &mut Context<'_, Self>) {
        self.confirm_document_types_dialog();
        cx.notify();
    }

    fn handle_document_types_destination_field_click(&mut self, cx: &mut Context<'_, Self>) {
        if let Some(DocumentTypesDialog::Remove(id, select)) = self.document_types_dialog.as_mut() {
            let list = document_types::destination_names(&self.document_types, *id);
            if select.is_open() {
                select.cancel();
            } else {
                select.open(&list);
            }
        }
        cx.notify();
    }

    fn handle_document_types_destination_option_click(
        &mut self,
        index: usize,
        cx: &mut Context<'_, Self>,
    ) {
        if let Some(DocumentTypesDialog::Remove(id, select)) = self.document_types_dialog.as_mut() {
            let list = document_types::destination_names(&self.document_types, *id);
            select.choose(&list, index);
        }
        cx.notify();
    }

    /// A click on a Financial year chip toggles it; any other field click just focuses it.
    fn handle_document_types_field_click(&mut self, field: FormField, cx: &mut Context<'_, Self>) {
        self.with_document_type_form(cx, |form| {
            form.focus(field);
            if field == FormField::FinancialYear {
                form.toggle_financial_year();
            }
        });
    }

    /// The open dialog as an element, if any.
    pub(super) fn render_document_types_dialog(
        &self,
        entity: &gpui::Entity<Shell>,
        cx: &gpui::App,
    ) -> Option<AnyElement> {
        let plain = |handler: fn(&mut Shell, &mut Context<'_, Shell>)| {
            let entity = entity.clone();
            let on_click: crate::dialog::OnClick = Rc::new(move |_window, cx| {
                entity.update(cx, handler);
            });
            on_click
        };
        let types = &self.document_types;
        match self.document_types_dialog.as_ref()? {
            DocumentTypesDialog::Add(form) => Some(view::render_form(
                view::FormProps {
                    form,
                    edit: None,
                    error: form.name_error(types, None),
                    valid: form.is_valid(types, None),
                    handlers: self.document_type_form_handlers(entity, plain),
                },
                cx,
            )),
            DocumentTypesDialog::Edit(id, form) => {
                let row = types.iter().find(|row| row.id == *id)?;
                Some(view::render_form(
                    view::FormProps {
                        form,
                        edit: Some(row),
                        error: form.name_error(types, Some(*id)),
                        valid: form.is_valid(types, Some(*id)),
                        handlers: self.document_type_form_handlers(entity, plain),
                    },
                    cx,
                ))
            }
            DocumentTypesDialog::Remove(id, select) => {
                let row = types.iter().find(|row| row.id == *id)?;
                let destinations = document_types::destination_names(types, *id);
                let on_option_click: crate::view::accounts::select_field::OnOptionClick = {
                    let entity = entity.clone();
                    Rc::new(move |index, _window, cx| {
                        entity.update(cx, |shell, cx| {
                            shell.handle_document_types_destination_option_click(index, cx);
                        });
                    })
                };
                Some(view::render_remove(
                    view::RemoveProps {
                        row,
                        destinations: &destinations,
                        destination: select,
                        handlers: view::RemoveHandlers {
                            on_field_click: plain(
                                Shell::handle_document_types_destination_field_click,
                            ),
                            on_option_click,
                            on_cancel: plain(Shell::handle_document_types_dialog_cancel),
                            on_confirm: plain(Shell::handle_document_types_dialog_confirm),
                        },
                    },
                    cx,
                ))
            }
            DocumentTypesDialog::DefaultNotice => Some(view::render_default_notice(
                plain(Shell::handle_document_types_dialog_cancel),
                cx,
            )),
        }
    }

    fn document_type_form_handlers(
        &self,
        entity: &gpui::Entity<Shell>,
        plain: impl Fn(fn(&mut Shell, &mut Context<'_, Shell>)) -> crate::dialog::OnClick,
    ) -> view::FormHandlers {
        let on_field_click: view::OnFieldClick = {
            let entity = entity.clone();
            Rc::new(move |field, _window, cx| {
                entity.update(cx, |shell, cx| {
                    shell.handle_document_types_field_click(field, cx);
                });
            })
        };
        let on_tracks_click: crate::view::settings::add_unit_dialog::OnSegmentClick<
            Option<TracksDate>,
        > = {
            let entity = entity.clone();
            Rc::new(move |option, _window, cx| {
                entity.update(cx, |shell, cx| {
                    shell.with_document_type_form(cx, |form| {
                        form.focus(FormField::TracksDate);
                        form.set_tracks_date(option);
                    });
                });
            })
        };
        let on_remind_click: crate::view::settings::add_unit_dialog::OnSegmentClick<
            Option<RemindLead>,
        > = {
            let entity = entity.clone();
            Rc::new(move |option, _window, cx| {
                entity.update(cx, |shell, cx| {
                    shell.with_document_type_form(cx, |form| {
                        if form.remind_enabled() {
                            form.focus(FormField::Remind);
                        }
                        form.set_remind(option);
                    });
                });
            })
        };
        view::FormHandlers {
            on_field_click,
            on_tracks_click,
            on_remind_click,
            on_cancel: plain(Shell::handle_document_types_dialog_cancel),
            on_confirm: plain(Shell::handle_document_types_dialog_confirm),
        }
    }
}
