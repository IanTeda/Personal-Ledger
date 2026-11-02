//! The Inbox's view model: each Unfiled Document's row with its Suggested Link and match meter, and
//! the detail pane's Extracted Facts, Suggested Link card and other candidates. The Suggested Link
//! is derived here on every build, never stored.

use chrono::NaiveDate;
use lib_core::{DocumentId, TransactionId};
use lib_locale::format::format_date;

use super::library::{Fact, Lookups, extension, link_name, type_label, unsigned_amount};
use crate::types::DocumentTypeRow;
use crate::{
    CANDIDATE_WINDOW_DAYS, Candidate, Document, DocumentLink, FileKind, Signals, Source,
    Suggestion, msg, suggestion,
};

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

/// An Inbox row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InboxRowView {
    pub id: DocumentId,
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
    /// The row's own button: Accept, or File… when there is nothing to accept.
    pub action: String,
    /// Set aside with Skip for this session, so drawn dimmed.
    pub skipped: bool,
}

/// One Transaction offered in the detail pane: the suggested one, or another candidate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CandidateView {
    pub transaction_id: TransactionId,
    /// "29 Sep · Woolworths · −212.40".
    pub line: String,
    /// "ANZ Platinum · no document yet"; empty for an Other candidate.
    pub sub: String,
    pub signals: Signals,
    pub signals_text: String,
}

/// The Inbox's detail pane on the focused row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InboxDetailView {
    pub id: DocumentId,
    pub extension: &'static str,
    pub title: String,
    pub meta: String,
    pub facts: Vec<Fact>,
    pub state: InboxState,
    pub suggested: Option<CandidateView>,
    pub others: Vec<CandidateView>,
    /// "Other candidates: none within 7 days", shown only when there are no others.
    pub others_none: String,
    /// The footer's two buttons: Accept & next and Link elsewhere…, or File… and Skip when there
    /// is nothing to accept.
    pub primary: String,
    pub secondary: String,
}

fn source_text(source: Source, date: NaiveDate, lookups: &Lookups<'_>) -> String {
    let date = format_date(date, lookups.date_style);
    match source {
        Source::Scanned => msg::documents_source_scanned(&date),
        Source::Emailed => msg::documents_source_emailed(&date),
        Source::Downloads => msg::documents_source_downloads(&date),
        Source::WatchedFolder => msg::documents_source_watched(&date),
        Source::Dropped => msg::documents_source_dropped(&date),
        Source::Import => msg::documents_source_import(&date),
    }
}

/// The sentence under the match meter: which Signals agree.
pub fn signals_text(signals: Signals) -> String {
    match (signals.amount, signals.date, signals.payee) {
        (true, true, true) => msg::documents_signals_all(),
        (true, true, false) => msg::documents_signals_amount_date(),
        (true, false, true) => msg::documents_signals_amount_payee(),
        (false, true, true) => msg::documents_signals_date_payee(),
        (true, false, false) => msg::documents_signals_amount(),
        (false, false, true) => msg::documents_signals_payee(),
        (false, true, false) => msg::documents_signals_date(),
        (false, false, false) => String::new(),
    }
}

fn candidate_view(
    candidate: &Candidate,
    with_sub: bool,
    lookups: &Lookups<'_>,
) -> Option<CandidateView> {
    let line = link_name(DocumentLink::Transaction(candidate.transaction_id), lookups)?;
    let sub = if with_sub {
        let records = lookups.records;
        let account = records
            .transaction(candidate.transaction_id)
            .and_then(|transaction| records.account(transaction.account_id))
            .map(|account| account.name.clone())
            .unwrap_or_default();
        let documents = if candidate.already_linked {
            msg::documents_suggested_has_document()
        } else {
            msg::documents_suggested_no_document()
        };
        msg::documents_suggested_detail(&account, &documents)
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
        .map_or_else(msg::documents_fact_none, |kind| type_label(types, kind));
    let amount = facts
        .and_then(|facts| facts.total.as_ref())
        .map(unsigned_amount)
        .unwrap_or_default();
    match facts.and_then(|facts| facts.merchant.as_deref()) {
        Some(merchant) => msg::documents_suggest_summary(&kind, merchant, &amount),
        None => msg::documents_suggest_summary_bare(&kind, &amount),
    }
}

fn suggestion_for(
    document: &Document,
    documents: &[Document],
    lookups: &Lookups<'_>,
) -> Suggestion {
    suggestion(
        document,
        documents,
        &lookups.records.transactions,
        &lookups.records.payees,
    )
}

/// An Inbox row, dimmed when `skipped`.
pub fn row(
    document: &Document,
    documents: &[Document],
    lookups: &Lookups<'_>,
    skipped: bool,
) -> InboxRowView {
    let (state, summary, hint, signals) = match &suggestion_for(document, documents, lookups) {
        Suggestion::Unreadable => (
            InboxState::Unreadable,
            msg::documents_suggest_unreadable(),
            msg::documents_suggest_unreadable_hint(),
            None,
        ),
        Suggestion::Nothing => (
            InboxState::NoSuggestion,
            msg::documents_suggest_none(CANDIDATE_WINDOW_DAYS),
            msg::documents_suggest_none_hint(),
            None,
        ),
        Suggestion::Link { best, .. } => (
            InboxState::Link,
            summary_text(lookups.types, document),
            link_name(DocumentLink::Transaction(best.transaction_id), lookups).unwrap_or_default(),
            Some(best.signals),
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
        action: if state == InboxState::Link {
            msg::documents_accept()
        } else {
            msg::documents_file_button()
        },
        skipped,
    }
}

/// The Inbox's focused-row detail: what was read, the Suggested Link and the next candidates.
pub fn detail(
    document: &Document,
    documents: &[Document],
    lookups: &Lookups<'_>,
) -> InboxDetailView {
    let facts = document
        .intake
        .as_ref()
        .map(|intake| intake.facts.clone())
        .unwrap_or_default();
    let none = msg::documents_fact_none();
    let fact = |label: String, value: Option<String>| Fact {
        label,
        value: value.unwrap_or_else(|| none.clone()),
        red: false,
    };
    let (state, suggested, others) = match &suggestion_for(document, documents, lookups) {
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
        (InboxState::Unreadable, _) => msg::documents_inbox_meta_unreadable(),
        (_, FileKind::Pdf) => msg::documents_inbox_meta_file(),
        (_, _) => msg::documents_inbox_meta_image(),
    };
    let readable = state == InboxState::Link;
    InboxDetailView {
        id: document.id,
        extension: extension(document.kind),
        title: document.file_name(),
        meta,
        facts: vec![
            fact(msg::documents_fact_merchant(), facts.merchant.clone()),
            fact(
                msg::documents_detail_date(),
                facts.date.map(|date| format_date(date, lookups.date_style)),
            ),
            fact(
                msg::documents_fact_total(),
                facts.total.as_ref().map(unsigned_amount),
            ),
            fact(
                msg::documents_detail_type(),
                facts.doc_type.map(|kind| type_label(lookups.types, kind)),
            ),
        ],
        state,
        suggested,
        others,
        others_none: msg::documents_other_none(CANDIDATE_WINDOW_DAYS),
        primary: if readable {
            msg::documents_accept_next()
        } else {
            msg::documents_file_button()
        },
        secondary: if readable {
            msg::documents_link_elsewhere()
        } else {
            msg::documents_skip()
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::presenter::test_support::world;

    #[test]
    fn a_strong_row_summarises_what_was_read_and_names_its_transaction() {
        let world = world();
        let lookups = world.lookups();
        let view = row(
            world.file("IMG_4471.jpg"),
            &world.documents,
            &lookups,
            false,
        );
        assert_eq!(view.state, InboxState::Link);
        assert!(view.summary.starts_with("Receipts · "), "{}", view.summary);
        assert!(view.summary.contains("212.40"), "{}", view.summary);
        assert_eq!(view.signals.map(Signals::count), Some(3));
        assert!(view.hint.contains("212.40"), "{}", view.hint);
        assert_eq!(view.action, "Accept");
        assert!(!view.skipped);
    }

    #[test]
    fn an_unreadable_row_never_guesses_and_a_row_with_nothing_asks_to_be_filed_by_hand() {
        let world = world();
        let lookups = world.lookups();
        let unreadable = row(
            world.file("scan0012.pdf"),
            &world.documents,
            &lookups,
            false,
        );
        assert_eq!(unreadable.state, InboxState::Unreadable);
        assert_eq!(unreadable.summary, "Unreadable — no amount found");
        assert_eq!(unreadable.signals, None);
        assert_eq!(unreadable.action, "File…");

        let nothing = row(
            world.file("ATO_NOA_2026.pdf"),
            &world.documents,
            &lookups,
            false,
        );
        assert_eq!(nothing.state, InboxState::NoSuggestion);
        assert!(
            nothing.summary.contains("nothing within 7 days"),
            "{}",
            nothing.summary
        );
    }

    #[test]
    fn the_meter_text_names_exactly_the_signals_that_agree() {
        let _world = world();
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
    fn the_detail_shows_the_facts_the_suggested_link_and_its_buttons() {
        let world = world();
        let lookups = world.lookups();
        let view = detail(world.file("IMG_4471.jpg"), &world.documents, &lookups);
        assert_eq!(view.title, "IMG_4471.jpg");
        assert_eq!(view.meta, "Read from the image · check before accepting");
        let labels: Vec<_> = view.facts.iter().map(|fact| fact.label.as_str()).collect();
        assert_eq!(labels, ["Merchant", "Document date", "Total", "Type"]);
        let card = view.suggested.expect("a readable file has a suggestion");
        assert!(card.line.contains("212.40"), "{}", card.line);
        assert!(card.sub.ends_with("no document yet"), "{}", card.sub);
        assert_eq!(card.signals_text, "amount, date and payee match");
        assert!(view.others_none.contains("7 days"));
        assert_eq!(
            (view.primary.as_str(), view.secondary.as_str()),
            ("Accept & next", "Link elsewhere…")
        );
    }

    #[test]
    fn an_unreadable_detail_has_blank_facts_no_card_and_file_or_skip() {
        let world = world();
        let lookups = world.lookups();
        let view = detail(world.file("scan0012.pdf"), &world.documents, &lookups);
        assert_eq!(view.state, InboxState::Unreadable);
        assert_eq!(view.suggested, None);
        assert_eq!(view.meta, "Nothing could be read from this file");
        let total = view
            .facts
            .iter()
            .find(|fact| fact.label == "Total")
            .expect("a Total fact");
        assert_eq!(total.value, "—");
        assert_eq!(
            (view.primary.as_str(), view.secondary.as_str()),
            ("File…", "Skip")
        );
    }
}
