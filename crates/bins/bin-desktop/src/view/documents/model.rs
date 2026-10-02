//! The Documents page's display models: the Library's rows, the detail pane's facts and links, and
//! the index rail's entries, built from the Documents and the shared stubs they link to. `gpui`-free
//! so the wording rules (which flags are red, what a chip says) are unit-tested without a window.

use chrono::{Datelike, NaiveDate};
use lib_core::DateStyle;
use lib_locale::format::{format_date, format_year_month, upper};

use crate::{
    accounts::Account,
    bills::BillPlan,
    documents::{
        Document, DocumentLink, DocumentType, FileKind, InventoryItem, KeyDate, KeyDateBand,
        KeyDateKind, LibraryScope, RailEntry, YearFacet, band, scope_count,
    },
    format,
    payees::Payee,
    transactions::Transaction,
};

/// The shared stubs a Document's Links point into, and the clock they are read against.
pub struct Lookups<'a> {
    pub accounts: &'a [Account],
    pub payees: &'a [Payee],
    pub plans: &'a [BillPlan],
    pub inventory: &'a [InventoryItem],
    pub transactions: &'a [Transaction],
    pub today: NaiveDate,
    pub date_style: Option<DateStyle>,
}

/// Which icon a link chip draws.
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

/// The right column's second line: the Type, or a Key Date's flag.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RowTail {
    Type(String),
    Flag {
        text: String,
        /// Only a Need Review flag is red; Upcoming and Stale ones are muted.
        red: bool,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RowView {
    pub id: u32,
    /// `PDF` / `JPG` / `PNG`, drawn inside the file glyph.
    pub extension: &'static str,
    pub title: String,
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

/// One LINKED TO row: the kind label and the record's name, which navigates when clicked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkRow {
    pub kind: String,
    pub name: String,
    pub link: DocumentLink,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DetailView {
    pub id: u32,
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
}

pub fn extension(kind: FileKind) -> &'static str {
    match kind {
        FileKind::Pdf => "PDF",
        FileKind::Jpg => "JPG",
        FileKind::Png => "PNG",
    }
}

/// The Type in the singular, for the detail facts and the row's caps label.
pub fn type_label(kind: DocumentType) -> String {
    match kind {
        DocumentType::Receipt => crate::msg::desktop_documents_type_one_receipt(),
        DocumentType::Statement => crate::msg::desktop_documents_type_one_statement(),
        DocumentType::Tax => crate::msg::desktop_documents_type_one_tax(),
        DocumentType::Insurance => crate::msg::desktop_documents_type_one_insurance(),
        DocumentType::WarrantyManual => crate::msg::desktop_documents_type_one_warranty(),
        DocumentType::Contract => crate::msg::desktop_documents_type_one_contract(),
        DocumentType::Identity => crate::msg::desktop_documents_type_one_identity(),
        DocumentType::Bill => crate::msg::desktop_documents_type_one_bill(),
    }
}

/// The Type in the plural, for the index rail.
pub fn type_plural_label(kind: DocumentType) -> String {
    match kind {
        DocumentType::Receipt => crate::msg::desktop_documents_type_receipts(),
        DocumentType::Statement => crate::msg::desktop_documents_type_statements(),
        DocumentType::Tax => crate::msg::desktop_documents_type_tax(),
        DocumentType::Insurance => crate::msg::desktop_documents_type_insurance(),
        DocumentType::WarrantyManual => crate::msg::desktop_documents_type_warranties(),
        DocumentType::Contract => crate::msg::desktop_documents_type_contracts(),
        DocumentType::Identity => crate::msg::desktop_documents_type_identity(),
        DocumentType::Bill => crate::msg::desktop_documents_type_bills(),
    }
}

/// `FY 2026–27` for the Financial Year starting in `start`.
pub fn year_label(start: i32) -> String {
    crate::msg::desktop_documents_rail_fy(
        &start.to_string(),
        &format!("{:02}", (start + 1).rem_euclid(100)),
    )
}

fn scope_label(scope: LibraryScope) -> String {
    match scope {
        LibraryScope::All => crate::msg::desktop_documents_rail_all(),
        LibraryScope::Type(kind) => type_plural_label(kind),
        LibraryScope::Year(YearFacet::Year(start)) => year_label(start),
        LibraryScope::Year(YearFacet::Before(_)) => crate::msg::desktop_documents_rail_earlier(),
    }
}

/// The index rail's rows, each with its count: the Inbox's Unfiled Documents, or the Filed
/// Documents in a scope (search is not applied).
pub fn rail_rows(documents: &[Document], today: NaiveDate) -> Vec<RailRow> {
    crate::documents::rail_entries(today)
        .into_iter()
        .map(|entry| match entry {
            RailEntry::Inbox => RailRow {
                entry,
                label: crate::msg::desktop_documents_rail_inbox(),
                count: crate::documents::inbox_count(documents),
            },
            RailEntry::Scope(scope) => RailRow {
                entry,
                label: scope_label(scope),
                count: scope_count(documents, scope),
            },
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
        (KeyDateKind::Renews, false) => crate::msg::desktop_documents_flag_renews(&date),
        (KeyDateKind::Renews, true) => crate::msg::desktop_documents_flag_renewed(&date),
        (KeyDateKind::Ends, false) => crate::msg::desktop_documents_flag_ends(&date),
        (KeyDateKind::Ends, true) => crate::msg::desktop_documents_flag_ended(&date),
        (KeyDateKind::Expires, false) => crate::msg::desktop_documents_flag_expires(&date),
        (KeyDateKind::Expires, true) => crate::msg::desktop_documents_flag_expired(&date),
        (KeyDateKind::Revalue, false) => crate::msg::desktop_documents_flag_revalue(&date),
        (KeyDateKind::Revalue, true) => crate::msg::desktop_documents_flag_revalue_overdue(&date),
    }
}

/// The name a Link shows, or `None` when the record has gone, in which case the Link is not drawn.
fn link_name(link: DocumentLink, lookups: &Lookups<'_>) -> Option<(ChipKind, String)> {
    match link {
        DocumentLink::Transaction(id) => {
            let transaction = lookups.transactions.iter().find(|t| t.id == id)?;
            let (_, text) = format::signed_amount(&transaction.total());
            Some((ChipKind::Amount, text))
        }
        DocumentLink::InventoryItem(id) => lookups
            .inventory
            .iter()
            .find(|item| item.id == id)
            .map(|item| (ChipKind::Item, item.name.clone())),
        DocumentLink::Account(id) => lookups
            .accounts
            .iter()
            .find(|account| account.id == id)
            .map(|account| (ChipKind::Account, account.name.clone())),
        DocumentLink::Payee(id) => crate::payees::get(lookups.payees, id)
            .map(|payee| (ChipKind::Payee, payee.name.clone())),
        DocumentLink::BillPlan(id) => {
            crate::bills::get(lookups.plans, id).map(|plan| (ChipKind::Bill, plan.name.clone()))
        }
    }
}

/// The name a LINKED TO row shows. A Transaction reads as its date, Payee-or-memo and amount rather
/// than the chip's bare amount.
fn link_row_name(link: DocumentLink, lookups: &Lookups<'_>) -> Option<String> {
    match link {
        DocumentLink::Transaction(id) => {
            let transaction = lookups.transactions.iter().find(|t| t.id == id)?;
            let (_, amount) = format::signed_amount(&transaction.total());
            let date = format_date(transaction.date, lookups.date_style);
            let what = transaction
                .description
                .clone()
                .or_else(|| {
                    transaction
                        .splits
                        .first()
                        .and_then(|split| split.payee_id)
                        .and_then(|id| crate::payees::get(lookups.payees, id))
                        .map(|payee| payee.name.clone())
                })
                .unwrap_or_default();
            Some(format!("{date} · {what} · {amount}"))
        }
        other => link_name(other, lookups).map(|(_, name)| name),
    }
}

fn link_kind_label(link: DocumentLink) -> String {
    match link {
        DocumentLink::Transaction(_) => crate::msg::desktop_documents_link_transaction(),
        DocumentLink::InventoryItem(_) => crate::msg::desktop_documents_link_inventory(),
        DocumentLink::Account(_) => crate::msg::desktop_documents_link_account(),
        DocumentLink::Payee(_) => crate::msg::desktop_documents_link_payee(),
        DocumentLink::BillPlan(_) => crate::msg::desktop_documents_link_bill(),
    }
}

pub fn row(document: &Document, lookups: &Lookups<'_>) -> RowView {
    let tail = match &document.key_date {
        Some(key_date) => RowTail::Flag {
            text: flag_text(key_date, lookups),
            red: band(key_date, lookups.today) == KeyDateBand::NeedReview,
        },
        None => RowTail::Type(upper(&type_label(document.doc_type))),
    };
    RowView {
        id: document.id,
        extension: extension(document.kind),
        title: document.title.clone(),
        chips: document
            .links
            .iter()
            .filter_map(|link| link_name(*link, lookups))
            .map(|(kind, label)| Chip { kind, label })
            .collect(),
        date: format_date(document.date, lookups.date_style),
        tail,
    }
}

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

fn key_kind_label(kind: KeyDateKind) -> String {
    match kind {
        KeyDateKind::Renews => crate::msg::desktop_documents_detail_key_renews(),
        KeyDateKind::Ends => crate::msg::desktop_documents_detail_key_ends(),
        KeyDateKind::Expires => crate::msg::desktop_documents_detail_key_expires(),
        KeyDateKind::Revalue => crate::msg::desktop_documents_detail_key_revalue(),
    }
}

pub fn detail(document: &Document, lookups: &Lookups<'_>) -> DetailView {
    let mut facts = vec![
        Fact {
            label: crate::msg::desktop_documents_detail_type(),
            value: type_label(document.doc_type),
            red: false,
        },
        Fact {
            label: crate::msg::desktop_documents_detail_date(),
            value: format_date(document.date, lookups.date_style),
            red: false,
        },
    ];
    if let Some(key_date) = &document.key_date {
        let date = format_date(key_date.date, lookups.date_style);
        facts.push(Fact {
            label: key_kind_label(key_date.kind),
            value: if key_date.reminder {
                crate::msg::desktop_documents_detail_reminder(&date)
            } else {
                date
            },
            red: band(key_date, lookups.today) == KeyDateBand::NeedReview,
        });
    }
    facts.push(Fact {
        label: crate::msg::desktop_documents_detail_year(),
        value: year_label(document.financial_year()),
        red: false,
    });
    DetailView {
        id: document.id,
        extension: extension(document.kind),
        title: document.title.clone(),
        meta: crate::msg::desktop_documents_detail_meta(
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
                    name: link_row_name(*link, lookups)?,
                    link: *link,
                })
            })
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        accounts::default_accounts,
        bills::default_bills,
        categories::default_categories,
        documents::{self, DocumentsSeed, default_documents},
        payees::default_payees,
        tags::default_tags,
        transactions::default_transactions,
    };

    struct World {
        accounts: Vec<Account>,
        payees: Vec<Payee>,
        plans: Vec<BillPlan>,
        transactions: Vec<Transaction>,
        seed: DocumentsSeed,
        today: NaiveDate,
    }

    fn world() -> World {
        crate::locale::init_for_tests();
        let today = NaiveDate::from_ymd_opt(2026, 10, 2).expect("a valid test date");
        let accounts = default_accounts();
        let categories = default_categories();
        let payees = default_payees();
        let tags = default_tags();
        let mut transactions = default_transactions(&accounts, &categories, &payees, &tags, today);
        let bills = default_bills(&accounts, &categories, &payees, &mut transactions, today);
        let seed = default_documents(
            &accounts,
            &categories,
            &payees,
            &bills.plans,
            &mut transactions,
            today,
        );
        World {
            accounts,
            payees,
            plans: bills.plans,
            transactions,
            seed,
            today,
        }
    }

    impl World {
        fn lookups(&self) -> Lookups<'_> {
            Lookups {
                accounts: &self.accounts,
                payees: &self.payees,
                plans: &self.plans,
                inventory: &self.seed.inventory,
                transactions: &self.transactions,
                today: self.today,
                date_style: Some(DateStyle::Iso),
            }
        }
    }

    #[test]
    fn only_a_need_review_flag_is_red() {
        let world = world();
        let lookups = world.lookups();
        let (mut red, mut muted) = (0, 0);
        for document in world.seed.documents.iter().filter(|d| d.is_filed()) {
            if let RowTail::Flag { red: is_red, .. } = row(document, &lookups).tail {
                if is_red {
                    assert!(document.needs_review(world.today));
                    red += 1;
                } else {
                    assert!(!document.needs_review(world.today));
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
        let lookups = world.lookups();
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
    fn an_unlinked_document_has_no_chips_and_a_type_tail() {
        let world = world();
        let lookups = world.lookups();
        let passport = world
            .seed
            .documents
            .iter()
            .find(|d| d.title == "Passport — scan")
            .expect("the seed holds the passport scan");
        let view = row(passport, &lookups);
        assert!(view.chips.is_empty());
        assert_eq!(view.extension, "JPG");
        assert!(matches!(view.tail, RowTail::Flag { .. }));

        let untagged = world
            .seed
            .documents
            .iter()
            .filter(|d| d.is_filed() && d.key_date.is_none())
            .map(|d| row(d, &lookups))
            .find(|view| matches!(view.tail, RowTail::Type(_)));
        assert!(
            untagged.is_some(),
            "a row without a Key Date shows its Type"
        );
    }

    #[test]
    fn the_rail_counts_filed_documents_and_the_inbox_separately() {
        let world = world();
        let rows = rail_rows(&world.seed.documents, world.today);
        let inbox = rows
            .iter()
            .find(|row| row.entry == RailEntry::Inbox)
            .expect("the rail opens with the Inbox");
        assert_eq!(inbox.count, documents::inbox_count(&world.seed.documents));
        let all = rows
            .iter()
            .find(|row| row.entry == RailEntry::Scope(LibraryScope::All))
            .expect("the rail has All documents");
        assert_eq!(
            all.count,
            world.seed.documents.iter().filter(|d| d.is_filed()).count()
        );
    }

    #[test]
    fn the_detail_marks_a_need_review_key_date_red_and_lists_its_links() {
        let world = world();
        let lookups = world.lookups();
        let document = world
            .seed
            .documents
            .iter()
            .find(|d| d.is_filed() && d.needs_review(world.today) && !d.links.is_empty())
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
        crate::locale::init_for_tests();
        assert_eq!(year_label(2026), "FY 2026–27");
        assert_eq!(year_label(1999), "FY 1999–00");
    }
}
