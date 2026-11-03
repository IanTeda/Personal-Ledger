//! The Library's index rail, scopes, sort and rows.

use chrono::NaiveDate;
use lib_core::{DocumentId, financial_year_start};

use super::document::{
    Document, DocumentLink, DocumentType, KeyDateBand, band, get_mut, tracks_financial_year,
};
use crate::types::DocumentTypeRow;

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
pub fn toggle_link(documents: &mut [Document], id: DocumentId, link: DocumentLink) -> Option<bool> {
    let document = get_mut(documents, id)?;
    if let Some(at) = document.links.iter().position(|existing| *existing == link) {
        document.links.remove(at);
        Some(false)
    } else {
        document.links.push(link);
        Some(true)
    }
}
