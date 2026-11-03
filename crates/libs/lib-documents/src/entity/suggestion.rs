//! The Inbox's Suggested Link: Signals, candidates and the Payee match.

use std::collections::HashSet;

use lib_core::{DocumentId, PayeeId, TransactionId};

use super::document::{
    CANDIDATE_WINDOW_DAYS, DATE_SIGNAL_DAYS, Document, DocumentLink, OTHER_CANDIDATES,
};
use crate::protocols::{DocumentPayee, DocumentTransaction};

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
    pub transaction_id: TransactionId,
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
pub fn merchant_payee(payees: &[DocumentPayee], merchant: &str) -> Option<PayeeId> {
    match_alias(payees, merchant).or_else(|| {
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

/// The active Payee with an alias the merchant contains, the longest alias winning: the Payees'
/// own alias rule (`lib_payees::match_alias`), over the rows Documents reads.
fn match_alias(payees: &[DocumentPayee], raw: &str) -> Option<PayeeId> {
    let haystack = raw.to_uppercase();
    payees
        .iter()
        .filter(|payee| payee.is_active)
        .flat_map(|payee| {
            payee
                .aliases
                .iter()
                .filter(|alias| haystack.contains(alias.as_str()))
                .map(move |alias| (alias.len(), payee))
        })
        .max_by(|(a_len, a), (b_len, b)| a_len.cmp(b_len).then_with(|| b.name.cmp(&a.name)))
        .map(|(_, payee)| payee.id)
}

/// Every qualifying candidate for an Unfiled Document's facts, best first. Empty when Unreadable.
pub fn candidates(
    document: &Document,
    documents: &[Document],
    transactions: &[DocumentTransaction],
    payees: &[DocumentPayee],
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
                amount: transaction.amount.0.abs() == magnitude,
                date: facts.date.is_some_and(|date| {
                    (transaction.date - date).num_days().abs() <= DATE_SIGNAL_DAYS
                }),
                payee: payee_id.is_some_and(|id| transaction.names_payee(id)),
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
    transactions: &[DocumentTransaction],
    payees: &[DocumentPayee],
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

/// The Inbox's rows: newest `received_at` first, the Documents in `skipped` after all others.
pub fn inbox_rows<'a>(
    documents: &'a [Document],
    skipped: &HashSet<DocumentId>,
) -> Vec<&'a Document> {
    let mut rows: Vec<&Document> = documents
        .iter()
        .filter(|document| document.is_unfiled())
        .collect();
    let received = |document: &Document| document.intake.as_ref().map(|intake| intake.received_at);
    rows.sort_by(|a, b| {
        skipped
            .contains(&a.id)
            .cmp(&skipped.contains(&b.id))
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
