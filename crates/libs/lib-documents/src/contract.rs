//! The contract every stored [`DocumentRepository`] keeps (ADR-0046), run against both the
//! in-memory repository here and the `lib-database` adapter (`adapter/documents.rs`), so the fakes
//! the tests use can be trusted.
//!
//! Each check is a function generic over a [`PortsFactory`]: a consumer runs it from its own test,
//! since an adapter's tests need their own set-up (a fresh database per test). Each writes through
//! the rules and the port, as the Ledger's Documents commands do until the Interactors (#965),
//! reloads as a restart would, and expects the same rows back. A Link names a record the factory
//! says storage holds ([`PortsFactory::records`]), since the adapter's Links are foreign keys.

use std::path::PathBuf;

use bigdecimal::BigDecimal;
use chrono::NaiveDate;
use lib_core::{AccountId, BillPlanId, DocumentId, DocumentTypeId, ItemId, Money, PayeeId};

use crate::fake::InMemoryDocuments;
use crate::protocols::{
    DocumentAccount, DocumentBillPlan, DocumentItem, DocumentPayee, DocumentRepository,
    DocumentTransaction, LinkRecords,
};
use crate::types::{DocumentTypeRow, RemindLead, TracksDate, default_types, remove_type};
use crate::{
    Document, DocumentLink, DocumentType, ExtractedFacts, FileKind, Filing, Intake, KeyDate,
    KeyDateKind, Source, toggle_link,
};

const TAX: DocumentTypeId = DocumentTypeId::seed(3);
const INSURANCE: DocumentTypeId = DocumentTypeId::seed(4);
const BILLS: DocumentTypeId = DocumentTypeId::seed(8);
const OTHER: DocumentTypeId = DocumentTypeId::seed(9);

/// The day every check runs on.
pub fn today() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 10, 2).expect("a real date")
}

/// Builds the repository a contract check runs on.
pub trait PortsFactory {
    type Ports: DocumentRepository;

    /// The records storage holds, which a Link may name: one of each kind at least.
    fn records(&self) -> LinkRecords;

    /// A repository holding `types` and `documents`, as if stored and loaded.
    fn create(&mut self, types: Vec<DocumentTypeRow>, documents: Vec<Document>) -> Self::Ports;

    /// Ends the writes made on `ports`, then loads a fresh repository from wherever they were
    /// stored, as a restart would.
    fn reload(&mut self, ports: Self::Ports) -> Self::Ports;

    /// Ends one write made on `ports`, sending its changes to storage as an adapter's owner does
    /// after each input.
    fn commit(&mut self, ports: &mut Self::Ports);

    /// Makes every write from now on fail, as a store that refuses it would.
    fn break_writes(&mut self, ports: &mut Self::Ports);

    /// Settles the writes sent so far, then returns `ports` reloaded to the last committed rows,
    /// with the number of failures reported naming the Documents domain.
    fn recover(&mut self, ports: Self::Ports) -> (Self::Ports, usize);

    /// One stored record of each kind storage can delete, which nothing but a Link names. A Bill
    /// Plan has no delete yet, so none is offered.
    fn deletable(&mut self) -> Vec<DocumentLink>;

    /// Deletes `record` as its own domain's delete does and commits it, which cascades to the
    /// Links naming it, then returns `ports` with the cascade read back, as a Client does after
    /// such a commit (ADR-0046's 2026-11-02 amendment).
    fn delete_record(&mut self, ports: Self::Ports, record: DocumentLink) -> Self::Ports;
}

/// The in-memory repository is its own storage, so a reload hands it back as it is. It keeps a
/// copy as of the last commit, which a failed write falls back to.
#[derive(Debug, Default)]
pub struct InMemoryFactory {
    committed: Option<InMemoryDocuments>,
    broken: bool,
    failures: usize,
}

impl PortsFactory for InMemoryFactory {
    type Ports = InMemoryDocuments;

    /// One record of each kind; nothing checks a Link in memory.
    fn records(&self) -> LinkRecords {
        LinkRecords {
            transactions: vec![DocumentTransaction {
                id: lib_core::TransactionId::seed(1),
                date: today(),
                amount: cents(-4_250),
                account_id: AccountId::seed(1),
                payee_ids: vec![Some(PayeeId::seed(1))],
                memo: None,
            }],
            payees: vec![DocumentPayee {
                id: PayeeId::seed(1),
                name: "Woolworths".into(),
                aliases: Vec::new(),
                is_active: true,
            }],
            accounts: vec![DocumentAccount {
                id: AccountId::seed(1),
                name: "Wallet".into(),
                institution: "Personal".into(),
            }],
            plans: vec![DocumentBillPlan {
                id: BillPlanId::seed(1),
                name: "Rent".into(),
                cadence: "Monthly".into(),
                payee_id: None,
            }],
            items: vec![DocumentItem {
                id: ItemId::seed(1),
                name: "Laptop".into(),
            }],
        }
    }

    fn create(
        &mut self,
        types: Vec<DocumentTypeRow>,
        documents: Vec<Document>,
    ) -> InMemoryDocuments {
        let ports = InMemoryDocuments { documents, types };
        self.committed = Some(ports.clone());
        ports
    }

    fn reload(&mut self, mut ports: InMemoryDocuments) -> InMemoryDocuments {
        self.commit(&mut ports);
        self.committed.clone().expect("create ran first")
    }

    fn commit(&mut self, ports: &mut InMemoryDocuments) {
        if self.broken {
            self.failures += 1;
        } else {
            self.committed = Some(ports.clone());
        }
    }

    fn break_writes(&mut self, _ports: &mut InMemoryDocuments) {
        self.broken = true;
    }

    fn recover(&mut self, _ports: InMemoryDocuments) -> (InMemoryDocuments, usize) {
        let committed = self.committed.clone().expect("create ran first");
        (committed, std::mem::take(&mut self.failures))
    }

    /// A second record of each kind beside [`Self::records`]' first.
    fn deletable(&mut self) -> Vec<DocumentLink> {
        vec![
            DocumentLink::Transaction(lib_core::TransactionId::seed(2)),
            DocumentLink::Account(AccountId::seed(2)),
            DocumentLink::Payee(PayeeId::seed(2)),
            DocumentLink::InventoryItem(ItemId::seed(2)),
        ]
    }

    /// The repository is its own storage, so the cascade lands on it and on the committed copy.
    fn delete_record(
        &mut self,
        mut ports: InMemoryDocuments,
        record: DocumentLink,
    ) -> InMemoryDocuments {
        self.commit(&mut ports);
        ports.cascade_delete(&[record]);
        self.committed = Some(ports.clone());
        ports
    }
}

fn cents(cents: i64) -> Money {
    Money(BigDecimal::new(cents.into(), 2))
}

/// A Filed Document of `doc_type` at `path`, with no Links, Key Date or Intake.
fn filed(id: u32, path: &str, doc_type: DocumentTypeId) -> Document {
    Document {
        id: DocumentId::seed(id),
        path: PathBuf::from(path),
        kind: FileKind::Pdf,
        pages: 2,
        bytes: 48_213,
        title: format!("Document {id}"),
        doc_type: DocumentType(doc_type),
        date: today(),
        key_date: None,
        links: Vec::new(),
        extracted_text: None,
        filing: Filing::Filed,
        intake: None,
    }
}

/// Every Link `records` offers, the first record of each kind.
fn every_link(records: &LinkRecords) -> Vec<DocumentLink> {
    vec![
        DocumentLink::Transaction(records.transactions[0].id),
        DocumentLink::Account(records.accounts[0].id),
        DocumentLink::Payee(records.payees[0].id),
        DocumentLink::BillPlan(records.plans[0].id),
        DocumentLink::InventoryItem(records.items[0].id),
    ]
}

/// Writes a Filed Document with a Key Date, extracted text and a Link of every kind, and an
/// Unfiled one with its Intake and every Extracted Fact, reloads, and gets the same rows; then a
/// Document losing Links is written again and its Links are replaced whole.
pub fn a_document_with_links_and_intake_reloads_the_same<F: PortsFactory>(factory: &mut F) {
    let records = factory.records();
    let mut ports = factory.create(default_types(), Vec::new());

    let mut policy = filed(1, "insurance/home-2026.pdf", INSURANCE);
    policy.key_date = Some(KeyDate {
        kind: KeyDateKind::Renews,
        date: today() + chrono::Duration::days(40),
        reminder: true,
    });
    policy.extracted_text = Some("Home and contents, sum insured".into());
    policy.links = every_link(&records);
    let receipt = Document {
        id: DocumentId::seed(2),
        path: PathBuf::from("inbox/IMG_0412.jpg"),
        kind: FileKind::Jpg,
        pages: 1,
        bytes: 912_004,
        title: "IMG_0412.jpg".into(),
        doc_type: DocumentType(OTHER),
        date: today(),
        key_date: None,
        links: Vec::new(),
        extracted_text: None,
        filing: Filing::Unfiled,
        intake: Some(Intake {
            source: Source::Emailed,
            received_at: today(),
            facts: ExtractedFacts {
                merchant: Some("Woolworths".into()),
                date: Some(today() - chrono::Duration::days(1)),
                total: Some(cents(4_250)),
                doc_type: Some(DocumentType(TAX)),
            },
        }),
    };
    ports.upsert_document(&policy);
    ports.upsert_document(&receipt);
    factory.commit(&mut ports);

    let mut ports = factory.reload(ports);
    assert_eq!(ports.documents(), [policy.clone(), receipt.clone()]);
    assert_eq!(ports.document_types(), default_types());

    let mut documents = ports.documents();
    for link in [
        DocumentLink::Account(records.accounts[0].id),
        DocumentLink::BillPlan(records.plans[0].id),
    ] {
        assert_eq!(toggle_link(&mut documents, policy.id, link), Some(false));
    }
    ports.upsert_document(&documents[0]);
    let ports = factory.reload(ports);

    assert_eq!(ports.documents(), documents);
    assert_eq!(
        ports.documents()[0].links,
        [
            DocumentLink::Transaction(records.transactions[0].id),
            DocumentLink::Payee(records.payees[0].id),
            DocumentLink::InventoryItem(records.items[0].id),
        ]
    );
}

/// Moves a Document Type, adds one at the end, edits one and removes an empty one, reloads, and
/// gets the same types in the same order.
pub fn the_document_types_keep_their_order<F: PortsFactory>(factory: &mut F) {
    let mut ports = factory.create(default_types(), Vec::new());

    ports.move_document_type(DocumentTypeId::seed(1), 3);
    let warranty_claims = DocumentTypeRow {
        id: DocumentTypeId::seed(10),
        name: "Warranty claims".into(),
        tracks_date: Some(TracksDate::Ends),
        remind: Some(RemindLead::Months(2)),
        financial_year: false,
        files: 0,
        is_default: false,
    };
    ports.upsert_document_type(&warranty_claims);
    factory.commit(&mut ports);
    let mut insurance = ports
        .document_types()
        .into_iter()
        .find(|row| row.id == INSURANCE)
        .expect("a seeded type");
    insurance.name = "Policies".into();
    insurance.remind = Some(RemindLead::Days(14));
    ports.upsert_document_type(&insurance);
    ports.delete_document_type(BILLS);
    ports.move_document_type(warranty_claims.id, 0);
    factory.commit(&mut ports);

    let before = ports.document_types();
    let ports = factory.reload(ports);

    assert_eq!(ports.document_types(), before);
    let names: Vec<String> = before.into_iter().map(|row| row.name).collect();
    assert_eq!(
        names,
        [
            "Warranty claims",
            "Statements",
            "Tax",
            "Policies",
            "Receipts",
            "Warranties & manuals",
            "Contracts",
            "Identity",
            "Other",
        ]
    );
}

/// Removes a Document Type with Filed Documents as Remove does, in one write: the type's delete,
/// the destination's grown count and the moved Documents, in that order. Reloaded, the type is
/// gone and its Documents are the destination's.
pub fn a_type_removal_moves_its_documents_in_one_batch<F: PortsFactory>(factory: &mut F) {
    let return_2025 = filed(1, "tax/return-2025.pdf", TAX);
    let notice = filed(2, "tax/notice-2025.pdf", TAX);
    let policy = filed(3, "insurance/car.pdf", INSURANCE);
    let mut ports = factory.create(
        default_types(),
        vec![return_2025.clone(), notice.clone(), policy.clone()],
    );

    let mut types = ports.document_types();
    remove_type(&mut types, TAX, Some("Other")).expect("Other takes the files");
    ports.delete_document_type(TAX);
    let other = types
        .iter()
        .find(|row| row.id == OTHER)
        .expect("the Default");
    ports.upsert_document_type(other);
    for document in [&return_2025, &notice] {
        let mut moved = document.clone();
        moved.doc_type = DocumentType(OTHER);
        ports.upsert_document(&moved);
    }
    factory.commit(&mut ports);
    let ports = factory.reload(ports);

    assert_eq!(ports.document_types(), types);
    assert!(ports.document_types().iter().all(|row| row.id != TAX));
    let kinds: Vec<(DocumentId, DocumentTypeId)> = ports
        .documents()
        .iter()
        .map(|document| (document.id, document.doc_type.0))
        .collect();
    assert_eq!(
        kinds,
        [
            (return_2025.id, OTHER),
            (notice.id, OTHER),
            (policy.id, INSURANCE),
        ]
    );
}

/// Deleting a record a Link names drops that Link and keeps the Document (GLOSSARY Document
/// Link), for each kind storage can delete: the Link is gone from memory once the cascade is read
/// back, the Document's other Link stays, and the Document then writes again without it.
pub fn deleting_a_linked_record_drops_its_links_and_keeps_the_document<F: PortsFactory>(
    factory: &mut F,
) {
    let records = factory.records();
    let deletable = factory.deletable();
    assert_eq!(deletable.len(), 4, "a Transaction, Account, Payee and Item");
    let kept = DocumentLink::InventoryItem(records.items[0].id);
    let documents: Vec<Document> = (1..)
        .zip(&deletable)
        .map(|(id, record)| {
            let mut document = filed(id, &format!("linked-{id}.pdf"), TAX);
            document.links = vec![*record, kept];
            document
        })
        .collect();
    let mut ports = factory.create(default_types(), documents.clone());

    for (document, record) in documents.iter().zip(&deletable) {
        assert_ne!(*record, kept);
        ports = factory.delete_record(ports, *record);

        let held = ports
            .documents()
            .into_iter()
            .find(|held| held.id == document.id)
            .expect("the Document stays");
        assert_eq!(
            held.links,
            [kept],
            "{record:?}'s Link is dropped from memory"
        );

        let mut renamed = held;
        renamed.title = format!("{} renamed", renamed.title);
        ports.upsert_document(&renamed);
        ports = factory.reload(ports);
        let stored = ports
            .documents()
            .into_iter()
            .find(|stored| stored.id == document.id)
            .expect("the Document is still stored");
        assert_eq!(stored, renamed, "it writes again without the Link");
    }
    assert_eq!(
        ports.documents().len(),
        documents.len(),
        "no Document is removed"
    );
}

/// A write storage refuses is reported and reloads to the last committed rows.
pub fn a_failed_write_reloads_and_reports<F: PortsFactory>(factory: &mut F) {
    let mut ports = factory.create(default_types(), vec![filed(1, "a.pdf", TAX)]);
    let before = (ports.documents(), ports.document_types());

    factory.break_writes(&mut ports);
    let mut renamed = filed(1, "a.pdf", TAX);
    renamed.title = "Renamed".into();
    ports.upsert_document(&renamed);
    assert_eq!(ports.documents()[0].title, "Renamed");
    factory.commit(&mut ports);
    let (ports, failures) = factory.recover(ports);

    assert_eq!(failures, 1, "the failure is reported once");
    assert_eq!((ports.documents(), ports.document_types()), before);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn in_memory_a_document_with_links_and_intake_reloads_the_same() {
        a_document_with_links_and_intake_reloads_the_same(&mut InMemoryFactory::default());
    }

    #[test]
    fn in_memory_the_document_types_keep_their_order() {
        the_document_types_keep_their_order(&mut InMemoryFactory::default());
    }

    #[test]
    fn in_memory_a_type_removal_moves_its_documents_in_one_batch() {
        a_type_removal_moves_its_documents_in_one_batch(&mut InMemoryFactory::default());
    }

    #[test]
    fn in_memory_deleting_a_linked_record_drops_its_links_and_keeps_the_document() {
        deleting_a_linked_record_drops_its_links_and_keeps_the_document(
            &mut InMemoryFactory::default(),
        );
    }

    #[test]
    fn in_memory_a_failed_write_reloads_and_reports() {
        a_failed_write_reloads_and_reports(&mut InMemoryFactory::default());
    }
}
