//! The Document type Dialogs' confirm cycle (ADR-0037): the command each form describes and the
//! [`Apply`] that runs it against the Ledger.

use gpui::{App, Entity};
use lib_core::DocumentTypeId;
use lib_ledger::{AddDocumentType, EditDocumentType, RemoveDocumentType};

use lib_documents::types::{self, RemoveError};

use super::{dialogs::DocumentTypesDialog, form::DocumentTypeForm};
use crate::{
    chrome::{
        dialog_apply::{self, Apply, DialogForm, FormError},
        dialog_host::{ConfirmDialog, DialogScope, Landed},
    },
    ledger_store::LedgerStore,
};

/// What confirming a Document types Dialog asks for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DocumentTypeCommand {
    Add(DocumentTypeForm),
    Edit(DocumentTypeId, DocumentTypeForm),
    /// Remove the type, moving its Filed files to the type named `destination` (or the Default).
    Remove {
        id: DocumentTypeId,
        destination: Option<String>,
    },
    /// The Default's notice: nothing to change, the Dialog only closes.
    Dismiss,
}

/// What a landed [`DocumentTypeCommand`] did, for the selection and scope that follow it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DocumentTypeOutcome {
    /// Added or edited: the type to select.
    Saved(DocumentTypeId),
    /// Removed: the type that now sits where it did, to select. Its Filed files have moved.
    Removed {
        selected: Option<DocumentTypeId>,
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

/// Runs [`DocumentTypeCommand`]s against the Ledger as Ledger commands. Removing a type moves its
/// Filed Documents in the same command.
pub struct DocumentTypesApply {
    pub ledger: Entity<LedgerStore>,
}

impl ConfirmDialog for DocumentTypesDialog {
    type Outcome = DocumentTypeOutcome;

    /// Every outcome closes the Dialog with no Toast: a save or a removal selects its type.
    fn confirm(
        &mut self,
        scope: &DialogScope,
        cx: &mut App,
    ) -> Option<Landed<DocumentTypeOutcome>> {
        let mut apply = DocumentTypesApply {
            ledger: scope.ledger.clone(),
        };
        let outcome = dialog_apply::confirm(self, &mut apply, cx)?;
        Some(Landed {
            toast: None,
            outcome,
        })
    }
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
            DocumentTypeCommand::Add(form) => {
                let command = AddDocumentType(form.draft());
                let Ok(id) = self.ledger.update(cx, |store, cx| store.apply(cx, command));
                Ok(DocumentTypeOutcome::Saved(id))
            }
            DocumentTypeCommand::Edit(id, form) => {
                let command = EditDocumentType {
                    id,
                    draft: form.draft(),
                };
                self.ledger
                    .update(cx, |store, cx| store.apply(cx, command))
                    .map_err(|_| RemoveError::Unknown)?;
                Ok(DocumentTypeOutcome::Saved(id))
            }
            DocumentTypeCommand::Remove { id, destination } => {
                let position = types::position(self.ledger.read(cx).ledger().document_types(), id)
                    .unwrap_or(0);
                self.ledger.update(cx, |store, cx| {
                    store.apply(cx, RemoveDocumentType { id, destination })
                })?;
                let rows = self.ledger.read(cx).ledger().document_types();
                let selected = rows
                    .get(position.min(rows.len().saturating_sub(1)))
                    .map(|row| row.id);
                Ok(DocumentTypeOutcome::Removed { selected })
            }
            DocumentTypeCommand::Dismiss => Ok(DocumentTypeOutcome::Dismissed),
        }
    }
}

#[cfg(test)]
mod tests {
    use gpui::TestAppContext;
    use lib_ledger::LedgerParts;

    use super::*;
    use crate::chrome::dialog_apply::confirm;
    use crate::ledger_store::test_ledger;
    use crate::settings::documents::form::RemoveForm;

    fn apply(cx: &mut TestAppContext) -> DocumentTypesApply {
        let ledger = test_ledger(
            cx,
            LedgerParts {
                document_types: types::default_types(),
                ..LedgerParts::default()
            },
        );
        DocumentTypesApply { ledger }
    }

    #[gpui::test]
    fn removing_a_type_selects_its_neighbour_and_the_default_notice_only_closes(
        cx: &mut TestAppContext,
    ) {
        let mut apply = apply(cx);
        let rows = types::default_types();
        let mut remove = DocumentTypesDialog::Remove(
            DocumentTypeId::seed(3),
            RemoveForm::new(&rows, DocumentTypeId::seed(3)),
        );
        let outcome = cx.update(|cx| confirm(&mut remove, &mut apply, cx));
        assert!(matches!(
            outcome,
            Some(DocumentTypeOutcome::Removed { selected: Some(selected) })
                if selected != DocumentTypeId::seed(3)
        ));
        let remaining = cx.read(|cx| apply.ledger.read(cx).ledger().document_types().to_vec());
        assert!(
            remaining
                .iter()
                .all(|row| row.id != DocumentTypeId::seed(3))
        );

        let mut notice = DocumentTypesDialog::DefaultNotice;
        let outcome = cx.update(|cx| confirm(&mut notice, &mut apply, cx));
        assert_eq!(outcome, Some(DocumentTypeOutcome::Dismissed));
    }

    #[gpui::test]
    fn a_refused_remove_stays_open_with_the_refusal(cx: &mut TestAppContext) {
        let mut apply = apply(cx);
        let rows = types::default_types();
        // A type removed while its dialog was open: the form is valid, the domain refuses.
        let mut dialog = DocumentTypesDialog::Remove(
            DocumentTypeId::seed(999),
            RemoveForm::new(&rows, DocumentTypeId::seed(999)),
        );
        let outcome = cx.update(|cx| confirm(&mut dialog, &mut apply, cx));
        assert_eq!(outcome, None);
        assert!(matches!(
            &dialog,
            DocumentTypesDialog::Remove(_, form) if form.error == Some(RemoveError::Unknown)
        ));
    }
}
