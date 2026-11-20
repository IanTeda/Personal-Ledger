//! What every Interactor logs and writes the same way.

use lib_core::{DocumentId, DocumentTypeId};

use crate::protocols::DocumentRepository;
use crate::types::DocumentTypeRow;
use crate::{Document, Error, Result, get};

/// A refusal as a log line carries it: the kind alone, never a Title or a type's name (ADR-0047).
pub(super) fn reason(refusal: &Error) -> &'static str {
    match refusal {
        Error::UnsupportedFile => "unsupported file",
        Error::AlreadyInLibrary(_) => "already in the Library",
        Error::NotFound => "no such Document",
        Error::NoTitle => "blank Title",
        Error::AlreadyFiled => "already Filed",
        Error::NoSuchTransaction => "no such Transaction",
        Error::UnknownDocumentType => "no such Document Type",
        Error::DefaultDocumentType => "the Default Document Type",
        Error::NeedsDestination => "has Filed Documents and no destination",
        Error::Path(_) => "path refused",
        Error::Name(_) => "name refused",
    }
}

/// Logs how an intent on Document `id` ended and hands the outcome back.
pub(super) fn logged_document<R>(intent: &str, id: DocumentId, outcome: Result<R>) -> Result<R> {
    match &outcome {
        Ok(_) => tracing::info!(intent, document_id = %id, "Documents intent done"),
        Err(refusal) => tracing::info!(
            intent,
            document_id = %id,
            refusal = reason(refusal),
            "Documents intent refused"
        ),
    }
    outcome
}

/// Logs how an intent on Document Type `id` ended and hands the outcome back.
pub(super) fn logged_type<R>(intent: &str, id: DocumentTypeId, outcome: Result<R>) -> Result<R> {
    match &outcome {
        Ok(_) => tracing::info!(intent, document_type_id = %id, "Documents intent done"),
        Err(refusal) => tracing::info!(
            intent,
            document_type_id = %id,
            refusal = reason(refusal),
            "Documents intent refused"
        ),
    }
    outcome
}

/// Runs `rule` on a copy of the Documents and writes each Document it added or changed.
pub(super) fn write_documents<R, E>(
    repo: &mut dyn DocumentRepository,
    rule: impl FnOnce(&[DocumentTypeRow], &mut Vec<Document>) -> std::result::Result<R, E>,
) -> std::result::Result<R, E> {
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
