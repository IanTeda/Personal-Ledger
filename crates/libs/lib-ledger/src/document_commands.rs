//! The Documents and Document Types writes (ADR-0038): each a command value with its own
//! refusal, checked before anything is written so a refusal leaves the Ledger untouched, and
//! recording an upsert or delete per row it wrote (ADR-0039).
//!
//! Single-Document rules stay in `lib_documents`. What lives here crosses domains or rows: Accept
//! reads the Transactions and Payees it matches against, and removing a Document Type moves its
//! Filed Documents to another type in the same command.

use std::convert::Infallible;

use chrono::NaiveDate;
use lib_documents::{
    AddError, Document, DocumentLink, DocumentType, ExtractedFacts, FiledEntry, FilingError,
    FilingUndo, ImportLine, ImportOutcome, KeyDate, NewDocument, Source, accept, accept_all_strong,
    add_filed, drop_inventory_links, edit_facts, edit_metadata, file_by_hand, get, import_batch,
    skip, toggle_link,
    types::{
        DocumentTypeDraft, DocumentTypeRow, RemoveError, add_type, edit_type, fallback_id,
        id_by_name, move_by, position, remove_type,
    },
    undo,
};

use crate::{Ledger, LedgerChange, LedgerCommand};

/// What the Library's `e` replaces on a Filed Document.
#[derive(Debug, Clone, PartialEq)]
pub struct DocumentEdit {
    pub title: String,
    pub doc_type: DocumentType,
    pub date: NaiveDate,
    pub key_date: Option<KeyDate>,
}

/// A Document Type that no longer exists, as an edit found it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnknownDocumentType;

impl Ledger {
    /// Adds a Filed Document and returns its id.
    pub fn add_document(&mut self, new: NewDocument) -> Result<u32, AddError> {
        let id = add_filed(&mut self.documents, new)?;
        self.record_document(id);
        Ok(id)
    }

    /// Replaces Document `id`'s Title, Document Type, date and Key Date.
    pub fn edit_document(&mut self, id: u32, edit: &DocumentEdit) -> Result<(), AddError> {
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
        id: u32,
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
        id: u32,
        link: DocumentLink,
    ) -> Result<bool, FilingError> {
        let added = toggle_link(&mut self.documents, id, link).ok_or(FilingError::NotFound)?;
        self.record_document(id);
        Ok(added)
    }

    /// Files Unfiled Document `id` under Transaction `transaction_id`.
    pub fn accept_document(
        &mut self,
        id: u32,
        transaction_id: u32,
    ) -> Result<FiledEntry, FilingError> {
        let entry = accept(
            &self.document_types,
            &mut self.documents,
            &self.transactions,
            id,
            transaction_id,
        )?;
        self.record_document(id);
        Ok(entry)
    }

    /// Files Unfiled Document `id` by hand with a Document Type and at most one Link.
    pub fn file_document_by_hand(
        &mut self,
        id: u32,
        doc_type: DocumentType,
        link: Option<DocumentLink>,
    ) -> Result<FiledEntry, FilingError> {
        let entry = file_by_hand(&mut self.documents, id, doc_type, link)?;
        self.record_document(id);
        Ok(entry)
    }

    /// Sets Unfiled Document `id` aside to the bottom of the Inbox.
    pub fn skip_document(&mut self, id: u32) -> Result<(), FilingError> {
        skip(&mut self.documents, id)?;
        self.record_document(id);
        Ok(())
    }

    /// Reverses a filing action and returns the Document focus goes back to.
    pub fn undo_filing(&mut self, record: FilingUndo) -> Option<u32> {
        let ids: Vec<u32> = record.entries.iter().map(|entry| entry.id).collect();
        let focus = undo(&mut self.documents, record);
        for id in ids {
            self.record_document(id);
        }
        focus
    }

    /// Accepts every Unfiled Document whose Suggested Link is Strong, as one undoable batch.
    /// `focus` is the Inbox row the Client keeps in view. Returns `None` when none was strong.
    pub fn accept_all_documents(&mut self, focus: Option<u32>) -> Option<FilingUndo> {
        let batch = accept_all_strong(
            &self.document_types,
            &mut self.documents,
            &self.transactions,
            &self.payees,
            focus,
        )?;
        for entry in &batch.entries {
            self.record_document(entry.id);
        }
        Some(batch)
    }

    /// Drops every Link to the given Inventory Items, as removing a Property does. Returns how
    /// many Documents lost a Link.
    pub fn drop_document_inventory_links(&mut self, removed_items: &[u32]) -> usize {
        let affected = drop_inventory_links(&mut self.documents, removed_items);
        for id in &affected {
            self.record_document(*id);
        }
        affected.len()
    }

    /// Adds a Document Type at the end of the order and returns its id. Ids only count up, so
    /// removing the newest type never frees its id for a saved `type:<id>` filter to match against
    /// a different type.
    pub fn add_document_type(&mut self, draft: &DocumentTypeDraft) -> u32 {
        let id = self.document_type_next_id;
        self.document_type_next_id += 1;
        add_type(&mut self.document_types, id, draft);
        self.record_document_type(id);
        id
    }

    /// Applies `draft` to Document Type `id`.
    pub fn edit_document_type(
        &mut self,
        id: u32,
        draft: &DocumentTypeDraft,
    ) -> Result<(), UnknownDocumentType> {
        if !edit_type(&mut self.document_types, id, draft) {
            return Err(UnknownDocumentType);
        }
        self.record_document_type(id);
        Ok(())
    }

    /// Removes Document Type `id`. Its Filed Documents move to the type named `destination`, or to
    /// the Default when none is named.
    pub fn remove_document_type(
        &mut self,
        id: u32,
        destination: Option<&str>,
    ) -> Result<(), RemoveError> {
        let moved_to = destination
            .and_then(|name| id_by_name(&self.document_types, name))
            .filter(|moved_to| *moved_to != id)
            .unwrap_or_else(|| fallback_id(&self.document_types));
        let before = self.document_types.clone();
        remove_type(&mut self.document_types, id, destination)?;
        self.pending.push(LedgerChange::DeleteDocumentType(id));
        // The destination's Filed count grew, so it is written too.
        for row in &self.document_types {
            if before.iter().any(|old| old.id == row.id && old != row) {
                self.pending
                    .push(LedgerChange::UpsertDocumentType(row.clone()));
            }
        }
        // The Documents follow the list: Filed files move with their type.
        let mut moved = Vec::new();
        for document in &mut self.documents {
            if document.doc_type == DocumentType(id) {
                document.doc_type = DocumentType(moved_to);
                moved.push(document.id);
            }
        }
        for id in moved {
            self.record_document(id);
        }
        Ok(())
    }

    /// Swaps Document Type `id` one place down (`delta` 1) or up (`delta` -1), stopping at either
    /// end. Returns whether it moved.
    pub fn move_document_type(&mut self, id: u32, delta: isize) -> bool {
        let Some(from) = position(&self.document_types, id) else {
            return false;
        };
        if !move_by(&mut self.document_types, id, delta) {
            return false;
        }
        // Both rows changed place, and their order is the user's.
        let to = from.saturating_add_signed(delta);
        for row in [from, to]
            .into_iter()
            .filter_map(|at| self.document_types.get(at).cloned())
        {
            self.pending.push(LedgerChange::UpsertDocumentType(row));
        }
        true
    }

    fn record_document(&mut self, id: u32) {
        if let Some(document) = get(&self.documents, id) {
            self.pending
                .push(LedgerChange::UpsertDocument(Document::clone(document)));
        }
    }

    fn record_document_type(&mut self, id: u32) {
        if let Some(row) = self.document_types.iter().find(|row| row.id == id) {
            self.pending
                .push(LedgerChange::UpsertDocumentType(DocumentTypeRow::clone(
                    row,
                )));
        }
    }
}

/// [`Ledger::add_document`] as a value.
#[derive(Debug, Clone, PartialEq)]
pub struct AddDocument(pub NewDocument);

impl LedgerCommand for AddDocument {
    type Outcome = u32;
    type Refusal = AddError;
    fn run(self, ledger: &mut Ledger) -> Result<u32, AddError> {
        ledger.add_document(self.0)
    }
}

/// [`Ledger::edit_document`] as a value.
#[derive(Debug, Clone, PartialEq)]
pub struct EditDocument {
    pub id: u32,
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
    pub id: u32,
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
    pub id: u32,
    pub link: DocumentLink,
}

impl LedgerCommand for ToggleDocumentLink {
    type Outcome = bool;
    type Refusal = FilingError;
    fn run(self, ledger: &mut Ledger) -> Result<bool, FilingError> {
        ledger.toggle_document_link(self.id, self.link)
    }
}

/// [`Ledger::accept_document`] as a value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AcceptDocument {
    pub id: u32,
    pub transaction_id: u32,
}

impl LedgerCommand for AcceptDocument {
    type Outcome = FiledEntry;
    type Refusal = FilingError;
    fn run(self, ledger: &mut Ledger) -> Result<FiledEntry, FilingError> {
        ledger.accept_document(self.id, self.transaction_id)
    }
}

/// [`Ledger::file_document_by_hand`] as a value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FileDocumentByHand {
    pub id: u32,
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

/// [`Ledger::skip_document`] as a value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SkipDocument(pub u32);

impl LedgerCommand for SkipDocument {
    type Outcome = ();
    type Refusal = FilingError;
    fn run(self, ledger: &mut Ledger) -> Result<(), FilingError> {
        ledger.skip_document(self.0)
    }
}

/// [`Ledger::undo_filing`] as a value.
#[derive(Debug, Clone, PartialEq)]
pub struct UndoFiling(pub FilingUndo);

impl LedgerCommand for UndoFiling {
    type Outcome = Option<u32>;
    type Refusal = Infallible;
    fn run(self, ledger: &mut Ledger) -> Result<Option<u32>, Infallible> {
        Ok(ledger.undo_filing(self.0))
    }
}

/// [`Ledger::accept_all_documents`] as a value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AcceptAllDocuments {
    pub focus: Option<u32>,
}

impl LedgerCommand for AcceptAllDocuments {
    type Outcome = Option<FilingUndo>;
    type Refusal = Infallible;
    fn run(self, ledger: &mut Ledger) -> Result<Option<FilingUndo>, Infallible> {
        Ok(ledger.accept_all_documents(self.focus))
    }
}

/// [`Ledger::drop_document_inventory_links`] as a value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DropDocumentInventoryLinks(pub Vec<u32>);

impl LedgerCommand for DropDocumentInventoryLinks {
    type Outcome = usize;
    type Refusal = Infallible;
    fn run(self, ledger: &mut Ledger) -> Result<usize, Infallible> {
        Ok(ledger.drop_document_inventory_links(&self.0))
    }
}

/// [`Ledger::add_document_type`] as a value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddDocumentType(pub DocumentTypeDraft);

impl LedgerCommand for AddDocumentType {
    type Outcome = u32;
    type Refusal = Infallible;
    fn run(self, ledger: &mut Ledger) -> Result<u32, Infallible> {
        Ok(ledger.add_document_type(&self.0))
    }
}

/// [`Ledger::edit_document_type`] as a value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditDocumentType {
    pub id: u32,
    pub draft: DocumentTypeDraft,
}

impl LedgerCommand for EditDocumentType {
    type Outcome = ();
    type Refusal = UnknownDocumentType;
    fn run(self, ledger: &mut Ledger) -> Result<(), UnknownDocumentType> {
        ledger.edit_document_type(self.id, &self.draft)
    }
}

/// [`Ledger::remove_document_type`] as a value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoveDocumentType {
    pub id: u32,
    pub destination: Option<String>,
}

impl LedgerCommand for RemoveDocumentType {
    type Outcome = ();
    type Refusal = RemoveError;
    fn run(self, ledger: &mut Ledger) -> Result<(), RemoveError> {
        ledger.remove_document_type(self.id, self.destination.as_deref())
    }
}

/// [`Ledger::move_document_type`] as a value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MoveDocumentType {
    pub id: u32,
    pub delta: isize,
}

impl LedgerCommand for MoveDocumentType {
    type Outcome = bool;
    type Refusal = Infallible;
    fn run(self, ledger: &mut Ledger) -> Result<bool, Infallible> {
        Ok(ledger.move_document_type(self.id, self.delta))
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use lib_documents::{Filing, types::default_types};

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

    fn unfiled(ledger: &Ledger) -> u32 {
        ledger
            .documents()
            .iter()
            .find(|document| document.is_unfiled())
            .expect("the stub Inbox has a Document")
            .id
    }

    fn filed(ledger: &Ledger) -> u32 {
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
            title: name.to_string(),
            doc_type: DocumentType::TAX,
            date: today(),
            key_date: None,
        }
    }

    fn draft(name: &str) -> DocumentTypeDraft {
        DocumentTypeDraft {
            name: name.to_string(),
            tracks_date: None,
            remind: None,
            financial_year: false,
        }
    }

    /// Every recorded Document change names the row the Ledger now holds.
    fn assert_recorded_rows_match(ledger: &mut Ledger) {
        for change in ledger.drain_changes() {
            match change {
                LedgerChange::UpsertDocument(document) => {
                    assert_eq!(get(ledger.documents(), document.id), Some(&document));
                }
                LedgerChange::UpsertDocumentType(row) => {
                    assert!(ledger.document_types().contains(&row));
                }
                LedgerChange::DeleteDocumentType(id) => {
                    assert!(ledger.document_types().iter().all(|row| row.id != id));
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
            id: 9_999,
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
            exists,
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
        let refused = ToggleDocumentLink { id: 9_999, link }.run(&mut ledger);
        assert_eq!(refused, Err(FilingError::NotFound));
        assert_eq!(snapshot(&ledger), before);
        assert!(ledger.drain_changes().is_empty());
    }

    #[test]
    fn filing_skipping_and_undoing_record_each_document_they_touch() {
        let mut ledger = ledger();
        let id = unfiled(&ledger);
        let transaction_id = ledger.transactions()[0].id;

        SkipDocument(id).run(&mut ledger).unwrap();
        assert_recorded_rows_match(&mut ledger);

        let entry = AcceptDocument { id, transaction_id }
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
            transaction_id: 9_999,
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
            }
            .run(&mut ledger),
            Err(FilingError::AlreadyFiled)
        );
        assert_eq!(
            SkipDocument(filed).run(&mut ledger),
            Err(FilingError::AlreadyFiled)
        );
        assert_eq!(
            FileDocumentByHand {
                id: 9_999,
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
        let batch = AcceptAllDocuments { focus: None }.run(&mut ledger).unwrap();
        let changes = ledger.drain_changes();
        let filed = batch.map_or(0, |batch| batch.entries.len());
        assert_eq!(changes.len(), filed);

        // Nothing strong is left, so a second pass records nothing.
        assert_eq!(
            AcceptAllDocuments { focus: None }.run(&mut ledger),
            Ok(None)
        );
        assert!(ledger.drain_changes().is_empty());
    }

    #[test]
    fn dropping_inventory_links_records_the_documents_that_lost_one() {
        let mut ledger = ledger();
        let items: Vec<u32> = ledger
            .documents()
            .iter()
            .flat_map(|document| &document.links)
            .filter_map(|link| match link {
                DocumentLink::InventoryItem(id) => Some(*id),
                _ => None,
            })
            .collect();
        assert!(!items.is_empty(), "the stub links a Document to an Item");
        let lost = DropDocumentInventoryLinks(items).run(&mut ledger).unwrap();
        assert!(lost > 0);
        let changes = ledger.drain_changes();
        assert_eq!(changes.len(), lost);

        let before = snapshot(&ledger);
        DropDocumentInventoryLinks(vec![9_999])
            .run(&mut ledger)
            .unwrap();
        assert_eq!(snapshot(&ledger), before);
        assert!(ledger.drain_changes().is_empty());
    }

    #[test]
    fn a_document_type_id_is_never_reused() {
        let mut ledger = ledger();
        let first = AddDocumentType(draft("Warranties"))
            .run(&mut ledger)
            .unwrap();
        RemoveDocumentType {
            id: first,
            destination: None,
        }
        .run(&mut ledger)
        .unwrap();
        ledger.drain_changes();
        let second = AddDocumentType(draft("Manuals")).run(&mut ledger).unwrap();
        assert!(second > first);
        assert_recorded_rows_match(&mut ledger);
    }

    #[test]
    fn editing_a_document_type_records_it_and_refuses_an_unknown_one() {
        let mut ledger = ledger();
        EditDocumentType {
            id: 3,
            draft: draft(" Renamed "),
        }
        .run(&mut ledger)
        .unwrap();
        assert_eq!(
            ledger
                .document_types()
                .iter()
                .find(|row| row.id == 3)
                .unwrap()
                .name,
            "Renamed"
        );
        assert_recorded_rows_match(&mut ledger);

        let before = snapshot(&ledger);
        let refused = EditDocumentType {
            id: 9_999,
            draft: draft("Nope"),
        }
        .run(&mut ledger);
        assert_eq!(refused, Err(UnknownDocumentType));
        assert_eq!(snapshot(&ledger), before);
        assert!(ledger.drain_changes().is_empty());
    }

    #[test]
    fn removing_a_document_type_moves_its_documents_and_records_every_row() {
        let mut ledger = ledger();
        let doc = filed(&ledger);
        EditDocument {
            id: doc,
            edit: DocumentEdit {
                title: "Held".to_string(),
                doc_type: DocumentType::TAX,
                date: today(),
                key_date: None,
            },
        }
        .run(&mut ledger)
        .unwrap();
        ledger.drain_changes();

        RemoveDocumentType {
            id: DocumentType::TAX.0,
            destination: Some("Bills".to_string()),
        }
        .run(&mut ledger)
        .unwrap();
        assert_eq!(
            get(ledger.documents(), doc).unwrap().doc_type,
            DocumentType::BILLS
        );
        let changes = ledger.drain_changes();
        assert!(changes.contains(&LedgerChange::DeleteDocumentType(DocumentType::TAX.0)));
        assert!(changes.iter().any(
            |change| matches!(change, LedgerChange::UpsertDocumentType(row) if row.id == DocumentType::BILLS.0)
        ));
        assert!(
            changes
                .iter()
                .any(|change| matches!(change, LedgerChange::UpsertDocument(d) if d.id == doc))
        );
    }

    #[test]
    fn a_refused_remove_changes_nothing() {
        let mut ledger = ledger();
        let before = snapshot(&ledger);
        let remove = |id, destination: Option<&str>| RemoveDocumentType {
            id,
            destination: destination.map(str::to_string),
        };
        assert_eq!(
            remove(DocumentType::OTHER.0, Some("Bills")).run(&mut ledger),
            Err(RemoveError::IsDefault)
        );
        assert_eq!(
            remove(DocumentType::TAX.0, None).run(&mut ledger),
            Err(RemoveError::NeedsDestination)
        );
        assert_eq!(
            remove(9_999, None).run(&mut ledger),
            Err(RemoveError::Unknown)
        );
        assert_eq!(snapshot(&ledger), before);
        assert!(ledger.drain_changes().is_empty());
    }

    #[test]
    fn moving_a_document_type_records_both_rows_and_stops_at_the_ends() {
        let mut ledger = ledger();
        let first = default_types()[0].id;
        assert_eq!(
            MoveDocumentType {
                id: first,
                delta: -1
            }
            .run(&mut ledger),
            Ok(false)
        );
        assert!(ledger.drain_changes().is_empty());

        assert_eq!(
            MoveDocumentType {
                id: first,
                delta: 1
            }
            .run(&mut ledger),
            Ok(true)
        );
        assert_eq!(ledger.document_types()[1].id, first);
        assert_eq!(ledger.drain_changes().len(), 2);
    }
}
