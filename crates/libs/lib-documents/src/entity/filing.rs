//! The Inbox's filing actions: Accept, file by hand, Accept all strong, undo, skip and edit facts.

use lib_core::PayeeRef;
use std::collections::HashSet;

use chrono::NaiveDate;
use lib_core::{DocumentId, TransactionId};

use super::document::{Document, DocumentLink, DocumentType, ExtractedFacts, Filing, get, get_mut};
use super::suggestion::{inbox_rows, suggestion};
use crate::protocols::DocumentTransaction;
use crate::types;
use crate::types::DocumentTypeRow;
use crate::{Error, Result};

/// What one filing wrote to one Document: its Document Type, date and the Links the filing added,
/// as it was before. Filing is undone through the Shell's History (the Ledger's change journal),
/// so nothing here is read back to reverse it.
#[derive(Debug, Clone, PartialEq)]
pub struct FiledEntry {
    pub id: DocumentId,
    pub previous_type: DocumentType,
    pub previous_date: NaiveDate,
    pub added_links: Vec<DocumentLink>,
}

fn file_one(
    documents: &mut [Document],
    id: DocumentId,
    doc_type: DocumentType,
    date: NaiveDate,
    link: Option<DocumentLink>,
) -> Result<FiledEntry> {
    let document = get_mut(documents, id).ok_or(Error::NotFound)?;
    if document.is_filed() {
        return Err(Error::AlreadyFiled);
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
    transactions: &[DocumentTransaction],
    id: DocumentId,
    transaction_id: TransactionId,
) -> Result<FiledEntry> {
    let transaction = transactions
        .iter()
        .find(|transaction| transaction.id == transaction_id)
        .ok_or(Error::NoSuchTransaction)?;
    let facts = get(documents, id)
        .ok_or(Error::NotFound)?
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
    id: DocumentId,
    doc_type: DocumentType,
    link: Option<DocumentLink>,
) -> Result<FiledEntry> {
    let document = get(documents, id).ok_or(Error::NotFound)?;
    let date = document
        .intake
        .as_ref()
        .map(|intake| intake.facts.date.unwrap_or(intake.received_at))
        .unwrap_or(document.date);
    file_one(documents, id, doc_type, date, link)
}

/// Accept all strong matches: accepts every Unfiled Document whose Suggested Link is Strong, in
/// the Inbox's order (the `skipped` last). Returns what each one wrote; empty when none was Strong.
pub fn accept_all_strong(
    types: &[DocumentTypeRow],
    documents: &mut [Document],
    skipped: &HashSet<DocumentId>,
    transactions: &[DocumentTransaction],
    payees: &[PayeeRef],
) -> Vec<FiledEntry> {
    let strong: Vec<(DocumentId, TransactionId)> = inbox_rows(documents, skipped)
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
    strong
        .into_iter()
        .filter_map(|(id, transaction_id)| {
            accept(types, documents, transactions, id, transaction_id).ok()
        })
        .collect()
}

/// How many Unfiled Documents have a Strong Suggested Link: the button's and confirm's count.
pub fn strong_count(
    documents: &[Document],
    transactions: &[DocumentTransaction],
    payees: &[PayeeRef],
) -> usize {
    documents
        .iter()
        .filter(|document| document.is_unfiled())
        .filter(|document| suggestion(document, documents, transactions, payees).is_strong())
        .count()
}

/// Skip: sets an Unfiled Document aside to the bottom of the Inbox for the session, by adding it
/// to `skipped`, which the Client holds and never writes.
pub fn skip(
    documents: &[Document],
    skipped: &mut HashSet<DocumentId>,
    id: DocumentId,
) -> Result<()> {
    let document = get(documents, id).ok_or(Error::NotFound)?;
    match (&document.filing, &document.intake) {
        (Filing::Unfiled, Some(_)) => {
            skipped.insert(id);
            Ok(())
        }
        (Filing::Filed, _) => Err(Error::AlreadyFiled),
        (Filing::Unfiled, None) => Err(Error::NotFound),
    }
}

/// Edit: overwrites the Extracted Facts (there is no separate corrected copy). Editing also clears
/// the skip, which the Client does, since it holds the skipped set. Candidates recompute from the
/// new facts on the next [`suggestion`].
pub fn edit_facts(documents: &mut [Document], id: DocumentId, facts: ExtractedFacts) -> Result<()> {
    let document = get_mut(documents, id).ok_or(Error::NotFound)?;
    if document.is_filed() {
        return Err(Error::AlreadyFiled);
    }
    let intake = document.intake.as_mut().ok_or(Error::NotFound)?;
    intake.facts = facts;
    Ok(())
}
