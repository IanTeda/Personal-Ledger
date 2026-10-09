use super::*;
use lib_payees::default_payees;

fn types() -> Vec<DocumentTypeRow> {
    types::default_types()
}

fn date(y: i32, m: u32, d: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, d).unwrap()
}

fn today() -> NaiveDate {
    date(2026, 10, 2)
}

fn by_title<'a>(documents: &'a [Document], title: &str) -> &'a Document {
    documents.iter().find(|d| d.title == title).unwrap()
}

fn filed(id: u32, doc_type: DocumentType, on: NaiveDate) -> Document {
    Draft::filed("file.pdf", &format!("Doc {id}"), doc_type, on).build(id)
}

fn unfiled(id: u32, facts: ExtractedFacts) -> Document {
    Draft::unfiled("IMG.jpg", Source::Scanned, today(), facts).build(id)
}

fn payee_split(cents: i64, payee_id: Option<u32>) -> Split {
    Split {
        amount: cents_money(cents),
        category_id: 1,
        payee_id,
        tag_ids: Vec::new(),
    }
}

fn txn(id: u32, on: NaiveDate, cents: i64, payee_id: Option<u32>) -> Transaction {
    Transaction {
        id,
        date: on,
        account_id: 1,
        status: TransactionStatus::Cleared,
        is_flagged: false,
        description: None,
        splits: vec![payee_split(cents, payee_id)],
    }
}

fn facts(merchant: Option<&str>, on: Option<NaiveDate>, cents: Option<i64>) -> ExtractedFacts {
    ExtractedFacts {
        merchant: merchant.map(str::to_string),
        date: on,
        total: cents.map(cents_money),
        doc_type: None,
    }
}

// Financial Year and facets

#[test]
fn financial_year_derives_from_the_document_date() {
    assert_eq!(
        filed(1, DocumentType::TAX, date(2026, 6, 30)).financial_year(),
        2025
    );
    assert_eq!(
        filed(1, DocumentType::TAX, date(2026, 7, 1)).financial_year(),
        2026
    );
}

#[test]
fn year_facets_are_this_year_last_year_then_earlier() {
    assert_eq!(
        year_facets(today()),
        [
            YearFacet::Year(2026),
            YearFacet::Year(2025),
            YearFacet::Before(2025)
        ]
    );
    assert!(
        YearFacet::Before(2025).contains(&filed(1, DocumentType::TAX, date(2025, 6, 30)), &types())
    );
    assert!(
        !YearFacet::Before(2025).contains(&filed(1, DocumentType::TAX, date(2025, 7, 1)), &types())
    );
}

#[test]
fn scope_ids_round_trip_and_stale_years_fall_back_to_all() {
    for scope in [
        LibraryScope::All,
        LibraryScope::Type(DocumentType::WARRANTIES),
        LibraryScope::Year(YearFacet::Year(2025)),
        LibraryScope::Year(YearFacet::Before(2025)),
    ] {
        assert_eq!(LibraryScope::from_id(&scope.id(), &types(), today()), scope);
    }
    assert_eq!(
        LibraryScope::from_id("fy:2023", &types(), today()),
        LibraryScope::All
    );
    assert_eq!(
        LibraryScope::from_id("type:nope", &types(), today()),
        LibraryScope::All
    );
    assert_eq!(
        LibraryScope::from_id("garbage", &types(), today()),
        LibraryScope::All
    );
}

#[test]
fn counts_cover_filed_documents_only() {
    let documents = vec![
        filed(1, DocumentType::RECEIPTS, today()),
        filed(2, DocumentType::TAX, today()),
        unfiled(3, facts(None, None, Some(100))),
    ];
    assert_eq!(scope_count(&types(), &documents, LibraryScope::All), 2);
    assert_eq!(
        scope_count(
            &types(),
            &documents,
            LibraryScope::Type(DocumentType::RECEIPTS)
        ),
        1
    );
    assert_eq!(inbox_count(&documents), 1);
}

// Key Dates

fn keyed(id: u32, offset: i64) -> Document {
    let mut document = filed(
        id,
        DocumentType::INSURANCE,
        today() - Duration::days(id.into()),
    );
    document.key_date = Some(KeyDate {
        kind: KeyDateKind::Renews,
        date: today() + Duration::days(offset),
        reminder: false,
    });
    document
}

#[test]
fn the_attention_window_is_sixty_days_either_side_inclusive() {
    let at = |offset| band(&keyed(1, offset).key_date.unwrap(), today());
    assert_eq!(at(60), KeyDateBand::NeedReview);
    assert_eq!(at(61), KeyDateBand::Upcoming);
    assert_eq!(at(-60), KeyDateBand::NeedReview);
    assert_eq!(at(-61), KeyDateBand::Stale);
    assert_eq!(at(0), KeyDateBand::NeedReview);
}

#[test]
fn the_reminder_flag_does_not_change_the_band() {
    let mut document = keyed(1, 200);
    if let Some(key_date) = document.key_date.as_mut() {
        key_date.reminder = true;
    }
    assert_eq!(
        document.band(&types(), today()),
        Some(KeyDateBand::Upcoming)
    );
}

#[test]
fn expiring_sorts_need_review_then_upcoming_then_stale_descending_then_none() {
    let documents = vec![
        keyed(1, 300),
        keyed(2, -200),
        keyed(3, 10),
        keyed(4, -30),
        filed(5, DocumentType::RECEIPTS, today()),
        keyed(6, -100),
        keyed(7, 100),
    ];
    let ids: Vec<u32> = library_rows(
        &types(),
        &documents,
        LibraryScope::All,
        "",
        LibrarySort::Expiring,
        today(),
    )
    .iter()
    .map(|d| d.id)
    .collect();
    assert_eq!(ids, [4, 3, 7, 1, 6, 2, 5]);
}

#[test]
fn expiring_ties_go_to_the_newest_document_then_title() {
    let mut older = keyed(1, 10);
    older.date = today() - Duration::days(50);
    let mut newer = keyed(2, 10);
    newer.date = today() - Duration::days(5);
    let documents = vec![older, newer];
    let rows = library_rows(
        &types(),
        &documents,
        LibraryScope::All,
        "",
        LibrarySort::Expiring,
        today(),
    );
    assert_eq!(rows.iter().map(|d| d.id).collect::<Vec<_>>(), [2, 1]);
}

#[test]
fn need_review_counts_the_listed_filed_rows() {
    let mut documents = vec![keyed(1, 5), keyed(2, -5), keyed(3, 90)];
    documents[1].title = "Hidden".to_string();
    let rows = library_rows(
        &types(),
        &documents,
        LibraryScope::All,
        "doc",
        LibrarySort::Newest,
        today(),
    );
    assert_eq!(rows.len(), 2);
    assert_eq!(need_review_count(&types(), &rows, today()), 1);
}

// Search and Newest

#[test]
fn search_matches_title_file_name_and_extracted_text_case_insensitively() {
    let mut document = filed(1, DocumentType::RECEIPTS, today());
    document.extracted_text = Some("Sony A7 IV body".to_string());
    assert!(document.matches("sony a7"));
    assert!(document.matches("FILE.PDF"));
    assert!(document.matches("doc 1"));
    assert!(document.matches("   "));
    assert!(!document.matches("woolworths"));
}

#[test]
fn newest_sorts_by_document_date_then_title() {
    let mut a = filed(1, DocumentType::RECEIPTS, date(2026, 1, 1));
    a.title = "B".to_string();
    let mut b = filed(2, DocumentType::RECEIPTS, date(2026, 1, 1));
    b.title = "A".to_string();
    let c = filed(3, DocumentType::RECEIPTS, date(2026, 5, 1));
    let documents = vec![a, b, c];
    let ids: Vec<u32> = library_rows(
        &types(),
        &documents,
        LibraryScope::All,
        "",
        LibrarySort::Newest,
        today(),
    )
    .iter()
    .map(|d| d.id)
    .collect();
    assert_eq!(ids, [3, 2, 1]);
}

#[test]
fn removing_the_last_link_leaves_the_document_filed() {
    let mut documents = vec![filed(1, DocumentType::RECEIPTS, today())];
    assert_eq!(
        toggle_link(&mut documents, 1, DocumentLink::Payee(4)),
        Some(true)
    );
    assert_eq!(
        toggle_link(&mut documents, 1, DocumentLink::Payee(4)),
        Some(false)
    );
    assert!(documents[0].links.is_empty());
    assert!(documents[0].is_filed());
}

// Suggested Link

fn woolworths() -> Vec<Payee> {
    default_payees()
}

#[test]
fn three_agreeing_signals_are_strong() {
    let transactions = vec![txn(1, today(), -21_240, Some(1))];
    let document = unfiled(
        1,
        facts(Some("Woolworths Metro"), Some(today()), Some(21_240)),
    );
    let suggestion = suggestion(&document, &[], &transactions, &woolworths());
    let best = suggestion.best().unwrap();
    assert_eq!(best.signals.strength(), Some(SignalStrength::Strong));
    assert!(suggestion.is_strong());
}

#[test]
fn amount_ignores_the_sign_and_compares_a_split_transactions_whole_total() {
    let mut split = txn(1, today(), -10_000, None);
    split.splits.push(payee_split(-2_000, None));
    let document = unfiled(1, facts(None, Some(today()), Some(12_000)));
    let found = candidates(&document, &[], &[split], &[]);
    assert!(found[0].signals.amount);
}

#[test]
fn date_alone_never_makes_a_candidate() {
    let transactions = vec![txn(1, today(), -999, None)];
    let document = unfiled(1, facts(None, Some(today()), Some(100)));
    assert_eq!(
        suggestion(&document, &[], &transactions, &[]),
        Suggestion::Nothing
    );
}

#[test]
fn the_window_is_seven_days_and_the_date_signal_three() {
    let at = |offset: i64| txn(1, today() + Duration::days(offset), -500, None);
    let document = unfiled(1, facts(None, Some(today()), Some(500)));
    assert!(candidates(&document, &[], &[at(8)], &[]).is_empty());
    let edge = candidates(&document, &[], &[at(-7)], &[]);
    assert!(!edge[0].signals.date);
    assert!(candidates(&document, &[], &[at(3)], &[])[0].signals.date);
    assert!(!candidates(&document, &[], &[at(4)], &[])[0].signals.date);
}

#[test]
fn without_an_extracted_date_the_window_centres_on_received_and_date_never_agrees() {
    let mut document = unfiled(1, facts(None, None, Some(500)));
    if let Some(intake) = document.intake.as_mut() {
        intake.received_at = today() - Duration::days(20);
    }
    let near_received = txn(1, today() - Duration::days(20), -500, None);
    let found = candidates(
        &document,
        &[],
        &[near_received, txn(2, today(), -500, None)],
        &[],
    );
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].transaction_id, 1);
    assert!(!found[0].signals.date);
}

#[test]
fn payee_resolves_by_alias_then_by_name_contained_in_the_merchant() {
    let payees = default_payees();
    assert_eq!(merchant_payee(&payees, "WW SUPERMARKET 1234"), Some(1));
    assert_eq!(
        merchant_payee(&payees, "Cafe Vittoria Paddington"),
        Some(11)
    );
    assert_eq!(merchant_payee(&payees, "Australian Taxation Office"), None);
}

#[test]
fn ranking_prefers_strength_then_amount_then_gap_then_unlinked_then_id() {
    let payees = default_payees();
    let transactions = vec![
        txn(1, today(), -999, Some(1)),                     // payee + date
        txn(2, today() - Duration::days(5), -500, Some(1)), // amount + payee
        txn(3, today() - Duration::days(2), -500, None),    // amount + date
        txn(4, today() - Duration::days(1), -500, None),    // amount + date, nearer
        txn(5, today() - Duration::days(1), -500, None),    // as 4, but linked
    ];
    let mut linker = filed(9, DocumentType::RECEIPTS, today());
    linker.links.push(DocumentLink::Transaction(4));
    let document = unfiled(1, facts(Some("Woolworths"), Some(today()), Some(500)));
    let documents = vec![linker, document.clone()];
    let ids: Vec<u32> = candidates(&document, &documents, &transactions, &payees)
        .iter()
        .map(|c| c.transaction_id)
        .collect();
    assert_eq!(ids, [5, 4, 3, 2, 1]);
}

#[test]
fn others_are_the_next_two() {
    let transactions: Vec<Transaction> = (1..=5).map(|id| txn(id, today(), -500, None)).collect();
    let document = unfiled(1, facts(None, Some(today()), Some(500)));
    let Suggestion::Link { best, others } = suggestion(&document, &[], &transactions, &[]) else {
        panic!("expected a Suggested Link");
    };
    assert_eq!(best.transaction_id, 1);
    assert_eq!(
        others.iter().map(|c| c.transaction_id).collect::<Vec<_>>(),
        [2, 3]
    );
}

#[test]
fn no_total_is_unreadable() {
    let transactions = vec![txn(1, today(), -500, Some(1))];
    let document = unfiled(1, facts(Some("Woolworths"), Some(today()), None));
    assert_eq!(
        suggestion(&document, &[], &transactions, &default_payees()),
        Suggestion::Unreadable
    );
}

// Filing

#[test]
fn accept_files_with_the_link_extracted_type_and_date() {
    let transactions = vec![txn(7, today() - Duration::days(2), -500, None)];
    let mut documents = vec![unfiled(1, facts(None, Some(today()), Some(500)))];
    if let Some(intake) = documents[0].intake.as_mut() {
        intake.facts.doc_type = Some(DocumentType::BILLS);
    }
    accept(&types(), &mut documents, &transactions, 1, 7).unwrap();
    let document = &documents[0];
    assert!(document.is_filed());
    assert_eq!(document.links, [DocumentLink::Transaction(7)]);
    assert_eq!(document.doc_type, DocumentType::BILLS);
    assert_eq!(document.date, today());
}

#[test]
fn accept_without_extracted_type_or_date_uses_the_default_type_and_the_transaction_date() {
    let on = today() - Duration::days(2);
    let transactions = vec![txn(7, on, -500, None)];
    let mut documents = vec![unfiled(1, facts(None, None, Some(500)))];
    accept(&types(), &mut documents, &transactions, 1, 7).unwrap();
    assert_eq!(documents[0].doc_type, DocumentType::OTHER);
    assert_eq!(documents[0].date, on);
    assert_eq!(
        accept(&types(), &mut documents, &transactions, 1, 7),
        Err(FilingError::AlreadyFiled)
    );
}

#[test]
fn file_by_hand_takes_the_chosen_type_and_an_optional_link() {
    let mut documents = vec![unfiled(1, ExtractedFacts::default())];
    file_by_hand(&mut documents, 1, DocumentType::IDENTITY, None).unwrap();
    assert!(documents[0].is_filed());
    assert!(documents[0].links.is_empty());
    assert_eq!(documents[0].doc_type, DocumentType::IDENTITY);
    assert_eq!(documents[0].date, today());
}

#[test]
fn undo_returns_a_single_accept_to_unfiled_as_it_was() {
    let transactions = vec![txn(7, today(), -500, None)];
    let mut documents = vec![unfiled(1, facts(None, None, Some(500)))];
    let before = documents[0].clone();
    let entry = accept(&types(), &mut documents, &transactions, 1, 7).unwrap();
    let focus = undo(
        &mut documents,
        FilingUndo {
            entries: vec![entry],
            focus: Some(1),
        },
    );
    assert_eq!(documents[0], before);
    assert_eq!(focus, Some(1));
}

#[test]
fn skipped_rows_sort_last_and_editing_clears_the_skip() {
    let mut documents = vec![
        unfiled(1, ExtractedFacts::default()),
        unfiled(2, ExtractedFacts::default()),
    ];
    let order = |documents: &[Document]| {
        inbox_rows(documents)
            .iter()
            .map(|d| d.id)
            .collect::<Vec<_>>()
    };
    assert_eq!(order(&documents), [2, 1]);
    skip(&mut documents, 2).unwrap();
    assert_eq!(order(&documents), [1, 2]);
    edit_facts(&mut documents, 2, facts(None, None, Some(100))).unwrap();
    assert_eq!(order(&documents), [2, 1]);
}

#[test]
fn clearing_the_total_makes_a_document_unreadable() {
    let transactions = vec![txn(1, today(), -500, None)];
    let mut documents = vec![unfiled(1, facts(None, Some(today()), Some(500)))];
    edit_facts(&mut documents, 1, facts(None, Some(today()), None)).unwrap();
    assert_eq!(
        suggestion(&documents[0], &documents, &transactions, &[]),
        Suggestion::Unreadable
    );
}

#[test]
fn focus_moves_to_the_next_row_or_the_previous_when_last() {
    assert_eq!(focus_after_filing(0, 3), Some(0));
    assert_eq!(focus_after_filing(2, 3), Some(1));
    assert_eq!(focus_after_filing(0, 1), None);
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

#[test]
fn add_refuses_a_duplicate_path_an_unsupported_file_and_a_blank_title() {
    let mut documents = Vec::new();
    add_filed(&mut documents, new_document("/tmp/a.pdf", "A")).unwrap();
    assert_eq!(
        add_filed(&mut documents, new_document("/tmp/a.pdf", "Again")),
        Err(AddError::AlreadyInLibrary("A".to_string()))
    );
    assert_eq!(
        add_filed(&mut documents, new_document("/tmp/a.txt", "A")),
        Err(AddError::UnsupportedFile)
    );
    assert_eq!(
        add_filed(&mut documents, new_document("/tmp/b.pdf", "  ")),
        Err(AddError::NoTitle)
    );
    assert_eq!(documents.len(), 1);
}

#[test]
fn import_makes_an_unfiled_document_readable_only_from_the_catalogue() {
    let mut documents = Vec::new();
    let readable = import_path(
        &types(),
        &mut documents,
        PathBuf::from("/tmp/Coles-Receipt.jpg"),
        today(),
        Source::Import,
    )
    .unwrap();
    let unreadable = import_path(
        &types(),
        &mut documents,
        PathBuf::from("/tmp/scan0001.pdf"),
        today(),
        Source::Dropped,
    )
    .unwrap();

    let readable = get(&documents, readable).unwrap();
    assert!(readable.is_unfiled());
    let intake = readable.intake.as_ref().unwrap();
    assert_eq!(intake.source, Source::Import);
    assert_eq!(intake.facts.merchant.as_deref(), Some("Coles"));

    let unreadable = get(&documents, unreadable).unwrap();
    let intake = unreadable.intake.as_ref().unwrap();
    assert!(intake.facts.total.is_none());
    assert_eq!(intake.source, Source::Dropped);
    assert_eq!(inbox_count(&documents), 2);
}

#[test]
fn import_refuses_a_path_already_filed() {
    let mut documents = Vec::new();
    add_filed(&mut documents, new_document("/tmp/a.pdf", "A")).unwrap();
    assert_eq!(
        import_path(
            &types(),
            &mut documents,
            PathBuf::from("/tmp/a.pdf"),
            today(),
            Source::Dropped
        ),
        Err(AddError::AlreadyInLibrary("A".to_string()))
    );
}

#[test]
fn a_year_scope_prefills_today_inside_the_year_and_its_last_day_outside() {
    // 2 Oct 2026 is in the FY that started on 1 Jul 2026.
    let current = LibraryScope::Year(YearFacet::Year(2026));
    let last = LibraryScope::Year(YearFacet::Year(2025));
    let earlier = LibraryScope::Year(YearFacet::Before(2025));
    assert_eq!(date_for_scope(LibraryScope::All, today()), today());
    assert_eq!(date_for_scope(current, today()), today());
    assert_eq!(date_for_scope(last, today()), date(2026, 6, 30));
    assert_eq!(date_for_scope(earlier, today()), date(2025, 6, 30));
}

#[test]
fn the_rail_runs_inbox_all_types_then_years() {
    let entries = rail_entries(&types(), today());
    assert_eq!(entries.len(), 2 + types().len() + 3);
    assert_eq!(entries[0], RailEntry::Inbox);
    assert_eq!(entries[1], RailEntry::Scope(LibraryScope::All));
    assert_eq!(
        entries.last(),
        Some(&RailEntry::Scope(LibraryScope::Year(YearFacet::Before(
            2025
        ))))
    );
}

#[test]
fn mode_and_sort_ids_round_trip_and_default_when_unknown() {
    for mode in [DocumentsMode::Library, DocumentsMode::Inbox] {
        assert_eq!(DocumentsMode::from_id(Some(mode.id())), mode);
    }
    for sort in [LibrarySort::Newest, LibrarySort::Expiring] {
        assert_eq!(LibrarySort::from_id(Some(sort.id())), sort);
    }
    assert_eq!(
        DocumentsMode::from_id(Some("nonsense")),
        DocumentsMode::Library
    );
    assert_eq!(DocumentsMode::from_id(None), DocumentsMode::Library);
    assert_eq!(LibrarySort::from_id(None), LibrarySort::Newest);
}

#[test]
fn resolve_path_keeps_an_absolute_path_and_makes_a_relative_one_absolute() {
    // A rooted path with no drive letter is relative on Windows, so the absolute fixture needs one there.
    let absolute = if cfg!(windows) {
        "C:/tmp/a.pdf"
    } else {
        "/tmp/a.pdf"
    };
    assert_eq!(
        resolve_path(&format!(" {absolute} ")),
        PathBuf::from(absolute)
    );
    assert!(resolve_path("docs/a.pdf").is_absolute());
    assert_eq!(default_title(Path::new("/tmp/rates-q1.pdf")), "rates-q1");
}

fn set_type(
    types: &mut [DocumentTypeRow],
    kind: DocumentType,
    change: impl FnOnce(&mut DocumentTypeRow),
) {
    if let Some(row) = types.iter_mut().find(|row| row.id == kind.0) {
        change(row);
    }
}

#[test]
fn the_rail_lists_types_in_settings_order() {
    let mut types = types();
    types::move_by(&mut types, DocumentType::BILLS.0, -7);
    let listed: Vec<_> = rail_entries(&types, today())
        .into_iter()
        .filter_map(|entry| match entry {
            RailEntry::Scope(LibraryScope::Type(kind)) => Some(kind),
            _ => None,
        })
        .collect();
    assert_eq!(listed.first(), Some(&DocumentType::BILLS));
    assert_eq!(listed.len(), types.len());
}

#[test]
fn a_saved_scope_on_a_removed_type_falls_back_to_all() {
    let mut types = types();
    let scope = LibraryScope::Type(DocumentType::BILLS);
    assert_eq!(LibraryScope::from_id(&scope.id(), &types, today()), scope);
    types.retain(|row| row.id != DocumentType::BILLS.0);
    assert_eq!(
        LibraryScope::from_id(&scope.id(), &types, today()),
        LibraryScope::All
    );
}

#[test]
fn the_financial_year_flag_gates_the_year_match() {
    let mut types = types();
    let document = filed(1, DocumentType::TAX, date(2026, 6, 30));
    let facet = YearFacet::Year(document.financial_year());
    assert!(facet.contains(&document, &types));
    set_type(&mut types, DocumentType::TAX, |row| {
        row.financial_year = false;
    });
    assert!(!facet.contains(&document, &types));
    // The derived year itself is unchanged: only the match is gated.
    assert_eq!(document.financial_year(), facet_start(facet));
}

fn facet_start(facet: YearFacet) -> i32 {
    match facet {
        YearFacet::Year(start) | YearFacet::Before(start) => start,
    }
}

#[test]
fn the_key_date_kind_follows_the_type_and_goes_inert_when_it_tracks_nothing() {
    let mut types = types();
    let mut document = filed(1, DocumentType::INSURANCE, today());
    document.key_date = Some(KeyDate {
        kind: KeyDateKind::Revalue,
        date: today() + Duration::days(10),
        reminder: false,
    });
    let key = document.effective_key_date(&types).expect("tracks a date");
    assert_eq!(key.kind, KeyDateKind::Renews);
    assert!(key.reminder, "Insurance has a Remind lead time");

    set_type(&mut types, DocumentType::INSURANCE, |row| {
        row.tracks_date = Some(TracksDate::Expires);
        row.remind = None;
    });
    let key = document.effective_key_date(&types).expect("still tracked");
    assert_eq!((key.kind, key.reminder), (KeyDateKind::Expires, false));

    set_type(&mut types, DocumentType::INSURANCE, |row| {
        row.tracks_date = None;
    });
    assert_eq!(document.effective_key_date(&types), None);
    assert!(document.key_date.is_some(), "the date is kept, just inert");
    assert!(!document.needs_review(&types, today()));
}

#[test]
fn an_extracted_type_is_matched_by_name_and_absent_without_a_match() {
    let mut types = types();
    let facts = catalogue_facts(&types, Path::new("/tmp/Coles-Receipt.jpg"), today())
        .expect("in the catalogue");
    assert_eq!(facts.doc_type, Some(DocumentType::RECEIPTS));
    set_type(&mut types, DocumentType::RECEIPTS, |row| {
        row.name = "Dockets".to_string();
    });
    let facts = catalogue_facts(&types, Path::new("/tmp/Coles-Receipt.jpg"), today())
        .expect("in the catalogue");
    assert_eq!(facts.doc_type, None);
}

#[test]
fn a_missing_type_reads_as_the_default() {
    let mut types = types();
    types.retain(|row| row.id != DocumentType::TAX.0);
    assert_eq!(DocumentType::TAX.or_fallback(&types), DocumentType::OTHER);
    assert_eq!(DocumentType::BILLS.or_fallback(&types), DocumentType::BILLS);
}
