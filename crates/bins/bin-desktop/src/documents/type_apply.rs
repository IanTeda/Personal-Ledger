//! The Document type Dialogs' confirm cycle (ADR-0037): the command each form describes and the
//! [`Apply`] that runs it against the Documents store. When `LedgerStore` replaces the per-domain
//! stores only [`DocumentTypesApply`] changes.

use gpui::{App, Entity};

use super::types::{
    self, DocumentTypeForm, DocumentTypesDialog, RemoveError, add_type, edit_type, remove_type,
};
use crate::{
    chrome::dialog_apply::{Apply, DialogForm, FormError},
    documents::DocumentType,
    view::documents::state::DocumentsStore,
};

/// What confirming a Document types Dialog asks for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DocumentTypeCommand {
    Add(DocumentTypeForm),
    Edit(u32, DocumentTypeForm),
    /// Remove the type, moving its Filed files to the type named `destination` (or the Default).
    Remove {
        id: u32,
        destination: Option<String>,
    },
    /// The Default's notice: nothing to change, the Dialog only closes.
    Dismiss,
}

/// What a landed [`DocumentTypeCommand`] did, for the selection and scope that follow it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DocumentTypeOutcome {
    /// Added or edited: the type to select.
    Saved(u32),
    /// Removed: the type that now sits where it did, to select. Its Filed files have moved.
    Removed {
        id: u32,
        selected: Option<u32>,
    },
    Dismissed,
}

impl DialogForm for DocumentTypesDialog {
    type Command = DocumentTypeCommand;
    type Refusal = RemoveError;

    fn command(&self) -> Result<DocumentTypeCommand, FormError> {
        match self {
            Self::Add(form) if form.is_valid() => Ok(DocumentTypeCommand::Add(form.clone())),
            Self::Edit(id, form) if form.is_valid() => {
                Ok(DocumentTypeCommand::Edit(*id, form.clone()))
            }
            Self::Remove(id, form) if form.is_valid() => Ok(DocumentTypeCommand::Remove {
                id: *id,
                destination: form.select.value().map(str::to_string),
            }),
            Self::DefaultNotice => Ok(DocumentTypeCommand::Dismiss),
            _ => Err(FormError),
        }
    }

    fn refuse(&mut self, refusal: RemoveError) {
        if let Self::Remove(_, form) = self {
            form.error = Some(refusal);
        }
    }
}

/// Runs [`DocumentTypeCommand`]s against the Documents store, which holds the types and the Filed
/// files that follow them.
pub struct DocumentTypesApply {
    pub documents: Entity<DocumentsStore>,
}

impl Apply<DocumentTypeCommand> for DocumentTypesApply {
    type Outcome = DocumentTypeOutcome;
    type Refusal = RemoveError;

    fn apply(
        &mut self,
        command: DocumentTypeCommand,
        cx: &mut App,
    ) -> Result<DocumentTypeOutcome, RemoveError> {
        match command {
            DocumentTypeCommand::Add(form) => Ok(DocumentTypeOutcome::Saved(
                self.documents.update(cx, |store, cx| {
                    store.mutate(cx, |data| add_type(data.types, data.types_next_id, &form))
                }),
            )),
            DocumentTypeCommand::Edit(id, form) => {
                self.documents.update(cx, |store, cx| {
                    store.mutate(cx, |data| edit_type(data.types, id, &form));
                });
                Ok(DocumentTypeOutcome::Saved(id))
            }
            DocumentTypeCommand::Remove { id, destination } => {
                let (position, moved_to) = {
                    let rows = &self.documents.read(cx).types(cx);
                    let position = types::position(rows, id).unwrap_or(0);
                    let moved_to = destination
                        .as_deref()
                        .and_then(|name| types::id_by_name(rows, name))
                        .filter(|moved_to| *moved_to != id)
                        .unwrap_or_else(|| types::fallback_id(rows));
                    (position, moved_to)
                };
                self.documents.update(cx, |store, cx| {
                    store.mutate(cx, |data| {
                        remove_type(data.types, id, destination.as_deref())?;
                        // The Documents surface follows the list: Filed files move with their type.
                        for document in data.documents.iter_mut() {
                            if document.doc_type == DocumentType(id) {
                                document.doc_type = DocumentType(moved_to);
                            }
                        }
                        Ok(())
                    })
                })?;
                let rows = &self.documents.read(cx).types(cx);
                let selected = rows
                    .get(position.min(rows.len().saturating_sub(1)))
                    .map(|row| row.id);
                Ok(DocumentTypeOutcome::Removed { id, selected })
            }
            DocumentTypeCommand::Dismiss => Ok(DocumentTypeOutcome::Dismissed),
        }
    }
}

#[cfg(test)]
mod tests {
    use gpui::{AppContext, TestAppContext};

    use lib_ledger::LedgerParts;

    use super::*;
    use crate::ledger_store::test_ledger;
    use crate::{chrome::dialog_apply::confirm, documents::types::RemoveForm};

    fn apply(cx: &mut TestAppContext) -> DocumentTypesApply {
        let ledger = test_ledger(
            cx,
            LedgerParts {
                document_types: types::default_types(),
                ..LedgerParts::default()
            },
        );
        let documents = cx.new(|cx| DocumentsStore::new(ledger, cx));
        DocumentTypesApply { documents }
    }

    #[gpui::test]
    fn removing_a_type_selects_its_neighbour_and_the_default_notice_only_closes(
        cx: &mut TestAppContext,
    ) {
        let mut apply = apply(cx);
        let rows = types::default_types();
        let mut remove = DocumentTypesDialog::Remove(3, RemoveForm::new(&rows, 3));
        let outcome = cx.update(|cx| confirm(&mut remove, &mut apply, cx));
        assert!(matches!(
            outcome,
            Some(DocumentTypeOutcome::Removed {
                id: 3,
                selected: Some(_)
            })
        ));

        let mut notice = DocumentTypesDialog::DefaultNotice;
        let outcome = cx.update(|cx| confirm(&mut notice, &mut apply, cx));
        assert_eq!(outcome, Some(DocumentTypeOutcome::Dismissed));
    }

    #[gpui::test]
    fn a_refused_remove_stays_open_with_the_refusal(cx: &mut TestAppContext) {
        let mut apply = apply(cx);
        let rows = types::default_types();
        // A type removed while its dialog was open: the form is valid, the domain refuses.
        let mut dialog = DocumentTypesDialog::Remove(999, RemoveForm::new(&rows, 999));
        let outcome = cx.update(|cx| confirm(&mut dialog, &mut apply, cx));
        assert_eq!(outcome, None);
        assert!(matches!(
            &dialog,
            DocumentTypesDialog::Remove(_, form) if form.error == Some(RemoveError::Unknown)
        ));
    }
}
