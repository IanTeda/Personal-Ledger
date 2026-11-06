//! The Documents seed tests that read the other domains' seeds, so they need more than one domain:
//! the Documents seed's Links and Suggested Links over the Sample Ledger's records, read through
//! `lib-documents`' ports as the Desktop reads them. Lives here, beside the joined Sample Ledger, rather
//! than in `lib_documents`, which depends on no other domain (moved from `lib-ledger`, #1056). The
//! rest of the Documents model's tests live in `lib_documents`.

use std::path::PathBuf;

use bigdecimal::BigDecimal;
use chrono::NaiveDate;
use lib_core::{DocumentId, Money, TransactionId};
use lib_documents::{types::DocumentTypeRow, *};
use lib_inventory::Inventory;

use crate::seed::{sample_documents, sample_institutions, sample_inventory, sample_link_records};

fn cents_money(cents: i64) -> Money {
    Money(BigDecimal::new(cents.into(), 2))
}

fn new_document(path: &str, title: &str) -> NewDocument {
    NewDocument {
        path: PathBuf::from(path),
        bytes: 0,
        title: title.to_string(),
        doc_type: DocumentType::INSURANCE,
        date: today(),
        key_date: None,
    }
}

fn by_title<'a>(documents: &'a [Document], title: &str) -> &'a Document {
    documents.iter().find(|d| d.title == title).unwrap()
}

fn types() -> Vec<DocumentTypeRow> {
    types::default_types()
}

fn date(y: i32, m: u32, d: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, d).unwrap()
}

fn today() -> NaiveDate {
    date(2026, 10, 2)
}

struct World {
    records: LinkRecords,
    documents: Vec<Document>,
    inventory: Inventory,
}

/// The Sample Ledger's, seeded as the app does: Transactions, then Bills' payments, then
/// Documents' purchases.
fn world() -> World {
    world_on(today())
}

fn world_on(today: NaiveDate) -> World {
    let institutions = sample_institutions();
    World {
        records: sample_link_records(today, &institutions),
        documents: sample_documents(today, &institutions),
        inventory: sample_inventory(today),
    }
}

#[test]
fn accept_all_takes_strong_rows_only_and_undoes_as_one_batch() {
    let World {
        records:
            LinkRecords {
                payees,
                transactions,
                ..
            },
        mut documents,
        ..
    } = world();
    let before = documents.clone();
    assert_eq!(strong_count(&documents, &transactions, &payees), 2);
    let record = accept_all_strong(
        &types(),
        &mut documents,
        &std::collections::HashSet::new(),
        &transactions,
        &payees,
        Some(DocumentId::seed(42)),
    )
    .unwrap();
    assert_eq!(record.entries.len(), 2);
    assert_eq!(inbox_count(&documents), 5);
    assert_eq!(strong_count(&documents, &transactions, &payees), 0);
    assert_eq!(undo(&mut documents, record), Some(DocumentId::seed(42)));
    assert_eq!(documents, before);
}

// Seed

fn strength_of(world: &World, file: &str) -> Suggestion {
    let document = world
        .documents
        .iter()
        .find(|d| d.file_name() == file)
        .unwrap();
    suggestion(
        document,
        &world.documents,
        &world.records.transactions,
        &world.records.payees,
    )
}

#[test]
fn the_inbox_seed_reproduces_each_sample_suggestion_on_any_day() {
    for today in [
        today(),
        date(2026, 1, 1),
        date(2027, 3, 15),
        date(2025, 7, 31),
    ] {
        assert_inbox_samples(&world_on(today));
    }
}

fn assert_inbox_samples(world: &World) {
    let strength = |file| {
        strength_of(world, file)
            .best()
            .and_then(|best| best.signals.strength())
    };
    assert_eq!(strength("IMG_4471.jpg"), Some(SignalStrength::Strong));
    assert_eq!(
        strength("bunnings-invoice-INV88213.pdf"),
        Some(SignalStrength::Strong)
    );
    assert_eq!(
        strength("Sonos_order_confirmation.pdf"),
        Some(SignalStrength::Likely)
    );
    assert_eq!(
        strength("origin-energy-bill.pdf"),
        Some(SignalStrength::Likely)
    );
    assert_eq!(strength("IMG_4460.jpg"), Some(SignalStrength::Weak));
    assert_eq!(strength_of(world, "ATO_NOA_2026.pdf"), Suggestion::Nothing);
    assert_eq!(strength_of(world, "scan0012.pdf"), Suggestion::Unreadable);
    assert_eq!(inbox_count(&world.documents), 7);
}

#[test]
fn the_library_seed_has_the_sample_rows_with_three_needing_review() {
    let world = world();
    let rows = library_rows(
        &types(),
        &world.documents,
        LibraryScope::All,
        "",
        LibrarySort::Newest,
        today(),
    );
    assert!(rows.len() > 100, "filler gives the facets weight");
    assert_eq!(need_review_count(&types(), &rows, today()), 3);
    assert_eq!(rows[0].title, "Amex Platinum statement — Sep 2026");

    let nrma = by_title(&world.documents, "NRMA home contents — PDS & schedule");
    assert_eq!(nrma.band(&types(), today()), Some(KeyDateBand::NeedReview));
    assert!(nrma.key_date.unwrap().reminder);
    let passport = by_title(&world.documents, "Passport — scan");
    assert!(passport.links.is_empty());
    assert_eq!(passport.kind, FileKind::Jpg);
    let aami = by_title(&world.documents, "AAMI car insurance — policy schedule");
    assert_eq!(
        aami.links.len(),
        3,
        "Payee, Bill Plan and Inventory Item all resolve"
    );
    let camera = by_title(&world.documents, "Receipt — Camera House");
    let Some(DocumentLink::Transaction(id)) = camera.links.get(1).copied() else {
        panic!("the Camera House receipt links its Transaction");
    };
    let purchase = world.records.transaction(id).unwrap();
    assert_eq!(purchase.amount, cents_money(-438_000));
}

#[test]
fn every_seeded_link_resolves() {
    let world = world();
    for document in &world.documents {
        for link in &document.links {
            let records = &world.records;
            let resolves = match *link {
                DocumentLink::Transaction(id) => records.transaction(id).is_some(),
                DocumentLink::InventoryItem(id) => world.inventory.item(id).is_some(),
                DocumentLink::Payee(id) => records.payee(id).is_some(),
                DocumentLink::Account(id) => records.account(id).is_some(),
                DocumentLink::BillPlan(id) => records.plan(id).is_some(),
            };
            assert!(resolves, "{}: {link:?}", document.title);
        }
    }
}

#[test]
fn the_seed_is_deterministic_with_unique_ids() {
    let first = world();
    let second = world();
    assert_eq!(first.documents, second.documents);
    let mut ids: Vec<DocumentId> = first.documents.iter().map(|d| d.id).collect();
    ids.dedup();
    assert_eq!(ids.len(), first.documents.len());
    let transactions = &first.records.transactions;
    let mut transaction_ids: Vec<TransactionId> = transactions.iter().map(|t| t.id).collect();
    transaction_ids.sort_unstable();
    transaction_ids.dedup();
    assert_eq!(transaction_ids.len(), transactions.len());
}

#[test]
fn every_facet_has_documents() {
    let world = world();
    // The Default type (Other) is the catch-all, so it may legitimately be empty.
    for kind in types()
        .iter()
        .filter(|row| !row.is_default)
        .map(|row| DocumentType(row.id))
    {
        assert!(
            scope_count(&types(), &world.documents, LibraryScope::Type(kind)) > 0,
            "{kind:?}"
        );
    }
    for facet in year_facets(today()) {
        assert!(
            scope_count(&types(), &world.documents, LibraryScope::Year(facet)) > 0,
            "{facet:?}"
        );
    }
}

#[test]
fn add_files_a_document_with_no_links() {
    let mut world = world();
    let before = world.documents.len();
    let id = add_filed(
        &mut world.documents,
        new_document("/tmp/policy.pdf", " Policy "),
    )
    .unwrap();
    let added = get(&world.documents, id).unwrap();
    assert_eq!(world.documents.len(), before + 1);
    assert!(added.is_filed());
    assert!(added.links.is_empty());
    assert_eq!(added.title, "Policy");
    assert_eq!(added.kind, FileKind::Pdf);
}

#[test]
fn editing_metadata_moves_the_financial_year_with_the_date() {
    let mut world = world();
    let id = world.documents.iter().find(|d| d.is_filed()).unwrap().id;
    let key = KeyDate {
        kind: KeyDateKind::Renews,
        date: date(2027, 1, 14),
        reminder: true,
    };
    edit_metadata(
        &mut world.documents,
        id,
        "Renamed",
        DocumentType::TAX,
        date(2019, 8, 1),
        Some(key),
    )
    .unwrap();
    let edited = get(&world.documents, id).unwrap();
    assert_eq!(edited.title, "Renamed");
    assert_eq!(edited.doc_type, DocumentType::TAX);
    assert_eq!(edited.financial_year(), 2019);
    assert_eq!(edited.key_date, Some(key));
    assert_eq!(
        edit_metadata(
            &mut world.documents,
            id,
            " ",
            DocumentType::TAX,
            today(),
            None
        ),
        Err(AddError::NoTitle)
    );
    assert_eq!(
        edit_metadata(
            &mut world.documents,
            DocumentId::seed(9_999),
            "X",
            DocumentType::TAX,
            today(),
            None
        ),
        Err(AddError::NotFound)
    );
}
