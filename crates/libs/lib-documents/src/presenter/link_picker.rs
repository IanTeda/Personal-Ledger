//! The link picker: one palette-framed modal searching every record kind a Document can link to
//! (Transaction, Account, Payee, Bill Plan, Inventory Item), opened for one of three reasons.
//!
//! - **Link** (the Library's `l`): a pick toggles a Link on the Filed Document, its current Links
//!   pinned at the top and ticked.
//! - **File** (the Inbox's File… and Link elsewhere…): a pick files the Unfiled Document by hand
//!   under the chosen Document Type and at most one Link, **File without link** pinned first. A
//!   pick waits until a Document Type is chosen.
//! - **Follow** (the Library's `L` on a Document with several Links): the Document's Links alone,
//!   and a pick follows one.
//!
//! Rows are never stored: they are derived from the query, the kind filter and the records on every
//! read, so they can't drift from either. The records, the Document's Links and the Document Types
//! are read when the picker opens, as it is modal and nothing changes them under it.

use chrono::NaiveDate;
use lib_core::DocumentId;
use lib_locale::format::format_date;
use lib_toast::{Notice, ToastKind};
use lib_viper::keymap::{BACK, Keymap, NEXT_FIELD, SELECT};
use lib_viper::view::Hint;

use super::DialogPorts;
use super::document_form::document;
use super::library::{Lookups, signed_amount, type_label};
use crate::interactor::ToggleLink;
use crate::protocols::{DocumentTransaction, DocumentsOutcome, DocumentsRouter, LinkRecords};
use crate::types::DocumentTypeRow;
use crate::{DocumentLink, DocumentType, msg};

/// Days either side of the Document's date in which a Transaction is offered before any search.
pub const NEAR_DAYS: i64 = 30;

/// Why the picker is open, and which Document it acts on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Purpose {
    /// A pick toggles a Link on Filed Document `id`.
    Link(DocumentId),
    /// A pick files Unfiled Document `id`.
    File(DocumentId),
    /// A pick follows one of Filed Document `id`'s Links.
    Follow(DocumentId),
}

impl Purpose {
    pub fn document(self) -> DocumentId {
        match self {
            Self::Link(id) | Self::File(id) | Self::Follow(id) => id,
        }
    }
}

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
    pub const ALL: [Self; 6] = [
        Self::All,
        Self::Transaction,
        Self::Account,
        Self::Payee,
        Self::Bill,
        Self::Inventory,
    ];

    pub fn next(self) -> Self {
        let at = Self::ALL.iter().position(|kind| *kind == self).unwrap_or(0);
        Self::ALL[(at + 1) % Self::ALL.len()]
    }

    pub fn label(self) -> String {
        match self {
            Self::All => msg::documents_dialog_picker_kind_all(),
            Self::Transaction => msg::documents_link_transaction(),
            Self::Account => msg::documents_link_account(),
            Self::Payee => msg::documents_link_payee(),
            Self::Bill => msg::documents_link_bill(),
            Self::Inventory => msg::documents_link_inventory(),
        }
    }

    fn admits(self, link: DocumentLink) -> bool {
        matches!(
            (self, link),
            (Self::All, _)
                | (Self::Transaction, DocumentLink::Transaction(_))
                | (Self::Account, DocumentLink::Account(_))
                | (Self::Payee, DocumentLink::Payee(_))
                | (Self::Bill, DocumentLink::BillPlan(_))
                | (Self::Inventory, DocumentLink::InventoryItem(_))
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LinkPickerEvent {
    /// The query's whole new text, as the View's input holds it. The highlight goes back to the
    /// top.
    Query(String),
    /// `tab`: the next kind filter.
    CycleKind,
    /// The highlight down or up a row, stopping at either end.
    Down,
    Up,
    /// `←`/`→` while filing: the previous or next Document Type, wrapping.
    StepType {
        forward: bool,
    },
    /// `enter`: acts on the highlighted row.
    Pick,
    /// A click on row `index`.
    PickRow(usize),
    Cancel,
}

/// A kind filter tab: its label and whether it is the one applied.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KindTab {
    pub label: String,
    pub active: bool,
}

/// The Document Type line shown while filing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PickerTypeLine {
    pub label: String,
    /// The chosen type's name, `None` until one is chosen.
    pub chosen: Option<String>,
    /// What shows until then.
    pub none: String,
}

/// One listed row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PickerRowView {
    /// `None` is filing's pinned **File without link** row.
    pub link: Option<DocumentLink>,
    pub kind: String,
    pub text: String,
    /// A current Link in the Library: pinned at the top and ticked, and a pick removes it.
    pub checked: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkPickerViewModel {
    /// Why it is open: Link, File or Follow.
    pub title: String,
    pub query: String,
    pub placeholder: String,
    /// The kind filter's tabs; following has none.
    pub kinds: Option<Vec<KindTab>>,
    /// The applied kind filter's label.
    pub kind: String,
    pub type_line: Option<PickerTypeLine>,
    pub rows: Vec<PickerRowView>,
    /// The highlighted row, within `rows`.
    pub selected: usize,
    /// What shows when no row matches.
    pub empty: String,
    /// The keys, under the list.
    pub footer: String,
}

pub struct LinkPickerPresenter {
    ports: DialogPorts,
    purpose: Purpose,
    records: LinkRecords,
    types: Vec<DocumentTypeRow>,
    /// The Document's Links: the Library's pinned, ticked rows, and what Follow lists.
    current: Vec<DocumentLink>,
    /// The date Transactions are ranked against: the document date in the Library, the extracted
    /// date in the Inbox.
    anchor: NaiveDate,
    query: String,
    kind: KindFilter,
    selected: usize,
    /// The Document Type a File pick files under, prefilled from the Extracted Facts.
    doc_type: Option<DocumentType>,
}

impl LinkPickerPresenter {
    /// The picker on `purpose`'s Document, or `None` when it has gone.
    pub fn new(ports: DialogPorts, purpose: Purpose) -> Option<Self> {
        let document = document(&ports, purpose.document())?;
        let (anchor, doc_type) = match (purpose, &document.intake) {
            (Purpose::File(_), Some(intake)) => (
                intake.facts.date.unwrap_or(intake.received_at),
                intake.facts.doc_type,
            ),
            _ => (document.date, None),
        };
        let records = LinkRecords::read(&*ports.reads);
        let types = ports.documents.borrow().document_types();
        Some(Self {
            ports,
            purpose,
            records,
            types,
            current: document.links,
            anchor,
            query: String::new(),
            kind: KindFilter::All,
            selected: 0,
            doc_type,
        })
    }

    pub fn purpose(&self) -> Purpose {
        self.purpose
    }

    pub fn handle(&mut self, event: LinkPickerEvent, router: &mut dyn DocumentsRouter) {
        match event {
            LinkPickerEvent::Query(text) => {
                self.query = text;
                self.selected = 0;
            }
            LinkPickerEvent::CycleKind => {
                self.kind = self.kind.next();
                self.selected = 0;
            }
            LinkPickerEvent::Down => {
                let last = self.rows().len().saturating_sub(1);
                self.selected = (self.selected + 1).min(last);
            }
            LinkPickerEvent::Up => self.selected = self.selected.saturating_sub(1),
            LinkPickerEvent::StepType { forward } => self.step_type(forward),
            LinkPickerEvent::Pick => self.pick(self.selected, router),
            LinkPickerEvent::PickRow(index) => self.pick(index, router),
            LinkPickerEvent::Cancel => router.close_dialog(None),
        }
    }

    pub fn view_model(&self) -> LinkPickerViewModel {
        let rows = self.rows();
        let (title, footer) = match self.purpose {
            Purpose::Link(_) => (
                msg::documents_dialog_picker_title_link(),
                msg::documents_dialog_picker_hint_toggle(),
            ),
            Purpose::File(_) => (
                msg::documents_dialog_picker_title_file(),
                msg::documents_dialog_picker_hint_pick(),
            ),
            Purpose::Follow(_) => (
                msg::documents_dialog_picker_title_follow(),
                msg::documents_dialog_picker_hint_follow(),
            ),
        };
        LinkPickerViewModel {
            title,
            query: self.query.clone(),
            placeholder: msg::documents_dialog_picker_placeholder(),
            kinds: (!matches!(self.purpose, Purpose::Follow(_))).then(|| {
                KindFilter::ALL
                    .iter()
                    .map(|kind| KindTab {
                        label: kind.label(),
                        active: *kind == self.kind,
                    })
                    .collect()
            }),
            kind: self.kind.label(),
            type_line: matches!(self.purpose, Purpose::File(_)).then(|| PickerTypeLine {
                label: msg::documents_dialog_picker_type(),
                chosen: self.doc_type.map(|kind| type_label(&self.types, kind)),
                none: msg::documents_dialog_picker_type_none(),
            }),
            selected: self.selected.min(rows.len().saturating_sub(1)),
            rows,
            empty: msg::documents_dialog_picker_empty(),
            footer,
        }
    }

    /// The status line's legend: cancel, the kind filter and pick.
    pub fn legend<E: Clone>(&self, keymap: &Keymap<E>) -> Vec<Hint> {
        let key = |command| keymap.chord(command).unwrap_or_default().to_string();
        vec![
            Hint::new(key(BACK), msg::documents_dialog_hint_cancel()),
            Hint::new(key(NEXT_FIELD), msg::documents_dialog_hint_kind()),
            Hint::new(key(SELECT), msg::documents_dialog_hint_confirm()),
        ]
    }

    fn lookups(&self) -> Lookups<'_> {
        Lookups {
            types: &self.types,
            records: &self.records,
            today: self.ports.today,
            date_style: self.ports.date_style,
        }
    }

    /// The Document Types in the user's order, from the first when none is chosen yet.
    fn step_type(&mut self, forward: bool) {
        if !matches!(self.purpose, Purpose::File(_)) || self.types.is_empty() {
            return;
        }
        let all: Vec<DocumentType> = self.types.iter().map(|row| DocumentType(row.id)).collect();
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

    fn pick(&mut self, index: usize, router: &mut dyn DocumentsRouter) {
        let Some(row) = self.rows().into_iter().nth(index) else {
            return;
        };
        match self.purpose {
            Purpose::Link(id) => {
                let Some(link) = row.link else {
                    return;
                };
                let toggled = ToggleLink { id, link }.run(&mut *self.ports.documents.borrow_mut());
                if let Ok(added) = toggled {
                    router.notify(Notice::Toast(
                        ToastKind::Info,
                        if added {
                            msg::documents_dialog_toast_linked(&row.text)
                        } else {
                            msg::documents_toast_unlinked(&row.text)
                        },
                    ));
                }
                router.close_dialog(None);
            }
            Purpose::File(id) => {
                let Some(doc_type) = self.doc_type else {
                    router.notify(Notice::Status(msg::documents_dialog_picker_need_type()));
                    return;
                };
                router.close_dialog(Some(DocumentsOutcome::FileByHand {
                    id,
                    doc_type,
                    link: row.link,
                }));
            }
            Purpose::Follow(_) => {
                router.close_dialog(row.link.map(DocumentsOutcome::Follow));
            }
        }
    }

    /// The rows to show, in order: pinned rows (the current Links in the Library, **File without
    /// link** while filing), then Transactions within [`NEAR_DAYS`] of the anchor nearest first (a
    /// search also reaches those further out, after them), then Accounts, Payees, Bill Plans and
    /// Inventory Items as alphabetical blocks. No fuzzy scoring: a query is one case-insensitive
    /// substring.
    fn rows(&self) -> Vec<PickerRowView> {
        let lookups = self.lookups();
        if let Purpose::Follow(_) = self.purpose {
            return self
                .current
                .iter()
                .filter_map(|link| {
                    let (kind, text, _) = describe(*link, &lookups, false)?;
                    Some(PickerRowView {
                        link: Some(*link),
                        kind,
                        text,
                        checked: false,
                    })
                })
                .collect();
        }

        let needle = self.query.trim().to_lowercase();
        let mut out: Vec<PickerRowView> = Vec::new();
        if matches!(self.purpose, Purpose::File(_)) {
            out.push(PickerRowView {
                link: None,
                kind: String::new(),
                text: msg::documents_dialog_picker_file_without_link(),
                checked: false,
            });
        }

        let push = |out: &mut Vec<PickerRowView>, link: DocumentLink, checked: bool| {
            if !self.kind.admits(link) {
                return;
            }
            let Some((kind, text, haystack)) = describe(link, &lookups, true) else {
                return;
            };
            if matches_query(&haystack, &needle) {
                out.push(PickerRowView {
                    link: Some(link),
                    kind,
                    text,
                    checked,
                });
            }
        };

        for link in &self.current {
            push(&mut out, *link, true);
        }
        let pinned = |link: &DocumentLink| self.current.contains(link);

        // Transactions: the near window first, nearest first; a search then reaches the rest.
        let records = &self.records;
        let mut ranked: Vec<(&DocumentTransaction, i64)> = records
            .transactions
            .iter()
            .map(|transaction| {
                (
                    transaction,
                    (transaction.date - self.anchor).num_days().abs(),
                )
            })
            .filter(|(transaction, gap)| {
                (*gap <= NEAR_DAYS || !needle.is_empty())
                    && !pinned(&DocumentLink::Transaction(transaction.id))
            })
            .collect();
        ranked.sort_by(|(a, a_gap), (b, b_gap)| {
            (*a_gap > NEAR_DAYS)
                .cmp(&(*b_gap > NEAR_DAYS))
                .then_with(|| a_gap.cmp(b_gap))
                .then_with(|| a.id.cmp(&b.id))
        });
        for (transaction, _) in ranked {
            push(&mut out, DocumentLink::Transaction(transaction.id), false);
        }

        let mut blocks: Vec<(String, DocumentLink)> = Vec::new();
        let mut block = |mut named: Vec<(String, DocumentLink)>| {
            named.sort_by_key(|(name, _)| name.to_lowercase());
            blocks.extend(named);
        };
        block(
            records
                .accounts
                .iter()
                .map(|row| (row.name.clone(), DocumentLink::Account(row.id)))
                .collect(),
        );
        block(
            records
                .payees
                .iter()
                .map(|row| (row.name.clone(), DocumentLink::Payee(row.id)))
                .collect(),
        );
        block(
            records
                .plans
                .iter()
                .map(|row| (row.name.clone(), DocumentLink::BillPlan(row.id)))
                .collect(),
        );
        block(
            records
                .items
                .iter()
                .map(|row| (row.name.clone(), DocumentLink::InventoryItem(row.id)))
                .collect(),
        );
        for (_, link) in blocks {
            if !pinned(&link) {
                push(&mut out, link, false);
            }
        }
        out
    }
}

/// A record's kind label, its text and (when `searchable`) the lower-case text a query is matched
/// against. `None` when the record has gone.
fn describe(
    link: DocumentLink,
    lookups: &Lookups<'_>,
    searchable: bool,
) -> Option<(String, String, String)> {
    let records = lookups.records;
    match link {
        DocumentLink::Transaction(id) => {
            let transaction = records.transaction(id)?;
            let account = records
                .account(transaction.account_id)
                .map(|account| account.name.clone())
                .unwrap_or_default();
            let what = transaction
                .first_payee()
                .and_then(|id| records.payee(id))
                .map(|payee| payee.name.clone())
                .or_else(|| transaction.memo.clone())
                .unwrap_or_default();
            let amount = signed_amount(&transaction.amount);
            let date = format_date(transaction.date, lookups.date_style);
            let text = format!("{date} \u{b7} {what} \u{b7} {amount} \u{b7} {account}");
            let haystack = if searchable {
                transaction_haystack(transaction, &account, records)
            } else {
                String::new()
            };
            Some((msg::documents_link_transaction(), text, haystack))
        }
        DocumentLink::Account(id) => {
            let account = records.account(id)?;
            let text = format!("{} \u{b7} {}", account.name, account.institution);
            Some((
                msg::documents_link_account(),
                text.clone(),
                text.to_lowercase(),
            ))
        }
        DocumentLink::Payee(id) => {
            let payee = records.payee(id)?;
            let haystack = std::iter::once(payee.name.as_str())
                .chain(payee.aliases.iter().map(String::as_str))
                .collect::<Vec<_>>()
                .join("\n")
                .to_lowercase();
            Some((msg::documents_link_payee(), payee.name.clone(), haystack))
        }
        DocumentLink::BillPlan(id) => {
            let plan = records.plan(id)?;
            let payee = plan
                .payee_id
                .and_then(|id| records.payee(id))
                .map(|payee| payee.name.as_str())
                .unwrap_or_default();
            let text = format!("{} \u{b7} {}", plan.name, plan.cadence);
            let haystack = format!("{}\n{payee}", plan.name).to_lowercase();
            Some((msg::documents_link_bill(), text, haystack))
        }
        DocumentLink::InventoryItem(id) => {
            let item = records.item(id)?;
            Some((
                msg::documents_link_inventory(),
                item.name.clone(),
                item.name.to_lowercase(),
            ))
        }
    }
}

/// The lower-case text a Transaction is searched by: its Payee names and aliases, memo, Account
/// name, the amount as plain digits (`4380.00`, so `4380`, `4,380` and `-4380.00` all find it) and
/// the date in both ISO and day-month form (`2026-09`, `14 sep`).
fn transaction_haystack(
    transaction: &DocumentTransaction,
    account_name: &str,
    records: &LinkRecords,
) -> String {
    let mut parts: Vec<String> = Vec::new();
    for payee in transaction
        .payee_ids
        .iter()
        .flatten()
        .filter_map(|id| records.payee(*id))
    {
        parts.push(payee.name.clone());
        parts.extend(payee.aliases.iter().cloned());
    }
    if let Some(memo) = &transaction.memo {
        parts.push(memo.clone());
    }
    parts.push(account_name.to_string());
    parts.push(amount_digits(transaction));
    parts.push(transaction.date.format("%Y-%m-%d").to_string());
    parts.push(transaction.date.format("%-d %b %Y").to_string());
    parts.join("\n").to_lowercase()
}

/// `|total|` to two places, unsigned and ungrouped.
fn amount_digits(transaction: &DocumentTransaction) -> String {
    transaction.amount.0.abs().with_scale(2).to_string()
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

#[cfg(test)]
mod tests {
    use super::super::test_support::{dialog_ports, world};
    use super::*;
    use crate::fake::{DocumentsCall, InMemoryFiles, RecordingDocumentsRouter};

    /// A Filed Document with a Payee, a Bill Plan and an Item Link.
    const POLICY: &str = "aami-car-policy.pdf";
    /// An Unfiled Document read as a Receipts purchase.
    const RECEIPT: &str = "IMG_4471.jpg";

    struct Picker {
        presenter: LinkPickerPresenter,
        ports: DialogPorts,
        router: RecordingDocumentsRouter,
    }

    fn picker(purpose: impl Fn(DocumentId) -> Purpose, file: &str) -> Picker {
        let ports = dialog_ports(InMemoryFiles::default());
        let id = world().file(file).id;
        Picker {
            presenter: LinkPickerPresenter::new(ports.clone(), purpose(id))
                .expect("the Document is there"),
            ports,
            router: RecordingDocumentsRouter::default(),
        }
    }

    impl Picker {
        fn send(&mut self, event: LinkPickerEvent) {
            self.presenter.handle(event, &mut self.router);
        }

        fn rows(&self) -> Vec<PickerRowView> {
            self.presenter.view_model().rows
        }
    }

    fn rank(link: Option<DocumentLink>) -> u8 {
        match link {
            None => 0,
            Some(DocumentLink::Transaction(_)) => 1,
            Some(DocumentLink::Account(_)) => 2,
            Some(DocumentLink::Payee(_)) => 3,
            Some(DocumentLink::BillPlan(_)) => 4,
            Some(DocumentLink::InventoryItem(_)) => 5,
        }
    }

    #[test]
    fn the_current_links_are_pinned_first_ticked_and_listed_once() {
        let picker = picker(Purpose::Link, POLICY);
        let current = world().file(POLICY).links.clone();
        let rows = picker.rows();
        let pinned: Vec<_> = rows[..current.len()].iter().map(|row| row.link).collect();
        assert_eq!(
            pinned,
            current.iter().copied().map(Some).collect::<Vec<_>>()
        );
        assert!(rows[..current.len()].iter().all(|row| row.checked));
        assert!(rows[current.len()..].iter().all(|row| !row.checked));
        for link in current {
            assert_eq!(rows.iter().filter(|row| row.link == Some(link)).count(), 1);
        }
    }

    #[test]
    fn transactions_come_nearest_first_then_each_kind_in_its_own_block() {
        let picker = picker(Purpose::Link, RECEIPT_FILED);
        let rows = picker.rows();
        let ranks: Vec<u8> = rows.iter().map(|row| rank(row.link)).collect();
        assert!(ranks.windows(2).all(|pair| pair[0] <= pair[1]), "{ranks:?}");
        assert_eq!(ranks.last(), Some(&5), "Inventory closes the list");
        let payees: Vec<String> = rows
            .iter()
            .filter(|row| matches!(row.link, Some(DocumentLink::Payee(_))))
            .map(|row| row.text.to_lowercase())
            .collect();
        let mut sorted = payees.clone();
        sorted.sort();
        assert_eq!(payees, sorted, "a block is alphabetical");
    }

    /// A Filed Document with no Links, so nothing is pinned.
    const RECEIPT_FILED: &str = "passport-scan.jpg";

    #[test]
    fn tab_cycles_the_kind_filter_and_a_query_narrows_it() {
        let mut picker = picker(Purpose::Link, RECEIPT_FILED);
        for kind in [
            KindFilter::Transaction,
            KindFilter::Account,
            KindFilter::Payee,
        ] {
            picker.send(LinkPickerEvent::CycleKind);
            assert_eq!(picker.presenter.view_model().kind, kind.label());
        }
        assert!(
            picker
                .rows()
                .iter()
                .all(|row| matches!(row.link, Some(DocumentLink::Payee(_))))
        );
        picker.send(LinkPickerEvent::Query("woolw".to_string()));
        let rows = picker.rows();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].text, "Woolworths");
    }

    #[test]
    fn a_money_query_matches_however_it_is_typed() {
        let mut picker = picker(Purpose::Link, RECEIPT_FILED);
        picker.send(LinkPickerEvent::CycleKind);
        for query in ["212.40", "-212.40", "$212.4"] {
            picker.send(LinkPickerEvent::Query(query.to_string()));
            assert!(
                picker
                    .rows()
                    .iter()
                    .any(|row| row.text.contains("Woolworths")),
                "{query} finds the Woolworths purchase"
            );
        }
    }

    #[test]
    fn the_highlight_stops_at_either_end() {
        let mut picker = picker(Purpose::Follow, POLICY);
        let last = picker.rows().len() - 1;
        picker.send(LinkPickerEvent::Up);
        assert_eq!(picker.presenter.view_model().selected, 0);
        for _ in 0..last + 3 {
            picker.send(LinkPickerEvent::Down);
        }
        assert_eq!(picker.presenter.view_model().selected, last);
    }

    #[test]
    fn a_link_pick_toggles_it_and_says_so() {
        let mut picker = picker(Purpose::Link, POLICY);
        let id = picker.presenter.purpose().document();
        let first = picker.rows()[0].clone();
        let link = first.link.expect("a current Link");

        picker.send(LinkPickerEvent::Pick);

        assert!(
            !document(&picker.ports, id)
                .expect("still there")
                .links
                .contains(&link)
        );
        assert_eq!(
            picker.router.take(),
            [
                DocumentsCall::Notice(Notice::Toast(
                    ToastKind::Info,
                    format!("Unlinked {}.", first.text)
                )),
                DocumentsCall::CloseDialog(None),
            ]
        );
    }

    #[test]
    fn filing_pins_file_without_link_and_waits_for_a_type() {
        let mut picker = picker(Purpose::File, UNREAD);
        let id = picker.presenter.purpose().document();
        let view = picker.presenter.view_model();
        assert_eq!(view.rows[0].link, None);
        assert!(view.rows[1..].iter().all(|row| row.link.is_some()));
        assert_eq!(view.type_line.and_then(|line| line.chosen), None);

        picker.send(LinkPickerEvent::Pick);
        assert_eq!(
            picker.router.take(),
            [DocumentsCall::Notice(Notice::Status(
                "Choose a document type first (← →).".to_string()
            ))]
        );

        picker.send(LinkPickerEvent::StepType { forward: true });
        picker.send(LinkPickerEvent::StepType { forward: false });
        let types = world().types;
        assert_eq!(
            picker
                .presenter
                .view_model()
                .type_line
                .and_then(|line| line.chosen),
            Some(types[types.len() - 1].name.clone()),
            "back from the first wraps to the last"
        );
        picker.send(LinkPickerEvent::PickRow(0));
        assert_eq!(
            picker.router.take(),
            [DocumentsCall::CloseDialog(Some(
                DocumentsOutcome::FileByHand {
                    id,
                    doc_type: DocumentType(types[types.len() - 1].id),
                    link: None,
                }
            ))]
        );
    }

    /// An Unfiled Document nothing could be read off, so no Document Type is prefilled.
    const UNREAD: &str = "scan0012.pdf";

    #[test]
    fn filing_starts_on_the_type_read_off_the_file() {
        let picker = picker(Purpose::File, RECEIPT);
        assert_eq!(
            picker
                .presenter
                .view_model()
                .type_line
                .and_then(|line| line.chosen),
            Some("Receipts".to_string())
        );
    }

    #[test]
    fn follow_lists_exactly_the_links_and_a_pick_follows_one() {
        let mut picker = picker(Purpose::Follow, POLICY);
        let current = world().file(POLICY).links.clone();
        let view = picker.presenter.view_model();
        assert!(view.kinds.is_none(), "following has no kind filter");
        let links: Vec<_> = view.rows.iter().filter_map(|row| row.link).collect();
        assert_eq!(links, current);

        picker.send(LinkPickerEvent::Down);
        picker.send(LinkPickerEvent::Pick);
        assert_eq!(
            picker.router.take(),
            [DocumentsCall::CloseDialog(Some(DocumentsOutcome::Follow(
                current[1]
            )))]
        );
    }
}
