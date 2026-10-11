//! The Documents Dialogs' confirm cycle (ADR-0037): the command each form describes and the
//! [`Apply`] that runs it against the Documents store, reading the Transactions and Payees an
//! Accept all matches against. When `LedgerStore` replaces the per-domain stores only
//! [`DocumentsApply`] changes.
//!
//! The link picker answers with a request that needs the live rows and may open another Dialog,
//! so it stays `Shell`'s.

use std::path::Path;

use chrono::NaiveDate;
use gpui::{App, Entity};

use super::{
    self as documents, AddError, ExtractedFacts, FilingError, FilingUndo,
    form::{self, DocumentForm, DocumentsDialog, ImportForm, ImportOutcome},
};
use crate::{
    chrome::dialog_apply::{Apply, DialogForm, FormError},
    payees::PayeesStore,
    transactions::TransactionsStore,
    view::documents::state::DocumentsStore,
};

/// Whether the path exists on disk, passed to the form checks so they stay testable.
pub fn exists_on_disk(path: &Path) -> bool {
    path.exists()
}

/// What confirming a Documents Dialog asks for.
#[derive(Debug, Clone)]
pub enum DocumentCommand {
    /// Add (`editing` is `None`) or save a Filed Document from the form.
    Save {
        editing: Option<u32>,
        form: Box<DocumentForm>,
    },
    /// Every typed path that exists lands in the Inbox.
    Import(ImportForm),
    SetFacts {
        id: u32,
        facts: ExtractedFacts,
    },
    AcceptAll,
}

/// What a landed [`DocumentCommand`] did, for the View state and Toast that follow it.
#[derive(Debug, Clone)]
pub enum DocumentOutcome {
    Added {
        id: u32,
        title: String,
    },
    Saved {
        id: u32,
        title: String,
    },
    Imported {
        imported: usize,
        skipped: usize,
    },
    FactsSaved {
        id: u32,
        title: String,
    },
    /// Accept all filed this batch, or `None` when nothing was strong enough.
    Accepted(Option<FilingUndo>),
}

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

/// Runs [`DocumentCommand`]s against the Documents store. `focus` is the Inbox row Accept all
/// keeps in view.
pub struct DocumentsApply {
    pub documents: Entity<DocumentsStore>,
    pub transactions: Entity<TransactionsStore>,
    pub payees: Entity<PayeesStore>,
    pub today: NaiveDate,
    pub focus: Option<u32>,
}

impl Apply<DocumentCommand> for DocumentsApply {
    type Outcome = DocumentOutcome;
    type Refusal = DocumentRefusal;

    fn apply(
        &mut self,
        command: DocumentCommand,
        cx: &mut App,
    ) -> Result<DocumentOutcome, DocumentRefusal> {
        match command {
            DocumentCommand::Save { editing, form } => {
                let problems = form.problems(self.documents.read(cx).documents(cx), exists_on_disk);
                let new = form
                    .can_save(&problems)
                    .then(|| form.build())
                    .flatten()
                    .ok_or(DocumentRefusal::Problems)?;
                let title = new.title.clone();
                match editing {
                    None => {
                        let id = self
                            .documents
                            .update(cx, |store, cx| {
                                store.mutate(cx, |data| documents::add_filed(data.documents, new))
                            })
                            .map_err(DocumentRefusal::Add)?;
                        Ok(DocumentOutcome::Added { id, title })
                    }
                    Some(id) => {
                        self.documents
                            .update(cx, |store, cx| {
                                store.mutate(cx, |data| {
                                    documents::edit_metadata(
                                        data.documents,
                                        id,
                                        &new.title,
                                        new.doc_type,
                                        new.date,
                                        new.key_date,
                                    )
                                })
                            })
                            .map_err(DocumentRefusal::Add)?;
                        Ok(DocumentOutcome::Saved { id, title })
                    }
                }
            }
            DocumentCommand::Import(form) => {
                let today = self.today;
                let outcomes = self.documents.update(cx, |store, cx| {
                    store.mutate(cx, |data| {
                        form::import_all(data.types, &form, data.documents, today, exists_on_disk)
                    })
                });
                let imported = outcomes
                    .iter()
                    .filter(|outcome| matches!(outcome, ImportOutcome::Imported(_)))
                    .count();
                let notes: Vec<String> = outcomes
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
                    .collect();
                if imported == 0 {
                    return Err(DocumentRefusal::Import(notes));
                }
                Ok(DocumentOutcome::Imported {
                    imported,
                    skipped: notes.len(),
                })
            }
            DocumentCommand::SetFacts { id, facts } => {
                let title = documents::get(self.documents.read(cx).documents(cx), id)
                    .map(|document| document.title.clone())
                    .unwrap_or_default();
                self.documents
                    .update(cx, |store, cx| {
                        store.mutate(cx, |data| documents::edit_facts(data.documents, id, facts))
                    })
                    .map_err(DocumentRefusal::Facts)?;
                Ok(DocumentOutcome::FactsSaved { id, title })
            }
            DocumentCommand::AcceptAll => {
                // Copied out so the Documents can be written while the others are read.
                let transactions = self.transactions.read(cx).transactions(cx).to_vec();
                let payees = self.payees.read(cx).payees(cx).to_vec();
                let focus = self.focus;
                let batch = self.documents.update(cx, |store, cx| {
                    store.mutate(cx, |data| {
                        documents::accept_all_strong(
                            data.types,
                            data.documents,
                            &transactions,
                            &payees,
                            focus,
                        )
                    })
                });
                Ok(DocumentOutcome::Accepted(batch))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use gpui::{AppContext, TestAppContext};
    use lib_ledger::LedgerParts;

    use super::*;
    use crate::chrome::dialog_apply::confirm;
    use crate::ledger_store::test_ledger;

    fn apply(cx: &mut TestAppContext) -> DocumentsApply {
        let ledger = test_ledger(
            cx,
            LedgerParts {
                document_types: documents::types::default_types(),
                ..LedgerParts::default()
            },
        );
        DocumentsApply {
            documents: cx.new(|cx| DocumentsStore::new(ledger.clone(), cx)),
            transactions: cx.new(|cx| TransactionsStore::new(ledger.clone(), cx)),
            payees: cx.new(|cx| PayeesStore::new(ledger, cx)),
            today: NaiveDate::from_ymd_opt(2026, 9, 19).unwrap(),
            focus: None,
        }
    }

    #[gpui::test]
    fn an_import_of_a_missing_path_stays_open_and_says_why(cx: &mut TestAppContext) {
        let mut apply = apply(cx);
        let mut form = ImportForm::default();
        form.paths = crate::form::field::TextField::new("/definitely/not/here.pdf");
        let mut dialog = DocumentsDialog::Import(form, Vec::new());
        let outcome = cx.update(|cx| confirm(&mut dialog, &mut apply, cx));
        assert!(outcome.is_none());
        assert!(matches!(&dialog, DocumentsDialog::Import(_, notes) if notes.len() == 1));
    }

    #[gpui::test]
    fn accept_all_with_nothing_strong_closes_without_a_batch(cx: &mut TestAppContext) {
        let mut apply = apply(cx);
        let mut dialog = DocumentsDialog::AcceptAll(0);
        let outcome = cx.update(|cx| confirm(&mut dialog, &mut apply, cx));
        assert!(matches!(outcome, Some(DocumentOutcome::Accepted(None))));
    }
}
