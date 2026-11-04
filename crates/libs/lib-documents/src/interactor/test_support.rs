//! The fixtures every Interactor's tests share: the recording repository and a small Library.

use std::path::PathBuf;

use bigdecimal::BigDecimal;
use chrono::NaiveDate;
use lib_core::{AccountId, DocumentId, DocumentTypeId, Money, PayeeId, TransactionId};

use crate::fake::InMemoryDocuments;
use crate::interactor::EditDocument;
use crate::protocols::{DocumentPayee, DocumentRepository, DocumentTransaction, LinkRecords};
use crate::types::{DocumentTypeDraft, DocumentTypeRow, default_types};
use crate::{
    Document, DocumentType, ExtractedFacts, FileKind, Filing, Intake, NewDocument, Source, get,
};

pub(super) const RECEIPTS: DocumentType = DocumentType(DocumentTypeId::seed(1));
pub(super) const TAX: DocumentType = DocumentType(DocumentTypeId::seed(3));
pub(super) const OTHER: DocumentType = DocumentType(DocumentTypeId::seed(9));

pub(super) const LEASE: DocumentId = DocumentId::seed(1);
pub(super) const WOOLWORTHS_RECEIPT: DocumentId = DocumentId::seed(2);
pub(super) const UNMATCHED: DocumentId = DocumentId::seed(3);
pub(super) const COLES_RECEIPT: DocumentId = DocumentId::seed(4);

pub(super) const WOOLWORTHS_SHOP: TransactionId = TransactionId::seed(1);
pub(super) const COLES_SHOP: TransactionId = TransactionId::seed(2);

/// One write the repository was asked for.
#[derive(Debug, Clone, PartialEq)]
pub(super) enum Write {
    Document(DocumentId),
    DeleteDocument(DocumentId),
    Type(DocumentTypeId),
    DeleteType(DocumentTypeId),
    MoveType(DocumentTypeId, usize),
}

/// The in-memory repository, recording each write in order.
#[derive(Debug, Clone, Default)]
pub(super) struct Recording {
    pub(super) repo: InMemoryDocuments,
    pub(super) writes: Vec<Write>,
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

    fn delete_document(&mut self, id: DocumentId) {
        self.writes.push(Write::DeleteDocument(id));
        self.repo.delete_document(id);
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
    pub(super) fn document(&self, id: DocumentId) -> Document {
        get(&self.repo.documents, id).cloned().expect("a Document")
    }

    pub(super) fn type_row(&self, id: DocumentTypeId) -> Option<&DocumentTypeRow> {
        self.repo.types.iter().find(|row| row.id == id)
    }

    /// Runs `refused`, which must refuse, and checks it wrote nothing.
    pub(super) fn refuses<R: std::fmt::Debug, E: std::fmt::Debug + PartialEq>(
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

pub(super) fn today() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 10, 2).expect("a real date")
}

pub(super) fn money(cents: i64) -> Money {
    Money(BigDecimal::new(cents.into(), 2))
}

pub(super) fn document(
    id: DocumentId,
    path: &str,
    doc_type: DocumentType,
    filing: Filing,
) -> Document {
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
pub(super) fn receipt(
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
pub(super) fn ports() -> Recording {
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

pub(super) fn shop(
    id: TransactionId,
    payee: u32,
    cents: i64,
    days_ago: i64,
) -> DocumentTransaction {
    DocumentTransaction {
        id,
        date: today() - chrono::Duration::days(days_ago),
        amount: money(-cents),
        account_id: AccountId::seed(1),
        payee_ids: vec![Some(PayeeId::seed(payee))],
        memo: None,
    }
}

pub(super) fn payee(id: u32, name: &str) -> DocumentPayee {
    DocumentPayee {
        id: PayeeId::seed(id),
        name: name.to_string(),
        aliases: Vec::new(),
        is_active: true,
    }
}

pub(super) fn records() -> LinkRecords {
    LinkRecords {
        transactions: vec![
            shop(WOOLWORTHS_SHOP, 1, 4_250, 0),
            shop(COLES_SHOP, 2, 1_000, 1),
        ],
        payees: vec![payee(1, "Woolworths"), payee(2, "Coles")],
        ..LinkRecords::default()
    }
}

pub(super) fn new_document(path: &str, title: &str) -> NewDocument {
    NewDocument {
        path: PathBuf::from(path),
        bytes: 2_048,
        title: title.to_string(),
        doc_type: TAX,
        date: today(),
        key_date: None,
    }
}

pub(super) fn draft(name: &str) -> DocumentTypeDraft {
    DocumentTypeDraft {
        name: name.to_string(),
        tracks_date: None,
        remind: None,
        financial_year: false,
    }
}

pub(super) fn edit(id: DocumentId, title: &str) -> EditDocument {
    EditDocument {
        id,
        title: title.to_string(),
        doc_type: RECEIPTS,
        date: today(),
        key_date: None,
    }
}
