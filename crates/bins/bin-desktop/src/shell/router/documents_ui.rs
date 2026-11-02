//! `Root`'s side of the Documents destination: the legacy Dialog host's Documents Dialogs (drawn,
//! confirmed and, for the link picker, picked through) and the persisted mode, scope and sort. The
//! page runs on `lib-documents`' Presenter in its Entity (`documents::view::page`), whose Router
//! opens these Dialogs; what a confirmed one did goes back to it as a `DocumentsOutcome`, until the
//! Dialogs move onto Presenters too (#968).

use lib_shell::NavInput;
use std::rc::Rc;

use gpui::{Context, Window};
use lib_documents::DocumentsOutcome;
use lib_documents::presenter::library::{Lookups, type_label};
use lib_ledger::ToggleDocumentLink;
use lib_toast::ToastKind;

use super::{OpenDialog, Root};
use crate::{
    documents::apply::{DocumentOutcome, OnDocuments},
    documents::files::DiskFiles,
    documents::form::{DocumentField, DocumentForm, DocumentsDialog, FactsField, FactsForm},
    documents::picker::{PickerRequest, PickerRow, PickerState, Purpose},
    documents::types::DocumentTypeRow,
    documents::view::dialogs,
    documents::{Document, DocumentLink, DocumentsMode, LibraryScope, LibrarySort},
    shell::view::dialog_apply,
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
            .update(cx, |page, cx| page.set_persisted(mode, scope, sort, cx));
    }

    pub fn documents_persisted(
        &self,
        cx: &gpui::App,
    ) -> (DocumentsMode, LibraryScope, LibrarySort) {
        self.views.documents.read(cx).persisted()
    }

    /// The picker's rows for `state`, derived afresh from the stores and the Document's Links, as
    /// the Documents page words them.
    pub(super) fn documents_picker_rows(
        &self,
        state: &PickerState,
        cx: &gpui::App,
    ) -> Vec<PickerRow> {
        let records = self.ledger.read(cx).document_records();
        let lookups = Lookups {
            types: self.document_types(cx),
            records: &records,
            today: self.today,
            date_style: self.views.documents.read(cx).date_style(),
        };
        let current: Vec<DocumentLink> =
            crate::documents::get(self.documents(cx), state.purpose.document())
                .map(|document| document.links.clone())
                .unwrap_or_default();
        crate::documents::picker::rows(
            state,
            &crate::documents::picker::Sources {
                lookups: &lookups,
                current: &current,
            },
        )
    }

    /// Hands what a closed Documents Dialog did back to the page.
    fn documents_landed(&self, outcome: DocumentsOutcome, cx: &mut gpui::App) {
        self.views
            .documents
            .update(cx, |page, cx| page.dialog_landed(outcome, cx));
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
                        lib_documents::msg::documents_toast_unlinked(&row.text)
                    },
                );
            }
            Purpose::File(id) => {
                let Some(doc_type) = state.doc_type else {
                    self.set_status(crate::msg::desktop_documents_picker_need_type());
                    return;
                };
                self.close_documents_dialog();
                self.documents_landed(
                    DocumentsOutcome::FileByHand {
                        id,
                        doc_type,
                        link: row.link,
                    },
                    cx,
                );
            }
            Purpose::Follow(_) => {
                self.close_documents_dialog();
                if let Some(link) = row.link {
                    self.documents_landed(DocumentsOutcome::Follow(link), cx);
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
        let page = self.views.documents.read(cx);
        let focus = page.selected_document().map(|document| document.id);
        let skipped = page.skipped();
        let records = self.ledger.read(cx).document_records();
        let Some(dialog) = self.dialog_mut::<DocumentsDialog>() else {
            return;
        };
        let Some(landed) =
            dialog_apply::confirm_ledger_with(dialog, &ledger, cx, |command| OnDocuments {
                command,
                today,
                focus,
                skipped,
                records,
            })
        else {
            return;
        };
        self.close_dialog();
        match landed.outcome {
            DocumentOutcome::Added { id, title } => {
                self.documents_landed(DocumentsOutcome::Shown(id), cx);
                self.raise_toast(
                    ToastKind::Success,
                    crate::msg::desktop_documents_toast_added(&title),
                );
            }
            DocumentOutcome::Saved { id, title } => {
                self.documents_landed(DocumentsOutcome::Shown(id), cx);
                self.raise_toast(
                    ToastKind::Success,
                    crate::msg::desktop_documents_toast_saved(&title),
                );
            }
            DocumentOutcome::Imported { imported, skipped } => {
                self.raise_toast(
                    ToastKind::Success,
                    if skipped == 0 {
                        lib_documents::msg::documents_toast_imported(
                            i64::try_from(imported).unwrap_or(i64::MAX),
                        )
                    } else {
                        lib_documents::msg::documents_toast_import_partial(
                            &imported.to_string(),
                            &skipped.to_string(),
                        )
                    },
                );
                self.documents_landed(DocumentsOutcome::Imported, cx);
            }
            DocumentOutcome::FactsSaved { id, title } => {
                self.documents_landed(DocumentsOutcome::FactsSaved(id), cx);
                self.raise_toast(
                    ToastKind::Success,
                    crate::msg::desktop_documents_toast_facts_saved(&title),
                );
            }
            DocumentOutcome::Accepted(None) => {}
            DocumentOutcome::Accepted(Some(batch)) => {
                self.documents_landed(DocumentsOutcome::AcceptedAll(batch), cx);
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
        self.views
            .documents
            .update(cx, |page, cx| page.drop_paths(paths, cx));
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
                    // A click is one input, as `enter` on the row is.
                    Rc::new(move |index, _window, cx| {
                        entity.update(cx, |shell, cx| {
                            crate::input::send(cx, |cx| shell.documents_picker_pick(index, cx));
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
                            .map(|kind| type_label(self.document_types(cx), kind)),
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
                let problems = form.problems(self.documents(cx), &DiskFiles);
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
