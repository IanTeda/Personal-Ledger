//! The Documents page's display models: the Library's rows, the detail pane's facts and links, and
//! the index rail's entries, built from the Documents and the shared stubs they link to. `gpui`-free
//! so the wording rules (which flags are red, what a chip says) are unit-tested without a window.

use chrono::{Datelike, NaiveDate};
use lib_core::DateStyle;
use lib_locale::format::{format_date, format_year_month, upper};

use crate::{
    accounts::Account,
    bills::BillPlan,
    document_types::{self, DocumentTypeRow},
    documents::{
        CANDIDATE_WINDOW_DAYS, Candidate, Document, DocumentLink, DocumentType, FileKind, KeyDate,
        KeyDateBand, KeyDateKind, LibraryScope, RailEntry, Signals, Source, Suggestion, YearFacet,
        band, scope_count, tracks_financial_year,
    },
    format,
    inventory::Inventory,
    payees::Payee,
    transactions::Transaction,
};

/// The shared stubs a Document's Links point into, and the clock they are read against.
pub struct Lookups<'a> {
    pub types: &'a [DocumentTypeRow],
    pub accounts: &'a [Account],
    pub payees: &'a [Payee],
    pub plans: &'a [BillPlan],
    pub inventory: &'a Inventory,
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

/// The Type's name, for the detail facts, the row's caps label, the rail and the pickers. A type
/// that is gone reads as the Default type, as the Document would be filed under it.
pub fn type_label(types: &[DocumentTypeRow], kind: DocumentType) -> String {
    let kind = kind.or_fallback(types);
    document_types::get(types, kind.0)
        .map(|row| row.name.clone())
        .unwrap_or_default()
}

/// `FY 2026–27` for the Financial Year starting in `start`.
pub fn year_label(start: i32) -> String {
    crate::msg::desktop_documents_rail_fy(
        &start.to_string(),
        &format!("{:02}", (start + 1).rem_euclid(100)),
    )
}

fn scope_label(types: &[DocumentTypeRow], scope: LibraryScope) -> String {
    match scope {
        LibraryScope::All => crate::msg::desktop_documents_rail_all(),
        LibraryScope::Type(kind) => type_label(types, kind),
        LibraryScope::Year(YearFacet::Year(start)) => year_label(start),
        LibraryScope::Year(YearFacet::Before(_)) => crate::msg::desktop_documents_rail_earlier(),
    }
}

/// The index rail's rows, each with its count: the Inbox's Unfiled Documents, or the Filed
/// Documents in a scope (search is not applied).
pub fn rail_rows(
    types: &[DocumentTypeRow],
    documents: &[Document],
    today: NaiveDate,
) -> Vec<RailRow> {
    crate::documents::rail_entries(types, today)
        .into_iter()
        .map(|entry| match entry {
            RailEntry::Inbox => RailRow {
                entry,
                label: crate::msg::desktop_documents_rail_inbox(),
                count: crate::documents::inbox_count(documents),
            },
            RailEntry::Scope(scope) => RailRow {
                entry,
                label: scope_label(types, scope),
                count: scope_count(types, documents, scope),
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
            .items
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
            value: type_label(lookups.types, document.doc_type),
            red: false,
        },
        Fact {
            label: crate::msg::desktop_documents_detail_date(),
            value: format_date(document.date, lookups.date_style),
            red: false,
        },
    ];
    if let Some(key_date) = document.effective_key_date(lookups.types) {
        let date = format_date(key_date.date, lookups.date_style);
        facts.push(Fact {
            label: key_kind_label(key_date.kind),
            value: if key_date.reminder {
                crate::msg::desktop_documents_detail_reminder(&date)
            } else {
                date
            },
            red: band(&key_date, lookups.today) == KeyDateBand::NeedReview,
        });
    }
    if tracks_financial_year(lookups.types, document.doc_type) {
        facts.push(Fact {
            label: crate::msg::desktop_documents_detail_year(),
            value: year_label(document.financial_year()),
            red: false,
        });
    }
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

// ---------------------------------------------------------------------------------------------
// The Inbox
// ---------------------------------------------------------------------------------------------

/// What an Inbox row's Suggested Link column says.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InboxState {
    /// A Transaction is suggested; Accept files it.
    Link,
    /// No amount was read: never guess, file by hand.
    Unreadable,
    /// Readable, but nothing within the window: file by hand.
    NoSuggestion,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InboxRowView {
    pub id: u32,
    pub extension: &'static str,
    pub name: String,
    /// "Scanned 29 Sep", "Downloads · 26 Sep".
    pub source: String,
    pub state: InboxState,
    /// The bold line: "Receipt · Woolworths · 212.40", or the Unreadable notice.
    pub summary: String,
    /// The muted line beside the meter: the target Transaction, or what to do instead.
    pub hint: String,
    pub signals: Option<Signals>,
    /// The Transaction Accept would link.
    pub best: Option<u32>,
    pub skipped: bool,
}

/// One Transaction offered in the detail pane: the suggested one, or another candidate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CandidateView {
    pub transaction_id: u32,
    /// "29 Sep · Woolworths · −212.40".
    pub line: String,
    /// "ANZ Platinum · no document yet"; empty for an Other candidate.
    pub sub: String,
    pub signals: Signals,
    pub signals_text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InboxDetailView {
    pub id: u32,
    pub extension: &'static str,
    pub title: String,
    pub meta: String,
    pub facts: Vec<Fact>,
    pub state: InboxState,
    pub suggested: Option<CandidateView>,
    pub others: Vec<CandidateView>,
    /// "Other candidates: none within 7 days", shown only when there are no others.
    pub others_none: String,
}

fn source_text(source: Source, date: NaiveDate, lookups: &Lookups<'_>) -> String {
    let date = format_date(date, lookups.date_style);
    match source {
        Source::Scanned => crate::msg::desktop_documents_source_scanned(&date),
        Source::Emailed => crate::msg::desktop_documents_source_emailed(&date),
        Source::Downloads => crate::msg::desktop_documents_source_downloads(&date),
        Source::WatchedFolder => crate::msg::desktop_documents_source_watched(&date),
        Source::Dropped => crate::msg::desktop_documents_source_dropped(&date),
        Source::Import => crate::msg::desktop_documents_source_import(&date),
    }
}

/// The sentence under the match meter: which Signals agree.
pub fn signals_text(signals: Signals) -> String {
    match (signals.amount, signals.date, signals.payee) {
        (true, true, true) => crate::msg::desktop_documents_signals_all(),
        (true, true, false) => crate::msg::desktop_documents_signals_amount_date(),
        (true, false, true) => crate::msg::desktop_documents_signals_amount_payee(),
        (false, true, true) => crate::msg::desktop_documents_signals_date_payee(),
        (true, false, false) => crate::msg::desktop_documents_signals_amount(),
        (false, false, true) => crate::msg::desktop_documents_signals_payee(),
        (false, true, false) => crate::msg::desktop_documents_signals_date(),
        (false, false, false) => String::new(),
    }
}

fn candidate_view(
    candidate: &Candidate,
    with_sub: bool,
    lookups: &Lookups<'_>,
) -> Option<CandidateView> {
    let line = link_row_name(DocumentLink::Transaction(candidate.transaction_id), lookups)?;
    let sub = if with_sub {
        let account = lookups
            .transactions
            .iter()
            .find(|transaction| transaction.id == candidate.transaction_id)
            .and_then(|transaction| {
                lookups
                    .accounts
                    .iter()
                    .find(|account| account.id == transaction.account_id)
            })
            .map(|account| account.name.clone())
            .unwrap_or_default();
        let documents = if candidate.already_linked {
            crate::msg::desktop_documents_suggested_has_document()
        } else {
            crate::msg::desktop_documents_suggested_no_document()
        };
        crate::msg::desktop_documents_suggested_detail(&account, &documents)
    } else {
        String::new()
    };
    Some(CandidateView {
        transaction_id: candidate.transaction_id,
        line,
        sub,
        signals: candidate.signals,
        signals_text: signals_text(candidate.signals),
    })
}

/// "Receipt · Woolworths · 212.40" from what was read; a missing merchant drops its slot.
fn summary_text(types: &[DocumentTypeRow], document: &Document) -> String {
    let facts = document.intake.as_ref().map(|intake| &intake.facts);
    // No match by name leaves the type absent rather than guessing one.
    let kind = facts
        .and_then(|facts| facts.doc_type)
        .map_or_else(crate::msg::desktop_documents_fact_none, |kind| {
            type_label(types, kind)
        });
    let amount = facts
        .and_then(|facts| facts.total.as_ref())
        .map(|total| format::amount(&lib_core::Money(total.0.abs())).1)
        .unwrap_or_default();
    match facts.and_then(|facts| facts.merchant.as_deref()) {
        Some(merchant) => crate::msg::desktop_documents_suggest_summary(&kind, merchant, &amount),
        None => crate::msg::desktop_documents_suggest_summary_bare(&kind, &amount),
    }
}

/// An Inbox row. The Suggested Link is derived here, on every render, never stored.
pub fn inbox_row(
    document: &Document,
    documents: &[Document],
    lookups: &Lookups<'_>,
) -> InboxRowView {
    let suggestion =
        crate::documents::suggestion(document, documents, lookups.transactions, lookups.payees);
    let (state, summary, hint, signals, best) = match &suggestion {
        Suggestion::Unreadable => (
            InboxState::Unreadable,
            crate::msg::desktop_documents_suggest_unreadable(),
            crate::msg::desktop_documents_suggest_unreadable_hint(),
            None,
            None,
        ),
        Suggestion::Nothing => (
            InboxState::NoSuggestion,
            crate::msg::desktop_documents_suggest_none(&CANDIDATE_WINDOW_DAYS.to_string()),
            crate::msg::desktop_documents_suggest_none_hint(),
            None,
            None,
        ),
        Suggestion::Link { best, .. } => (
            InboxState::Link,
            summary_text(lookups.types, document),
            link_row_name(DocumentLink::Transaction(best.transaction_id), lookups)
                .unwrap_or_default(),
            Some(best.signals),
            Some(best.transaction_id),
        ),
    };
    let (source, received) = document
        .intake
        .as_ref()
        .map_or((Source::Import, document.date), |intake| {
            (intake.source, intake.received_at)
        });
    InboxRowView {
        id: document.id,
        extension: extension(document.kind),
        name: document.file_name(),
        source: source_text(source, received, lookups),
        state,
        summary,
        hint,
        signals,
        best,
        skipped: document.is_skipped(),
    }
}

/// The Inbox's focused-row detail: what was read, the Suggested Link and the next candidates.
pub fn inbox_detail(
    document: &Document,
    documents: &[Document],
    lookups: &Lookups<'_>,
) -> InboxDetailView {
    let suggestion =
        crate::documents::suggestion(document, documents, lookups.transactions, lookups.payees);
    let facts = document
        .intake
        .as_ref()
        .map(|intake| intake.facts.clone())
        .unwrap_or_default();
    let none = crate::msg::desktop_documents_fact_none();
    let fact = |label: String, value: Option<String>| Fact {
        label,
        value: value.unwrap_or_else(|| none.clone()),
        red: false,
    };
    let (state, suggested, others) = match &suggestion {
        Suggestion::Unreadable => (InboxState::Unreadable, None, Vec::new()),
        Suggestion::Nothing => (InboxState::NoSuggestion, None, Vec::new()),
        Suggestion::Link { best, others } => (
            InboxState::Link,
            candidate_view(best, true, lookups),
            others
                .iter()
                .filter_map(|other| candidate_view(other, false, lookups))
                .collect(),
        ),
    };
    let meta = match (state, document.kind) {
        (InboxState::Unreadable, _) => crate::msg::desktop_documents_inbox_meta_unreadable(),
        (_, FileKind::Pdf) => crate::msg::desktop_documents_inbox_meta_file(),
        (_, _) => crate::msg::desktop_documents_inbox_meta_image(),
    };
    InboxDetailView {
        id: document.id,
        extension: extension(document.kind),
        title: document.file_name(),
        meta,
        facts: vec![
            fact(
                crate::msg::desktop_documents_fact_merchant(),
                facts.merchant.clone(),
            ),
            fact(
                crate::msg::desktop_documents_detail_date(),
                facts.date.map(|date| format_date(date, lookups.date_style)),
            ),
            fact(
                crate::msg::desktop_documents_fact_total(),
                facts
                    .total
                    .as_ref()
                    .map(|total| format::amount(&lib_core::Money(total.0.abs())).1),
            ),
            fact(
                crate::msg::desktop_documents_detail_type(),
                facts.doc_type.map(|kind| type_label(lookups.types, kind)),
            ),
        ],
        state,
        suggested,
        others,
        others_none: crate::msg::desktop_documents_other_none(&CANDIDATE_WINDOW_DAYS.to_string()),
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
        types: Vec<DocumentTypeRow>,
        accounts: Vec<Account>,
        payees: Vec<Payee>,
        plans: Vec<BillPlan>,
        transactions: Vec<Transaction>,
        seed: DocumentsSeed,
        inventory: Inventory,
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
            types: document_types::default_types(),
            accounts,
            payees,
            plans: bills.plans,
            transactions,
            seed,
            inventory,
            today,
        }
    }

    impl World {
        fn lookups(&self) -> Lookups<'_> {
            Lookups {
                types: &self.types,
                accounts: &self.accounts,
                payees: &self.payees,
                plans: &self.plans,
                inventory: &self.inventory,
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
        let rows = rail_rows(&world.types, &world.seed.documents, world.today);
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
        crate::locale::init_for_tests();
        assert_eq!(year_label(2026), "FY 2026–27");
        assert_eq!(year_label(1999), "FY 1999–00");
    }

    fn inbox_named<'a>(world: &'a World, file: &str) -> &'a Document {
        world
            .seed
            .documents
            .iter()
            .find(|d| d.is_unfiled() && d.file_name() == file)
            .expect("the seed holds that Inbox file")
    }

    #[test]
    fn a_strong_row_summarises_what_was_read_and_names_its_transaction() {
        let world = world();
        let lookups = world.lookups();
        let row = inbox_row(
            inbox_named(&world, "IMG_4471.jpg"),
            &world.seed.documents,
            &lookups,
        );
        assert_eq!(row.state, InboxState::Link);
        assert!(row.summary.starts_with("Receipts · "), "{}", row.summary);
        assert!(row.summary.contains("212.40"), "{}", row.summary);
        let signals = row.signals.expect("a linked row has a meter");
        assert_eq!(signals.count(), 3);
        assert!(row.best.is_some());
        assert!(row.hint.contains("212.40"), "{}", row.hint);
        assert!(!row.skipped);
    }

    #[test]
    fn an_unreadable_row_never_guesses_and_a_row_with_nothing_asks_to_be_filed_by_hand() {
        let world = world();
        let lookups = world.lookups();
        let unreadable = inbox_row(
            inbox_named(&world, "scan0012.pdf"),
            &world.seed.documents,
            &lookups,
        );
        assert_eq!(unreadable.state, InboxState::Unreadable);
        assert_eq!(unreadable.summary, "Unreadable — no amount found");
        assert_eq!(unreadable.best, None);
        assert_eq!(unreadable.signals, None);

        let nothing = inbox_row(
            inbox_named(&world, "ATO_NOA_2026.pdf"),
            &world.seed.documents,
            &lookups,
        );
        assert_eq!(nothing.state, InboxState::NoSuggestion);
        assert!(
            nothing.summary.contains("nothing within 7 days"),
            "{}",
            nothing.summary
        );
        assert_eq!(nothing.best, None);
    }

    #[test]
    fn the_meter_text_names_exactly_the_signals_that_agree() {
        crate::locale::init_for_tests();
        let signals = |amount, date, payee| Signals {
            amount,
            date,
            payee,
        };
        assert_eq!(
            signals_text(signals(true, true, true)),
            "amount, date and payee match"
        );
        assert_eq!(
            signals_text(signals(true, false, true)),
            "amount and payee match"
        );
        assert_eq!(signals_text(signals(true, false, false)), "amount matches");
        assert_eq!(signals_text(signals(false, false, true)), "payee matches");
        assert_eq!(signals_text(signals(false, false, false)), "");
    }

    #[test]
    fn the_inbox_detail_shows_the_facts_the_suggested_link_and_the_others() {
        let world = world();
        let lookups = world.lookups();
        let detail = inbox_detail(
            inbox_named(&world, "IMG_4471.jpg"),
            &world.seed.documents,
            &lookups,
        );
        assert_eq!(detail.title, "IMG_4471.jpg");
        assert_eq!(detail.meta, "Read from the image · check before accepting");
        let labels: Vec<_> = detail
            .facts
            .iter()
            .map(|fact| fact.label.as_str())
            .collect();
        assert_eq!(labels, ["Merchant", "Document date", "Total", "Type"]);
        let card = detail.suggested.expect("a readable file has a suggestion");
        assert!(card.line.contains("212.40"), "{}", card.line);
        assert!(card.sub.ends_with("no document yet"), "{}", card.sub);
        assert_eq!(card.signals_text, "amount, date and payee match");
        assert!(detail.others_none.contains("7 days"));

        let pdf = inbox_detail(
            inbox_named(&world, "bunnings-invoice-INV88213.pdf"),
            &world.seed.documents,
            &lookups,
        );
        assert_eq!(pdf.meta, "Read from the file · check before accepting");
    }

    #[test]
    fn an_unreadable_detail_has_blank_facts_and_no_card() {
        let world = world();
        let lookups = world.lookups();
        let detail = inbox_detail(
            inbox_named(&world, "scan0012.pdf"),
            &world.seed.documents,
            &lookups,
        );
        assert_eq!(detail.state, InboxState::Unreadable);
        assert_eq!(detail.suggested, None);
        assert!(detail.others.is_empty());
        assert_eq!(detail.meta, "Nothing could be read from this file");
        let total = detail
            .facts
            .iter()
            .find(|fact| fact.label == "Total")
            .expect("a Total fact");
        assert_eq!(total.value, "—");
    }
}
