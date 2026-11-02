//! The Library's view model: the index rail's rows, a Document's row with its chips and flag, and
//! the detail pane's facts and Links, worded from the Documents and the records their Links name.
//! The wording rules (which flag is red, what a chip says) are here so they are tested without a
//! window. The link picker, still the Desktop's (#968), words its rows with the same functions.

use chrono::{Datelike, NaiveDate};
use lib_core::{DateStyle, DocumentId, Money};
use lib_locale::format::{AmountStyle, format_amount, format_date, format_year_month, upper};

use crate::types::{self, DocumentTypeRow};
use crate::{
    Document, DocumentLink, DocumentType, FileKind, KeyDate, KeyDateBand, KeyDateKind,
    LibraryScope, LinkRecords, RailEntry, YearFacet, band, inbox_count, msg, rail_entries,
    scope_count, tracks_financial_year,
};

/// What a Document's rows, detail and Links are worded from: the Document Types, the records a
/// Link names, the day and the Display Preference dates follow.
pub struct Lookups<'a> {
    pub types: &'a [DocumentTypeRow],
    pub records: &'a LinkRecords,
    pub today: NaiveDate,
    pub date_style: Option<DateStyle>,
}

/// Which mark a link chip draws.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChipKind {
    Account,
    Item,
    Amount,
    Payee,
    Bill,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chip {
    pub kind: ChipKind,
    pub label: String,
}

/// A row's right column's second line: the Type, or a Key Date's flag.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RowTail {
    Type(String),
    Flag {
        text: String,
        /// Only a Need Review flag is red; Upcoming and Stale ones are muted.
        red: bool,
    },
}

/// A Library row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RowView {
    pub id: DocumentId,
    /// `PDF` / `JPG` / `PNG`, drawn inside the file glyph.
    pub extension: &'static str,
    pub title: String,
    /// What the Document links to; empty draws [`LibraryViewModel::not_linked`].
    ///
    /// [`LibraryViewModel::not_linked`]: super::LibraryViewModel::not_linked
    pub chips: Vec<Chip>,
    pub date: String,
    pub tail: RowTail,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fact {
    pub label: String,
    pub value: String,
    pub red: bool,
}

/// One LINKED TO row: the kind label and the record's name, which reveals the record when clicked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkRow {
    pub kind: String,
    pub name: String,
    pub link: DocumentLink,
}

/// The Library's detail pane on the selected Document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DetailView {
    pub id: DocumentId,
    pub extension: &'static str,
    pub title: String,
    pub meta: String,
    pub facts: Vec<Fact>,
    pub links: Vec<LinkRow>,
}

/// One row of the index rail.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RailRow {
    pub entry: RailEntry,
    pub label: String,
    pub count: usize,
    /// The caps section label drawn above the row: on the first Type and the first Financial Year.
    pub section: Option<String>,
}

pub fn extension(kind: FileKind) -> &'static str {
    match kind {
        FileKind::Pdf => "PDF",
        FileKind::Jpg => "JPG",
        FileKind::Png => "PNG",
    }
}

/// The Type's name, for the detail facts, the row's caps label, the rail and the pickers. A type
/// that is gone reads as the Default type, as the Document would be filed under it.
pub fn type_label(types: &[DocumentTypeRow], kind: DocumentType) -> String {
    let kind = kind.or_fallback(types);
    types::get(types, kind.0)
        .map(|row| row.name.clone())
        .unwrap_or_default()
}

/// `FY 2026–27` for the Financial Year starting in `start`.
pub fn year_label(start: i32) -> String {
    msg::documents_rail_fy(
        &start.to_string(),
        &format!("{:02}", (start + 1).rem_euclid(100)),
    )
}

fn scope_label(types: &[DocumentTypeRow], scope: LibraryScope) -> String {
    match scope {
        LibraryScope::All => msg::documents_rail_all(),
        LibraryScope::Type(kind) => type_label(types, kind),
        LibraryScope::Year(YearFacet::Year(start)) => year_label(start),
        LibraryScope::Year(YearFacet::Before(_)) => msg::documents_rail_earlier(),
    }
}

/// The index rail's rows, each with its count: the Inbox's Unfiled Documents, or the Filed
/// Documents in a scope (search is not applied).
pub fn rail_rows(
    types: &[DocumentTypeRow],
    documents: &[Document],
    today: NaiveDate,
) -> Vec<RailRow> {
    let mut previous: Option<RailEntry> = None;
    rail_entries(types, today)
        .into_iter()
        .map(|entry| {
            let section = match (entry, previous) {
                (
                    RailEntry::Scope(LibraryScope::Type(_)),
                    Some(RailEntry::Scope(LibraryScope::Type(_))),
                )
                | (
                    RailEntry::Scope(LibraryScope::Year(_)),
                    Some(RailEntry::Scope(LibraryScope::Year(_))),
                ) => None,
                (RailEntry::Scope(LibraryScope::Type(_)), _) => Some(msg::documents_rail_type()),
                (RailEntry::Scope(LibraryScope::Year(_)), _) => Some(msg::documents_rail_year()),
                _ => None,
            };
            previous = Some(entry);
            match entry {
                RailEntry::Inbox => RailRow {
                    entry,
                    label: msg::documents_rail_inbox(),
                    count: inbox_count(documents),
                    section,
                },
                RailEntry::Scope(scope) => RailRow {
                    entry,
                    label: scope_label(types, scope),
                    count: scope_count(types, documents, scope),
                    section,
                },
            }
        })
        .collect()
}

/// A Key Date's flag text. A Need Review one shows the full date; the muted Upcoming and Stale ones
/// only the month and year. A date that has passed switches to the past tense.
pub fn flag_text(key_date: &KeyDate, lookups: &Lookups<'_>) -> String {
    let date = match band(key_date, lookups.today) {
        KeyDateBand::NeedReview => format_date(key_date.date, lookups.date_style),
        KeyDateBand::Upcoming | KeyDateBand::Stale => {
            format_year_month(key_date.date.year(), key_date.date.month())
        }
    };
    let past = key_date.is_past(lookups.today);
    match (key_date.kind, past) {
        (KeyDateKind::Renews, false) => msg::documents_flag_renews(&date),
        (KeyDateKind::Renews, true) => msg::documents_flag_renewed(&date),
        (KeyDateKind::Ends, false) => msg::documents_flag_ends(&date),
        (KeyDateKind::Ends, true) => msg::documents_flag_ended(&date),
        (KeyDateKind::Expires, false) => msg::documents_flag_expires(&date),
        (KeyDateKind::Expires, true) => msg::documents_flag_expired(&date),
        (KeyDateKind::Revalue, false) => msg::documents_flag_revalue(&date),
        (KeyDateKind::Revalue, true) => msg::documents_flag_revalue_overdue(&date),
    }
}

/// An amount as a chip or a Transaction's line shows it: its own places, `+` on income.
pub(super) fn signed_amount(amount: &Money) -> String {
    format_amount(amount, AmountStyle::own().with_plus()).text
}

/// An amount read off a file, which has no sign: its size, in its own places.
pub(super) fn unsigned_amount(amount: &Money) -> String {
    format_amount(&Money(amount.0.abs()), AmountStyle::own()).text
}

/// The chip a Link draws, or `None` when the record has gone, in which case it draws none.
fn chip(link: DocumentLink, lookups: &Lookups<'_>) -> Option<Chip> {
    let records = lookups.records;
    let (kind, label) = match link {
        DocumentLink::Transaction(id) => (
            ChipKind::Amount,
            signed_amount(&records.transaction(id)?.amount),
        ),
        DocumentLink::InventoryItem(id) => (ChipKind::Item, records.item(id)?.name.clone()),
        DocumentLink::Account(id) => (ChipKind::Account, records.account(id)?.name.clone()),
        DocumentLink::Payee(id) => (ChipKind::Payee, records.payee(id)?.name.clone()),
        DocumentLink::BillPlan(id) => (ChipKind::Bill, records.plan(id)?.name.clone()),
    };
    Some(Chip { kind, label })
}

/// The name a LINKED TO row, a Toast or a candidate shows for `link`, or `None` when the record
/// has gone. A Transaction reads as its date, Payee-or-memo and amount rather than the chip's bare
/// amount.
pub fn link_name(link: DocumentLink, lookups: &Lookups<'_>) -> Option<String> {
    let records = lookups.records;
    match link {
        DocumentLink::Transaction(id) => {
            let transaction = records.transaction(id)?;
            let date = format_date(transaction.date, lookups.date_style);
            let what = transaction
                .memo
                .clone()
                .or_else(|| {
                    // The first Split's Payee only, not the first any Split names.
                    transaction
                        .payee_ids
                        .first()
                        .copied()
                        .flatten()
                        .and_then(|id| records.payee(id))
                        .map(|payee| payee.name.clone())
                })
                .unwrap_or_default();
            Some(format!(
                "{date} · {what} · {}",
                signed_amount(&transaction.amount)
            ))
        }
        other => chip(other, lookups).map(|chip| chip.label),
    }
}

/// What kind of record a Link names, as the LINKED TO rows and the picker's kinds label it.
pub fn link_kind_label(link: DocumentLink) -> String {
    match link {
        DocumentLink::Transaction(_) => msg::documents_link_transaction(),
        DocumentLink::InventoryItem(_) => msg::documents_link_inventory(),
        DocumentLink::Account(_) => msg::documents_link_account(),
        DocumentLink::Payee(_) => msg::documents_link_payee(),
        DocumentLink::BillPlan(_) => msg::documents_link_bill(),
    }
}

/// A Library row.
pub fn row(document: &Document, lookups: &Lookups<'_>) -> RowView {
    let tail = match document.effective_key_date(lookups.types) {
        Some(key_date) => RowTail::Flag {
            text: flag_text(&key_date, lookups),
            red: band(&key_date, lookups.today) == KeyDateBand::NeedReview,
        },
        None => RowTail::Type(upper(&type_label(lookups.types, document.doc_type))),
    };
    RowView {
        id: document.id,
        extension: extension(document.kind),
        title: document.title.clone(),
        chips: document
            .links
            .iter()
            .filter_map(|link| chip(*link, lookups))
            .collect(),
        date: format_date(document.date, lookups.date_style),
        tail,
    }
}

/// A stored size as the status line and the detail show it: KB, MB or GB.
pub fn size_text(bytes: u64) -> String {
    const KB: f64 = 1_000.0;
    const MB: f64 = 1_000_000.0;
    const GB: f64 = 1_000_000_000.0;
    // Display only: precision beyond one decimal place is never shown.
    #[expect(
        clippy::cast_precision_loss,
        reason = "a file size shown to one decimal place"
    )]
    let size = bytes as f64;
    if size >= GB {
        format!("{:.1} GB", size / GB)
    } else if size >= MB {
        format!("{:.1} MB", size / MB)
    } else {
        format!("{:.0} KB", (size / KB).max(1.0))
    }
}

/// A Key Date's kind, as the detail pane labels it and the Desktop's forms offer it.
pub fn key_kind_label(kind: KeyDateKind) -> String {
    match kind {
        KeyDateKind::Renews => msg::documents_detail_key_renews(),
        KeyDateKind::Ends => msg::documents_detail_key_ends(),
        KeyDateKind::Expires => msg::documents_detail_key_expires(),
        KeyDateKind::Revalue => msg::documents_detail_key_revalue(),
    }
}

/// The Library's detail pane on `document`.
pub fn detail(document: &Document, lookups: &Lookups<'_>) -> DetailView {
    let mut facts = vec![
        Fact {
            label: msg::documents_detail_type(),
            value: type_label(lookups.types, document.doc_type),
            red: false,
        },
        Fact {
            label: msg::documents_detail_date(),
            value: format_date(document.date, lookups.date_style),
            red: false,
        },
    ];
    if let Some(key_date) = document.effective_key_date(lookups.types) {
        let date = format_date(key_date.date, lookups.date_style);
        facts.push(Fact {
            label: key_kind_label(key_date.kind),
            value: if key_date.reminder {
                msg::documents_detail_reminder(&date)
            } else {
                date
            },
            red: band(&key_date, lookups.today) == KeyDateBand::NeedReview,
        });
    }
    if tracks_financial_year(lookups.types, document.doc_type) {
        facts.push(Fact {
            label: msg::documents_detail_year(),
            value: year_label(document.financial_year()),
            red: false,
        });
    }
    DetailView {
        id: document.id,
        extension: extension(document.kind),
        title: document.title.clone(),
        meta: msg::documents_detail_meta(
            extension(document.kind),
            i64::from(document.pages),
            &size_text(document.bytes),
        ),
        facts,
        links: document
            .links
            .iter()
            .filter_map(|link| {
                Some(LinkRow {
                    kind: link_kind_label(*link),
                    name: link_name(*link, lookups)?,
                    link: *link,
                })
            })
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::presenter::test_support::{World, world};

    #[test]
    fn only_a_need_review_flag_is_red() {
        let world = world();
        let lookups = world.lookups();
        let (mut red, mut muted) = (0, 0);
        for document in world.documents.iter().filter(|d| d.is_filed()) {
            if let RowTail::Flag { red: is_red, .. } = row(document, &lookups).tail {
                if is_red {
                    assert!(document.needs_review(&world.types, world.today));
                    red += 1;
                } else {
                    assert!(!document.needs_review(&world.types, world.today));
                    muted += 1;
                }
            }
        }
        assert_eq!(red, 3, "the seed puts three Library rows in the window");
        assert!(muted > 0);
    }

    #[test]
    fn a_muted_flag_shows_month_and_year_and_a_red_one_the_full_date() {
        let world = world();
        let lookups = Lookups {
            date_style: Some(DateStyle::Iso),
            ..world.lookups()
        };
        let key = |days: i64, kind| KeyDate {
            kind,
            date: world.today + chrono::Duration::days(days),
            reminder: false,
        };
        let near = flag_text(&key(10, KeyDateKind::Renews), &lookups);
        assert!(near.contains("2026-10-12"), "{near}");
        let far = flag_text(&key(400, KeyDateKind::Ends), &lookups);
        assert!(far.starts_with("Ends "), "{far}");
        // Month and year only, so no ISO day in it (the near flag above shows one).
        assert!(!far.contains("2027-"), "{far}");
        assert!(far.contains("2027"), "{far}");
        let lapsed = flag_text(&key(-10, KeyDateKind::Expires), &lookups);
        assert!(lapsed.starts_with("Expired "), "{lapsed}");
    }

    #[test]
    fn an_unlinked_document_has_no_chips_and_a_row_without_a_key_date_shows_its_type() {
        let world = world();
        let lookups = world.lookups();
        let passport = world.named("Passport — scan");
        let view = row(passport, &lookups);
        assert!(view.chips.is_empty());
        assert_eq!(view.extension, "JPG");
        assert!(matches!(view.tail, RowTail::Flag { .. }));

        let untagged = world
            .documents
            .iter()
            .filter(|d| d.is_filed() && d.key_date.is_none())
            .map(|d| row(d, &lookups))
            .find(|view| matches!(view.tail, RowTail::Type(_)));
        assert!(untagged.is_some());
    }

    #[test]
    fn a_link_whose_record_has_gone_draws_no_chip() {
        let world = world();
        let mut records = world.records.clone();
        let document = world
            .documents
            .iter()
            .find(|d| d.is_filed() && d.links.len() > 1)
            .expect("a Document with several Links");
        let drawn = row(document, &world.lookups()).chips.len();
        records.accounts.clear();
        records.payees.clear();
        records.items.clear();
        records.plans.clear();
        records.transactions.clear();
        let lookups = Lookups {
            records: &records,
            ..world.lookups()
        };

        assert!(drawn > 0);
        assert!(row(document, &lookups).chips.is_empty());
        assert!(detail(document, &lookups).links.is_empty());
    }

    #[test]
    fn the_rail_counts_filed_documents_and_the_inbox_and_labels_each_section_once() {
        let world = world();
        let rows = rail_rows(&world.types, &world.documents, world.today);
        assert_eq!(rows[0].entry, RailEntry::Inbox);
        assert_eq!(rows[0].count, inbox_count(&world.documents));
        assert_eq!(
            rows[1].count,
            world.documents.iter().filter(|d| d.is_filed()).count()
        );
        let sections: Vec<&str> = rows
            .iter()
            .filter_map(|row| row.section.as_deref())
            .collect();
        assert_eq!(sections, ["Type", "Financial year"]);
    }

    #[test]
    fn the_detail_marks_a_need_review_key_date_red_and_lists_its_links() {
        let world = world();
        let lookups = world.lookups();
        let document = world
            .documents
            .iter()
            .find(|d| {
                d.is_filed() && d.needs_review(&world.types, world.today) && !d.links.is_empty()
            })
            .expect("a Need Review document with links");
        let view = detail(document, &lookups);
        assert!(view.facts.iter().any(|fact| fact.red));
        assert_eq!(view.links.len(), document.links.len());
        assert!(view.meta.starts_with("PDF") || view.meta.starts_with("JPG"));
    }

    #[test]
    fn sizes_read_in_kb_mb_and_gb() {
        assert_eq!(size_text(500), "1 KB");
        assert_eq!(size_text(1_400_000), "1.4 MB");
        assert_eq!(size_text(1_800_000_000), "1.8 GB");
    }

    #[test]
    fn financial_years_label_as_a_span() {
        let _world: World = world();
        assert_eq!(year_label(2026), "FY 2026–27");
        assert_eq!(year_label(1999), "FY 1999–00");
    }
}
