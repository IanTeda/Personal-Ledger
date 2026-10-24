//! `Root`'s side of the Documents destination: the Dialog host's Documents dialogs (drawn,
//! confirmed and, for the link picker, picked through), the `documents <verb>` palette commands,
//! files dropped on the window and the persisted mode, scope and sort. The page itself renders and
//! handles its own input in its Entity (`view::documents`, ADR-0037), which builds the dialogs;
//! what a confirmed dialog changes on the page goes back to it through its named methods.

use lib_shell::NavInput;
use std::rc::Rc;

use gpui::{Context, Window};
use lib_ledger::{ImportDocuments, ToggleDocumentLink};
use lib_toast::ToastKind;

use super::{OpenDialog, Root};
use crate::{
    documents::apply::{DocumentOutcome, OnDocuments, exists_on_disk},
    documents::form::{
        DocumentField, DocumentForm, DocumentsDialog, FactsField, FactsForm, import_lines,
    },
    documents::model as view_model,
    documents::picker::{PickerRequest, PickerRow, PickerState, Purpose},
    documents::types::DocumentTypeRow,
    documents::{Document, DocumentsMode, ImportOutcome, LibraryScope, LibrarySort, Source},
    navigation::nav::Noun,
    shell::view::dialog_apply,
    view::documents::dialogs,
};

impl Root {
    /// The Documents, read from their store Entity.
    pub(super) fn documents<'a>(&self, cx: &'a gpui::App) -> &'a [Document] {
        self.ledger.read(cx).ledger().documents()
    }

    /// The Document Types in the user's order, read from the store Entity.
    pub(super) fn document_types<'a>(&self, cx: &'a gpui::App) -> &'a [DocumentTypeRow] {
        self.ledger.read(cx).ledger().document_types()
    }

    pub(super) fn set_documents_persisted(
        &self,
        mode: DocumentsMode,
        scope: LibraryScope,
        sort: LibrarySort,
        cx: &mut gpui::App,
    ) {
        self.views
            .documents
            .update(cx, |view, cx| view.set_persisted(mode, scope, sort, cx));
    }

    pub fn documents_persisted(
        &self,
        cx: &gpui::App,
    ) -> (DocumentsMode, LibraryScope, LibrarySort) {
        self.views.documents.read(cx).persisted()
    }

    /// The `?` cheat-sheet's Documents group as `(action, keys)`. Empty off the page.
    pub(super) fn documents_cheat_sheet(&self, cx: &gpui::App) -> Vec<(String, &'static str)> {
        if self.nav().noun() != Noun::Documents {
            return Vec::new();
        }
        self.views.documents.read(cx).cheat_sheet()
    }

    /// The picker's rows for `state`, as the Documents page words them.
    fn documents_picker_rows(&self, state: &PickerState, cx: &gpui::App) -> Vec<PickerRow> {
        self.views.documents.read(cx).picker_rows(state, cx)
    }

    /// What `enter` or a click on picker row `index` does: toggles a Link, files the Unfiled
    /// Document, or follows a Link, according to why the picker opened.
    pub(super) fn documents_picker_pick(&mut self, index: usize, cx: &mut Context<'_, Self>) {
        let Some(DocumentsDialog::Picker(state)) = self.documents_dialog().cloned() else {
            return;
        };
        let Some(row) = self.documents_picker_rows(&state, cx).get(index).cloned() else {
            return;
        };
        match state.purpose {
            Purpose::Link(id) => {
                let Some(link) = row.link else {
                    return;
                };
                let Ok(added) = self.ledger.update(cx, |store, cx| {
                    store.apply(cx, ToggleDocumentLink { id, link })
                }) else {
                    return;
                };
                self.close_documents_dialog();
                self.raise_toast(
                    ToastKind::Info,
                    if added {
                        crate::msg::desktop_documents_toast_linked(&row.text)
                    } else {
                        crate::msg::desktop_documents_toast_unlinked(&row.text)
                    },
                );
            }
            Purpose::File(id) => {
                let Some(doc_type) = state.doc_type else {
                    self.set_status(crate::msg::desktop_documents_picker_need_type());
                    return;
                };
                self.close_documents_dialog();
                self.views.documents.update(cx, |view, cx| {
                    view.file_by_hand(id, doc_type, row.link, cx);
                });
            }
            Purpose::Follow(_) => {
                self.close_documents_dialog();
                if let Some(link) = row.link {
                    self.views
                        .documents
                        .update(cx, |view, cx| view.goto_link(link, cx));
                }
            }
        }
    }

    fn handle_facts_field_click(&mut self, field: FactsField, cx: &mut Context<'_, Self>) {
        if let Some(DocumentsDialog::Facts(_, form)) = self.documents_dialog_mut() {
            if FactsForm::is_select(field) {
                form.click_select();
            } else {
                form.focus(field);
            }
        }
        cx.notify();
    }

    fn handle_facts_option_click(&mut self, index: usize, cx: &mut Context<'_, Self>) {
        if let Some(DocumentsDialog::Facts(_, form)) = self.documents_dialog_mut() {
            form.choose(index);
        }
        cx.notify();
    }

    // -----------------------------------------------------------------------------------------
    // Commands
    // -----------------------------------------------------------------------------------------

    /// A `documents <verb>` palette command.
    pub(super) fn run_documents_command(
        &mut self,
        verb: crate::navigation::command::DocumentsVerb,
        cx: &mut gpui::App,
    ) {
        use crate::navigation::command::DocumentsVerb;
        let noun_before = self.nav().noun();
        self.navigate(NavInput::Show(Noun::Documents));
        if noun_before != Noun::Documents {
            self.reset_view_scroll(cx);
        }
        self.views.documents.update(cx, |view, cx| match verb {
            DocumentsVerb::Inbox => view.set_mode(DocumentsMode::Inbox, cx),
            DocumentsVerb::Library => view.set_mode(DocumentsMode::Library, cx),
            DocumentsVerb::AcceptAll => {
                view.set_mode(DocumentsMode::Inbox, cx);
                view.open_accept_all(cx);
            }
            DocumentsVerb::Add => view.open_add(cx),
            DocumentsVerb::Import => view.open_import(cx),
        });
    }

    // -----------------------------------------------------------------------------------------
    // Dialogs
    // -----------------------------------------------------------------------------------------

    pub(super) fn close_documents_dialog(&mut self) {
        self.close_dialog();
    }

    pub(super) fn documents_dialog(&self) -> Option<&DocumentsDialog> {
        match self.state.dialog.as_ref()? {
            OpenDialog::Documents(dialog) => Some(dialog),
            _ => None,
        }
    }

    fn documents_dialog_mut(&mut self) -> Option<&mut DocumentsDialog> {
        match self.state.dialog.as_mut()? {
            OpenDialog::Documents(dialog) => Some(dialog),
            _ => None,
        }
    }

    fn open_documents_dialog(&mut self, dialog: DocumentsDialog) {
        self.open_dialog(OpenDialog::Documents(dialog));
    }

    /// Confirms the open Documents dialog (`Enter`, `Ctrl-Enter` in Import, and the confirm
    /// button). The forms go through the host's cycle: the command runs against the Documents
    /// store, a refusal stays on the form (an import that took nothing says why each path
    /// failed), and a landed one closes the dialog. The link picker answers with a request that
    /// needs the live rows, run by [`Self::apply_documents_picker`].
    pub(super) fn confirm_documents_dialog(&mut self, cx: &mut Context<'_, Self>) {
        if matches!(self.documents_dialog(), Some(DocumentsDialog::Picker(_))) {
            let Some(OpenDialog::Documents(DocumentsDialog::Picker(state))) =
                self.state.dialog.take()
            else {
                return;
            };
            self.navigate(NavInput::ExitMode);
            self.apply_documents_picker(*state, cx);
            return;
        }
        let ledger = self.ledger.clone();
        let today = self.today;
        let focus = self
            .views
            .documents
            .read(cx)
            .selected_document(cx)
            .map(|document| document.id);
        let Some(dialog) = self.dialog_mut::<DocumentsDialog>() else {
            return;
        };
        let Some(landed) =
            dialog_apply::confirm_ledger_with(dialog, &ledger, cx, |command| OnDocuments {
                command,
                today,
                focus,
            })
        else {
            return;
        };
        self.close_dialog();
        match landed.outcome {
            DocumentOutcome::Added { id, title } => {
                self.views
                    .documents
                    .update(cx, |view, cx| view.select_document(id, cx));
                self.raise_toast(
                    ToastKind::Success,
                    crate::msg::desktop_documents_toast_added(&title),
                );
            }
            DocumentOutcome::Saved { id, title } => {
                self.views
                    .documents
                    .update(cx, |view, cx| view.select_document(id, cx));
                self.raise_toast(
                    ToastKind::Success,
                    crate::msg::desktop_documents_toast_saved(&title),
                );
            }
            DocumentOutcome::Imported { imported, skipped } => {
                self.raise_toast(
                    ToastKind::Success,
                    if skipped == 0 {
                        crate::msg::desktop_documents_toast_imported(
                            i64::try_from(imported).unwrap_or(i64::MAX),
                        )
                    } else {
                        crate::msg::desktop_documents_toast_import_partial(
                            &imported.to_string(),
                            &skipped.to_string(),
                        )
                    },
                );
                self.views
                    .documents
                    .update(cx, |view, cx| view.set_mode(DocumentsMode::Inbox, cx));
            }
            DocumentOutcome::FactsSaved { id, title } => {
                // Editing clears a skip, so the Document may have moved up the list: follow it.
                self.views
                    .documents
                    .update(cx, |view, cx| view.focus_inbox_document(id, cx));
                self.raise_toast(
                    ToastKind::Success,
                    crate::msg::desktop_documents_toast_facts_saved(&title),
                );
            }
            DocumentOutcome::Accepted(None) => {}
            DocumentOutcome::Accepted(Some(batch)) => {
                let count = batch.entries.len();
                self.views
                    .documents
                    .update(cx, |view, cx| view.accepted_batch(batch, cx));
                self.raise_toast(
                    ToastKind::Success,
                    crate::msg::desktop_documents_toast_filed_many(&count.to_string()),
                );
            }
        }
    }

    /// A picker key that needed the live rows: moves the highlight over them, or acts on the
    /// highlighted one.
    fn apply_documents_picker(&mut self, mut state: PickerState, cx: &mut Context<'_, Self>) {
        let Some(request) = state.request.take() else {
            return;
        };
        let rows = self.documents_picker_rows(&state, cx).len();
        let at = state.selected.min(rows.saturating_sub(1));
        match request {
            PickerRequest::Pick => {
                self.open_documents_dialog(DocumentsDialog::Picker(Box::new(state)));
                self.documents_picker_pick(at, cx);
            }
            PickerRequest::Step { down } => {
                state.step(down, rows);
                self.open_documents_dialog(DocumentsDialog::Picker(Box::new(state)));
            }
        }
    }

    fn documents_form_mut(&mut self) -> Option<&mut DocumentForm> {
        match self.documents_dialog_mut() {
            Some(DocumentsDialog::Add(form) | DocumentsDialog::Edit(_, form)) => Some(form),
            _ => None,
        }
    }

    /// Files dropped anywhere on the window land in the Inbox like `Import…`, but the page stays
    /// where it is: the Toast and the rail badge say what arrived.
    pub fn drop_documents(&mut self, paths: &[std::path::PathBuf], cx: &mut gpui::App) {
        let typed: Vec<String> = paths
            .iter()
            .map(|path| path.display().to_string())
            .collect();
        let command = ImportDocuments {
            lines: import_lines(typed.iter().map(String::as_str), exists_on_disk),
            today: self.today,
            source: Source::Dropped,
        };
        let Ok(outcomes) = self.ledger.update(cx, |store, cx| store.apply(cx, command));
        let imported = outcomes
            .iter()
            .filter(|outcome| matches!(outcome, ImportOutcome::Imported(_)))
            .count();
        let skipped = outcomes.len() - imported;
        if imported == 0 {
            if skipped > 0 {
                self.raise_toast(
                    ToastKind::Error,
                    crate::msg::desktop_documents_toast_import_partial("0", &skipped.to_string()),
                );
            }
            return;
        }
        let toast = if skipped == 0 {
            crate::msg::desktop_documents_toast_imported(
                i64::try_from(imported).unwrap_or(i64::MAX),
            )
        } else {
            crate::msg::desktop_documents_toast_import_partial(
                &imported.to_string(),
                &skipped.to_string(),
            )
        };
        self.raise_toast(ToastKind::Success, toast);
    }

    fn handle_documents_field_click(&mut self, field: DocumentField, cx: &mut Context<'_, Self>) {
        if let Some(form) = self.documents_form_mut() {
            if DocumentForm::is_select(field) {
                form.click_select(field);
            } else if field == DocumentField::Reminder {
                form.focus(field);
                form.toggle_reminder();
            } else {
                form.focus(field);
            }
        }
        cx.notify();
    }

    fn handle_documents_option_click(
        &mut self,
        field: DocumentField,
        index: usize,
        cx: &mut Context<'_, Self>,
    ) {
        if let Some(form) = self.documents_form_mut() {
            form.choose(field, index);
        }
        cx.notify();
    }

    /// The open Documents dialog as an overlay element, or `None` when none is open.
    pub(super) fn render_documents_dialog(
        &self,
        entity: &gpui::Entity<Root>,
        cx: &gpui::App,
    ) -> Option<gpui::AnyElement> {
        let dialog = self.documents_dialog()?;
        let on_cancel: crate::dialog::OnClick = {
            let entity = entity.clone();
            Rc::new(move |_window: &mut Window, cx: &mut gpui::App| {
                entity.update(cx, |shell, cx| {
                    shell.close_documents_dialog();
                    cx.notify();
                });
            })
        };
        let on_confirm: crate::dialog::OnClick = {
            let entity = entity.clone();
            Rc::new(move |_window: &mut Window, cx: &mut gpui::App| {
                entity.update(cx, |shell, cx| {
                    shell.confirm_open_dialog(cx);
                    cx.notify();
                });
            })
        };
        match dialog {
            DocumentsDialog::Picker(state) => {
                let rows = self.documents_picker_rows(state, cx);
                let on_pick: dialogs::OnPickerRow = {
                    let entity = entity.clone();
                    Rc::new(move |index, _window, cx| {
                        entity.update(cx, |shell, cx| {
                            shell.documents_picker_pick(index, cx);
                            cx.notify();
                        });
                    })
                };
                Some(dialogs::render_picker(
                    dialogs::PickerProps {
                        state,
                        rows: &rows,
                        type_label: state
                            .doc_type
                            .map(|kind| view_model::type_label(self.document_types(cx), kind)),
                        on_pick,
                    },
                    cx,
                ))
            }
            DocumentsDialog::AcceptAll(count) => Some(dialogs::render_accept_all(
                *count, on_cancel, on_confirm, cx,
            )),
            DocumentsDialog::Facts(_, form) => {
                let problems = form.problems();
                let on_field_click: dialogs::OnFactsFieldClick = {
                    let entity = entity.clone();
                    Rc::new(move |field, _window, cx| {
                        entity.update(cx, |shell, cx| shell.handle_facts_field_click(field, cx));
                    })
                };
                let on_option_click: dialogs::OnFactsOptionClick = {
                    let entity = entity.clone();
                    Rc::new(move |index, _window, cx| {
                        entity.update(cx, |shell, cx| shell.handle_facts_option_click(index, cx));
                    })
                };
                Some(dialogs::render_facts(
                    dialogs::FactsProps {
                        form,
                        options: &form.options,
                        valid: form.can_save(&problems),
                        problems: &problems,
                        on_field_click,
                        on_option_click,
                        on_cancel,
                        on_confirm,
                    },
                    cx,
                ))
            }
            DocumentsDialog::Import(form, notes) => Some(dialogs::render_import(
                dialogs::ImportProps {
                    form,
                    notes: notes.clone(),
                    valid: form.can_save(),
                    on_cancel,
                    on_confirm,
                },
                cx,
            )),
            DocumentsDialog::Add(form) | DocumentsDialog::Edit(_, form) => {
                let problems = form.problems(self.documents(cx), exists_on_disk);
                let on_field_click: dialogs::OnFieldClick = {
                    let entity = entity.clone();
                    Rc::new(move |field, _window, cx| {
                        entity.update(cx, |shell, cx| {
                            shell.handle_documents_field_click(field, cx)
                        });
                    })
                };
                let on_option_click: dialogs::OnOptionClick = {
                    let entity = entity.clone();
                    Rc::new(move |field, index, _window, cx| {
                        entity.update(cx, |shell, cx| {
                            shell.handle_documents_option_click(field, index, cx)
                        });
                    })
                };
                Some(dialogs::render_form(
                    dialogs::FormProps {
                        form,
                        options: &form.options,
                        valid: form.is_complete() && form.can_save(&problems),
                        problems: &problems,
                        handlers: dialogs::FormHandlers {
                            on_field_click,
                            on_option_click,
                            on_cancel,
                            on_confirm,
                        },
                    },
                    cx,
                ))
            }
        }
    }
}
