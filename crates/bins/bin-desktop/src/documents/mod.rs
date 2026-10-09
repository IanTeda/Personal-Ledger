//! The Desktop's half of Documents: the label (`model`, `picker`) and form (`form`, `types`) halves
//! that need the Desktop's messages and dialogs. The `gpui`-free model, rules and stub seed live in
//! `lib_documents` and are re-exported here, so every `documents::X` path keeps working.

pub(crate) mod form;
pub(crate) mod model;
pub(crate) mod picker;
pub(crate) mod types;

pub use lib_documents::*;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::inventory::Inventory;
    use bigdecimal::BigDecimal;
    use chrono::NaiveDate;
    use lib_core::Money;
    use lib_payees::Payee;
    use lib_transactions::Transaction;
    use std::path::PathBuf;
    use types::DocumentTypeRow;

    fn cents_money(cents: i64) -> Money {
        Money(BigDecimal::new(cents.into(), 2))
    }

    fn new_document(path: &str, title: &str) -> NewDocument {
        NewDocument {
            path: PathBuf::from(path),
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
    use crate::{
        accounts::default_accounts, bills::default_bills, categories::default_categories,
        payees::default_payees, tags::default_tags, transactions::default_transactions,
    };

    fn date(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    fn today() -> NaiveDate {
        date(2026, 10, 2)
    }

    struct World {
        payees: Vec<Payee>,
        transactions: Vec<Transaction>,
        documents: Vec<Document>,
        inventory: Inventory,
    }

    /// Seeded as the app does: Transactions, then Bills' payments, then Documents' purchases.
    fn world() -> World {
        world_on(today())
    }

    fn world_on(today: NaiveDate) -> World {
        let accounts = default_accounts();
        let categories = default_categories();
        let payees = default_payees();
        let tags = default_tags();
        let mut transactions = default_transactions(&accounts, &categories, &payees, &tags, today);
        let bills = default_bills(&accounts, &categories, &payees, &mut transactions, today);
        let inventory = crate::inventory::default_inventory(today);
        let seed = default_documents(
            &accounts,
            &categories,
            &payees,
            &bills.plans,
            &inventory,
            &mut transactions,
            today,
        );
        World {
            payees,
            transactions,
            documents: seed.documents,
            inventory,
        }
    }

    #[test]
    fn dropping_inventory_links_keeps_the_documents() {
        let mut world = world();
        let fridge = world
            .inventory
            .items
            .iter()
            .find(|i| i.name == "Fridge")
            .unwrap()
            .id;
        let count = world.documents.len();
        let had = |docs: &[Document]| {
            docs.iter()
                .filter(|d| d.links.contains(&DocumentLink::InventoryItem(fridge)))
                .count()
        };
        let linked = had(&world.documents);
        assert!(linked > 0, "the seed links a Document to the Fridge");
        assert_eq!(
            drop_inventory_links(&mut world.documents, &[fridge]),
            linked
        );
        assert_eq!(had(&world.documents), 0);
        assert_eq!(world.documents.len(), count, "no Document is removed");
        assert_eq!(drop_inventory_links(&mut world.documents, &[fridge]), 0);
    }

    #[test]
    fn removing_a_property_drops_only_its_items_links() {
        let mut world = world();
        let elm = world.inventory.properties[0].id;
        let removed = crate::inventory::remove_property(&mut world.inventory, elm).unwrap();
        let lost = drop_inventory_links(&mut world.documents, &removed.items);
        assert!(lost > 0);
        for document in &world.documents {
            for link in &document.links {
                if let DocumentLink::InventoryItem(id) = link {
                    assert!(world.inventory.item(*id).is_some(), "no dangling Item Link");
                }
            }
        }
    }

    #[test]
    fn accept_all_takes_strong_rows_only_and_undoes_as_one_batch() {
        let World {
            payees,
            transactions,
            mut documents,
            ..
        } = world();
        let before = documents.clone();
        assert_eq!(strong_count(&documents, &transactions, &payees), 2);
        let record =
            accept_all_strong(&types(), &mut documents, &transactions, &payees, Some(42)).unwrap();
        assert_eq!(record.entries.len(), 2);
        assert_eq!(inbox_count(&documents), 5);
        assert_eq!(strong_count(&documents, &transactions, &payees), 0);
        assert_eq!(undo(&mut documents, record), Some(42));
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
            &world.transactions,
            &world.payees,
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
        let purchase = world.transactions.iter().find(|t| t.id == id).unwrap();
        assert_eq!(purchase.total(), cents_money(-438_000));
    }

    #[test]
    fn every_seeded_link_resolves() {
        let world = world();
        for document in &world.documents {
            for link in &document.links {
                let resolves = match *link {
                    DocumentLink::Transaction(id) => world.transactions.iter().any(|t| t.id == id),
                    DocumentLink::InventoryItem(id) => world.inventory.item(id).is_some(),
                    DocumentLink::Payee(id) => lib_payees::get(&world.payees, id).is_some(),
                    DocumentLink::Account(id) => default_accounts().iter().any(|a| a.id == id),
                    DocumentLink::BillPlan(_) => true,
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
        let mut ids: Vec<u32> = first.documents.iter().map(|d| d.id).collect();
        ids.dedup();
        assert_eq!(ids.len(), first.documents.len());
        let mut transaction_ids: Vec<u32> = first.transactions.iter().map(|t| t.id).collect();
        transaction_ids.sort_unstable();
        transaction_ids.dedup();
        assert_eq!(transaction_ids.len(), first.transactions.len());
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
                9_999,
                "X",
                DocumentType::TAX,
                today(),
                None
            ),
            Err(AddError::NotFound)
        );
    }
}
