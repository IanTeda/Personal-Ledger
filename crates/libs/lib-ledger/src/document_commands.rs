//! The Documents writes (ADR-0038): each a command value with its own
//! refusal, checked before anything is written so a refusal leaves the Ledger untouched, and
//! recording an upsert or delete per row it wrote (ADR-0039).
//!
//! Single-Document rules stay in `lib_documents`. What lives here crosses rows: Accept matches
//! against the Transactions and Payees its caller read from their stores and hands over (#963).
//! Skip is the Client's session state and writes nothing, so it is no command. The Document Types
//! are written by `lib_documents`' Interactors (#966).

use lib_core::{DocumentId, TransactionId};
use std::collections::HashSet;
use std::convert::Infallible;

use chrono::NaiveDate;
use lib_documents::{
    AddError, Document, DocumentLink, DocumentType, ExtractedFacts, FiledEntry, FilingError,
    FilingUndo, ImportLine, ImportOutcome, KeyDate, NewDocument, Source, accept, accept_all_strong,
    add_filed, edit_facts, edit_metadata, file_by_hand, get, import_batch, toggle_link, undo,
};

use lib_documents::{DocumentPayee, DocumentTransaction};

use crate::{Ledger, LedgerChange, LedgerCommand};

/// What the Library's `e` replaces on a Filed Document.
#[derive(Debug, Clone, PartialEq)]
pub struct DocumentEdit {
    pub title: String,
    pub doc_type: DocumentType,
    pub date: NaiveDate,
    pub key_date: Option<KeyDate>,
}

impl Ledger {
    /// Adds a Filed Document and returns its id.
    pub fn add_document(&mut self, new: NewDocument) -> Result<DocumentId, AddError> {
        let id = add_filed(&mut self.documents, new)?;
        self.record_document(id);
        Ok(id)
    }

    /// Replaces Document `id`'s Title, Document Type, date and Key Date.
    pub fn edit_document(&mut self, id: DocumentId, edit: &DocumentEdit) -> Result<(), AddError> {
        edit_metadata(
            &mut self.documents,
            id,
            &edit.title,
            edit.doc_type,
            edit.date,
            edit.key_date,
        )?;
        self.record_document(id);
        Ok(())
    }

    /// Overwrites the Extracted Facts of Unfiled Document `id`.
    pub fn set_document_facts(
        &mut self,
        id: DocumentId,
        facts: ExtractedFacts,
    ) -> Result<(), FilingError> {
        edit_facts(&mut self.documents, id, facts)?;
        self.record_document(id);
        Ok(())
    }

    /// Imports each line that exists into the Inbox, reporting what became of every line.
    pub fn import_documents(
        &mut self,
        lines: &[ImportLine],
        today: NaiveDate,
        source: Source,
    ) -> Vec<ImportOutcome> {
        let outcomes = import_batch(
            &self.document_types,
            &mut self.documents,
            lines,
            today,
            source,
        );
        for outcome in &outcomes {
            if let ImportOutcome::Imported(id) = outcome {
                self.record_document(*id);
            }
        }
        outcomes
    }

    /// Adds `link` to Document `id`, or removes it when it has it. Returns whether it was added.
    pub fn toggle_document_link(
        &mut self,
        id: DocumentId,
        link: DocumentLink,
    ) -> Result<bool, FilingError> {
        let added = toggle_link(&mut self.documents, id, link).ok_or(FilingError::NotFound)?;
        self.record_document(id);
        Ok(added)
    }

    /// Files Unfiled Document `id` under Transaction `transaction_id`, one of `transactions`.
    pub fn accept_document(
        &mut self,
        id: DocumentId,
        transaction_id: TransactionId,
        transactions: &[DocumentTransaction],
    ) -> Result<FiledEntry, FilingError> {
        let entry = accept(
            &self.document_types,
            &mut self.documents,
            transactions,
            id,
            transaction_id,
        )?;
        self.record_document(id);
        Ok(entry)
    }

    /// Files Unfiled Document `id` by hand with a Document Type and at most one Link.
    pub fn file_document_by_hand(
        &mut self,
        id: DocumentId,
        doc_type: DocumentType,
        link: Option<DocumentLink>,
    ) -> Result<FiledEntry, FilingError> {
        let entry = file_by_hand(&mut self.documents, id, doc_type, link)?;
        self.record_document(id);
        Ok(entry)
    }

    /// Reverses a filing action and returns the Document focus goes back to.
    pub fn undo_filing(&mut self, record: FilingUndo) -> Option<DocumentId> {
        let ids: Vec<DocumentId> = record.entries.iter().map(|entry| entry.id).collect();
        let focus = undo(&mut self.documents, record);
        for id in ids {
            self.record_document(id);
        }
        focus
    }

    /// Accepts every Unfiled Document whose Suggested Link to one of `transactions` is Strong, as
    /// one undoable batch, in the Inbox's order with the `skipped` last. `focus` is the Inbox row
    /// the Client keeps in view. Returns `None` when none was strong.
    pub fn accept_all_documents(
        &mut self,
        focus: Option<DocumentId>,
        skipped: &HashSet<DocumentId>,
        transactions: &[DocumentTransaction],
        payees: &[DocumentPayee],
    ) -> Option<FilingUndo> {
        let batch = accept_all_strong(
            &self.document_types,
            &mut self.documents,
            skipped,
            transactions,
            payees,
            focus,
        )?;
        for entry in &batch.entries {
            self.record_document(entry.id);
        }
        Some(batch)
    }

    fn record_document(&mut self, id: DocumentId) {
        if let Some(document) = get(&self.documents, id) {
            self.pending
                .push(LedgerChange::UpsertDocument(Document::clone(document)));
        }
    }
}

/// [`Ledger::add_document`] as a value.
#[derive(Debug, Clone, PartialEq)]
pub struct AddDocument(pub NewDocument);

impl LedgerCommand for AddDocument {
    type Outcome = DocumentId;
    type Refusal = AddError;
    fn run(self, ledger: &mut Ledger) -> Result<DocumentId, AddError> {
        ledger.add_document(self.0)
    }
}

/// [`Ledger::edit_document`] as a value.
#[derive(Debug, Clone, PartialEq)]
pub struct EditDocument {
    pub id: DocumentId,
    pub edit: DocumentEdit,
}

impl LedgerCommand for EditDocument {
    type Outcome = ();
    type Refusal = AddError;
    fn run(self, ledger: &mut Ledger) -> Result<(), AddError> {
        ledger.edit_document(self.id, &self.edit)
    }
}

/// [`Ledger::set_document_facts`] as a value.
#[derive(Debug, Clone, PartialEq)]
pub struct SetDocumentFacts {
    pub id: DocumentId,
    pub facts: ExtractedFacts,
}

impl LedgerCommand for SetDocumentFacts {
    type Outcome = ();
    type Refusal = FilingError;
    fn run(self, ledger: &mut Ledger) -> Result<(), FilingError> {
        ledger.set_document_facts(self.id, self.facts)
    }
}

/// [`Ledger::import_documents`] as a value.
#[derive(Debug, Clone, PartialEq)]
pub struct ImportDocuments {
    pub lines: Vec<ImportLine>,
    pub today: NaiveDate,
    pub source: Source,
}

impl LedgerCommand for ImportDocuments {
    type Outcome = Vec<ImportOutcome>;
    type Refusal = Infallible;
    fn run(self, ledger: &mut Ledger) -> Result<Vec<ImportOutcome>, Infallible> {
        Ok(ledger.import_documents(&self.lines, self.today, self.source))
    }
}

/// [`Ledger::toggle_document_link`] as a value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ToggleDocumentLink {
    pub id: DocumentId,
    pub link: DocumentLink,
}

impl LedgerCommand for ToggleDocumentLink {
    type Outcome = bool;
    type Refusal = FilingError;
    fn run(self, ledger: &mut Ledger) -> Result<bool, FilingError> {
        ledger.toggle_document_link(self.id, self.link)
    }
}

/// [`Ledger::accept_document`] as a value, with the Transactions it matches against.
#[derive(Debug, Clone, PartialEq)]
pub struct AcceptDocument {
    pub id: DocumentId,
    pub transaction_id: TransactionId,
    pub transactions: Vec<DocumentTransaction>,
}

impl LedgerCommand for AcceptDocument {
    type Outcome = FiledEntry;
    type Refusal = FilingError;
    fn run(self, ledger: &mut Ledger) -> Result<FiledEntry, FilingError> {
        ledger.accept_document(self.id, self.transaction_id, &self.transactions)
    }
}

/// [`Ledger::file_document_by_hand`] as a value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FileDocumentByHand {
    pub id: DocumentId,
    pub doc_type: DocumentType,
    pub link: Option<DocumentLink>,
}

impl LedgerCommand for FileDocumentByHand {
    type Outcome = FiledEntry;
    type Refusal = FilingError;
    fn run(self, ledger: &mut Ledger) -> Result<FiledEntry, FilingError> {
        ledger.file_document_by_hand(self.id, self.doc_type, self.link)
    }
}

/// [`Ledger::undo_filing`] as a value.
#[derive(Debug, Clone, PartialEq)]
pub struct UndoFiling(pub FilingUndo);

impl LedgerCommand for UndoFiling {
    type Outcome = Option<DocumentId>;
    type Refusal = Infallible;
    fn run(self, ledger: &mut Ledger) -> Result<Option<DocumentId>, Infallible> {
        Ok(ledger.undo_filing(self.0))
    }
}

/// [`Ledger::accept_all_documents`] as a value, with the session's skipped Documents and the
/// Transactions and Payees it matches against.
#[derive(Debug, Clone, PartialEq)]
pub struct AcceptAllDocuments {
    pub focus: Option<DocumentId>,
    pub skipped: HashSet<DocumentId>,
    pub transactions: Vec<DocumentTransaction>,
    pub payees: Vec<DocumentPayee>,
}

impl LedgerCommand for AcceptAllDocuments {
    type Outcome = Option<FilingUndo>;
    type Refusal = Infallible;
    fn run(self, ledger: &mut Ledger) -> Result<Option<FilingUndo>, Infallible> {
        Ok(
            ledger.accept_all_documents(
                self.focus,
                &self.skipped,
                &self.transactions,
                &self.payees,
            ),
        )
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use lib_documents::{Filing, types::DocumentTypeRow};

    use super::*;

    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 10, 2).unwrap()
    }

    fn ledger() -> Ledger {
        Ledger::stub_on(today())
    }

    /// Everything a Documents command can write, to prove a refusal changed nothing.
    fn snapshot(ledger: &Ledger) -> (Vec<Document>, Vec<DocumentTypeRow>) {
        (
            ledger.documents().to_vec(),
            ledger.document_types().to_vec(),
        )
    }

    /// The Transactions as the caller reads them from their store, here the stub's.
    fn transactions(ledger: &Ledger) -> Vec<DocumentTransaction> {
        crate::document_reads::document_transactions(ledger.transactions())
    }

    fn accept_all(ledger: &Ledger) -> AcceptAllDocuments {
        AcceptAllDocuments {
            focus: None,
            skipped: HashSet::new(),
            transactions: transactions(ledger),
            payees: crate::document_reads::document_payees(ledger.payees()),
        }
    }

    fn unfiled(ledger: &Ledger) -> DocumentId {
        ledger
            .documents()
            .iter()
            .find(|document| document.is_unfiled())
            .expect("the stub Inbox has a Document")
            .id
    }

    fn filed(ledger: &Ledger) -> DocumentId {
        ledger
            .documents()
            .iter()
            .find(|document| document.is_filed())
            .expect("the stub Library has a Document")
            .id
    }

    fn new_document(name: &str) -> NewDocument {
        NewDocument {
            path: PathBuf::from(format!("/tmp/{name}.pdf")),
            bytes: 0,
            title: name.to_string(),
            doc_type: DocumentType::TAX,
            date: today(),
            key_date: None,
        }
    }

    /// Every recorded Document change names the row the Ledger now holds.
    fn assert_recorded_rows_match(ledger: &mut Ledger) {
        for change in ledger.drain_changes() {
            match change {
                LedgerChange::UpsertDocument(document) => {
                    assert_eq!(get(ledger.documents(), document.id), Some(&document));
                }
                other => panic!("a Documents command recorded {other:?}"),
            }
        }
    }

    #[test]
    fn adding_a_document_records_it_and_a_refused_add_changes_nothing() {
        let mut ledger = ledger();
        let id = AddDocument(new_document("lease")).run(&mut ledger).unwrap();
        assert_eq!(get(ledger.documents(), id).unwrap().title, "lease");
        assert_eq!(ledger.drain_changes().len(), 1);

        let before = snapshot(&ledger);
        let mut blank = new_document("blank");
        blank.title = "  ".to_string();
        assert_eq!(AddDocument(blank).run(&mut ledger), Err(AddError::NoTitle));
        let mut text = new_document("notes");
        text.path = PathBuf::from("/tmp/notes.txt");
        assert_eq!(
            AddDocument(text).run(&mut ledger),
            Err(AddError::UnsupportedFile)
        );
        assert_eq!(snapshot(&ledger), before);
        assert!(ledger.drain_changes().is_empty());
    }

    #[test]
    fn editing_a_document_records_it_and_a_refused_edit_changes_nothing() {
        let mut ledger = ledger();
        let id = filed(&ledger);
        let edit = DocumentEdit {
            title: " Renamed ".to_string(),
            doc_type: DocumentType::BILLS,
            date: today(),
            key_date: None,
        };
        EditDocument {
            id,
            edit: edit.clone(),
        }
        .run(&mut ledger)
        .unwrap();
        assert_eq!(get(ledger.documents(), id).unwrap().title, "Renamed");
        assert_recorded_rows_match(&mut ledger);

        let before = snapshot(&ledger);
        let refused = EditDocument {
            id: DocumentId::seed(9_999),
            edit: edit.clone(),
        }
        .run(&mut ledger);
        assert_eq!(refused, Err(AddError::NotFound));
        let blank = DocumentEdit {
            title: String::new(),
            ..edit
        };
        assert_eq!(
            EditDocument { id, edit: blank }.run(&mut ledger),
            Err(AddError::NoTitle)
        );
        assert_eq!(snapshot(&ledger), before);
        assert!(ledger.drain_changes().is_empty());
    }

    #[test]
    fn setting_facts_records_the_document_and_refuses_a_filed_one() {
        let mut ledger = ledger();
        let inbox = unfiled(&ledger);
        let facts = ExtractedFacts {
            merchant: Some("Coles".to_string()),
            ..ExtractedFacts::default()
        };
        SetDocumentFacts {
            id: inbox,
            facts: facts.clone(),
        }
        .run(&mut ledger)
        .unwrap();
        assert_recorded_rows_match(&mut ledger);

        let before = snapshot(&ledger);
        let refused = SetDocumentFacts {
            id: filed(&ledger),
            facts,
        }
        .run(&mut ledger);
        assert_eq!(refused, Err(FilingError::AlreadyFiled));
        assert_eq!(snapshot(&ledger), before);
        assert!(ledger.drain_changes().is_empty());
    }

    #[test]
    fn importing_records_only_the_documents_that_landed() {
        let mut ledger = ledger();
        let line = |typed: &str, exists| ImportLine {
            typed: typed.to_string(),
            path: PathBuf::from(typed),
            exists,
            bytes: 0,
        };
        let outcomes = ImportDocuments {
            lines: vec![
                line("/tmp/coles-receipt.jpg", true),
                line("/tmp/missing.pdf", false),
                line("/tmp/readme.txt", true),
            ],
            today: today(),
            source: Source::Import,
        }
        .run(&mut ledger)
        .unwrap();
        assert!(matches!(outcomes[0], ImportOutcome::Imported(_)));
        assert_eq!(
            outcomes[1],
            ImportOutcome::NotFound("/tmp/missing.pdf".into())
        );
        let changes = ledger.drain_changes();
        assert_eq!(changes.len(), 1);
        assert!(matches!(&changes[0], LedgerChange::UpsertDocument(d) if d.is_unfiled()));

        // An import that took nothing records nothing.
        let before = snapshot(&ledger);
        ImportDocuments {
            lines: vec![line("/tmp/missing.pdf", false)],
            today: today(),
            source: Source::Dropped,
        }
        .run(&mut ledger)
        .unwrap();
        assert_eq!(snapshot(&ledger), before);
        assert!(ledger.drain_changes().is_empty());
    }

    #[test]
    fn toggling_a_link_records_the_document_and_refuses_an_unknown_one() {
        let mut ledger = ledger();
        let id = filed(&ledger);
        let link = DocumentLink::Account(ledger.accounts()[0].id);
        let toggle = ToggleDocumentLink { id, link };
        let had = get(ledger.documents(), id).unwrap().links.contains(&link);
        assert_eq!(toggle.run(&mut ledger), Ok(!had));
        assert_eq!(toggle.run(&mut ledger), Ok(had));
        assert_eq!(ledger.drain_changes().len(), 2);

        let before = snapshot(&ledger);
        let refused = ToggleDocumentLink {
            id: DocumentId::seed(9_999),
            link,
        }
        .run(&mut ledger);
        assert_eq!(refused, Err(FilingError::NotFound));
        assert_eq!(snapshot(&ledger), before);
        assert!(ledger.drain_changes().is_empty());
    }

    #[test]
    fn filing_and_undoing_record_each_document_they_touch() {
        let mut ledger = ledger();
        let id = unfiled(&ledger);
        let transaction_id = ledger.transactions()[0].id;

        let entry = AcceptDocument {
            id,
            transaction_id,
            transactions: transactions(&ledger),
        }
        .run(&mut ledger)
        .unwrap();
        assert_eq!(get(ledger.documents(), id).unwrap().filing, Filing::Filed);
        assert_recorded_rows_match(&mut ledger);

        let focus = UndoFiling(FilingUndo {
            entries: vec![entry],
            focus: Some(id),
        })
        .run(&mut ledger)
        .unwrap();
        assert_eq!(focus, Some(id));
        assert!(get(ledger.documents(), id).unwrap().is_unfiled());
        assert_recorded_rows_match(&mut ledger);

        FileDocumentByHand {
            id,
            doc_type: DocumentType::OTHER,
            link: None,
        }
        .run(&mut ledger)
        .unwrap();
        assert!(get(ledger.documents(), id).unwrap().is_filed());
        assert_recorded_rows_match(&mut ledger);
    }

    #[test]
    fn a_refused_filing_changes_nothing() {
        let mut ledger = ledger();
        let before = snapshot(&ledger);
        let id = unfiled(&ledger);
        let missing_transaction = AcceptDocument {
            id,
            transaction_id: TransactionId::seed(9_999),
            transactions: transactions(&ledger),
        };
        assert_eq!(
            missing_transaction.run(&mut ledger),
            Err(FilingError::NoSuchTransaction)
        );
        let filed = filed(&ledger);
        assert_eq!(
            AcceptDocument {
                id: filed,
                transaction_id: ledger.transactions()[0].id,
                transactions: transactions(&ledger),
            }
            .run(&mut ledger),
            Err(FilingError::AlreadyFiled)
        );
        assert_eq!(
            FileDocumentByHand {
                id: DocumentId::seed(9_999),
                doc_type: DocumentType::OTHER,
                link: None,
            }
            .run(&mut ledger),
            Err(FilingError::NotFound)
        );
        assert_eq!(snapshot(&ledger), before);
        assert!(ledger.drain_changes().is_empty());
    }

    #[test]
    fn accept_all_records_every_document_it_filed() {
        let mut ledger = ledger();
        let batch = accept_all(&ledger).run(&mut ledger).unwrap();
        let changes = ledger.drain_changes();
        let filed = batch.map_or(0, |batch| batch.entries.len());
        assert_eq!(changes.len(), filed);

        // Nothing strong is left, so a second pass records nothing.
        assert_eq!(accept_all(&ledger).run(&mut ledger), Ok(None));
        assert!(ledger.drain_changes().is_empty());
    }
}
