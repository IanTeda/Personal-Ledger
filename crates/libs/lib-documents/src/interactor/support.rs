//! What every Interactor logs and writes the same way.

use lib_core::{DocumentId, DocumentTypeId};

use crate::protocols::DocumentRepository;
use crate::types::{DocumentTypeRow, RemoveError, UnknownDocumentType};
use crate::{AddError, Document, FilingError, get};

/// A refusal as a log line carries it: the kind alone, never a Title or a type's name (ADR-0047).
pub(super) trait Refusal {
    fn reason(&self) -> &'static str;
}

impl Refusal for AddError {
    fn reason(&self) -> &'static str {
        match self {
            AddError::UnsupportedFile => "unsupported file",
            AddError::AlreadyInLibrary(_) => "already in the Library",
            AddError::NotFound => "no such Document",
            AddError::NoTitle => "blank Title",
        }
    }
}

impl Refusal for FilingError {
    fn reason(&self) -> &'static str {
        match self {
            FilingError::NotFound => "no such Document",
            FilingError::AlreadyFiled => "already Filed",
            FilingError::NoSuchTransaction => "no such Transaction",
        }
    }
}

impl Refusal for RemoveError {
    fn reason(&self) -> &'static str {
        match self {
            RemoveError::Unknown => "no such Document Type",
            RemoveError::IsDefault => "the Default Document Type",
            RemoveError::NeedsDestination => "has Filed Documents and no destination",
        }
    }
}

impl Refusal for UnknownDocumentType {
    fn reason(&self) -> &'static str {
        "no such Document Type"
    }
}

/// Logs how an intent on Document `id` ended and hands the outcome back.
pub(super) fn logged_document<R, E: Refusal>(
    intent: &str,
    id: DocumentId,
    outcome: Result<R, E>,
) -> Result<R, E> {
    match &outcome {
        Ok(_) => tracing::info!(intent, document_id = %id, "Documents intent done"),
        Err(refusal) => tracing::info!(
            intent,
            document_id = %id,
            refusal = refusal.reason(),
            "Documents intent refused"
        ),
    }
    outcome
}

/// Logs how an intent on Document Type `id` ended and hands the outcome back.
pub(super) fn logged_type<R, E: Refusal>(
    intent: &str,
    id: DocumentTypeId,
    outcome: Result<R, E>,
) -> Result<R, E> {
    match &outcome {
        Ok(_) => tracing::info!(intent, document_type_id = %id, "Documents intent done"),
        Err(refusal) => tracing::info!(
            intent,
            document_type_id = %id,
            refusal = refusal.reason(),
            "Documents intent refused"
        ),
    }
    outcome
}

/// Runs `rule` on a copy of the Documents and writes each Document it added or changed.
pub(super) fn write_documents<R, E>(
    repo: &mut dyn DocumentRepository,
    rule: impl FnOnce(&[DocumentTypeRow], &mut Vec<Document>) -> Result<R, E>,
) -> Result<R, E> {
    let types = repo.document_types();
    let before = repo.documents();
    let mut after = before.clone();
    let result = rule(&types, &mut after)?;
    for document in &after {
        if get(&before, document.id) != Some(document) {
            repo.upsert_document(document);
        }
    }
    Ok(result)
}
