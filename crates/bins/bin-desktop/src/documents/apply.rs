//! The Documents Dialogs' confirm cycle (ADR-0037): the command each form describes, wrapped with
//! the page's context into an [`OnDocuments`] `lib_ledger` command, confirmed through
//! [`confirm_ledger_with`](crate::chrome::dialog_apply::confirm_ledger_with). Accept all reads the
//! Transactions and Payees it matches against inside its own Ledger command.
//!
//! The link picker answers with a request that needs the live rows and may open another Dialog,
//! so it stays `Shell`'s.

use lib_core::DocumentId;
use std::path::Path;

use chrono::NaiveDate;

use lib_ledger::{
    AcceptAllDocuments, AddDocument, DocumentEdit, EditDocument, ImportDocuments, Ledger,
    LedgerCommand, SetDocumentFacts,
};

use super::{
    self as documents, AddError, ExtractedFacts, FilingError, FilingUndo, ImportOutcome, Source,
    form::{self, DocumentForm, DocumentsDialog, ImportForm},
};
use crate::chrome::dialog_apply::{DialogForm, FormError, OutcomeToast};

/// Whether the path exists on disk, passed to the form checks so they stay testable.
pub fn exists_on_disk(path: &Path) -> bool {
    path.exists()
}

/// What confirming a Documents Dialog asks for.
#[derive(Debug, Clone)]
pub enum DocumentCommand {
    /// Add (`editing` is `None`) or save a Filed Document from the form.
    Save {
        editing: Option<DocumentId>,
        form: Box<DocumentForm>,
    },
    /// Every typed path that exists lands in the Inbox.
    Import(ImportForm),
    SetFacts {
        id: DocumentId,
        facts: ExtractedFacts,
    },
    AcceptAll,
}

/// A [`DocumentCommand`] with the context the form does not hold: today, and the Inbox row Accept
/// all keeps in view (`focus`).
#[derive(Debug, Clone)]
pub struct OnDocuments {
    pub command: DocumentCommand,
    pub today: NaiveDate,
    pub focus: Option<DocumentId>,
}

/// What a landed [`OnDocuments`] did, for the View state and Toast that follow it.
#[derive(Debug, Clone)]
pub enum DocumentOutcome {
    Added {
        id: DocumentId,
        title: String,
    },
    Saved {
        id: DocumentId,
        title: String,
    },
    Imported {
        imported: usize,
        skipped: usize,
    },
    FactsSaved {
        id: DocumentId,
        title: String,
    },
    /// Accept all filed this batch, or `None` when nothing was strong enough.
    Accepted(Option<FilingUndo>),
}

/// No Outcome carries a Toast from the host: the page raises its own from the outcome it is handed.
impl OutcomeToast for DocumentOutcome {}

/// Why a Documents command didn't land.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DocumentRefusal {
    /// The form has a problem only the Library or the disk can see (a missing file, a duplicate
    /// path); the form lists it itself.
    Problems,
    Add(AddError),
    Facts(FilingError),
    /// An import that took nothing, with why each path failed.
    Import(Vec<String>),
}

impl LedgerCommand for OnDocuments {
    type Outcome = DocumentOutcome;
    type Refusal = DocumentRefusal;

    fn run(self, ledger: &mut Ledger) -> Result<DocumentOutcome, DocumentRefusal> {
        let Self {
            command,
            today,
            focus,
        } = self;
        match command {
            DocumentCommand::Save { editing, form } => {
                let problems = form.problems(ledger.documents(), exists_on_disk);
                let new = form
                    .can_save(&problems)
                    .then(|| form.build())
                    .flatten()
                    .ok_or(DocumentRefusal::Problems)?;
                let title = new.title.clone();
                match editing {
                    None => {
                        let id = AddDocument(new).run(ledger).map_err(DocumentRefusal::Add)?;
                        Ok(DocumentOutcome::Added { id, title })
                    }
                    Some(id) => {
                        let edit = DocumentEdit {
                            title: new.title,
                            doc_type: new.doc_type,
                            date: new.date,
                            key_date: new.key_date,
                        };
                        EditDocument { id, edit }
                            .run(ledger)
                            .map_err(DocumentRefusal::Add)?;
                        Ok(DocumentOutcome::Saved { id, title })
                    }
                }
            }
            DocumentCommand::Import(form) => {
                let lines = form::import_lines(form.lines(), exists_on_disk);
                let Ok(outcomes) = ImportDocuments {
                    lines,
                    today,
                    source: Source::Import,
                }
                .run(ledger);
                let imported = outcomes
                    .iter()
                    .filter(|outcome| matches!(outcome, ImportOutcome::Imported(_)))
                    .count();
                let notes = import_notes(&outcomes);
                if imported == 0 {
                    return Err(DocumentRefusal::Import(notes));
                }
                Ok(DocumentOutcome::Imported {
                    imported,
                    skipped: notes.len(),
                })
            }
            DocumentCommand::SetFacts { id, facts } => {
                let title = documents::get(ledger.documents(), id)
                    .map(|document| document.title.clone())
                    .unwrap_or_default();
                SetDocumentFacts { id, facts }
                    .run(ledger)
                    .map_err(DocumentRefusal::Facts)?;
                Ok(DocumentOutcome::FactsSaved { id, title })
            }
            DocumentCommand::AcceptAll => {
                let Ok(batch) = AcceptAllDocuments { focus }.run(ledger);
                Ok(DocumentOutcome::Accepted(batch))
            }
        }
    }
}

impl DialogForm for DocumentsDialog {
    type Command = DocumentCommand;
    type Refusal = DocumentRefusal;

    fn command(&self) -> Result<DocumentCommand, FormError> {
        match self {
            Self::Add(form) if form.is_complete() => Ok(DocumentCommand::Save {
                editing: None,
                form: form.clone(),
            }),
            Self::Edit(id, form) if form.is_complete() => Ok(DocumentCommand::Save {
                editing: Some(*id),
                form: form.clone(),
            }),
            Self::Import(form, _) if form.can_save() => Ok(DocumentCommand::Import(form.clone())),
            Self::Facts(id, form) => {
                let facts = form.build().ok_or(FormError)?;
                Ok(DocumentCommand::SetFacts { id: *id, facts })
            }
            Self::AcceptAll(_) => Ok(DocumentCommand::AcceptAll),
            _ => Err(FormError),
        }
    }

    fn refuse(&mut self, refusal: DocumentRefusal) {
        // The Add and Edit forms list their own problems and the Facts form is checked live, so
        // only an import has something to add: why each path failed.
        if let (Self::Import(_, notes), DocumentRefusal::Import(why)) = (self, refusal) {
            *notes = why;
        }
    }
}

/// Why each line of an import that didn't land didn't, worded for the Import dialog and Toast.
pub fn import_notes(outcomes: &[ImportOutcome]) -> Vec<String> {
    outcomes
        .iter()
        .filter_map(|outcome| match outcome {
            ImportOutcome::Imported(_) => None,
            ImportOutcome::NotFound(path) => {
                Some(crate::msg::desktop_documents_error_path_missing(path))
            }
            ImportOutcome::Unsupported(path) => Some(format!(
                "{path}: {}",
                crate::msg::desktop_documents_error_unsupported()
            )),
            ImportOutcome::Already(title) => {
                Some(crate::msg::desktop_documents_error_already(title))
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use gpui::{Entity, TestAppContext};
    use lib_ledger::LedgerParts;

    use super::*;
    use crate::chrome::dialog_apply::confirm_ledger_with;
    use crate::ledger_store::{LedgerStore, test_ledger};

    fn ledger(cx: &mut TestAppContext) -> Entity<LedgerStore> {
        test_ledger(
            cx,
            LedgerParts {
                document_types: documents::types::default_types(),
                ..LedgerParts::default()
            },
        )
    }

    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 9, 19).unwrap()
    }

    #[gpui::test]
    fn an_import_of_a_missing_path_stays_open_and_says_why(cx: &mut TestAppContext) {
        let ledger = ledger(cx);
        let mut form = ImportForm::default();
        form.paths = crate::form::field::TextField::new("/definitely/not/here.pdf");
        let mut dialog = DocumentsDialog::Import(form, Vec::new());
        let landed = cx.update(|cx| {
            confirm_ledger_with(&mut dialog, &ledger, cx, |command| OnDocuments {
                command,
                today: today(),
                focus: None,
            })
        });
        assert!(landed.is_none());
        assert!(matches!(&dialog, DocumentsDialog::Import(_, notes) if notes.len() == 1));
    }

    #[gpui::test]
    fn accept_all_with_nothing_strong_closes_without_a_batch(cx: &mut TestAppContext) {
        let ledger = ledger(cx);
        let mut dialog = DocumentsDialog::AcceptAll(0);
        let landed = cx.update(|cx| {
            confirm_ledger_with(&mut dialog, &ledger, cx, |command| OnDocuments {
                command,
                today: today(),
                focus: None,
            })
        });
        assert!(matches!(
            landed.map(|landed| landed.outcome),
            Some(DocumentOutcome::Accepted(None))
        ));
    }
}
