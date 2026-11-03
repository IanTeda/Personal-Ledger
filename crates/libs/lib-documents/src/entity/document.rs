//! The Document, its Document Type and Key Date, and the Financial Year and Link rules on them.

use std::path::PathBuf;

use chrono::{Duration, NaiveDate};
use lib_core::{
    AccountId, BillPlanId, DocumentId, DocumentTypeId, ItemId, Money, PayeeId, TransactionId,
    financial_year_start,
};

use crate::types;
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
pub struct DocumentType(pub DocumentTypeId);

impl DocumentType {
    // The nine seeded types' ids, for the mock data and tests that name one.
    pub const RECEIPTS: DocumentType = DocumentType(DocumentTypeId::seed(1));
    pub const STATEMENTS: DocumentType = DocumentType(DocumentTypeId::seed(2));
    pub const TAX: DocumentType = DocumentType(DocumentTypeId::seed(3));
    pub const INSURANCE: DocumentType = DocumentType(DocumentTypeId::seed(4));
    pub const WARRANTIES: DocumentType = DocumentType(DocumentTypeId::seed(5));
    pub const CONTRACTS: DocumentType = DocumentType(DocumentTypeId::seed(6));
    pub const IDENTITY: DocumentType = DocumentType(DocumentTypeId::seed(7));
    pub const BILLS: DocumentType = DocumentType(DocumentTypeId::seed(8));
    pub const OTHER: DocumentType = DocumentType(DocumentTypeId::seed(9));

    /// The id persisted in a Library scope (`type:<id>`); never shown.
    pub fn id(self) -> String {
        self.0.to_string()
    }

    /// Parses a persisted id, only when `types` still has that type.
    pub fn from_id(id: &str, types: &[DocumentTypeRow]) -> Option<Self> {
        let id: DocumentTypeId = id.parse().ok()?;
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
    Transaction(TransactionId),
    InventoryItem(ItemId),
    Account(AccountId),
    Payee(PayeeId),
    BillPlan(BillPlanId),
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
    pub id: DocumentId,
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

pub fn get(documents: &[Document], id: DocumentId) -> Option<&Document> {
    documents.iter().find(|document| document.id == id)
}

pub(super) fn get_mut(documents: &mut [Document], id: DocumentId) -> Option<&mut Document> {
    documents.iter_mut().find(|document| document.id == id)
}
