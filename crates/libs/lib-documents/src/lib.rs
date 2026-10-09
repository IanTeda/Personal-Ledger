//! Pure Documents model, rules and stub seed (`docs/ux/desktop-mockups/04-documents/`): `gpui`-free and
//! side-effect free, so every rule is unit-tested without a window. All data is stubbed and
//! in-memory; nothing here reads `lib_database` or the file system.
//!
//! The rules are the Desktop Documents Surface map's decision tickets (#431–#433, #435):
//!
//! - There is one entity, the [`Document`], Unfiled (the Inbox) or Filed. Only the user files a
//!   Document, never gaining a Link, so a Filed Document with no Links is valid.
//! - The Financial Year is always derived from the document date ([`Document::financial_year`]),
//!   never stored.
//! - A Key Date [`band`]s as Need Review inside [`KEY_DATE_ATTENTION_DAYS`] either side of today,
//!   both ends inclusive; further ahead it is Upcoming, further back Stale. The reminder flag
//!   changes neither.
//! - The Suggested Link is derived on every call ([`suggestion`]) and never stored: only
//!   Transactions within [`CANDIDATE_WINDOW_DAYS`] of the Document's date are candidates, each must
//!   agree on amount or payee, and the date Signal allows [`DATE_SIGNAL_DAYS`].
//! - Skip is session-only, and undo is one level: [`undo`] reverses the last [`FilingUndo`] whole.

pub mod types;

#[cfg(test)]
mod tests;

use std::path::{Path, PathBuf};

use bigdecimal::BigDecimal;
use chrono::{Datelike, Duration, NaiveDate};
use lib_core::{Money, TransactionStatus};

use lib_accounts::Account;
use lib_bills::{BillPlan, FINANCIAL_YEAR_START_MONTH, financial_year_start};
use lib_categories::{self as categories, Category};
use lib_inventory::Inventory;
use lib_payees::{self as payees, Payee};
use lib_transactions::{Split, Transaction};

use crate::types::{DocumentTypeRow, TracksDate};

/// Days either side of today, inclusive, in which a Key Date makes a Filed Document Need Review.
/// A fixed constant until the Notifications effort decides on a user-set lead.
pub const KEY_DATE_ATTENTION_DAYS: i64 = 60;

/// Days either side of the Document's date in which a Transaction can be a candidate.
pub const CANDIDATE_WINDOW_DAYS: i64 = 7;

/// Days either side of the extracted date in which a candidate's date Signal agrees.
pub const DATE_SIGNAL_DAYS: i64 = 3;

/// How many candidates after the Suggested Link the Inbox offers as "Other candidates".
pub const OTHER_CANDIDATES: usize = 2;

/// A Document's type: the stable id of a user-managed [`DocumentTypeRow`] (ADR-0031). Names, order
/// and the per-type flags live in the list, so a Document never carries them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct DocumentType(pub u32);

impl DocumentType {
    // The nine seeded types' ids, for the mock data and tests that name one.
    pub const RECEIPTS: DocumentType = DocumentType(1);
    pub const STATEMENTS: DocumentType = DocumentType(2);
    pub const TAX: DocumentType = DocumentType(3);
    pub const INSURANCE: DocumentType = DocumentType(4);
    pub const WARRANTIES: DocumentType = DocumentType(5);
    pub const CONTRACTS: DocumentType = DocumentType(6);
    pub const IDENTITY: DocumentType = DocumentType(7);
    pub const BILLS: DocumentType = DocumentType(8);
    pub const OTHER: DocumentType = DocumentType(9);

    /// The id persisted in a Library scope (`type:<id>`); never shown.
    pub fn id(self) -> String {
        self.0.to_string()
    }

    /// Parses a persisted id, only when `types` still has that type.
    pub fn from_id(id: &str, types: &[DocumentTypeRow]) -> Option<Self> {
        let id = id.parse().ok()?;
        types::position(types, id).map(|_| DocumentType(id))
    }

    /// The Default type's id when the Document's own is gone.
    pub fn or_fallback(self, types: &[DocumentTypeRow]) -> Self {
        types::position(types, self.0).map_or(DocumentType(types::fallback_id(types)), |_| self)
    }
}

/// Whether the type's Financial year flag is on (#476): off for a type that is gone.
pub fn tracks_financial_year(types: &[DocumentTypeRow], kind: DocumentType) -> bool {
    types::get(types, kind.0).is_some_and(|row| row.financial_year)
}

/// The kind of a [`KeyDate`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyDateKind {
    Renews,
    /// Also a warranty's end.
    Ends,
    Expires,
    Revalue,
}

/// The one forward-dated obligation a Document may carry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyDate {
    pub kind: KeyDateKind,
    pub date: NaiveDate,
    /// Intent only: it adds "· reminder set" to the detail facts and changes nothing else.
    pub reminder: bool,
}

impl KeyDate {
    /// Whether the date has passed, which switches the flag to its past-tense label ("Expired …").
    pub fn is_past(&self, today: NaiveDate) -> bool {
        self.date < today
    }
}

/// Where a Key Date sits against today.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyDateBand {
    /// Within [`KEY_DATE_ATTENTION_DAYS`] either side of today, inclusive: red and counted.
    NeedReview,
    /// Further ahead than the window: muted.
    Upcoming,
    /// Lapsed for longer than the window: muted.
    Stale,
}

pub fn band(key_date: &KeyDate, today: NaiveDate) -> KeyDateBand {
    let window = Duration::days(KEY_DATE_ATTENTION_DAYS);
    if key_date.date > today + window {
        KeyDateBand::Upcoming
    } else if key_date.date < today - window {
        KeyDateBand::Stale
    } else {
        KeyDateBand::NeedReview
    }
}

/// A Document's reference to one record it proves.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DocumentLink {
    /// The whole Transaction, never one Split.
    Transaction(u32),
    InventoryItem(u32),
    Account(u32),
    Payee(u32),
    BillPlan(u32),
}

/// The file format, for the glyph's extension label.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileKind {
    Pdf,
    Jpg,
    Png,
}

/// Where an Unfiled Document arrived from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    Scanned,
    Emailed,
    Downloads,
    WatchedFolder,
    Dropped,
    /// Typed into the `Import…` dialog.
    Import,
}

/// What was read from a file; any part may be absent. No `total` makes it Unreadable.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ExtractedFacts {
    pub merchant: Option<String>,
    pub date: Option<NaiveDate>,
    /// The magnitude read from the file; its sign is ignored when compared.
    pub total: Option<Money>,
    pub doc_type: Option<DocumentType>,
}

/// How a Document came in through the Inbox. It stays on the Document once Filed, so [`undo`] can
/// put it back.
#[derive(Debug, Clone, PartialEq)]
pub struct Intake {
    pub source: Source,
    pub received_at: NaiveDate,
    pub facts: ExtractedFacts,
    /// Session-only: set aside to the bottom of the Inbox. Never persisted.
    pub skipped: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Filing {
    /// In the Inbox.
    Unfiled,
    Filed,
}

/// A reference to one file beside the Ledger, plus its metadata.
#[derive(Debug, Clone, PartialEq)]
pub struct Document {
    pub id: u32,
    /// Relative to the Ledger's data directory; never renamed or moved.
    pub path: PathBuf,
    pub kind: FileKind,
    pub pages: u32,
    pub bytes: u64,
    pub title: String,
    pub doc_type: DocumentType,
    /// The document date. An Unfiled Document carries its received date until filed.
    pub date: NaiveDate,
    pub key_date: Option<KeyDate>,
    pub links: Vec<DocumentLink>,
    /// The seeded stand-in for text read out of the file, which Search also matches.
    pub extracted_text: Option<String>,
    pub filing: Filing,
    /// `None` for a Document filed without passing through the Inbox.
    pub intake: Option<Intake>,
}

impl Document {
    /// The calendar year its Financial Year starts in, derived from the document date.
    pub fn financial_year(&self) -> i32 {
        financial_year_start(self.date)
    }

    pub fn is_filed(&self) -> bool {
        self.filing == Filing::Filed
    }

    pub fn is_unfiled(&self) -> bool {
        self.filing == Filing::Unfiled
    }

    pub fn is_skipped(&self) -> bool {
        self.is_unfiled() && self.intake.as_ref().is_some_and(|intake| intake.skipped)
    }

    /// The file name, the Title's default.
    pub fn file_name(&self) -> String {
        self.path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default()
    }

    /// The Key Date as its type reads it (#474, #475): the kind comes from the type's Tracks date,
    /// the reminder from its Remind lead time, and a type that tracks no date leaves the stored
    /// date inert (kept, but neither shown nor counted).
    pub fn effective_key_date(&self, types: &[DocumentTypeRow]) -> Option<KeyDate> {
        let stored = self.key_date.as_ref()?;
        let row = types::get(types, self.doc_type.0)?;
        let kind = match row.tracks_date? {
            TracksDate::Renews => KeyDateKind::Renews,
            TracksDate::Ends => KeyDateKind::Ends,
            TracksDate::Expires => KeyDateKind::Expires,
            TracksDate::Revalue => KeyDateKind::Revalue,
        };
        Some(KeyDate {
            kind,
            date: stored.date,
            reminder: row.remind.is_some(),
        })
    }

    pub fn band(&self, types: &[DocumentTypeRow], today: NaiveDate) -> Option<KeyDateBand> {
        self.effective_key_date(types)
            .map(|key_date| band(&key_date, today))
    }

    pub fn needs_review(&self, types: &[DocumentTypeRow], today: NaiveDate) -> bool {
        self.is_filed() && self.band(types, today) == Some(KeyDateBand::NeedReview)
    }

    /// Case-insensitive substring over the Title, file name, Extracted Facts merchant and the
    /// extracted text. A blank query matches everything.
    pub fn matches(&self, query: &str) -> bool {
        let needle = query.trim().to_lowercase();
        if needle.is_empty() {
            return true;
        }
        let merchant = self
            .intake
            .as_ref()
            .and_then(|intake| intake.facts.merchant.as_deref());
        [
            Some(self.title.as_str()),
            Some(self.file_name().as_str()),
            merchant,
            self.extracted_text.as_deref(),
        ]
        .into_iter()
        .flatten()
        .any(|haystack| haystack.to_lowercase().contains(&needle))
    }
}

pub fn get(documents: &[Document], id: u32) -> Option<&Document> {
    documents.iter().find(|document| document.id == id)
}

fn get_mut(documents: &mut [Document], id: u32) -> Option<&mut Document> {
    documents.iter_mut().find(|document| document.id == id)
}

// ---------------------------------------------------------------------------------------------
// Library
// ---------------------------------------------------------------------------------------------

/// A Financial Year facet on the index rail.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum YearFacet {
    /// The Financial Year starting in that calendar year.
    Year(i32),
    /// Every Financial Year before the one starting in that calendar year.
    Before(i32),
}

impl YearFacet {
    /// Only a Document whose type has the Financial year flag on matches (#476).
    pub fn contains(self, document: &Document, types: &[DocumentTypeRow]) -> bool {
        if !tracks_financial_year(types, document.doc_type) {
            return false;
        }
        match self {
            YearFacet::Year(start) => document.financial_year() == start,
            YearFacet::Before(start) => document.financial_year() < start,
        }
    }
}

/// The rail's Financial Year facets, in order: this one, the last one, then everything earlier.
pub fn year_facets(today: NaiveDate) -> [YearFacet; 3] {
    let current = financial_year_start(today);
    [
        YearFacet::Year(current),
        YearFacet::Year(current - 1),
        YearFacet::Before(current - 1),
    ]
}

/// What the Library lists. Facets are single-select, each replacing the scope.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LibraryScope {
    #[default]
    All,
    Type(DocumentType),
    Year(YearFacet),
}

impl LibraryScope {
    /// The stable id persisted between runs: `all`, `type:<id>`, `fy:<start year>` or
    /// `fy:before:<start year>`.
    pub fn id(self) -> String {
        match self {
            LibraryScope::All => "all".to_string(),
            LibraryScope::Type(kind) => format!("type:{}", kind.id()),
            LibraryScope::Year(YearFacet::Year(start)) => format!("fy:{start}"),
            LibraryScope::Year(YearFacet::Before(start)) => format!("fy:before:{start}"),
        }
    }

    /// Parses a persisted id, falling back to All for one that is malformed, whose type was removed,
    /// or whose Financial Year is no longer on the rail (the year rolled over since it was saved).
    pub fn from_id(id: &str, types: &[DocumentTypeRow], today: NaiveDate) -> Self {
        let parsed = if id == "all" {
            Some(LibraryScope::All)
        } else if let Some(kind) = id.strip_prefix("type:") {
            DocumentType::from_id(kind, types).map(LibraryScope::Type)
        } else if let Some(start) = id.strip_prefix("fy:before:") {
            start
                .parse()
                .ok()
                .map(|start| LibraryScope::Year(YearFacet::Before(start)))
        } else if let Some(start) = id.strip_prefix("fy:") {
            start
                .parse()
                .ok()
                .map(|start| LibraryScope::Year(YearFacet::Year(start)))
        } else {
            None
        };
        match parsed {
            Some(LibraryScope::Year(facet)) if !year_facets(today).contains(&facet) => {
                LibraryScope::All
            }
            Some(scope) => scope,
            None => LibraryScope::All,
        }
    }

    pub fn contains(self, document: &Document, types: &[DocumentTypeRow]) -> bool {
        match self {
            LibraryScope::All => true,
            LibraryScope::Type(kind) => document.doc_type == kind,
            LibraryScope::Year(facet) => facet.contains(document, types),
        }
    }
}

/// Which half of the destination shows: the Library's scoped list or the Inbox. Moving the index
/// rail onto the Inbox row is what switches it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DocumentsMode {
    #[default]
    Library,
    Inbox,
}

impl DocumentsMode {
    /// The stable id persisted between runs.
    pub fn id(self) -> &'static str {
        match self {
            DocumentsMode::Library => "library",
            DocumentsMode::Inbox => "inbox",
        }
    }

    /// Parses a persisted id; anything unknown is the Library.
    pub fn from_id(id: Option<&str>) -> Self {
        match id {
            Some("inbox") => DocumentsMode::Inbox,
            _ => DocumentsMode::Library,
        }
    }
}

/// One selectable row of the index rail.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RailEntry {
    Inbox,
    Scope(LibraryScope),
}

/// The selectable rows in rail order: the Inbox, All documents, every Document Type in Settings
/// order, then the Financial Years. The section labels between them are not rows.
pub fn rail_entries(types: &[DocumentTypeRow], today: NaiveDate) -> Vec<RailEntry> {
    let mut entries = vec![RailEntry::Inbox, RailEntry::Scope(LibraryScope::All)];
    entries.extend(
        types
            .iter()
            .map(|row| RailEntry::Scope(LibraryScope::Type(DocumentType(row.id)))),
    );
    entries.extend(
        year_facets(today)
            .into_iter()
            .map(|facet| RailEntry::Scope(LibraryScope::Year(facet))),
    );
    entries
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LibrarySort {
    #[default]
    Newest,
    Expiring,
}

impl LibrarySort {
    /// The stable id persisted between runs.
    pub fn id(self) -> &'static str {
        match self {
            LibrarySort::Newest => "newest",
            LibrarySort::Expiring => "expiring",
        }
    }

    /// Parses a persisted id; anything unknown is Newest.
    pub fn from_id(id: Option<&str>) -> Self {
        match id {
            Some("expiring") => LibrarySort::Expiring,
            _ => LibrarySort::Newest,
        }
    }

    pub fn toggle(self) -> Self {
        match self {
            LibrarySort::Newest => LibrarySort::Expiring,
            LibrarySort::Expiring => LibrarySort::Newest,
        }
    }
}

/// How many Filed Documents a scope holds, for the rail's counts (search is not applied).
pub fn scope_count(
    types: &[DocumentTypeRow],
    documents: &[Document],
    scope: LibraryScope,
) -> usize {
    documents
        .iter()
        .filter(|document| document.is_filed() && scope.contains(document, types))
        .count()
}

/// The Library's rows: Filed Documents in `scope` matching `query`, in `sort` order.
pub fn library_rows<'a>(
    types: &[DocumentTypeRow],
    documents: &'a [Document],
    scope: LibraryScope,
    query: &str,
    sort: LibrarySort,
    today: NaiveDate,
) -> Vec<&'a Document> {
    let mut rows: Vec<&Document> = documents
        .iter()
        .filter(|document| {
            document.is_filed() && scope.contains(document, types) && document.matches(query)
        })
        .collect();
    // Newest document date, then Title A→Z, then id so equal rows never swap between renders.
    let newest = |a: &&Document, b: &&Document| {
        b.date
            .cmp(&a.date)
            .then_with(|| a.title.cmp(&b.title))
            .then_with(|| a.id.cmp(&b.id))
    };
    match sort {
        LibrarySort::Newest => rows.sort_by(newest),
        LibrarySort::Expiring => rows.sort_by(|a, b| {
            expiring_key(types, a, today)
                .cmp(&expiring_key(types, b, today))
                .then_with(|| newest(a, b))
        }),
    }
    rows
}

/// Need Review ascending, Upcoming ascending, Stale descending (most recently lapsed first), then
/// no Key Date.
fn expiring_key(types: &[DocumentTypeRow], document: &Document, today: NaiveDate) -> (u8, i64) {
    match document.effective_key_date(types) {
        Some(key_date) => {
            let days = (key_date.date - today).num_days();
            match band(&key_date, today) {
                KeyDateBand::NeedReview => (0, days),
                KeyDateBand::Upcoming => (1, days),
                KeyDateBand::Stale => (2, -days),
            }
        }
        None => (3, 0),
    }
}

/// The count line's "N NEED REVIEW": Need Review rows among those listed.
pub fn need_review_count(types: &[DocumentTypeRow], rows: &[&Document], today: NaiveDate) -> usize {
    rows.iter()
        .filter(|document| document.needs_review(types, today))
        .count()
}

/// The Inbox badge: every Unfiled Document.
pub fn inbox_count(documents: &[Document]) -> usize {
    documents
        .iter()
        .filter(|document| document.is_unfiled())
        .count()
}

/// The status bar's stored size: every Document's file, Filed or not.
pub fn total_bytes(documents: &[Document]) -> u64 {
    documents.iter().map(|document| document.bytes).sum()
}

/// Adds `link` when the Document lacks it, removes it when it has it, and returns whether it was
/// added. Filing never changes: removing the last Link leaves the Document Filed and "not linked".
pub fn toggle_link(documents: &mut [Document], id: u32, link: DocumentLink) -> Option<bool> {
    let document = get_mut(documents, id)?;
    if let Some(at) = document.links.iter().position(|existing| *existing == link) {
        document.links.remove(at);
        Some(false)
    } else {
        document.links.push(link);
        Some(true)
    }
}

// ---------------------------------------------------------------------------------------------
// Inbox: the Suggested Link
// ---------------------------------------------------------------------------------------------

/// Which of amount, date and payee agree with a candidate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Signals {
    pub amount: bool,
    pub date: bool,
    pub payee: bool,
}

impl Signals {
    pub fn count(self) -> usize {
        [self.amount, self.date, self.payee]
            .into_iter()
            .filter(|agrees| *agrees)
            .count()
    }

    /// `None` when nothing agrees.
    pub fn strength(self) -> Option<SignalStrength> {
        match self.count() {
            3 => Some(SignalStrength::Strong),
            2 => Some(SignalStrength::Likely),
            1 => Some(SignalStrength::Weak),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SignalStrength {
    Weak,
    Likely,
    Strong,
}

/// A Transaction that could be an Unfiled Document's Link.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Candidate {
    pub transaction_id: u32,
    pub signals: Signals,
    /// Days between the Transaction and the Document's date, unsigned.
    pub gap_days: i64,
    /// Whether another Document already links this Transaction; it still qualifies, ranked lower.
    pub already_linked: bool,
}

/// What the Inbox offers for one Unfiled Document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Suggestion {
    /// No total was read: never guess, file by hand.
    Unreadable,
    /// Readable but nothing qualifies within the window: file by hand.
    Nothing,
    Link {
        best: Candidate,
        /// Up to [`OTHER_CANDIDATES`], each acceptable instead.
        others: Vec<Candidate>,
    },
}

impl Suggestion {
    pub fn best(&self) -> Option<&Candidate> {
        match self {
            Suggestion::Link { best, .. } => Some(best),
            _ => None,
        }
    }

    pub fn is_strong(&self) -> bool {
        self.best()
            .is_some_and(|best| best.signals.strength() == Some(SignalStrength::Strong))
    }
}

/// The Payee an extracted merchant names: an alias match, else the active Payee whose name the
/// merchant contains (case-insensitive, the longest name winning).
pub fn merchant_payee(payees: &[Payee], merchant: &str) -> Option<u32> {
    payees::match_alias(payees, merchant).or_else(|| {
        let haystack = merchant.to_lowercase();
        payees
            .iter()
            .filter(|payee| payee.is_active && haystack.contains(&payee.name.to_lowercase()))
            .max_by(|a, b| {
                a.name
                    .len()
                    .cmp(&b.name.len())
                    .then_with(|| b.name.cmp(&a.name))
            })
            .map(|payee| payee.id)
    })
}

/// Every qualifying candidate for an Unfiled Document's facts, best first. Empty when Unreadable.
pub fn candidates(
    document: &Document,
    documents: &[Document],
    transactions: &[Transaction],
    payees: &[Payee],
) -> Vec<Candidate> {
    let Some(intake) = &document.intake else {
        return Vec::new();
    };
    let facts = &intake.facts;
    let Some(total) = &facts.total else {
        return Vec::new();
    };
    let anchor = facts.date.unwrap_or(intake.received_at);
    let magnitude = total.0.abs();
    let payee_id = facts
        .merchant
        .as_deref()
        .and_then(|merchant| merchant_payee(payees, merchant));

    let mut found: Vec<Candidate> = transactions
        .iter()
        .filter(|transaction| (transaction.date - anchor).num_days().abs() <= CANDIDATE_WINDOW_DAYS)
        .filter_map(|transaction| {
            let signals = Signals {
                amount: transaction.total().0.abs() == magnitude,
                date: facts.date.is_some_and(|date| {
                    (transaction.date - date).num_days().abs() <= DATE_SIGNAL_DAYS
                }),
                payee: payee_id.is_some_and(|id| {
                    transaction
                        .splits
                        .iter()
                        .any(|split| split.payee_id == Some(id))
                }),
            };
            // Date alone never makes a candidate.
            (signals.amount || signals.payee).then(|| Candidate {
                transaction_id: transaction.id,
                signals,
                gap_days: (transaction.date - anchor).num_days().abs(),
                already_linked: documents.iter().any(|other| {
                    other.id != document.id
                        && other
                            .links
                            .contains(&DocumentLink::Transaction(transaction.id))
                }),
            })
        })
        .collect();
    found.sort_by(|a, b| {
        b.signals
            .count()
            .cmp(&a.signals.count())
            .then_with(|| b.signals.amount.cmp(&a.signals.amount))
            .then_with(|| a.gap_days.cmp(&b.gap_days))
            .then_with(|| a.already_linked.cmp(&b.already_linked))
            .then_with(|| a.transaction_id.cmp(&b.transaction_id))
    });
    found
}

pub fn suggestion(
    document: &Document,
    documents: &[Document],
    transactions: &[Transaction],
    payees: &[Payee],
) -> Suggestion {
    let readable = document
        .intake
        .as_ref()
        .is_some_and(|intake| intake.facts.total.is_some());
    if !readable {
        return Suggestion::Unreadable;
    }
    let mut found = candidates(document, documents, transactions, payees).into_iter();
    match found.next() {
        Some(best) => Suggestion::Link {
            best,
            others: found.take(OTHER_CANDIDATES).collect(),
        },
        None => Suggestion::Nothing,
    }
}

/// The Inbox's rows: newest `received_at` first, Skipped Documents after all others.
pub fn inbox_rows(documents: &[Document]) -> Vec<&Document> {
    let mut rows: Vec<&Document> = documents
        .iter()
        .filter(|document| document.is_unfiled())
        .collect();
    let received = |document: &Document| document.intake.as_ref().map(|intake| intake.received_at);
    rows.sort_by(|a, b| {
        a.is_skipped()
            .cmp(&b.is_skipped())
            .then_with(|| received(b).cmp(&received(a)))
            .then_with(|| b.id.cmp(&a.id))
    });
    rows
}

/// Which row takes focus once the row at `index` of `len` has been filed away: the next row, or
/// the previous one when it was the last. `None` when the Inbox is now empty.
pub fn focus_after_filing(index: usize, len: usize) -> Option<usize> {
    let remaining = len.saturating_sub(1);
    (remaining > 0).then(|| index.min(remaining - 1))
}

// ---------------------------------------------------------------------------------------------
// Inbox: filing actions
// ---------------------------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilingError {
    /// No Document has that id.
    NotFound,
    /// The Document is already Filed.
    AlreadyFiled,
    /// The Transaction to link does not exist.
    NoSuchTransaction,
}

/// What one filed Document looked like before, so [`undo`] can restore it.
#[derive(Debug, Clone, PartialEq)]
pub struct FiledEntry {
    pub id: u32,
    pub previous_type: DocumentType,
    pub previous_date: NaiveDate,
    pub added_links: Vec<DocumentLink>,
}

/// The one-level undo: the most recent filing action as a unit (a single Accept or File…, or a
/// whole Accept-all batch). A new filing action replaces it. Session-only.
#[derive(Debug, Clone, PartialEq)]
pub struct FilingUndo {
    pub entries: Vec<FiledEntry>,
    /// The Document focused when the action began, restored on undo.
    pub focus: Option<u32>,
}

fn file_one(
    documents: &mut [Document],
    id: u32,
    doc_type: DocumentType,
    date: NaiveDate,
    link: Option<DocumentLink>,
) -> Result<FiledEntry, FilingError> {
    let document = get_mut(documents, id).ok_or(FilingError::NotFound)?;
    if document.is_filed() {
        return Err(FilingError::AlreadyFiled);
    }
    let entry = FiledEntry {
        id,
        previous_type: document.doc_type,
        previous_date: document.date,
        added_links: link
            .filter(|link| !document.links.contains(link))
            .into_iter()
            .collect(),
    };
    document.links.extend(entry.added_links.iter().copied());
    document.doc_type = doc_type;
    document.date = date;
    document.filing = Filing::Filed;
    Ok(entry)
}

/// Accept: files the Document with a `Transaction` Link, the extracted Document Type (the Default
/// type when none was read) and the extracted date (the Transaction's when none was read).
pub fn accept(
    types: &[DocumentTypeRow],
    documents: &mut [Document],
    transactions: &[Transaction],
    id: u32,
    transaction_id: u32,
) -> Result<FiledEntry, FilingError> {
    let transaction = transactions
        .iter()
        .find(|transaction| transaction.id == transaction_id)
        .ok_or(FilingError::NoSuchTransaction)?;
    let facts = get(documents, id)
        .ok_or(FilingError::NotFound)?
        .intake
        .as_ref()
        .map(|intake| intake.facts.clone())
        .unwrap_or_default();
    file_one(
        documents,
        id,
        facts
            .doc_type
            .unwrap_or(DocumentType(types::fallback_id(types))),
        facts.date.unwrap_or(transaction.date),
        Some(DocumentLink::Transaction(transaction_id)),
    )
}

/// File… / Link elsewhere…: files the Document by hand with the chosen Document Type and at most
/// one Link. The date is the extracted date, else the received date.
pub fn file_by_hand(
    documents: &mut [Document],
    id: u32,
    doc_type: DocumentType,
    link: Option<DocumentLink>,
) -> Result<FiledEntry, FilingError> {
    let document = get(documents, id).ok_or(FilingError::NotFound)?;
    let date = document
        .intake
        .as_ref()
        .map(|intake| intake.facts.date.unwrap_or(intake.received_at))
        .unwrap_or(document.date);
    file_one(documents, id, doc_type, date, link)
}

/// Accept all strong matches: accepts every Unfiled Document whose Suggested Link is Strong, as
/// one undoable batch. Returns `None` when there were none.
pub fn accept_all_strong(
    types: &[DocumentTypeRow],
    documents: &mut [Document],
    transactions: &[Transaction],
    payees: &[Payee],
    focus: Option<u32>,
) -> Option<FilingUndo> {
    let strong: Vec<(u32, u32)> = inbox_rows(documents)
        .into_iter()
        .filter_map(|document| {
            let suggestion = suggestion(document, documents, transactions, payees);
            suggestion
                .is_strong()
                .then(|| {
                    suggestion
                        .best()
                        .map(|best| (document.id, best.transaction_id))
                })
                .flatten()
        })
        .collect();
    let entries: Vec<FiledEntry> = strong
        .into_iter()
        .filter_map(|(id, transaction_id)| {
            accept(types, documents, transactions, id, transaction_id).ok()
        })
        .collect();
    (!entries.is_empty()).then_some(FilingUndo { entries, focus })
}

/// How many Unfiled Documents have a Strong Suggested Link: the button's and confirm's count.
pub fn strong_count(
    documents: &[Document],
    transactions: &[Transaction],
    payees: &[Payee],
) -> usize {
    documents
        .iter()
        .filter(|document| document.is_unfiled())
        .filter(|document| suggestion(document, documents, transactions, payees).is_strong())
        .count()
}

/// Reverses a filing action: each Document returns to Unfiled with its earlier Document Type and
/// date, and the Links the action added are removed. Returns the focus to restore.
pub fn undo(documents: &mut [Document], record: FilingUndo) -> Option<u32> {
    for entry in record.entries {
        if let Some(document) = get_mut(documents, entry.id) {
            document
                .links
                .retain(|link| !entry.added_links.contains(link));
            document.doc_type = entry.previous_type;
            document.date = entry.previous_date;
            document.filing = Filing::Unfiled;
        }
    }
    record.focus
}

/// Skip: sets an Unfiled Document aside to the bottom of the Inbox for the session.
pub fn skip(documents: &mut [Document], id: u32) -> Result<(), FilingError> {
    let document = get_mut(documents, id).ok_or(FilingError::NotFound)?;
    match (&document.filing, &mut document.intake) {
        (Filing::Unfiled, Some(intake)) => {
            intake.skipped = true;
            Ok(())
        }
        (Filing::Filed, _) => Err(FilingError::AlreadyFiled),
        (Filing::Unfiled, None) => Err(FilingError::NotFound),
    }
}

/// Edit: overwrites the Extracted Facts (there is no separate corrected copy) and clears the skip.
/// Candidates recompute from the new facts on the next [`suggestion`].
pub fn edit_facts(
    documents: &mut [Document],
    id: u32,
    facts: ExtractedFacts,
) -> Result<(), FilingError> {
    let document = get_mut(documents, id).ok_or(FilingError::NotFound)?;
    if document.is_filed() {
        return Err(FilingError::AlreadyFiled);
    }
    let intake = document.intake.as_mut().ok_or(FilingError::NotFound)?;
    intake.facts = facts;
    intake.skipped = false;
    Ok(())
}

// ---------------------------------------------------------------------------------------------
// Library: adding, importing and editing
// ---------------------------------------------------------------------------------------------

/// Why a path cannot become a Document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AddError {
    /// Only PDF, JPG and PNG files are Documents.
    UnsupportedFile,
    /// One Document per path: the Library already holds this one, under this Title.
    AlreadyInLibrary(String),
    /// No Document has that id.
    NotFound,
    /// A blank Title.
    NoTitle,
}

impl FileKind {
    /// The kind a path's extension names, or `None` for anything that is not a Document.
    pub fn from_path(path: &Path) -> Option<Self> {
        match path.extension()?.to_str()?.to_ascii_lowercase().as_str() {
            "pdf" => Some(FileKind::Pdf),
            "jpg" | "jpeg" => Some(FileKind::Jpg),
            "png" => Some(FileKind::Png),
            _ => None,
        }
    }
}

/// The Title a new Document starts with: its file name without the extension.
pub fn default_title(path: &Path) -> String {
    path.file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// A path as the user typed it, made absolute: `~` is the home directory and a relative path is
/// taken from the working directory. Not canonicalised, so it never touches the file system.
pub fn resolve_path(typed: &str) -> PathBuf {
    let typed = typed.trim();
    if let Some(rest) = typed.strip_prefix("~/")
        && let Some(home) = dirs::home_dir()
    {
        return home.join(rest);
    }
    if typed == "~"
        && let Some(home) = dirs::home_dir()
    {
        return home;
    }
    let path = PathBuf::from(typed);
    if path.is_absolute() {
        path
    } else {
        std::env::current_dir().map_or(path.clone(), |dir| dir.join(path))
    }
}

/// The Document already holding `path`, whether Filed or Unfiled.
pub fn find_by_path<'a>(documents: &'a [Document], path: &Path) -> Option<&'a Document> {
    documents.iter().find(|document| document.path == path)
}

fn next_id(documents: &[Document]) -> u32 {
    documents
        .iter()
        .map(|document| document.id)
        .max()
        .unwrap_or(0)
        + 1
}

/// What the `+ Add` dialog collects.
#[derive(Debug, Clone, PartialEq)]
pub struct NewDocument {
    pub path: PathBuf,
    pub title: String,
    pub doc_type: DocumentType,
    pub date: NaiveDate,
    pub key_date: Option<KeyDate>,
}

/// `+ Add`: a Filed Document from a path the user already knows the meaning of. Its Links come
/// afterwards, through the picker. The size is read from disk when the file is there.
pub fn add_filed(documents: &mut Vec<Document>, new: NewDocument) -> Result<u32, AddError> {
    let kind = FileKind::from_path(&new.path).ok_or(AddError::UnsupportedFile)?;
    if let Some(existing) = find_by_path(documents, &new.path) {
        return Err(AddError::AlreadyInLibrary(existing.title.clone()));
    }
    let title = new.title.trim();
    if title.is_empty() {
        return Err(AddError::NoTitle);
    }
    let id = next_id(documents);
    documents.push(Document {
        id,
        bytes: std::fs::metadata(&new.path).map_or(0, |meta| meta.len()),
        path: new.path,
        kind,
        pages: 1,
        title: title.to_string(),
        doc_type: new.doc_type,
        date: new.date,
        key_date: new.key_date,
        links: Vec::new(),
        extracted_text: None,
        filing: Filing::Filed,
        intake: None,
    });
    Ok(id)
}

/// The extracted type, matched by name: absent when no type has it (it was renamed or removed).
fn type_named(types: &[DocumentTypeRow], name: &str) -> Option<DocumentType> {
    types::id_by_name(types, name).map(DocumentType)
}

/// The stub catalogue: files whose "extracted" facts the Inbox can show, in place of reading the
/// file. Matched by file name, so an `Import…` of one of these arrives readable.
pub fn catalogue_facts(
    types: &[DocumentTypeRow],
    path: &Path,
    today: NaiveDate,
) -> Option<ExtractedFacts> {
    let name = path.file_name()?.to_string_lossy().to_lowercase();
    match name.as_str() {
        "coles-receipt.jpg" => Some(ExtractedFacts {
            merchant: Some("Coles".to_string()),
            date: Some(today - Duration::days(2)),
            total: Some(cents_money(8_635)),
            doc_type: type_named(types, "Receipts"),
        }),
        "origin-energy-bill.pdf" => Some(ExtractedFacts {
            merchant: Some("Origin Energy".to_string()),
            date: Some(today - Duration::days(6)),
            total: Some(cents_money(24_110)),
            doc_type: type_named(types, "Bills"),
        }),
        _ => None,
    }
}

/// `Import…` and a file dropped on the window: one Unfiled Document, from `source`. It is readable
/// only when the path is in the stub catalogue; otherwise it carries no facts and is filed by hand.
pub fn import_path(
    types: &[DocumentTypeRow],
    documents: &mut Vec<Document>,
    path: PathBuf,
    today: NaiveDate,
    source: Source,
) -> Result<u32, AddError> {
    let kind = FileKind::from_path(&path).ok_or(AddError::UnsupportedFile)?;
    if let Some(existing) = find_by_path(documents, &path) {
        return Err(AddError::AlreadyInLibrary(existing.title.clone()));
    }
    let facts = catalogue_facts(types, &path, today).unwrap_or_default();
    let id = next_id(documents);
    documents.push(Document {
        id,
        bytes: std::fs::metadata(&path).map_or(0, |meta| meta.len()),
        title: default_title(&path),
        path,
        kind,
        pages: 1,
        doc_type: facts
            .doc_type
            .unwrap_or(DocumentType(types::fallback_id(types))),
        date: today,
        key_date: None,
        links: Vec::new(),
        extracted_text: None,
        filing: Filing::Unfiled,
        intake: Some(Intake {
            source,
            received_at: today,
            facts,
            skipped: false,
        }),
    });
    Ok(id)
}

/// The Library's `e`: replaces a Filed Document's Title, Document Type, document date and Key Date.
/// The Financial Year follows the date, since it is never stored.
pub fn edit_metadata(
    documents: &mut [Document],
    id: u32,
    title: &str,
    doc_type: DocumentType,
    date: NaiveDate,
    key_date: Option<KeyDate>,
) -> Result<(), AddError> {
    let title = title.trim();
    if title.is_empty() {
        return Err(AddError::NoTitle);
    }
    let document = get_mut(documents, id).ok_or(AddError::NotFound)?;
    document.title = title.to_string();
    document.doc_type = doc_type;
    document.date = date;
    document.key_date = key_date;
    Ok(())
}

/// The Financial Year scope's pre-fill for a new Document: today when it falls inside the Year,
/// otherwise the Year's last day.
pub fn date_for_scope(scope: LibraryScope, today: NaiveDate) -> NaiveDate {
    let LibraryScope::Year(facet) = scope else {
        return today;
    };
    let start = match facet {
        YearFacet::Year(start) => start,
        YearFacet::Before(start) => start - 1,
    };
    let last_day = |start: i32| {
        // The Year ends the day before the next one starts.
        NaiveDate::from_ymd_opt(start + 1, FINANCIAL_YEAR_START_MONTH, 1)
            .and_then(|next_start| next_start.pred_opt())
    };
    match facet {
        YearFacet::Year(_) if financial_year_start(today) == start => today,
        _ => last_day(start).unwrap_or(today),
    }
}

// ---------------------------------------------------------------------------------------------
// Seed
// ---------------------------------------------------------------------------------------------

/// The stub Documents. Their Inventory Links name Items the Inventory seeds.
#[derive(Debug, Clone, PartialEq)]
pub struct DocumentsSeed {
    pub documents: Vec<Document>,
}

/// Drops every Link to the given Items, as removing a Property does. The Documents themselves
/// stay, whatever Links they have left; returns how many lost a Link.
pub fn drop_inventory_links(documents: &mut [Document], removed_items: &[u32]) -> usize {
    let mut affected = 0;
    for document in documents {
        let before = document.links.len();
        document.links.retain(
            |link| !matches!(link, DocumentLink::InventoryItem(id) if removed_items.contains(id)),
        );
        affected += usize::from(document.links.len() != before);
    }
    affected
}

fn cents_money(cents: i64) -> Money {
    Money(BigDecimal::new(cents.into(), 2))
}

/// Inserts a Transaction newest first, as the stub keeps them, and returns its new id.
fn insert_transaction(transactions: &mut Vec<Transaction>, mut transaction: Transaction) -> u32 {
    let id = transactions.iter().map(|t| t.id).max().unwrap_or(0) + 1;
    transaction.id = id;
    let at = transactions
        .iter()
        .position(|t| t.date <= transaction.date)
        .unwrap_or(transactions.len());
    transactions.insert(at, transaction);
    id
}

/// Name lookups into the shared stubs, so the seed reads in names rather than ids.
struct Lookup<'a> {
    accounts: &'a [Account],
    categories: &'a [Category],
    payees: &'a [Payee],
    plans: &'a [BillPlan],
    inventory: &'a Inventory,
}

impl Lookup<'_> {
    fn account(&self, name: &str) -> Option<u32> {
        self.accounts.iter().find(|a| a.name == name).map(|a| a.id)
    }

    fn payee(&self, name: &str) -> Option<u32> {
        payees::find_by_name(self.payees, name)
    }

    fn plan(&self, name: &str) -> Option<u32> {
        self.plans.iter().find(|p| p.name == name).map(|p| p.id)
    }

    fn item(&self, name: &str) -> Option<u32> {
        self.inventory
            .items
            .iter()
            .find(|i| i.name == name)
            .map(|i| i.id)
    }

    /// A one-Split Transaction, or `None` when the stubs lack the Account or Category.
    fn transaction(
        &self,
        account: &str,
        date: NaiveDate,
        cents: i64,
        category: &str,
        payee: Option<&str>,
        memo: &str,
    ) -> Option<Transaction> {
        Some(Transaction {
            id: 0,
            date,
            account_id: self.account(account)?,
            status: TransactionStatus::Cleared,
            is_flagged: false,
            description: Some(memo.to_string()),
            splits: vec![Split {
                amount: cents_money(cents),
                category_id: categories::find_by_name(self.categories, category)?,
                payee_id: payee.and_then(|name| self.payee(name)),
                tag_ids: Vec::new(),
            }],
        })
    }
}

/// A Document under construction, filled in by the seed's builders.
struct Draft {
    file: String,
    kind: FileKind,
    pages: u32,
    bytes: u64,
    title: String,
    doc_type: DocumentType,
    date: NaiveDate,
    key_date: Option<KeyDate>,
    links: Vec<Option<DocumentLink>>,
    text: Option<&'static str>,
    intake: Option<Intake>,
}

impl Draft {
    fn filed(file: &str, title: &str, doc_type: DocumentType, date: NaiveDate) -> Self {
        let kind = if file.ends_with(".jpg") {
            FileKind::Jpg
        } else if file.ends_with(".png") {
            FileKind::Png
        } else {
            FileKind::Pdf
        };
        Draft {
            file: file.to_string(),
            kind,
            pages: 1,
            bytes: 180_000,
            title: title.to_string(),
            doc_type,
            date,
            key_date: None,
            links: Vec::new(),
            text: None,
            intake: None,
        }
    }

    fn unfiled(file: &str, source: Source, received_at: NaiveDate, facts: ExtractedFacts) -> Self {
        let title = file.rsplit_once('.').map_or(file, |(stem, _)| stem);
        let mut draft = Draft::filed(file, title, DocumentType::OTHER, received_at);
        draft.intake = Some(Intake {
            source,
            received_at,
            facts,
            skipped: false,
        });
        draft
    }

    fn size(mut self, pages: u32, bytes: u64) -> Self {
        self.pages = pages;
        self.bytes = bytes;
        self
    }

    fn key(mut self, kind: KeyDateKind, date: NaiveDate, reminder: bool) -> Self {
        self.key_date = Some(KeyDate {
            kind,
            date,
            reminder,
        });
        self
    }

    fn link(mut self, link: Option<DocumentLink>) -> Self {
        self.links.push(link);
        self
    }

    fn text(mut self, text: &'static str) -> Self {
        self.text = Some(text);
        self
    }

    fn build(self, id: u32) -> Document {
        let folder = if self.intake.is_some() {
            "inbox".to_string()
        } else {
            self.date.year().to_string()
        };
        Document {
            id,
            path: PathBuf::from("documents").join(folder).join(self.file),
            kind: self.kind,
            pages: self.pages,
            bytes: self.bytes,
            title: self.title,
            doc_type: self.doc_type,
            date: self.date,
            key_date: self.key_date,
            // A record the stubs lack is dropped rather than leaving a dangling Link.
            links: self.links.into_iter().flatten().collect(),
            extracted_text: self.text.map(str::to_string),
            filing: if self.intake.is_some() {
                Filing::Unfiled
            } else {
                Filing::Filed
            },
            intake: self.intake,
        }
    }
}

/// Seeds the handoff's sample Library and Inbox rows against the shared stubs, plus filler
/// statements and receipts so the facets have weight.
///
/// The handoff's own records that the stubs lack are swapped for ones they have (sample data is
/// not final copy): Amex Platinum for "ANZ Platinum", Westpac's Home Loan for Macquarie's, Vanguard
/// VAS for CMC Markets and Brisbane City Council for "City Council" (plus a lease, so no Type facet is empty), while the NRMA policy's
/// Payee and Bill links move to an AAMI car policy whose Payee and Bill Plan exist. The sample
/// purchases (Camera House, Woolworths, Bunnings, Sonos) are written to `transactions` as
/// `default_bills` writes its payments, so run this after it. Key Dates sit so three Library rows
/// Need Review on any `today`.
pub fn default_documents(
    accounts: &[Account],
    categories: &[Category],
    payees: &[Payee],
    plans: &[BillPlan],
    inventory: &Inventory,
    transactions: &mut Vec<Transaction>,
    today: NaiveDate,
) -> DocumentsSeed {
    let lookup = Lookup {
        accounts,
        categories,
        payees,
        plans,
        inventory,
    };
    let day = |offset: i64| today + Duration::days(offset);
    let mut add = |transaction: Option<Transaction>| {
        transaction.map(|transaction| insert_transaction(transactions, transaction))
    };

    let camera = add(lookup.transaction(
        "Amex Platinum",
        day(-212),
        -438_000,
        "Household",
        None,
        "CAMERA HOUSE — Sony A7 IV",
    ));
    // The Inbox's Strong pair and its Likely Sonos order (no Sonos Payee, so the payee Signal
    // fails while amount and date agree).
    add(lookup.transaction(
        "Amex Platinum",
        day(-3),
        -21_240,
        "Groceries",
        Some("Woolworths"),
        "WOOLWORTHS METRO",
    ));
    add(lookup.transaction(
        "ANZ Everyday",
        day(-6),
        -61_235,
        "Household",
        Some("Bunnings Warehouse"),
        "BUNNINGS INV88213",
    ));
    add(lookup.transaction(
        "Amex Platinum",
        day(-9),
        -49_900,
        "Household",
        None,
        "SONOS ONLINE",
    ));

    let transaction = |id: Option<u32>| id.map(DocumentLink::Transaction);
    let account = |name: &str| lookup.account(name).map(DocumentLink::Account);
    let payee = |name: &str| lookup.payee(name).map(DocumentLink::Payee);
    let plan = |name: &str| lookup.plan(name).map(DocumentLink::BillPlan);
    let item = |name: &str| lookup.item(name).map(DocumentLink::InventoryItem);
    let first_of_month = day(0).with_day(1).unwrap_or(today);
    let statement_month = (first_of_month - Duration::days(1)).format("%b %Y");
    let fy = financial_year_start(today) % 100;
    let money = |cents: i64| Some(cents_money(cents));
    use KeyDateKind::*;

    let mut drafts = vec![
        // The Library's sample rows, in the handoff's order.
        Draft::filed(
            "amex-platinum-statement.pdf",
            &format!("Amex Platinum statement — {statement_month}"),
            DocumentType::STATEMENTS,
            first_of_month,
        )
        .size(6, 420_000)
        .link(account("Amex Platinum"))
        .text("American Express Platinum Card statement of account closing balance"),
        Draft::filed(
            "receipt-camera-house.pdf",
            "Receipt — Camera House",
            DocumentType::RECEIPTS,
            day(-212),
        )
        .link(item("Sony A7 IV"))
        .link(transaction(camera))
        .text("Camera House tax invoice Sony A7 IV body 4,380.00 paid by card"),
        Draft::filed(
            "nrma-home-contents-pds.pdf",
            "NRMA home contents — PDS & schedule",
            DocumentType::INSURANCE,
            day(-320),
        )
        .size(24, 1_400_000)
        .key(Renews, day(45), true)
        .link(item("Fridge"))
        .link(item("Sony A7 IV"))
        .text("NRMA Insurance home contents product disclosure statement and schedule"),
        Draft::filed(
            "aami-car-policy.pdf",
            "AAMI car insurance — policy schedule",
            DocumentType::INSURANCE,
            day(-345),
        )
        .size(8, 610_000)
        .key(Renews, day(20), true)
        .link(payee("AAMI"))
        .link(plan("Car Insurance — AAMI"))
        .link(item("Toyota Corolla"))
        .text("AAMI comprehensive car insurance certificate of insurance"),
        Draft::filed(
            "westpac-home-loan-annual.pdf",
            "Westpac home loan — annual statement",
            DocumentType::CONTRACTS,
            day(-95),
        )
        .size(4, 380_000)
        .key(Ends, day(-30), false)
        .link(account("Home Loan"))
        .text("Westpac fixed rate period ends; interest charged for the year"),
        Draft::filed(
            "vanguard-tax-statement.pdf",
            &format!("Vanguard VAS — tax statement FY{fy:02}"),
            DocumentType::TAX,
            day(-80),
        )
        .size(3, 260_000)
        .link(account("Vanguard VAS"))
        .text("Vanguard annual tax statement franked distributions capital gains"),
        Draft::filed(
            "fridge-warranty-manual.pdf",
            "Fridge warranty & manual",
            DocumentType::WARRANTIES,
            day(-420),
        )
        .size(48, 3_200_000)
        .key(Ends, day(310), false)
        .link(item("Fridge"))
        .text("Warranty card and user manual, two year manufacturer's warranty"),
        Draft::filed(
            "engagement-ring-valuation.jpg",
            "Engagement ring valuation",
            DocumentType::INSURANCE,
            day(-560),
        )
        .size(1, 2_100_000)
        .key(Revalue, day(150), false)
        .link(item("Engagement ring"))
        .text("Valuation certificate for insurance purposes"),
        Draft::filed(
            "rates-notice-q1.pdf",
            "Rates notice — Q1",
            DocumentType::BILLS,
            day(-40),
        )
        .size(2, 210_000)
        .link(payee("Brisbane City Council"))
        .link(plan("Council Rates"))
        .text("Brisbane City Council rates notice quarter one"),
        Draft::filed(
            "lease-agreement-12-elm-st.pdf",
            "Lease agreement — 12 Elm St",
            DocumentType::CONTRACTS,
            day(-250),
        )
        .size(12, 900_000)
        .key(Ends, day(115), true)
        .link(payee("Ray White Rentals"))
        .text("Residential tenancy agreement fixed term lease Ray White"),
        Draft::filed(
            "passport-scan.jpg",
            "Passport — scan",
            DocumentType::IDENTITY,
            day(-700),
        )
        .size(1, 1_800_000)
        .key(Expires, day(2_040), false),
        // The Inbox's sample rows: two Strong, a Likely, a Likely and a Weak from the stubs' own
        // Transactions, one readable with nothing to suggest, and one Unreadable.
        Draft::unfiled(
            "IMG_4471.jpg",
            Source::Scanned,
            day(-3),
            ExtractedFacts {
                merchant: Some("Woolworths Metro".to_string()),
                date: Some(day(-3)),
                total: money(21_240),
                doc_type: Some(DocumentType::RECEIPTS),
            },
        )
        .size(1, 1_200_000)
        .text("WOOLWORTHS METRO tax invoice total 212.40"),
        Draft::unfiled(
            "bunnings-invoice-INV88213.pdf",
            Source::Downloads,
            day(-6),
            ExtractedFacts {
                merchant: Some("Bunnings Warehouse".to_string()),
                date: Some(day(-6)),
                total: money(61_235),
                doc_type: Some(DocumentType::RECEIPTS),
            },
        )
        .size(2, 240_000)
        .text("Bunnings Warehouse tax invoice INV88213 total 612.35"),
        Draft::unfiled(
            "Sonos_order_confirmation.pdf",
            Source::Emailed,
            day(-10),
            ExtractedFacts {
                merchant: Some("Sonos".to_string()),
                date: Some(day(-11)),
                total: money(49_900),
                doc_type: Some(DocumentType::RECEIPTS),
            },
        )
        .text("Sonos order confirmation Era 100 total 499.00"),
        Draft::unfiled(
            "origin-energy-bill.pdf",
            Source::Downloads,
            day(-14),
            ExtractedFacts {
                merchant: Some("Origin Energy".to_string()),
                date: Some(day(-16)),
                total: money(31_240),
                doc_type: Some(DocumentType::BILLS),
            },
        )
        .size(3, 300_000)
        .text("Origin Energy electricity bill amount due 312.40"),
        Draft::unfiled(
            "IMG_4460.jpg",
            Source::Scanned,
            day(-6),
            ExtractedFacts {
                merchant: Some("KMART".to_string()),
                // No date read, so the date Signal can never agree: payee alone, Weak.
                date: None,
                total: money(4_100),
                doc_type: None,
            },
        )
        .size(1, 980_000),
        Draft::unfiled(
            "ATO_NOA_2026.pdf",
            Source::Emailed,
            day(-18),
            ExtractedFacts {
                merchant: Some("Australian Taxation Office".to_string()),
                date: Some(day(-18)),
                total: money(128_400),
                doc_type: Some(DocumentType::TAX),
            },
        )
        .size(4, 150_000)
        .text("Australian Taxation Office notice of assessment refund 1,284.00"),
        Draft::unfiled(
            "scan0012.pdf",
            Source::Scanned,
            day(-1),
            ExtractedFacts::default(),
        )
        .size(1, 90_000),
    ];
    drafts.extend(filler(&lookup, transactions, today));

    let documents = drafts
        .into_iter()
        .zip(1..)
        .map(|(draft, id)| draft.build(id))
        .collect();
    DocumentsSeed { documents }
}

/// How many monthly statements each banking Account gets.
const FILLER_STATEMENT_MONTHS: u32 = 18;

/// Filler so the facets carry weight: a monthly statement for each banking Account, and a receipt
/// for every fourth purchase of 100.00 or more older than the Inbox's window.
fn filler(lookup: &Lookup<'_>, transactions: &[Transaction], today: NaiveDate) -> Vec<Draft> {
    let mut drafts = Vec::new();
    let mut month = today.with_day(1).unwrap_or(today);
    for _ in 0..FILLER_STATEMENT_MONTHS {
        // The latest Amex statement is the hand-authored sample row.
        let issued = month;
        month = (month - Duration::days(1)).with_day(1).unwrap_or(month);
        let label = month.format("%b %Y");
        for name in ["ANZ Everyday", "ANZ Offset", "Amex Platinum"] {
            if issued == today.with_day(1).unwrap_or(today) && name == "Amex Platinum" {
                continue;
            }
            let slug = name.to_lowercase().replace(' ', "-");
            drafts.push(
                Draft::filed(
                    &format!("{slug}-{}.pdf", month.format("%Y-%m")),
                    &format!("{name} statement — {label}"),
                    DocumentType::STATEMENTS,
                    // The month's last day, so the hand-authored sample (issued the 1st) leads.
                    issued - Duration::days(1),
                )
                .size(4, 350_000)
                .link(lookup.account(name).map(DocumentLink::Account)),
            );
        }
    }

    let cutoff = today - Duration::days(30);
    let hundred = cents_money(-10_000).0;
    let purchases = transactions
        .iter()
        .filter(|t| t.date < cutoff && t.total().0 <= hundred)
        .filter(|t| t.splits.iter().any(|split| split.payee_id.is_some()))
        .filter(|t| t.id % 4 == 0);
    for purchase in purchases {
        let Some(name) = purchase
            .splits
            .iter()
            .find_map(|split| split.payee_id)
            .and_then(|id| payees::get(lookup.payees, id))
            .map(|payee| payee.name.clone())
        else {
            continue;
        };
        let slug = name.to_lowercase().replace(' ', "-");
        drafts.push(
            Draft::filed(
                &format!("receipt-{slug}-{}.pdf", purchase.id),
                &format!("Receipt — {name}"),
                DocumentType::RECEIPTS,
                purchase.date,
            )
            .size(1, 120_000)
            .link(Some(DocumentLink::Transaction(purchase.id))),
        );
    }
    drafts
}
