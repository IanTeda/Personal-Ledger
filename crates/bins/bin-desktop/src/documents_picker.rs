//! The Documents link picker's rules: what it lists, in what order, and what a query keeps. One
//! palette-chrome modal searches every record kind a Document can link to (Transaction, Account,
//! Payee, Bill Plan, Inventory Item), so this is `gpui`-free and unit-tested without a window, the
//! same split `palette.rs` and `documents.rs` use. `shell/documents_ui.rs` owns the keys and the
//! effects of a pick; `view/documents/dialogs.rs` draws it.

use std::ops::Range;

use chrono::NaiveDate;
use lib_locale::format::format_date;

use crate::{
    documents::{DocumentLink, DocumentType},
    format,
    transactions::Transaction,
    view::documents::model::Lookups,
};

/// Days either side of the Document's date in which a Transaction is offered before any search.
pub const NEAR_DAYS: i64 = 30;

/// Rows drawn at once; the list scrolls past this, the palette's own capped-window idiom.
pub const VISIBLE_ROWS: usize = 9;

/// The kind filter `tab` cycles: All, then each record kind in the picker's block order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum KindFilter {
    #[default]
    All,
    Transaction,
    Account,
    Payee,
    Bill,
    Inventory,
}

impl KindFilter {
    pub fn next(self) -> Self {
        match self {
            KindFilter::All => KindFilter::Transaction,
            KindFilter::Transaction => KindFilter::Account,
            KindFilter::Account => KindFilter::Payee,
            KindFilter::Payee => KindFilter::Bill,
            KindFilter::Bill => KindFilter::Inventory,
            KindFilter::Inventory => KindFilter::All,
        }
    }

    pub fn label(self) -> String {
        match self {
            KindFilter::All => crate::msg::desktop_documents_picker_kind_all(),
            KindFilter::Transaction => crate::msg::desktop_documents_link_transaction(),
            KindFilter::Account => crate::msg::desktop_documents_link_account(),
            KindFilter::Payee => crate::msg::desktop_documents_link_payee(),
            KindFilter::Bill => crate::msg::desktop_documents_link_bill(),
            KindFilter::Inventory => crate::msg::desktop_documents_link_inventory(),
        }
    }

    fn admits(self, link: DocumentLink) -> bool {
        matches!(
            (self, link),
            (KindFilter::All, _)
                | (KindFilter::Transaction, DocumentLink::Transaction(_))
                | (KindFilter::Account, DocumentLink::Account(_))
                | (KindFilter::Payee, DocumentLink::Payee(_))
                | (KindFilter::Bill, DocumentLink::BillPlan(_))
                | (KindFilter::Inventory, DocumentLink::InventoryItem(_))
        )
    }
}

/// Why the picker is open, and which Document it acts on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Purpose {
    /// Library `l` / **+ Link to…**: `enter` toggles a Link on a Filed Document.
    Link(u32),
    /// Inbox **File…** / **Link elsewhere…**: a pick files the Unfiled Document.
    File(u32),
    /// Library `L` on a Document with several Links: `enter` follows one.
    Follow(u32),
}

impl Purpose {
    pub fn document(self) -> u32 {
        match self {
            Purpose::Link(id) | Purpose::File(id) | Purpose::Follow(id) => id,
        }
    }
}

/// The picker's live state. Rows are never stored: they are derived from this and the stubs on
/// every render and keystroke, so they cannot drift out of sync with either.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PickerState {
    pub purpose: Purpose,
    pub query: String,
    pub kind: KindFilter,
    pub selected: usize,
    /// The Inbox's Document Type select, prefilled from the Extracted Facts. `None` until chosen
    /// when none was read, and a pick is refused until then.
    pub doc_type: Option<DocumentType>,
    /// The date Transactions are ranked against: the document date in the Library, the extracted
    /// date in the Inbox.
    pub anchor: NaiveDate,
}

impl PickerState {
    pub fn new(purpose: Purpose, anchor: NaiveDate, doc_type: Option<DocumentType>) -> Self {
        Self {
            purpose,
            query: String::new(),
            kind: KindFilter::All,
            selected: 0,
            doc_type,
            anchor,
        }
    }

    pub fn push_char(&mut self, ch: char) {
        self.query.push(ch);
        self.selected = 0;
    }

    pub fn backspace(&mut self) {
        self.query.pop();
        self.selected = 0;
    }

    pub fn cycle_kind(&mut self) {
        self.kind = self.kind.next();
        self.selected = 0;
    }

    /// Steps the highlight, clamped to the `len` rows rather than wrapping.
    pub fn step(&mut self, down: bool, len: usize) {
        self.selected = if down {
            (self.selected + 1).min(len.saturating_sub(1))
        } else {
            self.selected.saturating_sub(1)
        };
    }

    /// The Inbox's Document Type select: previous or next of `all` (Settings order), from the first
    /// when none is chosen yet.
    pub fn step_type(&mut self, all: &[DocumentType], forward: bool) {
        if all.is_empty() {
            return;
        }
        let at = self
            .doc_type
            .and_then(|current| all.iter().position(|kind| *kind == current));
        let next = match (at, forward) {
            (None, _) => 0,
            (Some(at), true) => (at + 1) % all.len(),
            (Some(at), false) => (at + all.len() - 1) % all.len(),
        };
        self.doc_type = Some(all[next]);
    }
}

/// One listed row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PickerRow {
    /// `None` is the Inbox's pinned **File without link** row.
    pub link: Option<DocumentLink>,
    pub kind: String,
    pub text: String,
    /// A current Link in the Library: pinned at the top and ticked, and `enter` removes it.
    pub checked: bool,
}

/// What the rows are built from besides the picker's own state.
pub struct Sources<'a> {
    pub lookups: &'a Lookups<'a>,
    /// The Document's current Links (the Library's pinned, ticked rows).
    pub current: &'a [DocumentLink],
}

/// The rows to show, in order: pinned rows (the current Links in the Library, **File without link**
/// in the Inbox), then Transactions within [`NEAR_DAYS`] of the anchor nearest first (a search also
/// reaches those further out, after them), then Accounts, Payees, Bill Plans and Inventory Items as
/// alphabetical blocks. No fuzzy scoring: a query is one case-insensitive substring.
pub fn rows(state: &PickerState, sources: &Sources<'_>) -> Vec<PickerRow> {
    let lookups = sources.lookups;
    if let Purpose::Follow(_) = state.purpose {
        return sources
            .current
            .iter()
            .filter_map(|link| {
                let (kind, text, _) = describe(*link, lookups, false)?;
                Some(PickerRow {
                    link: Some(*link),
                    kind,
                    text,
                    checked: false,
                })
            })
            .collect();
    }

    let needle = state.query.trim().to_lowercase();
    let mut out: Vec<PickerRow> = Vec::new();

    if matches!(state.purpose, Purpose::File(_)) {
        out.push(PickerRow {
            link: None,
            kind: String::new(),
            text: crate::msg::desktop_documents_picker_file_without_link(),
            checked: false,
        });
    }

    let push = |out: &mut Vec<PickerRow>, link: DocumentLink, checked: bool| {
        if !state.kind.admits(link) {
            return;
        }
        let Some((kind, text, haystack)) = describe(link, lookups, true) else {
            return;
        };
        if !matches_query(&haystack, &needle) {
            return;
        }
        out.push(PickerRow {
            link: Some(link),
            kind,
            text,
            checked,
        });
    };

    for link in sources.current {
        push(&mut out, *link, true);
    }
    let pinned = |link: &DocumentLink| sources.current.contains(link);

    // Transactions: the near window first, nearest first; a search then reaches the rest.
    let mut ranked: Vec<(&Transaction, i64)> = lookups
        .transactions
        .iter()
        .map(|transaction| {
            (
                transaction,
                (transaction.date - state.anchor).num_days().abs(),
            )
        })
        .filter(|(transaction, gap)| {
            (*gap <= NEAR_DAYS || !needle.is_empty())
                && !pinned(&DocumentLink::Transaction(transaction.id))
        })
        .collect();
    ranked.sort_by(|(a, a_gap), (b, b_gap)| {
        let a_far = *a_gap > NEAR_DAYS;
        let b_far = *b_gap > NEAR_DAYS;
        a_far
            .cmp(&b_far)
            .then_with(|| a_gap.cmp(b_gap))
            .then_with(|| a.id.cmp(&b.id))
    });
    for (transaction, _) in ranked {
        push(&mut out, DocumentLink::Transaction(transaction.id), false);
    }

    let mut accounts: Vec<_> = lookups.accounts.iter().collect();
    accounts.sort_by_key(|account| account.name.to_lowercase());
    for account in accounts {
        let link = DocumentLink::Account(account.id);
        if !pinned(&link) {
            push(&mut out, link, false);
        }
    }
    let mut payees: Vec<_> = lookups.payees.iter().collect();
    payees.sort_by_key(|payee| payee.name.to_lowercase());
    for payee in payees {
        let link = DocumentLink::Payee(payee.id);
        if !pinned(&link) {
            push(&mut out, link, false);
        }
    }
    let mut plans: Vec<_> = lookups.plans.iter().collect();
    plans.sort_by_key(|plan| plan.name.to_lowercase());
    for plan in plans {
        let link = DocumentLink::BillPlan(plan.id);
        if !pinned(&link) {
            push(&mut out, link, false);
        }
    }
    let mut items: Vec<_> = lookups.inventory.iter().collect();
    items.sort_by_key(|item| item.name.to_lowercase());
    for item in items {
        let link = DocumentLink::InventoryItem(item.id);
        if !pinned(&link) {
            push(&mut out, link, false);
        }
    }
    out
}

/// A record's kind label, its display text and (when `searchable`) the lowercase text a query is
/// matched against. `None` when the record has gone.
fn describe(
    link: DocumentLink,
    lookups: &Lookups<'_>,
    searchable: bool,
) -> Option<(String, String, String)> {
    match link {
        DocumentLink::Transaction(id) => {
            let transaction = lookups.transactions.iter().find(|t| t.id == id)?;
            let account = lookups
                .accounts
                .iter()
                .find(|account| account.id == transaction.account_id)
                .map(|account| account.name.clone())
                .unwrap_or_default();
            let payee = transaction
                .splits
                .iter()
                .find_map(|split| split.payee_id)
                .and_then(|id| crate::payees::get(lookups.payees, id));
            let what = payee
                .map(|payee| payee.name.clone())
                .or_else(|| transaction.description.clone())
                .unwrap_or_default();
            let (_, amount) = format::signed_amount(&transaction.total());
            let date = format_date(transaction.date, lookups.date_style);
            let text = format!("{date} \u{b7} {what} \u{b7} {amount} \u{b7} {account}");
            let haystack = if searchable {
                transaction_haystack(transaction, &account, lookups)
            } else {
                String::new()
            };
            Some((
                crate::msg::desktop_documents_link_transaction(),
                text,
                haystack,
            ))
        }
        DocumentLink::Account(id) => {
            let account = lookups.accounts.iter().find(|account| account.id == id)?;
            let text = format!("{} \u{b7} {}", account.name, account.institution);
            Some((
                crate::msg::desktop_documents_link_account(),
                text.clone(),
                text.to_lowercase(),
            ))
        }
        DocumentLink::Payee(id) => {
            let payee = crate::payees::get(lookups.payees, id)?;
            let haystack = std::iter::once(payee.name.as_str())
                .chain(payee.aliases.iter().map(String::as_str))
                .collect::<Vec<_>>()
                .join("\n")
                .to_lowercase();
            Some((
                crate::msg::desktop_documents_link_payee(),
                payee.name.clone(),
                haystack,
            ))
        }
        DocumentLink::BillPlan(id) => {
            let plan = crate::bills::get(lookups.plans, id)?;
            let cadence = crate::view::bills::planner::recurrence_label(plan.recurrence);
            let payee = plan
                .payee_id
                .and_then(|id| crate::payees::get(lookups.payees, id))
                .map(|payee| payee.name.as_str())
                .unwrap_or_default();
            let text = format!("{} \u{b7} {cadence}", plan.name);
            let haystack = format!("{}\n{payee}", plan.name).to_lowercase();
            Some((crate::msg::desktop_documents_link_bill(), text, haystack))
        }
        DocumentLink::InventoryItem(id) => {
            let item = lookups.inventory.iter().find(|item| item.id == id)?;
            Some((
                crate::msg::desktop_documents_link_inventory(),
                item.name.clone(),
                item.name.to_lowercase(),
            ))
        }
    }
}

/// The lowercase text a Transaction is searched by: its Payee names and aliases, memo, Account
/// name, the amount as plain digits (`4380.00`, so `4380`, `4,380` and `-4380.00` all find it) and
/// the date in both ISO and day-month form (`2026-09`, `14 sep`).
fn transaction_haystack(
    transaction: &Transaction,
    account_name: &str,
    lookups: &Lookups<'_>,
) -> String {
    let mut parts: Vec<String> = Vec::new();
    for split in &transaction.splits {
        if let Some(payee) = split
            .payee_id
            .and_then(|id| crate::payees::get(lookups.payees, id))
        {
            parts.push(payee.name.clone());
            parts.extend(payee.aliases.iter().cloned());
        }
    }
    if let Some(memo) = &transaction.description {
        parts.push(memo.clone());
    }
    parts.push(account_name.to_string());
    parts.push(amount_digits(transaction));
    parts.push(transaction.date.format("%Y-%m-%d").to_string());
    parts.push(transaction.date.format("%-d %b %Y").to_string());
    parts.join("\n").to_lowercase()
}

/// `|total|` to two places, unsigned and ungrouped.
fn amount_digits(transaction: &Transaction) -> String {
    transaction.total().0.abs().with_scale(2).to_string()
}

/// A query matches when its text is a substring of the haystack, or, for a money-looking query
/// (`-4,380`, `$4380.00`), when its bare digits are.
fn matches_query(haystack: &str, needle: &str) -> bool {
    if needle.is_empty() || haystack.contains(needle) {
        return true;
    }
    let bare: String = needle
        .chars()
        .filter(|ch| !matches!(ch, '-' | ',' | '$' | ' '))
        .collect();
    !bare.is_empty()
        && bare.chars().all(|ch| ch.is_ascii_digit() || ch == '.')
        && haystack.contains(&bare)
}

/// The slice of `len` rows to draw so `selected` stays on screen in a `cap`-row window, scrolling
/// only as far as it must.
pub fn visible_window(selected: usize, len: usize, cap: usize) -> Range<usize> {
    let start = (selected + 1)
        .saturating_sub(cap)
        .min(len.saturating_sub(cap));
    start..(start + cap).min(len)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        accounts::{Account, default_accounts},
        bills::{BillPlan, default_bills},
        categories::default_categories,
        documents::{DocumentsSeed, default_documents},
        payees::{Payee, default_payees},
        tags::default_tags,
        transactions::default_transactions,
    };
    use lib_core::DateStyle;

    struct World {
        types: Vec<crate::document_types::DocumentTypeRow>,
        accounts: Vec<Account>,
        payees: Vec<Payee>,
        plans: Vec<BillPlan>,
        transactions: Vec<Transaction>,
        seed: DocumentsSeed,
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
        let seed = default_documents(
            &accounts,
            &categories,
            &payees,
            &bills.plans,
            &mut transactions,
            today,
        );
        World {
            types: crate::document_types::default_types(),
            accounts,
            payees,
            plans: bills.plans,
            transactions,
            seed,
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
                inventory: &self.seed.inventory,
                transactions: &self.transactions,
                today: self.today,
                date_style: Some(DateStyle::Iso),
            }
        }
    }

    fn state(world: &World, purpose: Purpose) -> PickerState {
        PickerState::new(purpose, world.today, None)
    }

    #[test]
    fn the_kind_filter_cycles_through_every_kind_and_back() {
        let mut kind = KindFilter::All;
        let mut seen = Vec::new();
        for _ in 0..6 {
            kind = kind.next();
            seen.push(kind);
        }
        assert_eq!(
            seen,
            [
                KindFilter::Transaction,
                KindFilter::Account,
                KindFilter::Payee,
                KindFilter::Bill,
                KindFilter::Inventory,
                KindFilter::All
            ]
        );
    }

    #[test]
    fn current_links_are_pinned_first_and_ticked() {
        let world = world();
        let lookups = world.lookups();
        let account = world.accounts[1].id;
        let current = [DocumentLink::Account(account)];
        let listed = rows(
            &state(&world, Purpose::Link(1)),
            &Sources {
                lookups: &lookups,
                current: &current,
            },
        );
        assert_eq!(listed[0].link, Some(DocumentLink::Account(account)));
        assert!(listed[0].checked);
        let again = listed
            .iter()
            .filter(|row| row.link == Some(DocumentLink::Account(account)))
            .count();
        assert_eq!(again, 1, "a current link is never listed twice");
        assert!(listed[1..].iter().all(|row| !row.checked));
    }

    #[test]
    fn transactions_come_nearest_first_then_the_other_kinds_in_blocks() {
        let world = world();
        let lookups = world.lookups();
        let state = state(&world, Purpose::Link(1));
        let listed = rows(
            &state,
            &Sources {
                lookups: &lookups,
                current: &[],
            },
        );
        let transaction_gaps: Vec<i64> = listed
            .iter()
            .filter_map(|row| match row.link {
                Some(DocumentLink::Transaction(id)) => world
                    .transactions
                    .iter()
                    .find(|t| t.id == id)
                    .map(|t| (t.date - state.anchor).num_days().abs()),
                _ => None,
            })
            .collect();
        assert!(!transaction_gaps.is_empty());
        assert!(transaction_gaps.iter().all(|gap| *gap <= NEAR_DAYS));
        assert!(transaction_gaps.windows(2).all(|pair| pair[0] <= pair[1]));

        let rank = |link: &Option<DocumentLink>| match link {
            Some(DocumentLink::Transaction(_)) => 0,
            Some(DocumentLink::Account(_)) => 1,
            Some(DocumentLink::Payee(_)) => 2,
            Some(DocumentLink::BillPlan(_)) => 3,
            Some(DocumentLink::InventoryItem(_)) => 4,
            None => 9,
        };
        let ranks: Vec<i32> = listed.iter().map(|row| rank(&row.link)).collect();
        assert!(ranks.windows(2).all(|pair| pair[0] <= pair[1]));
        assert_eq!(*ranks.last().expect("rows"), 4, "inventory closes the list");
    }

    #[test]
    fn a_payee_block_is_alphabetical() {
        let world = world();
        let lookups = world.lookups();
        let mut state = state(&world, Purpose::Link(1));
        state.kind = KindFilter::Payee;
        let listed = rows(
            &state,
            &Sources {
                lookups: &lookups,
                current: &[],
            },
        );
        let names: Vec<String> = listed.iter().map(|row| row.text.to_lowercase()).collect();
        let mut sorted = names.clone();
        sorted.sort();
        assert_eq!(names, sorted);
        assert_eq!(listed.len(), world.payees.len());
    }

    #[test]
    fn a_money_query_matches_however_it_is_typed() {
        let world = world();
        let lookups = world.lookups();
        let transaction = world
            .transactions
            .iter()
            .find(|t| t.total().0.abs() >= 1000)
            .expect("a large stub transaction");
        let plain = amount_digits(transaction);
        let whole = plain.trim_end_matches(".00").to_string();
        let grouped = {
            let (head, tail) = whole.split_at(whole.len() - 3);
            format!("{head},{tail}")
        };
        for query in [whole.clone(), grouped, format!("-{plain}")] {
            let mut state = state(&world, Purpose::Link(1));
            state.query = query.clone();
            state.kind = KindFilter::Transaction;
            let listed = rows(
                &state,
                &Sources {
                    lookups: &lookups,
                    current: &[],
                },
            );
            assert!(
                listed
                    .iter()
                    .any(|row| row.link == Some(DocumentLink::Transaction(transaction.id))),
                "query {query:?} should find the transaction"
            );
        }
    }

    #[test]
    fn a_search_reaches_transactions_past_the_near_window_after_the_near_ones() {
        let world = world();
        let lookups = world.lookups();
        let mut browse = state(&world, Purpose::Link(1));
        browse.kind = KindFilter::Transaction;
        let near = rows(
            &browse,
            &Sources {
                lookups: &lookups,
                current: &[],
            },
        )
        .len();
        let mut search = browse.clone();
        search.query = "20".into();
        let found = rows(
            &search,
            &Sources {
                lookups: &lookups,
                current: &[],
            },
        );
        assert!(found.len() > near, "the year-prefix reaches older rows");
    }

    #[test]
    fn the_inbox_pins_file_without_link_and_a_pick_there_is_not_a_toggle() {
        let world = world();
        let lookups = world.lookups();
        let listed = rows(
            &state(&world, Purpose::File(1)),
            &Sources {
                lookups: &lookups,
                current: &[],
            },
        );
        assert_eq!(listed[0].link, None);
        assert!(listed[1..].iter().all(|row| row.link.is_some()));
    }

    #[test]
    fn follow_lists_exactly_the_documents_links() {
        let world = world();
        let lookups = world.lookups();
        let current = [
            DocumentLink::Account(world.accounts[0].id),
            DocumentLink::Payee(world.payees[0].id),
        ];
        let listed = rows(
            &state(&world, Purpose::Follow(1)),
            &Sources {
                lookups: &lookups,
                current: &current,
            },
        );
        let links: Vec<_> = listed.iter().filter_map(|row| row.link).collect();
        assert_eq!(links, current);
    }

    #[test]
    fn the_window_scrolls_only_as_far_as_the_highlight_needs() {
        assert_eq!(visible_window(0, 20, 9), 0..9);
        assert_eq!(visible_window(8, 20, 9), 0..9);
        assert_eq!(visible_window(9, 20, 9), 1..10);
        assert_eq!(visible_window(19, 20, 9), 11..20);
        assert_eq!(visible_window(1, 3, 9), 0..3);
        assert_eq!(visible_window(0, 0, 9), 0..0);
    }

    #[test]
    fn the_type_select_starts_at_the_first_and_wraps() {
        let world = world();
        let mut state = state(&world, Purpose::File(1));
        let all: Vec<_> = crate::document_types::default_types()
            .iter()
            .map(|row| DocumentType(row.id))
            .collect();
        state.step_type(&all, true);
        assert_eq!(state.doc_type, Some(DocumentType::RECEIPTS));
        state.step_type(&all, false);
        assert_eq!(state.doc_type, Some(DocumentType::OTHER));
        state.step_type(&all, true);
        assert_eq!(state.doc_type, Some(DocumentType::RECEIPTS));
    }
}
