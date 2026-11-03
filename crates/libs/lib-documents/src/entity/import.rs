//! Adding, importing and editing a Document.

use std::path::{Path, PathBuf};

use chrono::{Duration, NaiveDate};
use lib_core::{DocumentId, FINANCIAL_YEAR_START_MONTH, financial_year_start};

use super::document::{
    Document, DocumentType, ExtractedFacts, FileKind, Filing, Intake, KeyDate, Source, get_mut,
};
use super::library::{LibraryScope, YearFacet};
use super::seed::cents_money;
use crate::protocols::DocumentFiles;
use crate::types;
use crate::types::DocumentTypeRow;

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

/// The Document already holding `path`, whether Filed or Unfiled.
pub fn find_by_path<'a>(documents: &'a [Document], path: &Path) -> Option<&'a Document> {
    documents.iter().find(|document| document.path == path)
}

/// Why a path typed for `+ Add` or `Import…` can't become a Document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PathRefusal {
    /// Only PDF, JPG and PNG files are Documents.
    Unsupported,
    /// One Document per path: the Library already holds this one, under this Title.
    Already(String),
    /// No file is there.
    NotFound,
}

/// What Add and Import refuse of a resolved path, in order: a file that is not a Document, then
/// a path the Library already holds, then a file that is not on disk.
fn path_refusal(documents: &[Document], path: &Path, exists: bool) -> Option<PathRefusal> {
    if FileKind::from_path(path).is_none() {
        Some(PathRefusal::Unsupported)
    } else if let Some(existing) = find_by_path(documents, path) {
        Some(PathRefusal::Already(existing.title.clone()))
    } else if !exists {
        Some(PathRefusal::NotFound)
    } else {
        None
    }
}

/// Resolves a typed path through `files` and checks it as Add and Import do, returning the path
/// when it can become a Document.
pub fn check_path(
    files: &dyn DocumentFiles,
    documents: &[Document],
    typed: &str,
) -> Result<PathBuf, PathRefusal> {
    let path = files.resolve(typed);
    let exists = files.exists(&path);
    match path_refusal(documents, &path, exists) {
        Some(refusal) => Err(refusal),
        None => Ok(path),
    }
}

/// What the `+ Add` dialog collects.
#[derive(Debug, Clone, PartialEq)]
pub struct NewDocument {
    pub path: PathBuf,
    /// The file's size, read from disk when the file is there, else 0.
    pub bytes: u64,
    pub title: String,
    pub doc_type: DocumentType,
    pub date: NaiveDate,
    pub key_date: Option<KeyDate>,
}

/// `+ Add`: a Filed Document from a path the user already knows the meaning of. Its Links come
/// afterwards, through the picker.
pub fn add_filed(documents: &mut Vec<Document>, new: NewDocument) -> Result<DocumentId, AddError> {
    let kind = FileKind::from_path(&new.path).ok_or(AddError::UnsupportedFile)?;
    if let Some(existing) = find_by_path(documents, &new.path) {
        return Err(AddError::AlreadyInLibrary(existing.title.clone()));
    }
    let title = new.title.trim();
    if title.is_empty() {
        return Err(AddError::NoTitle);
    }
    let id = DocumentId::new();
    documents.push(Document {
        id,
        bytes: new.bytes,
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

/// `Import…` and a file dropped on the window: one Unfiled Document of `bytes`, from `source`. It
/// is readable only when the path is in the stub catalogue; otherwise it carries no facts and is
/// filed by hand.
pub fn import_path(
    types: &[DocumentTypeRow],
    documents: &mut Vec<Document>,
    path: PathBuf,
    bytes: u64,
    today: NaiveDate,
    source: Source,
) -> Result<DocumentId, AddError> {
    let kind = FileKind::from_path(&path).ok_or(AddError::UnsupportedFile)?;
    if let Some(existing) = find_by_path(documents, &path) {
        return Err(AddError::AlreadyInLibrary(existing.title.clone()));
    }
    let facts = catalogue_facts(types, &path, today).unwrap_or_default();
    let id = DocumentId::new();
    documents.push(Document {
        id,
        bytes,
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
        }),
    });
    Ok(id)
}

/// A path typed or dropped for import, resolved, with what [`DocumentFiles`] found there: read
/// before the import so the batch itself is a value the Ledger runs, never touching the disk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportLine {
    pub typed: String,
    pub path: PathBuf,
    pub exists: bool,
    /// The file's size, 0 when it can't be read.
    pub bytes: u64,
}

/// Each typed or dropped path, resolved and looked up through `files`.
pub fn import_lines<'a>(
    files: &dyn DocumentFiles,
    typed: impl IntoIterator<Item = &'a str>,
) -> Vec<ImportLine> {
    typed
        .into_iter()
        .map(|typed| {
            let path = files.resolve(typed);
            ImportLine {
                typed: typed.to_string(),
                exists: files.exists(&path),
                bytes: files.size(&path).unwrap_or(0),
                path,
            }
        })
        .collect()
}

/// What an import made of one line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImportOutcome {
    Imported(DocumentId),
    NotFound(String),
    Unsupported(String),
    Already(String),
}

/// Imports every line that exists into the Inbox, reporting what became of each. A path repeated
/// in the same batch is refused as already present, since the first copy was added. A drop and
/// `Import…` share this, so they agree on what is refused and why.
pub fn import_batch(
    types: &[DocumentTypeRow],
    documents: &mut Vec<Document>,
    lines: &[ImportLine],
    today: NaiveDate,
    source: Source,
) -> Vec<ImportOutcome> {
    lines
        .iter()
        .map(|line| {
            match path_refusal(documents, &line.path, line.exists) {
                Some(PathRefusal::Unsupported) => {
                    return ImportOutcome::Unsupported(line.typed.clone());
                }
                Some(PathRefusal::Already(title)) => return ImportOutcome::Already(title),
                Some(PathRefusal::NotFound) => return ImportOutcome::NotFound(line.typed.clone()),
                None => {}
            }
            match import_path(
                types,
                documents,
                line.path.clone(),
                line.bytes,
                today,
                source,
            ) {
                Ok(id) => ImportOutcome::Imported(id),
                Err(AddError::AlreadyInLibrary(title)) => ImportOutcome::Already(title),
                Err(_) => ImportOutcome::Unsupported(line.typed.clone()),
            }
        })
        .collect()
}

/// The Library's `e`: replaces a Filed Document's Title, Document Type, document date and Key Date.
/// The Financial Year follows the date, since it is never stored.
pub fn edit_metadata(
    documents: &mut [Document],
    id: DocumentId,
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
