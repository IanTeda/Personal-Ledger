//! The Settings › Documents page's Add, Edit and Remove dialogs (#481): opening them, their keys
//! and clicks, applying the change to the in-memory types, and building their elements. The pure
//! rules live in `documents::types`; the chrome in `view::settings::document_type_dialogs`.

use std::rc::Rc;

use gpui::{AnyElement, Context};

use super::Shell;
use crate::{
    dialog_host::OpenDialog,
    documents,
    documents::types::{
        DocumentTypeForm, DocumentTypesDialog, FormField, RemindLead, RemoveForm, TracksDate,
    },
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
        let form = DocumentTypeForm::new(&self.document_types);
        self.open_dialog(OpenDialog::DocumentTypes(DocumentTypesDialog::Add(form)));
    }

    pub(super) fn open_edit_document_type_dialog(&mut self, id: u32) {
        let Some(row) = documents::types::get(&self.document_types, id) else {
            return;
        };
        let form = DocumentTypeForm::from_row(row, &self.document_types);
        self.open_dialog(OpenDialog::DocumentTypes(DocumentTypesDialog::Edit(
            id, form,
        )));
    }

    /// Other, the Default, has no remove action and gets a notice instead.
    pub(super) fn open_remove_document_type_dialog(&mut self, id: u32) {
        let Some(row) = documents::types::get(&self.document_types, id) else {
            return;
        };
        let dialog = if row.is_default {
            DocumentTypesDialog::DefaultNotice
        } else {
            DocumentTypesDialog::Remove(id, RemoveForm::new(&self.document_types, id))
        };
        self.open_dialog(OpenDialog::DocumentTypes(dialog));
    }

    /// Applies a confirmed Document types dialog (reached through [`Self::confirm_open_dialog`]
    /// from **Add type** / **Save** / **Remove** or `Enter`): applies the change and selects the
    /// type. The Default notice only closes.
    pub(super) fn apply_document_types_dialog(&mut self, dialog: DocumentTypesDialog) {
        match dialog {
            DocumentTypesDialog::Add(form) => {
                let id = documents::types::add_type(
                    &mut self.document_types,
                    &mut self.document_types_next_id,
                    &form,
                );
                self.settings_documents_selected = Some(id);
            }
            DocumentTypesDialog::Edit(id, form) => {
                documents::types::edit_type(&mut self.document_types, id, &form);
                self.settings_documents_selected = Some(id);
            }
            DocumentTypesDialog::Remove(id, form) => {
                // Keep the cursor on the neighbour that takes the removed row's place.
                let position = documents::types::position(&self.document_types, id).unwrap_or(0);
                let destination = form
                    .select
                    .value()
                    .and_then(|name| documents::types::id_by_name(&self.document_types, name))
                    .filter(|destination| *destination != id);
                if documents::types::remove_type(&mut self.document_types, id, form.select.value())
                    .is_err()
                {
                    return;
                }
                // The Documents surface follows the list: Filed files move with their type, to the
                // chosen destination or else the Default, and a scope on the type falls back to All.
                let moved_to = documents::DocumentType(
                    destination
                        .unwrap_or_else(|| documents::types::fallback_id(&self.document_types)),
                );
                for document in &mut self.documents {
                    if document.doc_type == documents::DocumentType(id) {
                        document.doc_type = moved_to;
                    }
                }
                if self.documents_scope
                    == documents::LibraryScope::Type(documents::DocumentType(id))
                {
                    self.documents_scope = documents::LibraryScope::All;
                }
                self.settings_documents_selected = self
                    .document_types
                    .get(position.min(self.document_types.len().saturating_sub(1)))
                    .map(|row| row.id);
            }
            DocumentTypesDialog::DefaultNotice => {}
        }
    }

    fn with_document_type_form(
        &mut self,
        cx: &mut Context<'_, Self>,
        change: impl FnOnce(&mut DocumentTypeForm),
    ) {
        if let Some(form) = self
            .document_types_dialog_mut()
            .and_then(DocumentTypesDialog::form_mut)
        {
            change(form);
        }
        cx.notify();
    }

    fn handle_document_types_dialog_cancel(&mut self, cx: &mut Context<'_, Self>) {
        self.close_dialog();
        cx.notify();
    }

    fn handle_document_types_dialog_confirm(&mut self, cx: &mut Context<'_, Self>) {
        self.confirm_open_dialog();
        cx.notify();
    }

    fn handle_document_types_destination_field_click(&mut self, cx: &mut Context<'_, Self>) {
        if let Some(DocumentTypesDialog::Remove(_, form)) = self.document_types_dialog_mut() {
            form.click_select();
        }
        cx.notify();
    }

    fn handle_document_types_destination_option_click(
        &mut self,
        index: usize,
        cx: &mut Context<'_, Self>,
    ) {
        if let Some(DocumentTypesDialog::Remove(_, form)) = self.document_types_dialog_mut() {
            form.choose(index);
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
        match self.document_types_dialog()? {
            DocumentTypesDialog::Add(form) => Some(view::render_form(
                view::FormProps {
                    form,
                    edit: None,
                    error: form.name_error(),
                    valid: form.is_valid(),
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
                        error: form.name_error(),
                        valid: form.is_valid(),
                        handlers: self.document_type_form_handlers(entity, plain),
                    },
                    cx,
                ))
            }
            DocumentTypesDialog::Remove(id, form) => {
                let row = types.iter().find(|row| row.id == *id)?;
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
                        destinations: &form.destinations,
                        destination: &form.select,
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
