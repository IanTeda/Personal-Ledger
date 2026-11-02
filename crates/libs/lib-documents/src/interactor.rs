//! The Documents intents, one Interactor each: it reads the Documents and Document Types from the
//! [`DocumentRepository`], applies the rule this crate holds on a copy, and writes each Document or
//! type the rule added or changed. A refusal writes nothing: the rule runs on the copy, and nothing
//! is written until it lands.
//!
//! Filing writes only Documents, so it is here rather than in a cross-domain module: Accept reads
//! the Transactions through [`DocumentTransactions`], Accept all strong reads the Payees too, and
//! Import reaches the disk through [`DocumentFiles`]. Skip has no Interactor: it is the Client's
//! session state and writes nothing.

use std::collections::HashSet;
use std::convert::Infallible;

use chrono::NaiveDate;
use lib_core::{DocumentId, DocumentTypeId, TransactionId};

use crate::protocols::{DocumentFiles, DocumentReads, DocumentRepository, DocumentTransactions};
use crate::types::{
    DocumentTypeDraft, DocumentTypeRow, RemoveError, UnknownDocumentType, add_type, edit_type,
    fallback_id, id_by_name, move_by, position, remove_type,
};
use crate::{
    AddError, Document, DocumentLink, DocumentType, ExtractedFacts, FiledEntry, FilingError,
    FilingUndo, ImportOutcome, KeyDate, NewDocument, Source, accept, accept_all_strong, add_filed,
    edit_facts, edit_metadata, file_by_hand, get, import_batch, import_lines, toggle_link, undo,
};

/// A refusal as a log line carries it: the kind alone, never a Title or a type's name (ADR-0047).
trait Refusal {
    fn reason(&self) -> &'static str;
}

impl Refusal for AddError {
    fn reason(&self) -> &'static str {
        match self {
            AddError::UnsupportedFile => "unsupported file",
            AddError::AlreadyInLibrary(_) => "already in the Library",
            AddError::NotFound => "no such Document",
            AddError::NoTitle => "blank Title",
        }
    }
}

impl Refusal for FilingError {
    fn reason(&self) -> &'static str {
        match self {
            FilingError::NotFound => "no such Document",
            FilingError::AlreadyFiled => "already Filed",
            FilingError::NoSuchTransaction => "no such Transaction",
        }
    }
}

impl Refusal for RemoveError {
    fn reason(&self) -> &'static str {
        match self {
            RemoveError::Unknown => "no such Document Type",
            RemoveError::IsDefault => "the Default Document Type",
            RemoveError::NeedsDestination => "has Filed Documents and no destination",
        }
    }
}

impl Refusal for UnknownDocumentType {
    fn reason(&self) -> &'static str {
        "no such Document Type"
    }
}

/// Logs how an intent on Document `id` ended and hands the outcome back.
fn logged_document<R, E: Refusal>(
    intent: &str,
    id: DocumentId,
    outcome: Result<R, E>,
) -> Result<R, E> {
    match &outcome {
        Ok(_) => tracing::info!(intent, document_id = %id, "Documents intent done"),
        Err(refusal) => tracing::info!(
            intent,
            document_id = %id,
            refusal = refusal.reason(),
            "Documents intent refused"
        ),
    }
    outcome
}

/// Logs how an intent on Document Type `id` ended and hands the outcome back.
fn logged_type<R, E: Refusal>(
    intent: &str,
    id: DocumentTypeId,
    outcome: Result<R, E>,
) -> Result<R, E> {
    match &outcome {
        Ok(_) => tracing::info!(intent, document_type_id = %id, "Documents intent done"),
        Err(refusal) => tracing::info!(
            intent,
            document_type_id = %id,
            refusal = refusal.reason(),
            "Documents intent refused"
        ),
    }
    outcome
}

/// Runs `rule` on a copy of the Documents and writes each Document it added or changed.
fn write_documents<R, E>(
    repo: &mut dyn DocumentRepository,
    rule: impl FnOnce(&[DocumentTypeRow], &mut Vec<Document>) -> Result<R, E>,
) -> Result<R, E> {
    let types = repo.document_types();
    let before = repo.documents();
    let mut after = before.clone();
    let result = rule(&types, &mut after)?;
    for document in &after {
        if get(&before, document.id) != Some(document) {
            repo.upsert_document(document);
        }
    }
    Ok(result)
}

/// `+ Add`: a Filed Document from a path the user already knows the meaning of. Returns its id.
#[derive(Debug, Clone, PartialEq)]
pub struct AddDocument(pub NewDocument);

impl AddDocument {
    pub fn run(self, repo: &mut dyn DocumentRepository) -> Result<DocumentId, AddError> {
        let outcome = write_documents(repo, |_, documents| add_filed(documents, self.0));
        match &outcome {
            Ok(id) => tracing::info!(document_id = %id, "Added a Document"),
            Err(refusal) => {
                tracing::info!(refusal = refusal.reason(), "Refused to add a Document");
            }
        }
        outcome
    }
}

/// `Import…` and a drop: each typed path, resolved and looked up through [`DocumentFiles`], into
/// the Inbox when it can become a Document. Reports what became of every path.
#[derive(Debug, Clone, PartialEq)]
pub struct ImportDocuments {
    pub typed: Vec<String>,
    pub today: NaiveDate,
    pub source: Source,
}

impl ImportDocuments {
    pub fn run(
        self,
        files: &dyn DocumentFiles,
        repo: &mut dyn DocumentRepository,
    ) -> Vec<ImportOutcome> {
        let lines = import_lines(files, self.typed.iter().map(String::as_str));
        let Ok(outcomes) = write_documents::<_, Infallible>(repo, |types, documents| {
            Ok(import_batch(
                types,
                documents,
                &lines,
                self.today,
                self.source,
            ))
        });
        let imported = outcomes
            .iter()
            .filter(|outcome| matches!(outcome, ImportOutcome::Imported(_)))
            .count();
        tracing::info!(
            imported,
            refused = outcomes.len() - imported,
            "Imported Documents"
        );
        outcomes
    }
}

/// The Library's `e`: replaces Filed Document `id`'s Title, Document Type, date and Key Date.
#[derive(Debug, Clone, PartialEq)]
pub struct EditDocument {
    pub id: DocumentId,
    pub title: String,
    pub doc_type: DocumentType,
    pub date: NaiveDate,
    pub key_date: Option<KeyDate>,
}

impl EditDocument {
    pub fn run(self, repo: &mut dyn DocumentRepository) -> Result<(), AddError> {
        logged_document(
            "edit document",
            self.id,
            write_documents(repo, |_, documents| {
                edit_metadata(
                    documents,
                    self.id,
                    &self.title,
                    self.doc_type,
                    self.date,
                    self.key_date,
                )
            }),
        )
    }
}

/// The Inbox's Edit: overwrites Unfiled Document `id`'s Extracted Facts.
#[derive(Debug, Clone, PartialEq)]
pub struct SetFacts {
    pub id: DocumentId,
    pub facts: ExtractedFacts,
}

impl SetFacts {
    pub fn run(self, repo: &mut dyn DocumentRepository) -> Result<(), FilingError> {
        logged_document(
            "set facts",
            self.id,
            write_documents(repo, |_, documents| {
                edit_facts(documents, self.id, self.facts)
            }),
        )
    }
}

/// Adds `link` to Document `id`, or removes it when it has it. Returns whether it was added.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ToggleLink {
    pub id: DocumentId,
    pub link: DocumentLink,
}

impl ToggleLink {
    pub fn run(self, repo: &mut dyn DocumentRepository) -> Result<bool, FilingError> {
        logged_document(
            "toggle link",
            self.id,
            write_documents(repo, |_, documents| {
                toggle_link(documents, self.id, self.link).ok_or(FilingError::NotFound)
            }),
        )
    }
}

/// Accept: files Unfiled Document `id` under Transaction `transaction_id`, with the extracted
/// Document Type and date. Returns what [`UndoFiling`] needs to reverse it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Accept {
    pub id: DocumentId,
    pub transaction_id: TransactionId,
}

impl Accept {
    pub fn run(
        self,
        transactions: &dyn DocumentTransactions,
        repo: &mut dyn DocumentRepository,
    ) -> Result<FiledEntry, FilingError> {
        let transactions = transactions.document_transactions();
        logged_document(
            "accept",
            self.id,
            write_documents(repo, |types, documents| {
                accept(
                    types,
                    documents,
                    &transactions,
                    self.id,
                    self.transaction_id,
                )
            }),
        )
    }
}

/// File… / Link elsewhere…: files Unfiled Document `id` by hand with a Document Type and at most
/// one Link.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FileByHand {
    pub id: DocumentId,
    pub doc_type: DocumentType,
    pub link: Option<DocumentLink>,
}

impl FileByHand {
    pub fn run(self, repo: &mut dyn DocumentRepository) -> Result<FiledEntry, FilingError> {
        logged_document(
            "file by hand",
            self.id,
            write_documents(repo, |_, documents| {
                file_by_hand(documents, self.id, self.doc_type, self.link)
            }),
        )
    }
}

/// Accept all strong matches: accepts every Unfiled Document whose Suggested Link is Strong, as
/// one undoable batch, in the Inbox's order with the `skipped` last. `focus` is the Inbox row the
/// Client keeps in view. Returns `None` when none was Strong.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AcceptAllStrong {
    pub focus: Option<DocumentId>,
    pub skipped: HashSet<DocumentId>,
}

impl AcceptAllStrong {
    pub fn run(
        self,
        reads: &dyn DocumentReads,
        repo: &mut dyn DocumentRepository,
    ) -> Option<FilingUndo> {
        let transactions = reads.document_transactions();
        let payees = reads.document_payees();
        let Ok(batch) = write_documents::<_, Infallible>(repo, |types, documents| {
            Ok(accept_all_strong(
                types,
                documents,
                &self.skipped,
                &transactions,
                &payees,
                self.focus,
            ))
        });
        let accepted = batch.as_ref().map_or(0, |batch| batch.entries.len());
        tracing::info!(accepted, "Accepted every Strong match");
        batch
    }
}

/// Reverses a filing action: each Document it filed returns to Unfiled with its earlier Document
/// Type and date, without the Links the action added. Returns the Document focus goes back to.
#[derive(Debug, Clone, PartialEq)]
pub struct UndoFiling(pub FilingUndo);

impl UndoFiling {
    pub fn run(self, repo: &mut dyn DocumentRepository) -> Option<DocumentId> {
        let documents = self.0.entries.len();
        let Ok(focus) = write_documents::<_, Infallible>(repo, |_, all| Ok(undo(all, self.0)));
        tracing::info!(documents, "Undid a filing");
        focus
    }
}

/// Adds a Document Type at the end of the order and returns its id, a fresh one so removing the
/// newest type never frees its id for a saved `type:<id>` filter to match a different type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddDocumentType(pub DocumentTypeDraft);

impl AddDocumentType {
    pub fn run(self, repo: &mut dyn DocumentRepository) -> DocumentTypeId {
        let id = DocumentTypeId::new();
        let mut types = repo.document_types();
        add_type(&mut types, id, &self.0);
        if let Some(row) = types.last() {
            repo.upsert_document_type(row);
        }
        tracing::info!(document_type_id = %id, "Added a Document Type");
        id
    }
}

/// Applies `draft` to Document Type `id`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditDocumentType {
    pub id: DocumentTypeId,
    pub draft: DocumentTypeDraft,
}

impl EditDocumentType {
    pub fn run(self, repo: &mut dyn DocumentRepository) -> Result<(), UnknownDocumentType> {
        let mut types = repo.document_types();
        let outcome = if edit_type(&mut types, self.id, &self.draft) {
            if let Some(row) = types.iter().find(|row| row.id == self.id) {
                repo.upsert_document_type(row);
            }
            Ok(())
        } else {
            Err(UnknownDocumentType)
        };
        logged_type("edit document type", self.id, outcome)
    }
}

/// Removes Document Type `id`. Its Documents move to the type named `destination`, or to the
/// Default when none is named, in the same write, and the destination's Filed count grows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoveDocumentType {
    pub id: DocumentTypeId,
    pub destination: Option<String>,
}

impl RemoveDocumentType {
    pub fn run(self, repo: &mut dyn DocumentRepository) -> Result<(), RemoveError> {
        let outcome = self.apply(repo);
        logged_type("remove document type", self.id, outcome)
    }

    fn apply(&self, repo: &mut dyn DocumentRepository) -> Result<(), RemoveError> {
        let before = repo.document_types();
        let moved_to = self
            .destination
            .as_deref()
            .and_then(|name| id_by_name(&before, name))
            .filter(|moved_to| *moved_to != self.id)
            .unwrap_or_else(|| fallback_id(&before));
        let mut after = before.clone();
        remove_type(&mut after, self.id, self.destination.as_deref())?;
        repo.delete_document_type(self.id);
        for row in after.iter().filter(|row| !before.contains(row)) {
            repo.upsert_document_type(row);
        }
        let Ok(()) = write_documents::<_, Infallible>(repo, |_, documents| {
            for document in documents
                .iter_mut()
                .filter(|document| document.doc_type == DocumentType(self.id))
            {
                document.doc_type = DocumentType(moved_to);
            }
            Ok(())
        });
        Ok(())
    }
}

/// Swaps Document Type `id` one place down (`delta` 1) or up (`delta` -1), stopping at either
/// end. Returns whether it moved.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MoveDocumentType {
    pub id: DocumentTypeId,
    pub delta: isize,
}

impl MoveDocumentType {
    pub fn run(self, repo: &mut dyn DocumentRepository) -> bool {
        let mut types = repo.document_types();
        let moved = move_by(&mut types, self.id, self.delta);
        if let Some(to) = moved.then(|| position(&types, self.id)).flatten() {
            repo.move_document_type(self.id, to);
        }
        tracing::info!(document_type_id = %self.id, moved, "Moved a Document Type");
        moved
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use bigdecimal::BigDecimal;
    use lib_core::{AccountId, Money, PayeeId};

    use super::*;
    use crate::fake::{InMemoryDocuments, InMemoryFiles};
    use crate::protocols::{DocumentPayee, DocumentTransaction, LinkRecords};
    use crate::types::default_types;
    use crate::{FileKind, Filing, Intake};

    const RECEIPTS: DocumentType = DocumentType(DocumentTypeId::seed(1));
    const TAX: DocumentType = DocumentType(DocumentTypeId::seed(3));
    const OTHER: DocumentType = DocumentType(DocumentTypeId::seed(9));

    const LEASE: DocumentId = DocumentId::seed(1);
    const WOOLWORTHS_RECEIPT: DocumentId = DocumentId::seed(2);
    const UNMATCHED: DocumentId = DocumentId::seed(3);
    const COLES_RECEIPT: DocumentId = DocumentId::seed(4);

    const WOOLWORTHS_SHOP: TransactionId = TransactionId::seed(1);
    const COLES_SHOP: TransactionId = TransactionId::seed(2);

    /// One write the repository was asked for.
    #[derive(Debug, Clone, PartialEq)]
    enum Write {
        Document(DocumentId),
        Type(DocumentTypeId),
        DeleteType(DocumentTypeId),
        MoveType(DocumentTypeId, usize),
    }

    /// The in-memory repository, recording each write in order.
    #[derive(Debug, Clone, Default)]
    struct Recording {
        repo: InMemoryDocuments,
        writes: Vec<Write>,
    }

    impl DocumentRepository for Recording {
        fn documents(&self) -> Vec<Document> {
            self.repo.documents()
        }

        fn document_types(&self) -> Vec<DocumentTypeRow> {
            self.repo.document_types()
        }

        fn upsert_document(&mut self, document: &Document) {
            self.writes.push(Write::Document(document.id));
            self.repo.upsert_document(document);
        }

        fn upsert_document_type(&mut self, row: &DocumentTypeRow) {
            self.writes.push(Write::Type(row.id));
            self.repo.upsert_document_type(row);
        }

        fn delete_document_type(&mut self, id: DocumentTypeId) {
            self.writes.push(Write::DeleteType(id));
            self.repo.delete_document_type(id);
        }

        fn move_document_type(&mut self, id: DocumentTypeId, to: usize) {
            self.writes.push(Write::MoveType(id, to));
            self.repo.move_document_type(id, to);
        }
    }

    impl Recording {
        fn document(&self, id: DocumentId) -> Document {
            get(&self.repo.documents, id).cloned().expect("a Document")
        }

        fn type_row(&self, id: DocumentTypeId) -> Option<&DocumentTypeRow> {
            self.repo.types.iter().find(|row| row.id == id)
        }

        /// Runs `refused`, which must refuse, and checks it wrote nothing.
        fn refuses<R: std::fmt::Debug, E: std::fmt::Debug + PartialEq>(
            &mut self,
            expected: E,
            refused: impl FnOnce(&mut Self) -> Result<R, E>,
        ) {
            let before = self.repo.clone();
            assert_eq!(refused(self).expect_err("a refusal"), expected);
            assert_eq!(self.repo, before);
            assert!(self.writes.is_empty(), "a refusal wrote {:?}", self.writes);
        }
    }

    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 10, 2).expect("a real date")
    }

    fn money(cents: i64) -> Money {
        Money(BigDecimal::new(cents.into(), 2))
    }

    fn document(id: DocumentId, path: &str, doc_type: DocumentType, filing: Filing) -> Document {
        Document {
            id,
            path: PathBuf::from(path),
            kind: FileKind::from_path(std::path::Path::new(path)).expect("a Document's file"),
            pages: 1,
            bytes: 1_024,
            title: path.to_string(),
            doc_type,
            date: today() - chrono::Duration::days(30),
            key_date: None,
            links: Vec::new(),
            extracted_text: None,
            filing,
            intake: None,
        }
    }

    /// An Unfiled receipt read as `cents` at `merchant`, `days_ago`.
    fn receipt(
        id: DocumentId,
        path: &str,
        merchant: Option<&str>,
        cents: i64,
        days_ago: i64,
    ) -> Document {
        Document {
            intake: Some(Intake {
                source: Source::Scanned,
                received_at: today(),
                facts: ExtractedFacts {
                    merchant: merchant.map(str::to_string),
                    date: Some(today() - chrono::Duration::days(days_ago)),
                    total: Some(money(cents)),
                    doc_type: Some(RECEIPTS),
                },
            }),
            date: today(),
            ..document(id, path, OTHER, Filing::Unfiled)
        }
    }

    /// A Filed lease, and three Unfiled receipts: two Strong matches, one matching nothing.
    fn ports() -> Recording {
        Recording {
            repo: InMemoryDocuments {
                documents: vec![
                    document(LEASE, "/work/lease.pdf", TAX, Filing::Filed),
                    receipt(
                        WOOLWORTHS_RECEIPT,
                        "woolworths.jpg",
                        Some("Woolworths"),
                        4_250,
                        0,
                    ),
                    receipt(UNMATCHED, "unknown.jpg", None, 9_999, 0),
                    receipt(COLES_RECEIPT, "coles.jpg", Some("Coles"), 1_000, 1),
                ],
                types: default_types(),
            },
            writes: Vec::new(),
        }
    }

    fn shop(id: TransactionId, payee: u32, cents: i64, days_ago: i64) -> DocumentTransaction {
        DocumentTransaction {
            id,
            date: today() - chrono::Duration::days(days_ago),
            amount: money(-cents),
            account_id: AccountId::seed(1),
            payee_ids: vec![Some(PayeeId::seed(payee))],
            memo: None,
        }
    }

    fn payee(id: u32, name: &str) -> DocumentPayee {
        DocumentPayee {
            id: PayeeId::seed(id),
            name: name.to_string(),
            aliases: Vec::new(),
            is_active: true,
        }
    }

    fn records() -> LinkRecords {
        LinkRecords {
            transactions: vec![
                shop(WOOLWORTHS_SHOP, 1, 4_250, 0),
                shop(COLES_SHOP, 2, 1_000, 1),
            ],
            payees: vec![payee(1, "Woolworths"), payee(2, "Coles")],
            ..LinkRecords::default()
        }
    }

    fn new_document(path: &str, title: &str) -> NewDocument {
        NewDocument {
            path: PathBuf::from(path),
            bytes: 2_048,
            title: title.to_string(),
            doc_type: TAX,
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

    #[test]
    fn add_writes_a_filed_document() {
        let mut ports = ports();

        let id = AddDocument(new_document("/tmp/will.pdf", " Will "))
            .run(&mut ports)
            .expect("a new path");

        assert_eq!(ports.writes, [Write::Document(id)]);
        let added = ports.document(id);
        assert_eq!(added.title, "Will");
        assert_eq!(added.filing, Filing::Filed);
        assert_eq!(added.bytes, 2_048);
    }

    #[test]
    fn add_refuses_a_blank_title_a_non_document_and_a_held_path() {
        let mut ports = ports();

        ports.refuses(AddError::NoTitle, |ports| {
            AddDocument(new_document("/tmp/will.pdf", "  ")).run(ports)
        });
        ports.refuses(AddError::UnsupportedFile, |ports| {
            AddDocument(new_document("/tmp/notes.txt", "Notes")).run(ports)
        });
        ports.refuses(
            AddError::AlreadyInLibrary("/work/lease.pdf".into()),
            |ports| AddDocument(new_document("/work/lease.pdf", "Lease again")).run(ports),
        );
    }

    #[test]
    fn import_reaches_the_disk_through_the_files_port_and_reports_every_path() {
        let mut ports = ports();
        let files = InMemoryFiles::default().with("/work/coles-receipt.jpg", 3_000);

        let outcomes = ImportDocuments {
            typed: vec![
                "coles-receipt.jpg".into(),
                "missing.pdf".into(),
                "/work/notes.txt".into(),
                "lease.pdf".into(),
            ],
            today: today(),
            source: Source::Import,
        }
        .run(&files, &mut ports);

        let ImportOutcome::Imported(id) = outcomes[0] else {
            panic!("the file on disk imports, got {outcomes:?}");
        };
        assert_eq!(
            outcomes[1..],
            [
                ImportOutcome::NotFound("missing.pdf".into()),
                ImportOutcome::Unsupported("/work/notes.txt".into()),
                ImportOutcome::Already("/work/lease.pdf".into()),
            ]
        );
        assert_eq!(ports.writes, [Write::Document(id)]);
        let imported = ports.document(id);
        assert_eq!(imported.filing, Filing::Unfiled);
        assert_eq!(imported.bytes, 3_000);
        assert_eq!(imported.path, PathBuf::from("/work/coles-receipt.jpg"));
    }

    #[test]
    fn an_import_with_nothing_on_disk_writes_nothing() {
        let mut ports = ports();
        let before = ports.repo.clone();

        let outcomes = ImportDocuments {
            typed: vec!["missing.pdf".into()],
            today: today(),
            source: Source::Dropped,
        }
        .run(&InMemoryFiles::default(), &mut ports);

        assert_eq!(outcomes, [ImportOutcome::NotFound("missing.pdf".into())]);
        assert_eq!(ports.repo, before);
        assert!(ports.writes.is_empty());
    }

    fn edit(id: DocumentId, title: &str) -> EditDocument {
        EditDocument {
            id,
            title: title.to_string(),
            doc_type: RECEIPTS,
            date: today(),
            key_date: None,
        }
    }

    #[test]
    fn edit_rewrites_the_documents_metadata() {
        let mut ports = ports();

        edit(LEASE, " Lease 2026 ")
            .run(&mut ports)
            .expect("a Document");

        assert_eq!(ports.writes, [Write::Document(LEASE)]);
        let edited = ports.document(LEASE);
        assert_eq!(
            (edited.title.as_str(), edited.doc_type, edited.date),
            ("Lease 2026", RECEIPTS, today())
        );
    }

    #[test]
    fn edit_refuses_a_blank_title_and_an_unknown_document() {
        let mut ports = ports();

        ports.refuses(AddError::NoTitle, |ports| edit(LEASE, " ").run(ports));
        ports.refuses(AddError::NotFound, |ports| {
            edit(DocumentId::seed(99), "Gone").run(ports)
        });
    }

    #[test]
    fn set_facts_overwrites_an_unfiled_documents_facts() {
        let mut ports = ports();
        let facts = ExtractedFacts {
            merchant: Some("Aldi".into()),
            ..ExtractedFacts::default()
        };

        SetFacts {
            id: UNMATCHED,
            facts: facts.clone(),
        }
        .run(&mut ports)
        .expect("an Unfiled Document");

        assert_eq!(ports.writes, [Write::Document(UNMATCHED)]);
        let intake = ports.document(UNMATCHED).intake.expect("an Intake");
        assert_eq!(intake.facts, facts);
    }

    #[test]
    fn set_facts_refuses_a_filed_and_an_unknown_document() {
        let mut ports = ports();
        let set = |id| SetFacts {
            id,
            facts: ExtractedFacts::default(),
        };

        ports.refuses(FilingError::AlreadyFiled, |ports| set(LEASE).run(ports));
        ports.refuses(FilingError::NotFound, |ports| {
            set(DocumentId::seed(99)).run(ports)
        });
    }

    #[test]
    fn toggle_link_adds_a_link_then_removes_it() {
        let mut ports = ports();
        let toggle = ToggleLink {
            id: LEASE,
            link: DocumentLink::Payee(PayeeId::seed(1)),
        };

        assert_eq!(toggle.run(&mut ports), Ok(true));
        assert_eq!(ports.document(LEASE).links, [toggle.link]);
        assert_eq!(toggle.run(&mut ports), Ok(false));
        assert!(ports.document(LEASE).links.is_empty());
        assert_eq!(
            ports.writes,
            [Write::Document(LEASE), Write::Document(LEASE)]
        );
    }

    #[test]
    fn toggle_link_refuses_an_unknown_document() {
        let mut ports = ports();

        ports.refuses(FilingError::NotFound, |ports| {
            ToggleLink {
                id: DocumentId::seed(99),
                link: DocumentLink::Payee(PayeeId::seed(1)),
            }
            .run(ports)
        });
    }

    #[test]
    fn accept_files_under_the_transaction_with_the_extracted_type_and_date() {
        let mut ports = ports();

        let entry = Accept {
            id: COLES_RECEIPT,
            transaction_id: COLES_SHOP,
        }
        .run(&records(), &mut ports)
        .expect("an Unfiled Document and a Transaction");

        assert_eq!(ports.writes, [Write::Document(COLES_RECEIPT)]);
        let filed = ports.document(COLES_RECEIPT);
        assert_eq!(filed.filing, Filing::Filed);
        assert_eq!(filed.doc_type, RECEIPTS);
        assert_eq!(filed.date, today() - chrono::Duration::days(1));
        assert_eq!(filed.links, [DocumentLink::Transaction(COLES_SHOP)]);
        assert_eq!(entry.added_links, filed.links);
        assert_eq!(entry.previous_type, OTHER);
    }

    #[test]
    fn accept_refuses_an_unknown_transaction_a_filed_and_an_unknown_document() {
        let mut ports = ports();
        let accept = |id, transaction_id| Accept { id, transaction_id };

        ports.refuses(FilingError::NoSuchTransaction, |ports| {
            accept(COLES_RECEIPT, TransactionId::seed(99)).run(&records(), ports)
        });
        ports.refuses(FilingError::AlreadyFiled, |ports| {
            accept(LEASE, COLES_SHOP).run(&records(), ports)
        });
        ports.refuses(FilingError::NotFound, |ports| {
            accept(DocumentId::seed(99), COLES_SHOP).run(&records(), ports)
        });
    }

    #[test]
    fn file_by_hand_files_with_the_chosen_type_and_link() {
        let mut ports = ports();
        let link = DocumentLink::Account(AccountId::seed(1));

        FileByHand {
            id: UNMATCHED,
            doc_type: TAX,
            link: Some(link),
        }
        .run(&mut ports)
        .expect("an Unfiled Document");

        assert_eq!(ports.writes, [Write::Document(UNMATCHED)]);
        let filed = ports.document(UNMATCHED);
        assert_eq!((filed.filing, filed.doc_type), (Filing::Filed, TAX));
        assert_eq!(filed.links, [link]);
    }

    #[test]
    fn file_by_hand_refuses_a_filed_and_an_unknown_document() {
        let mut ports = ports();
        let file = |id| FileByHand {
            id,
            doc_type: TAX,
            link: None,
        };

        ports.refuses(FilingError::AlreadyFiled, |ports| file(LEASE).run(ports));
        ports.refuses(FilingError::NotFound, |ports| {
            file(DocumentId::seed(99)).run(ports)
        });
    }

    #[test]
    fn accept_all_strong_writes_every_strong_match_in_one_batch() {
        let mut ports = ports();

        let batch = AcceptAllStrong {
            focus: Some(UNMATCHED),
            skipped: HashSet::from([WOOLWORTHS_RECEIPT]),
        }
        .run(&records(), &mut ports)
        .expect("two Strong matches");

        let filed: Vec<DocumentId> = batch.entries.iter().map(|entry| entry.id).collect();
        assert_eq!(
            filed,
            [COLES_RECEIPT, WOOLWORTHS_RECEIPT],
            "the skipped last"
        );
        assert_eq!(batch.focus, Some(UNMATCHED));
        assert_eq!(
            ports.writes,
            [
                Write::Document(WOOLWORTHS_RECEIPT),
                Write::Document(COLES_RECEIPT),
            ]
        );
        assert_eq!(ports.document(UNMATCHED).filing, Filing::Unfiled);
        assert_eq!(
            ports.document(WOOLWORTHS_RECEIPT).links,
            [DocumentLink::Transaction(WOOLWORTHS_SHOP)]
        );
    }

    #[test]
    fn accept_all_strong_with_no_strong_match_writes_nothing() {
        let mut ports = ports();
        let before = ports.repo.clone();

        let batch = AcceptAllStrong {
            focus: None,
            skipped: HashSet::new(),
        }
        .run(&LinkRecords::default(), &mut ports);

        assert_eq!(batch, None);
        assert_eq!(ports.repo, before);
        assert!(ports.writes.is_empty());
    }

    #[test]
    fn undo_restores_type_date_and_unfiled_and_removes_only_the_links_the_action_added() {
        let mut ports = ports();
        let kept = DocumentLink::Payee(PayeeId::seed(1));
        ToggleLink {
            id: WOOLWORTHS_RECEIPT,
            link: kept,
        }
        .run(&mut ports)
        .expect("an Unfiled Document");
        let before = ports.repo.clone();
        let batch = AcceptAllStrong {
            focus: Some(UNMATCHED),
            skipped: HashSet::new(),
        }
        .run(&records(), &mut ports)
        .expect("two Strong matches");
        ports.writes.clear();

        let focus = UndoFiling(batch).run(&mut ports);

        assert_eq!(focus, Some(UNMATCHED));
        assert_eq!(ports.repo, before);
        assert_eq!(ports.document(WOOLWORTHS_RECEIPT).links, [kept]);
        assert_eq!(
            ports.writes,
            [
                Write::Document(WOOLWORTHS_RECEIPT),
                Write::Document(COLES_RECEIPT),
            ]
        );
    }

    #[test]
    fn undo_keeps_a_link_the_document_held_before_it_was_accepted() {
        let mut ports = ports();
        let link = DocumentLink::Transaction(COLES_SHOP);
        ToggleLink {
            id: COLES_RECEIPT,
            link,
        }
        .run(&mut ports)
        .expect("an Unfiled Document");
        let entry = Accept {
            id: COLES_RECEIPT,
            transaction_id: COLES_SHOP,
        }
        .run(&records(), &mut ports)
        .expect("an Unfiled Document and a Transaction");
        assert!(entry.added_links.is_empty());

        UndoFiling(FilingUndo {
            entries: vec![entry],
            focus: None,
        })
        .run(&mut ports);

        let undone = ports.document(COLES_RECEIPT);
        assert_eq!(undone.filing, Filing::Unfiled);
        assert_eq!(undone.links, [link]);
    }

    #[test]
    fn add_document_type_appends_a_fresh_type_with_no_files() {
        let mut ports = ports();

        let id = AddDocumentType(draft(" Medical ")).run(&mut ports);

        assert_eq!(ports.writes, [Write::Type(id)]);
        let added = ports.repo.types.last().expect("a type");
        assert_eq!((added.id, added.name.as_str()), (id, "Medical"));
        assert_eq!((added.files, added.is_default), (0, false));
        assert_ne!(AddDocumentType(draft("Medical")).run(&mut ports), id);
    }

    #[test]
    fn edit_document_type_rewrites_the_type_in_place() {
        let mut ports = ports();

        EditDocumentType {
            id: TAX.0,
            draft: draft("Taxation"),
        }
        .run(&mut ports)
        .expect("a type");

        assert_eq!(ports.writes, [Write::Type(TAX.0)]);
        assert_eq!(ports.repo.types[2].name, "Taxation");
    }

    #[test]
    fn edit_document_type_refuses_an_unknown_type() {
        let mut ports = ports();

        ports.refuses(UnknownDocumentType, |ports| {
            EditDocumentType {
                id: DocumentTypeId::seed(99),
                draft: draft("Gone"),
            }
            .run(ports)
        });
    }

    #[test]
    fn remove_document_type_moves_its_documents_and_files_in_the_same_write() {
        let mut ports = ports();
        let tax_files = ports.type_row(TAX.0).expect("Tax").files;
        let other_files = ports.type_row(OTHER.0).expect("Other").files;

        RemoveDocumentType {
            id: TAX.0,
            destination: Some("Other".into()),
        }
        .run(&mut ports)
        .expect("Other takes the files");

        assert_eq!(
            ports.writes,
            [
                Write::DeleteType(TAX.0),
                Write::Type(OTHER.0),
                Write::Document(LEASE),
            ]
        );
        assert!(ports.type_row(TAX.0).is_none());
        assert_eq!(
            ports.type_row(OTHER.0).expect("Other").files,
            other_files + tax_files
        );
        assert_eq!(ports.document(LEASE).doc_type, OTHER);
    }

    #[test]
    fn removing_an_empty_type_moves_its_documents_to_the_default() {
        let mut ports = ports();
        let empty = AddDocumentType(draft("Medical")).run(&mut ports);
        EditDocument {
            doc_type: DocumentType(empty),
            ..edit(LEASE, "Lease")
        }
        .run(&mut ports)
        .expect("a Document");
        ports.writes.clear();

        RemoveDocumentType {
            id: empty,
            destination: None,
        }
        .run(&mut ports)
        .expect("an empty type");

        assert_eq!(
            ports.writes,
            [Write::DeleteType(empty), Write::Document(LEASE)]
        );
        assert_eq!(ports.document(LEASE).doc_type, OTHER);
    }

    #[test]
    fn remove_document_type_refuses_the_default_an_unknown_type_and_a_missing_destination() {
        let mut ports = ports();
        let remove = |id: DocumentType, destination: Option<&str>| RemoveDocumentType {
            id: id.0,
            destination: destination.map(str::to_string),
        };

        ports.refuses(RemoveError::IsDefault, |ports| {
            remove(OTHER, None).run(ports)
        });
        ports.refuses(RemoveError::Unknown, |ports| {
            remove(DocumentType(DocumentTypeId::seed(99)), Some("Other")).run(ports)
        });
        ports.refuses(RemoveError::NeedsDestination, |ports| {
            remove(TAX, None).run(ports)
        });
        ports.refuses(RemoveError::NeedsDestination, |ports| {
            remove(TAX, Some("Tax")).run(ports)
        });
    }

    #[test]
    fn move_document_type_swaps_with_its_neighbour_and_stops_at_either_end() {
        let mut ports = ports();
        let first = RECEIPTS.0;

        assert!(
            !MoveDocumentType {
                id: first,
                delta: -1
            }
            .run(&mut ports)
        );
        assert!(ports.writes.is_empty());

        assert!(
            MoveDocumentType {
                id: first,
                delta: 1
            }
            .run(&mut ports)
        );
        assert_eq!(ports.writes, [Write::MoveType(first, 1)]);
        assert_eq!(ports.repo.types[1].id, first);
    }
}
