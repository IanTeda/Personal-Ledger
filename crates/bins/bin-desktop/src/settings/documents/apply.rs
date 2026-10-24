//! The Document type Dialogs' confirm cycle (ADR-0037): the page command each form describes, a
//! `lib_ledger` command in each variant, confirmed through [`confirm_ledger`].

use gpui::App;
use lib_core::DocumentTypeId;
use lib_documents::types::{self, RemoveError};
use lib_ledger::{AddDocumentType, EditDocumentType, Ledger, LedgerCommand, RemoveDocumentType};

use super::dialogs::DocumentTypesDialog;
use crate::shell::view::{
    dialog_apply::{DialogForm, FormError, OutcomeToast, confirm_ledger},
    dialog_host::{ConfirmDialog, DialogScope, DialogToast, Landed},
};

/// What confirming a Document types Dialog asks the Ledger for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DocumentTypeCommand {
    Add(AddDocumentType),
    Edit(EditDocumentType),
    /// Remove the type, moving its Filed files to the type named `destination` (or the Default).
    Remove(RemoveDocumentType),
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

/// Every outcome closes the Dialog with no Toast: a save or a removal selects its type.
impl OutcomeToast for DocumentTypeOutcome {
    fn toast(&self) -> Option<DialogToast> {
        None
    }
}

/// Removing a type moves its Filed Documents in the same command.
impl LedgerCommand for DocumentTypeCommand {
    type Outcome = DocumentTypeOutcome;
    type Refusal = RemoveError;

    fn run(self, ledger: &mut Ledger) -> Result<DocumentTypeOutcome, RemoveError> {
        match self {
            Self::Add(add) => {
                let Ok(id) = add.run(ledger);
                Ok(DocumentTypeOutcome::Saved(id))
            }
            Self::Edit(edit) => {
                let id = edit.id;
                edit.run(ledger).map_err(|_| RemoveError::Unknown)?;
                Ok(DocumentTypeOutcome::Saved(id))
            }
            Self::Remove(remove) => {
                let position = types::position(ledger.document_types(), remove.id).unwrap_or(0);
                remove.run(ledger)?;
                let rows = ledger.document_types();
                let selected = rows
                    .get(position.min(rows.len().saturating_sub(1)))
                    .map(|row| row.id);
                Ok(DocumentTypeOutcome::Removed { selected })
            }
            Self::Dismiss => Ok(DocumentTypeOutcome::Dismissed),
        }
    }
}

impl DialogForm for DocumentTypesDialog {
    type Command = DocumentTypeCommand;
    type Refusal = RemoveError;

    fn command(&self) -> Result<DocumentTypeCommand, FormError> {
        match self {
            Self::Add(form) if form.is_valid() => {
                Ok(DocumentTypeCommand::Add(AddDocumentType(form.draft())))
            }
            Self::Edit(id, form) if form.is_valid() => {
                Ok(DocumentTypeCommand::Edit(EditDocumentType {
                    id: *id,
                    draft: form.draft(),
                }))
            }
            Self::Remove(id, form) if form.is_valid() => {
                Ok(DocumentTypeCommand::Remove(RemoveDocumentType {
                    id: *id,
                    destination: form.select.value().map(str::to_string),
                }))
            }
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

impl ConfirmDialog for DocumentTypesDialog {
    type Outcome = DocumentTypeOutcome;

    fn confirm(
        &mut self,
        scope: &DialogScope,
        cx: &mut App,
    ) -> Option<Landed<DocumentTypeOutcome>> {
        confirm_ledger(self, &scope.ledger, cx)
    }
}

#[cfg(test)]
mod tests {
    use gpui::{Entity, TestAppContext};
    use lib_ledger::LedgerParts;

    use super::*;
    use crate::{
        ledger_store::{LedgerStore, test_ledger},
        settings::documents::form::{DocumentTypeForm, RemoveForm},
    };

    fn ledger(cx: &mut TestAppContext) -> Entity<LedgerStore> {
        test_ledger(
            cx,
            LedgerParts {
                document_types: types::default_types(),
                ..LedgerParts::default()
            },
        )
    }

    fn rows(ledger: &Entity<LedgerStore>, cx: &TestAppContext) -> Vec<types::DocumentTypeRow> {
        ledger.read_with(cx, |store, _| store.ledger().document_types().to_vec())
    }

    #[gpui::test]
    fn an_invalid_form_stays_open_and_the_default_notice_only_closes(cx: &mut TestAppContext) {
        let ledger = ledger(cx);
        let before = rows(&ledger, cx).len();
        let mut add = DocumentTypesDialog::Add(DocumentTypeForm::new(&types::default_types()));
        assert_eq!(add.command(), Err(FormError), "a blank name");
        assert!(
            cx.update(|cx| confirm_ledger(&mut add, &ledger, cx))
                .is_none()
        );
        assert_eq!(rows(&ledger, cx).len(), before);

        let mut notice = DocumentTypesDialog::DefaultNotice;
        let landed = cx
            .update(|cx| confirm_ledger(&mut notice, &ledger, cx))
            .expect("the notice closes");
        assert_eq!(landed.outcome, DocumentTypeOutcome::Dismissed);
        assert_eq!(landed.toast, None);
    }

    #[gpui::test]
    fn removing_a_type_selects_its_neighbour(cx: &mut TestAppContext) {
        let ledger = ledger(cx);
        let mut remove = DocumentTypesDialog::Remove(
            DocumentTypeId::seed(3),
            RemoveForm::new(&types::default_types(), DocumentTypeId::seed(3)),
        );
        let landed = cx
            .update(|cx| confirm_ledger(&mut remove, &ledger, cx))
            .expect("the removal lands");
        assert!(matches!(
            landed.outcome,
            DocumentTypeOutcome::Removed { selected: Some(selected) }
                if selected != DocumentTypeId::seed(3)
        ));
        assert!(
            rows(&ledger, cx)
                .iter()
                .all(|row| row.id != DocumentTypeId::seed(3))
        );
    }

    #[gpui::test]
    fn a_refused_remove_stays_open_with_the_refusal(cx: &mut TestAppContext) {
        let ledger = ledger(cx);
        let before = rows(&ledger, cx).len();
        // A type removed while its dialog was open: the form is valid, the domain refuses.
        let mut dialog = DocumentTypesDialog::Remove(
            DocumentTypeId::seed(999),
            RemoveForm::new(&types::default_types(), DocumentTypeId::seed(999)),
        );
        assert!(
            cx.update(|cx| confirm_ledger(&mut dialog, &ledger, cx))
                .is_none()
        );
        assert!(matches!(
            &dialog,
            DocumentTypesDialog::Remove(_, form) if form.error == Some(RemoveError::Unknown)
        ));
        assert_eq!(rows(&ledger, cx).len(), before);
    }
}
