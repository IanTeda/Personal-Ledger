//! The Transactions page (`docs/ux/desktop-mockups/03-transactions/README.md`'s 4a): the table of
//! Transactions newest first, narrowed by the applied filters and the Search, with the count line,
//! the chip row, the footer's totals and the status line's legend. It holds the filter popover's
//! Presenter while the popover is open and sends it the popover's events, as the Bills page sends
//! each tab its own.
//!
//! - **Selection** is a position in the rows shown, moved through `lib-list-cursor` and clamped
//!   wherever it is read. A half page is what the Client measures its viewport to hold.
//! - **Search** is typed live: each key narrows the rows and puts the selection back on the first.
//!   Keeping it leaves the text; clearing it empties it. A Search left open when the page is left
//!   is closed when the page is entered again, its text kept.
//! - **Hand-offs:** a revealed Transaction is selected with the date range widened to reach it,
//!   and nothing else filtering it out; one that has gone leaves the page as it was. A filtered
//!   hand-off (an Account's, a Payee's, a Tag's, a Budget Category's) starts from the first row.
//! - **Not yet built:** `Enter`, `e` and `n` say so on the status line, since there is no
//!   Transaction form yet.
//!
//! The rows are read from the ports each time the view model is built, so a write any module makes
//! shows on the next draw.

use std::collections::HashSet;

use chrono::NaiveDate;
use lib_core::{AccountId, Clock, DateStyle, HexColor, Money, PayeeId, TagId, TransactionId};
use lib_core::{Total, TransactionStatus};
use lib_list_cursor::Step;
use lib_locale::format::{AmountStyle, format_amount, format_date};
use lib_toast::{Notice, Notify};
use lib_viper::keymap::{EDIT, Keymap, MOVE_DOWN, MOVE_UP, NEW, SELECT};
use lib_viper::view::Hint;

use super::filter::{
    ChipField, ChipView, FilterEvent, FilterOutcome, FilterPresenter, FilterViewModel,
};
use crate::keymap::FILTER;
use crate::protocols::{SharedPorts, TransactionsPorts};
use crate::query::{TransactionFilters, Visible, query};
use crate::{Transaction, TransactionCategory, TransactionPayee, TransactionTag, msg};

/// The mark for an empty Payee or Tags cell.
pub const EMPTY_CELL: &str = "\u{2014}";

/// What the page is told.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransactionsPageEvent {
    /// A movement of the selection.
    Move(Step),
    /// A click on row `n` of the rows shown.
    Select(usize),
    /// `Enter`, `n` (and **add transaction**) and `e`: not yet built.
    Open,
    New,
    Edit,
    /// `f` (`None`) or a click on a chip: the filter popover on a draft of the applied filters.
    OpenFilter(Option<ChipField>),
    /// An event for the open filter popover; nothing while it is closed.
    Filter(FilterEvent),
    /// The `✕` on an accent chip: just that filter back to its default.
    ClearChip(ChipField),
    /// **clear filters**: every filter back to the defaults. The Search is left alone.
    ClearFilters,
    /// `/` or a click on the search box.
    StartSearch,
    /// A character typed into the open Search, and `Backspace` on it.
    SearchType(char),
    SearchBackspace,
    /// `Enter` in the Search: leaves it, keeping the text.
    KeepSearch,
    /// `Esc` in the Search: leaves it, emptying the text.
    ClearSearch,
    /// A hand-off from another View: Transaction `id`, selected, the date range widened to reach
    /// it.
    RevealTransaction(TransactionId),
    /// A hand-off from an Account: its Transactions this year.
    RevealAccount(AccountId),
    /// A hand-off with its own filters, from the first row.
    ShowFiltered(TransactionFilters),
    /// The page became the destination.
    Entered,
    /// The Display Preference the dates show and are typed in.
    SetDateStyle(Option<DateStyle>),
}

/// The TAGS cell.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TagsCell {
    /// No Split carries a Tag: the cell shows [`EMPTY_CELL`].
    None,
    /// The first Tag as a chip (with its colour, if it has one), and how many further Tags follow.
    Chips {
        first: String,
        color: Option<HexColor>,
        more: usize,
    },
}

/// One row of the table, every cell formatted. The status glyphs are the Client's to draw, from
/// `status` and `flagged`.
#[derive(Debug, Clone, PartialEq)]
pub struct RowView {
    pub id: TransactionId,
    pub status: TransactionStatus,
    pub flagged: bool,
    pub date: String,
    pub account: String,
    pub payee: String,
    pub category: String,
    pub tags: TagsCell,
    pub amount: String,
    pub amount_negative: bool,
    /// `None` when the running total does not exist (mixed Units); the cell is blank.
    pub running: Option<String>,
    pub running_negative: bool,
}

/// The footer's grand total, formatted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FooterTotal {
    /// No rows are shown.
    Empty,
    /// One Unit: the figure (`negative` selects the negative token) and the Unit's code.
    Single {
        unit: String,
        negative: bool,
        text: String,
    },
    /// The rows shown span more than one Unit, so there is no sum.
    Mixed,
}

/// The footer bar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FooterView {
    /// `8 of 96 groceries transactions shown`.
    pub label: String,
    pub total: FooterTotal,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TransactionsPageViewModel {
    /// `700 transactions across 4 accounts`: the whole Ledger, and how many Accounts hold it.
    pub count_line: String,
    pub chips: Vec<ChipView>,
    /// A filter differs from the default, so **clear filters** has something to do.
    pub filters_changed: bool,
    pub search: String,
    /// The Search is open, so the box shows a caret.
    pub searching: bool,
    pub rows: Vec<RowView>,
    /// A position in `rows`, clamped: `0` when there are none.
    pub selected: usize,
    pub footer: FooterView,
    /// The status line's legend for the page.
    pub hints: Vec<Hint>,
    /// The filter popover, while it is open.
    pub filter: Option<FilterViewModel>,
}

pub struct TransactionsPagePresenter {
    ports: SharedPorts,
    /// Today, which the default filters and a date typed as `today` read.
    clock: Clock,
    keymap: Keymap<TransactionsPageEvent>,
    /// The Shell's chord for Search, which the legend names.
    search_chord: String,
    /// A position in the rows shown, clamped wherever it is read.
    selected: usize,
    /// How far a half-page move steps.
    half_page: usize,
    filters: TransactionFilters,
    search: String,
    searching: bool,
    /// The filter popover, `Some` while it is open.
    filter: Option<FilterPresenter>,
    date_style: Option<DateStyle>,
}

impl TransactionsPagePresenter {
    /// The page on its first row, filtered to the defaults (this year). `keymap` is the one the
    /// Client looks keys up in and `search_chord` the Shell's key for Search, so the legend shows
    /// the chords they bind.
    pub fn new(
        ports: SharedPorts,
        clock: Clock,
        keymap: Keymap<TransactionsPageEvent>,
        search_chord: String,
    ) -> Self {
        let filters = TransactionFilters::defaults(clock.today());
        Self {
            ports,
            clock,
            keymap,
            search_chord,
            selected: 0,
            half_page: lib_list_cursor::HALF_PAGE,
            filters,
            search: String::new(),
            searching: false,
            filter: None,
            date_style: None,
        }
    }

    /// Follows a change to `[keybindings]`.
    pub fn set_keymap(&mut self, keymap: Keymap<TransactionsPageEvent>, search_chord: String) {
        self.keymap = keymap;
        self.search_chord = search_chord;
    }

    /// How many rows a half-page move steps: what the Client measures half its viewport to hold.
    /// Never zero.
    pub fn set_half_page(&mut self, rows: usize) {
        self.half_page = rows.max(1);
    }

    /// The selected position, clamped to the rows shown.
    pub fn selected(&self) -> usize {
        self.selected.min(self.row_count().saturating_sub(1))
    }

    pub fn is_filtering(&self) -> bool {
        self.filter.is_some()
    }

    pub fn is_searching(&self) -> bool {
        self.searching
    }

    /// The open popover's Presenter, for the Client to read which field has focus.
    pub fn filter(&self) -> Option<&FilterPresenter> {
        self.filter.as_ref()
    }

    pub fn date_style(&self) -> Option<DateStyle> {
        self.date_style
    }

    pub fn handle(&mut self, event: TransactionsPageEvent, notify: &mut dyn Notify) {
        if let Some(filter) = self.filter.as_mut() {
            match event {
                TransactionsPageEvent::Filter(event) => {
                    match filter.handle(event) {
                        Some(FilterOutcome::Applied(filters)) => {
                            self.filters = filters;
                            self.filter = None;
                            self.selected = 0;
                        }
                        Some(FilterOutcome::Cancelled) => self.filter = None,
                        None => {}
                    }
                    return;
                }
                // A hand-off, or the page entered again, closes the popover, its draft discarded.
                TransactionsPageEvent::RevealTransaction(_)
                | TransactionsPageEvent::RevealAccount(_)
                | TransactionsPageEvent::ShowFiltered(_)
                | TransactionsPageEvent::Entered => self.filter = None,
                // The draft's dates follow a new style, so what was typed still reads as meant.
                TransactionsPageEvent::SetDateStyle(style) => filter.set_date_style(style),
                // The popover is modal: nothing else on the page is reached while it is open.
                _ => return,
            }
        }
        let today = self.clock.today();
        match event {
            TransactionsPageEvent::Move(step) => {
                let len = self.row_count();
                let at = self.selected.min(len.saturating_sub(1));
                self.selected = lib_list_cursor::step_by(at, len, step, self.half_page);
            }
            TransactionsPageEvent::Select(index) => self.selected = index,
            TransactionsPageEvent::Open => {
                notify.notify(Notice::Status(msg::transactions_open_not_yet_built()));
            }
            TransactionsPageEvent::New => {
                notify.notify(Notice::Status(msg::transactions_add_not_yet_built()));
            }
            TransactionsPageEvent::Edit => {
                notify.notify(Notice::Status(msg::transactions_edit_not_yet_built()));
            }
            // A chip clicked while the Search is open leaves the Search, its text kept, as `Enter`
            // would: the popover's mode takes the Search's place.
            TransactionsPageEvent::OpenFilter(chip) => {
                self.searching = false;
                self.filter = Some(FilterPresenter::open(
                    &*self.ports.borrow(),
                    &self.filters,
                    chip,
                    today,
                    self.date_style,
                ));
            }
            TransactionsPageEvent::Filter(_) => {}
            TransactionsPageEvent::ClearChip(chip) => {
                FilterPresenter::clear_chip(&mut self.filters, chip, today);
                self.selected = 0;
            }
            TransactionsPageEvent::ClearFilters => {
                FilterPresenter::clear_all(&mut self.filters, today);
                self.selected = 0;
            }
            TransactionsPageEvent::StartSearch => self.searching = true,
            TransactionsPageEvent::SearchType(ch) => {
                if self.searching && !ch.is_control() {
                    self.search.push(ch);
                    self.selected = 0;
                }
            }
            TransactionsPageEvent::SearchBackspace => {
                if self.searching {
                    self.search.pop();
                    self.selected = 0;
                }
            }
            TransactionsPageEvent::KeepSearch => self.searching = false,
            TransactionsPageEvent::ClearSearch => {
                self.search.clear();
                self.searching = false;
                self.selected = 0;
            }
            TransactionsPageEvent::RevealTransaction(id) => self.reveal(id, today),
            TransactionsPageEvent::RevealAccount(id) => {
                self.show_filtered(TransactionFilters::for_account(today, id));
            }
            TransactionsPageEvent::ShowFiltered(filters) => self.show_filtered(filters),
            TransactionsPageEvent::Entered => self.searching = false,
            TransactionsPageEvent::SetDateStyle(style) => self.date_style = style,
        }
    }

    /// The table filtered to `filters` from its first row, with no Search or draft left over.
    fn show_filtered(&mut self, filters: TransactionFilters) {
        self.filters = filters;
        self.search.clear();
        self.searching = false;
        self.filter = None;
        self.selected = 0;
    }

    /// Transaction `id`, selected, on the default filters with the date range widened to reach
    /// it; nothing for a Transaction that has gone.
    fn reveal(&mut self, id: TransactionId, today: NaiveDate) {
        let Some(date) = self
            .ports
            .borrow()
            .all_transactions()
            .into_iter()
            .find(|transaction| transaction.id == id)
            .map(|transaction| transaction.date)
        else {
            return;
        };
        let mut filters = TransactionFilters::defaults(today);
        filters.from = filters.from.map(|from| from.min(date));
        filters.to = filters.to.map(|to| to.max(date));
        self.show_filtered(filters);
        let ports = self.ports.borrow();
        let transactions = newest_first(&*ports);
        self.selected = query(&*ports, &transactions, &self.filters, &self.search)
            .rows
            .iter()
            .position(|row| row.transaction.id == id)
            .unwrap_or(0);
    }

    /// How many rows the filters and Search leave.
    fn row_count(&self) -> usize {
        let ports = self.ports.borrow();
        let transactions = newest_first(&*ports);
        query(&*ports, &transactions, &self.filters, &self.search)
            .rows
            .len()
    }

    pub fn view_model(&self) -> TransactionsPageViewModel {
        let ports = self.ports.borrow();
        let transactions = newest_first(&*ports);
        let visible = query(&*ports, &transactions, &self.filters, &self.search);
        let rows = rows(&*ports, &visible, self.date_style);
        let today = self.clock.today();
        TransactionsPageViewModel {
            count_line: count_line(&transactions),
            chips: FilterPresenter::chips(&*ports, &self.filters, today, self.date_style),
            filters_changed: !self.filters.is_default(today),
            search: self.search.clone(),
            searching: self.searching,
            selected: self.selected.min(rows.len().saturating_sub(1)),
            footer: footer(&*ports, &visible, &self.filters),
            rows,
            hints: self.hints(),
            filter: self.filter.as_ref().map(FilterPresenter::view_model),
        }
    }

    /// The page's legend, its chords read from the keymap so an override shows. The status line
    /// reads it on every draw, so it reads no rows.
    pub fn hints(&self) -> Vec<Hint> {
        let chord = |command: &str| self.keymap.chord(command).unwrap_or_default().to_string();
        let hint = |chord: String, action: String| Hint { chord, action };
        vec![
            hint(
                format!("{}/{}", chord(MOVE_DOWN), chord(MOVE_UP)),
                msg::transactions_hint_row(),
            ),
            hint(chord(SELECT), msg::transactions_hint_open()),
            hint(chord(EDIT), msg::transactions_hint_edit()),
            hint(chord(NEW), msg::transactions_hint_add()),
            hint(self.search_chord.clone(), msg::transactions_hint_search()),
            hint(chord(FILTER), msg::transactions_hint_filter()),
        ]
    }
}

/// Every Transaction, newest first and the later id first on one day, as the table lists them.
fn newest_first(ports: &dyn TransactionsPorts) -> Vec<Transaction> {
    let mut transactions = ports.all_transactions();
    transactions.sort_by(|a, b| b.date.cmp(&a.date).then(b.id.cmp(&a.id)));
    transactions
}

/// A count as a Message's plural argument.
fn count(n: usize) -> i64 {
    i64::try_from(n).unwrap_or(i64::MAX)
}

/// `700 transactions across 4 accounts`: the whole Ledger, and how many Accounts hold any of it.
fn count_line(transactions: &[Transaction]) -> String {
    let accounts: HashSet<AccountId> = transactions.iter().map(|t| t.account_id).collect();
    msg::transactions_count_line(count(transactions.len()), count(accounts.len()))
}

/// An amount at its own places, as `(is_negative, text)`; `signed` puts a `+` before income.
fn amount(money: &Money, signed: bool) -> (bool, String) {
    let style = if signed {
        AmountStyle::own().with_plus()
    } else {
        AmountStyle::own()
    };
    let formatted = format_amount(money, style);
    (formatted.is_negative, formatted.text)
}

/// Every row shown, formatted, with dates in `date_style`.
fn rows(
    ports: &dyn TransactionsPorts,
    visible: &Visible<'_>,
    date_style: Option<DateStyle>,
) -> Vec<RowView> {
    let accounts = ports.transaction_accounts();
    let categories = ports.transaction_categories();
    let payees = ports.transaction_payees();
    let tags = ports.transaction_tags();
    visible
        .rows
        .iter()
        .map(|row| {
            let transaction = row.transaction;
            let (amount_negative, amount_text) = amount(&row.amount, true);
            let (running_negative, running) = match &row.running {
                Some(running) => {
                    let (negative, text) = amount(running, false);
                    (negative, Some(text))
                }
                None => (false, None),
            };
            RowView {
                id: transaction.id,
                status: transaction.status.clone(),
                flagged: transaction.is_flagged,
                date: format_date(transaction.date, date_style),
                account: accounts
                    .iter()
                    .find(|account| account.id == transaction.account_id)
                    .map(|account| account.name.clone())
                    .unwrap_or_default(),
                payee: payee_summary(transaction, &payees),
                category: category_summary(transaction, &categories),
                tags: tags_summary(transaction, &tags),
                amount: amount_text,
                amount_negative,
                running,
                running_negative,
            }
        })
        .collect()
}

/// The PAYEE cell: the first Payee among the Splits, then `+N` for each further *different* Payee.
fn payee_summary(transaction: &Transaction, payees: &[TransactionPayee]) -> String {
    let ids: Vec<PayeeId> = transaction
        .splits
        .iter()
        .filter_map(|split| split.payee_id)
        .collect();
    let Some(&first) = ids.first() else {
        return EMPTY_CELL.to_string();
    };
    let name = payees
        .iter()
        .find(|payee| payee.id == first)
        .map_or(EMPTY_CELL, |payee| payee.name.as_str());
    let mut others: Vec<PayeeId> = ids.into_iter().filter(|id| *id != first).collect();
    others.sort_unstable();
    others.dedup();
    if others.is_empty() {
        name.to_string()
    } else {
        format!("{name} +{}", others.len())
    }
}

/// The CATEGORY cell: the leaf's own name for one Split, `split · N` for several.
fn category_summary(transaction: &Transaction, categories: &[TransactionCategory]) -> String {
    match transaction.splits.as_slice() {
        [only] => categories
            .iter()
            .find(|category| category.id == only.category_id)
            .map_or_else(|| EMPTY_CELL.to_string(), |category| category.name.clone()),
        several => msg::transactions_split_summary(&several.len().to_string()),
    }
}

/// The TAGS cell: the union of every Split's Tags in first-seen order.
fn tags_summary(transaction: &Transaction, tags: &[TransactionTag]) -> TagsCell {
    let mut ids: Vec<TagId> = Vec::new();
    for split in &transaction.splits {
        for id in &split.tag_ids {
            if !ids.contains(id) {
                ids.push(*id);
            }
        }
    }
    let found: Vec<&TransactionTag> = ids
        .iter()
        .filter_map(|id| tags.iter().find(|tag| tag.id == *id))
        .collect();
    match found.split_first() {
        None => TagsCell::None,
        Some((first, rest)) => TagsCell::Chips {
            first: first.name.clone(),
            color: first.color.clone(),
            more: rest.len(),
        },
    }
}

/// The footer for the rows shown: `N of M`, naming a Category filter's Category, and the total.
fn footer(
    ports: &dyn TransactionsPorts,
    visible: &Visible<'_>,
    filters: &TransactionFilters,
) -> FooterView {
    let category = filters
        .category
        .and_then(|id| {
            ports
                .transaction_categories()
                .into_iter()
                .find(|category| category.id == id)
        })
        .map(|category| format!("{} ", category.name.to_lowercase()))
        .unwrap_or_default();
    let total = match &visible.total {
        Total::Empty => FooterTotal::Empty,
        Total::Mixed => FooterTotal::Mixed,
        Total::Single { unit, amount: sum } => {
            let (negative, text) = amount(sum, false);
            FooterTotal::Single {
                unit: unit.clone(),
                negative,
                text,
            }
        }
    };
    FooterView {
        label: msg::transactions_footer(
            count(visible.ledger_size),
            &visible.rows.len().to_string(),
            &category,
        ),
        total,
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::rc::Rc;

    use lib_config::KeyBindingConfig;
    use lib_core::{CategoryId, UnitId};

    use super::*;
    use crate::Split;
    use crate::fake::{InMemoryPorts, InMemoryTransactions};
    use crate::keymap::transactions_page_keymap;
    use crate::presenter::FilterField;
    use crate::presenter::test_support::*;
    use crate::{TransactionRepository, TransactionUnit};

    const SHOP: TransactionId = TransactionId::seed(1);
    const RENT: TransactionId = TransactionId::seed(2);
    const TRIP: TransactionId = TransactionId::seed(3);
    const REFUND: TransactionId = TransactionId::seed(4);
    const LAST_YEAR: TransactionId = TransactionId::seed(5);

    fn split(amount: &str, category: CategoryId, payee: Option<PayeeId>, tags: &[TagId]) -> Split {
        Split {
            amount: amount.parse().expect("an amount"),
            category_id: category,
            payee_id: payee,
            tag_ids: tags.to_vec(),
        }
    }

    fn transaction(
        id: TransactionId,
        date: NaiveDate,
        account: AccountId,
        status: TransactionStatus,
        description: &str,
        splits: Vec<Split>,
    ) -> Transaction {
        Transaction {
            id,
            date,
            account_id: account,
            status,
            is_flagged: false,
            description: Some(description.to_string()),
            splits,
        }
    }

    /// Four Transactions this year and one last year, in no particular order, as a port lists them.
    fn stub() -> InMemoryPorts {
        let mut reads = reads();
        reads.units = vec![TransactionUnit {
            id: UnitId::SEED_AUD,
            code: "aud".to_string(),
        }];
        let transactions = vec![
            transaction(
                RENT,
                date(2026, 9, 10),
                EVERYDAY,
                TransactionStatus::Cleared,
                "rent",
                vec![split("-1200.00", HOUSING, None, &[])],
            ),
            transaction(
                SHOP,
                date(2026, 9, 18),
                EVERYDAY,
                TransactionStatus::Open,
                "weekly shop",
                vec![split("-50.00", GROCERIES, Some(WOOLWORTHS), &[])],
            ),
            transaction(
                LAST_YEAR,
                date(2025, 6, 1),
                EVERYDAY,
                TransactionStatus::Reconciled,
                "last year",
                vec![split("-10.00", HOUSING, None, &[])],
            ),
            transaction(
                TRIP,
                date(2026, 8, 1),
                WALLET,
                TransactionStatus::Reconciled,
                "trip",
                vec![
                    split("-20.00", GROCERIES, Some(WOOLWORTHS), &[WORK_TRIP]),
                    split("-5.00", HOUSING, None, &[WORK_TRIP]),
                ],
            ),
            transaction(
                REFUND,
                date(2026, 3, 2),
                EVERYDAY,
                TransactionStatus::Open,
                "refund",
                vec![split("100.00", FOOD, None, &[])],
            ),
        ];
        InMemoryPorts {
            transactions: InMemoryTransactions { transactions },
            reads,
        }
    }

    struct Page {
        ports: Rc<RefCell<InMemoryPorts>>,
        presenter: TransactionsPagePresenter,
        notices: Vec<Notice>,
    }

    impl Page {
        fn new() -> Self {
            Self::with_keys(&KeyBindingConfig::default())
        }

        fn with_keys(config: &KeyBindingConfig) -> Self {
            locale();
            let (ports, shared) = stub().shared();
            let presenter = TransactionsPagePresenter::new(
                shared,
                Clock::fixed(today()),
                transactions_page_keymap(config),
                "/".to_string(),
            );
            Self {
                ports,
                presenter,
                notices: Vec::new(),
            }
        }

        fn send(&mut self, event: TransactionsPageEvent) {
            self.presenter.handle(event, &mut self.notices);
        }

        fn view(&self) -> TransactionsPageViewModel {
            self.presenter.view_model()
        }

        /// The ids of the rows shown, in order.
        fn ids(&self) -> Vec<TransactionId> {
            self.view().rows.iter().map(|row| row.id).collect()
        }

        fn selected_id(&self) -> TransactionId {
            let view = self.view();
            view.rows[view.selected].id
        }

        fn search(&mut self, text: &str) {
            self.send(TransactionsPageEvent::StartSearch);
            for ch in text.chars() {
                self.send(TransactionsPageEvent::SearchType(ch));
            }
        }
    }

    #[test]
    fn the_page_opens_on_this_year_newest_first_on_the_first_row() {
        let page = Page::new();

        assert_eq!(page.ids(), [SHOP, RENT, TRIP, REFUND]);
        let view = page.view();
        assert_eq!(view.selected, 0);
        assert!(!view.filters_changed);
        assert_eq!(view.count_line, "5 transactions across 2 accounts");
    }

    #[test]
    fn a_row_reads_its_cells_with_splits_summarised() {
        let page = Page::new();
        let view = page.view();

        let shop = &view.rows[0];
        assert_eq!(shop.account, "Everyday");
        assert_eq!(shop.payee, "Woolworths");
        assert_eq!(shop.category, "Groceries");
        assert_eq!(shop.tags, TagsCell::None);
        assert_eq!(shop.amount, "\u{2212}50.00");
        assert!(shop.amount_negative);
        let trip = &view.rows[2];
        assert_eq!(trip.category, "split \u{b7} 2");
        assert_eq!(
            trip.tags,
            TagsCell::Chips {
                first: "work-trip".to_string(),
                color: None,
                more: 0
            }
        );
        let rent = &view.rows[1];
        assert_eq!(rent.payee, EMPTY_CELL);
        let refund = &view.rows[3];
        assert_eq!(refund.amount, "+100.00", "income is signed");
        assert!(!refund.amount_negative);
    }

    #[test]
    fn filters_and_search_narrow_the_rows() {
        let mut page = Page::new();

        page.send(TransactionsPageEvent::ShowFiltered(
            TransactionFilters::for_account(today(), WALLET),
        ));
        assert_eq!(page.ids(), [TRIP]);

        page.send(TransactionsPageEvent::ClearFilters);
        page.search("wool");
        assert_eq!(page.ids(), [SHOP, TRIP], "Search matches a Payee's name");
        page.send(TransactionsPageEvent::SearchBackspace);
        page.send(TransactionsPageEvent::SearchBackspace);
        page.send(TransactionsPageEvent::SearchBackspace);
        page.send(TransactionsPageEvent::SearchBackspace);
        page.search("rent");
        assert_eq!(page.ids(), [RENT], "and a description");
        assert_eq!(page.view().search, "rent");
    }

    #[test]
    fn the_filter_popover_drafts_until_apply_and_takes_the_page_s_events_while_open() {
        let mut page = Page::new();
        page.send(TransactionsPageEvent::Move(Step::Next));

        page.send(TransactionsPageEvent::OpenFilter(None));
        assert!(page.view().filter.is_some());
        page.send(TransactionsPageEvent::Filter(FilterEvent::Focus(
            FilterField::Payee,
        )));
        for ch in "wool".chars() {
            page.send(TransactionsPageEvent::Filter(FilterEvent::Type(ch)));
        }
        page.send(TransactionsPageEvent::Move(Step::Next));
        assert_eq!(page.ids().len(), 4, "the draft filters nothing yet");
        assert_eq!(page.view().selected, 1, "the page is out of reach");

        page.send(TransactionsPageEvent::Filter(FilterEvent::Apply));

        let view = page.view();
        assert!(view.filter.is_none());
        assert!(view.filters_changed);
        assert!(view.chips[2].active, "the payee chip turns accent");
        assert_eq!(page.ids(), [SHOP, TRIP]);
        assert_eq!(view.selected, 0, "applying starts from the first row");
    }

    #[test]
    fn cancel_discards_the_draft_and_a_chip_clear_resets_only_its_filter() {
        let mut page = Page::new();
        page.send(TransactionsPageEvent::OpenFilter(Some(ChipField::Payee)));
        page.send(TransactionsPageEvent::Filter(FilterEvent::Type('x')));
        page.send(TransactionsPageEvent::Filter(FilterEvent::Cancel));
        assert!(page.view().filter.is_none());
        assert!(!page.view().filters_changed);

        let mut filters = TransactionFilters::for_account(today(), EVERYDAY);
        filters.payee = "wool".to_string();
        page.send(TransactionsPageEvent::ShowFiltered(filters));
        assert_eq!(page.ids(), [SHOP]);
        page.send(TransactionsPageEvent::ClearChip(ChipField::Account));
        assert_eq!(page.ids(), [SHOP, TRIP]);
        assert!(page.view().filters_changed, "the payee filter stays");
    }

    #[test]
    fn the_selection_moves_by_rows_ends_and_half_pages_and_stays_in_range() {
        let mut page = Page::new();
        page.presenter.set_half_page(2);

        page.send(TransactionsPageEvent::Move(Step::Prev));
        assert_eq!(page.view().selected, 0, "the first row is the top");
        page.send(TransactionsPageEvent::Move(Step::Next));
        assert_eq!(page.selected_id(), RENT);
        page.send(TransactionsPageEvent::Move(Step::Last));
        assert_eq!(page.selected_id(), REFUND);
        page.send(TransactionsPageEvent::Move(Step::Next));
        assert_eq!(page.selected_id(), REFUND, "the last row is the bottom");
        page.send(TransactionsPageEvent::Move(Step::First));
        page.send(TransactionsPageEvent::Move(Step::HalfPageDown));
        assert_eq!(page.view().selected, 2);
        page.send(TransactionsPageEvent::Move(Step::HalfPageUp));
        assert_eq!(page.view().selected, 0);

        page.send(TransactionsPageEvent::Select(3));
        page.search("rent");
        assert_eq!(page.view().selected, 0, "typing starts from the first row");
    }

    #[test]
    fn a_reveal_widens_the_date_range_and_selects_the_transaction() {
        let mut page = Page::new();
        page.search("zzz");
        page.send(TransactionsPageEvent::KeepSearch);

        page.send(TransactionsPageEvent::RevealTransaction(LAST_YEAR));

        let view = page.view();
        assert_eq!(view.search, "", "the Search is cleared");
        assert_eq!(page.selected_id(), LAST_YEAR);
        assert_eq!(page.ids(), [SHOP, RENT, TRIP, REFUND, LAST_YEAR]);
        assert!(view.chips[4].active, "the date chip shows the wider range");
    }

    #[test]
    fn a_reveal_of_a_transaction_that_has_gone_leaves_the_page_as_it_was() {
        let mut page = Page::new();
        page.send(TransactionsPageEvent::Move(Step::Next));
        page.ports
            .borrow_mut()
            .transactions
            .delete_transaction(LAST_YEAR);
        let before = page.view();

        page.send(TransactionsPageEvent::RevealTransaction(LAST_YEAR));

        assert_eq!(page.view(), before);
    }

    #[test]
    fn a_filtered_hand_off_starts_from_the_first_row_with_no_search_or_draft() {
        let mut page = Page::new();
        page.search("e");
        page.send(TransactionsPageEvent::KeepSearch);
        page.send(TransactionsPageEvent::Move(Step::Last));
        page.send(TransactionsPageEvent::OpenFilter(None));

        page.send(TransactionsPageEvent::RevealAccount(EVERYDAY));

        let view = page.view();
        assert_eq!(view.selected, 0);
        assert_eq!(view.search, "");
        assert!(view.filter.is_none());
        assert_eq!(page.ids(), [SHOP, RENT, REFUND]);
    }

    #[test]
    fn the_footer_counts_the_rows_and_totals_them_with_a_running_balance() {
        let mut page = Page::new();
        let view = page.view();
        assert_eq!(view.footer.label, "4 of 5 transactions shown");
        assert_eq!(
            view.footer.total,
            FooterTotal::Single {
                unit: "aud".to_string(),
                negative: true,
                text: "\u{2212}1,175.00".to_string(),
            }
        );
        let running: Vec<_> = view.rows.iter().map(|row| row.running.clone()).collect();
        assert_eq!(
            running,
            [
                Some("\u{2212}50.00".to_string()),
                Some("\u{2212}1,250.00".to_string()),
                Some("\u{2212}1,275.00".to_string()),
                Some("\u{2212}1,175.00".to_string()),
            ]
        );

        let mut groceries = TransactionFilters::defaults(today());
        groceries.category = Some(GROCERIES);
        page.send(TransactionsPageEvent::ShowFiltered(groceries));
        let view = page.view();
        assert_eq!(view.footer.label, "2 of 5 groceries transactions shown");
        assert_eq!(
            view.rows[1].amount, "\u{2212}20.00",
            "only the matching Split"
        );
    }

    #[test]
    fn rows_in_more_than_one_unit_have_no_total_or_running_balance() {
        let mut page = Page::new();
        page.ports.borrow_mut().reads.accounts[1].unit = UnitId::SEED_BTC;

        let view = page.view();
        assert_eq!(view.footer.total, FooterTotal::Mixed);
        assert!(view.rows.iter().all(|row| row.running.is_none()));

        page.search("zzz");
        assert_eq!(page.view().footer.total, FooterTotal::Empty);
    }

    #[test]
    fn set_date_style_formats_the_dates() {
        let mut page = Page::new();

        page.send(TransactionsPageEvent::SetDateStyle(Some(DateStyle::Iso)));

        assert_eq!(page.view().rows[0].date, "2026-09-18");
        assert_eq!(page.presenter.date_style(), Some(DateStyle::Iso));
    }

    #[test]
    fn enter_n_and_e_answer_that_they_are_not_yet_built() {
        let mut page = Page::new();

        page.send(TransactionsPageEvent::Open);
        page.send(TransactionsPageEvent::New);
        page.send(TransactionsPageEvent::Edit);

        assert_eq!(
            page.notices,
            [
                Notice::Status("open transaction \u{2014} not yet built".to_string()),
                Notice::Status("add transaction \u{2014} not yet built".to_string()),
                Notice::Status("edit transaction \u{2014} not yet built".to_string()),
            ]
        );
    }

    #[test]
    fn keep_holds_the_search_text_clear_empties_it_and_entering_closes_it() {
        let mut page = Page::new();
        page.search("rent");

        page.send(TransactionsPageEvent::KeepSearch);
        assert!(!page.view().searching);
        assert_eq!(page.ids(), [RENT]);
        page.send(TransactionsPageEvent::SearchType('x'));
        assert_eq!(
            page.view().search,
            "rent",
            "a closed Search takes no typing"
        );

        page.send(TransactionsPageEvent::StartSearch);
        page.send(TransactionsPageEvent::Entered);
        let view = page.view();
        assert!(!view.searching, "a Search left open closes on entering");
        assert_eq!(view.search, "rent");

        page.send(TransactionsPageEvent::StartSearch);
        page.send(TransactionsPageEvent::ClearSearch);
        assert_eq!(page.view().search, "");
        assert_eq!(page.ids().len(), 4);
    }

    #[test]
    fn a_renamed_payee_shows_on_the_next_view_model() {
        let page = Page::new();
        page.ports.borrow_mut().reads.payees[0].name = "Woolies".to_string();

        assert_eq!(page.view().rows[0].payee, "Woolies");
    }

    #[test]
    fn the_legend_reads_its_chords_from_the_keymap() {
        let mut config = KeyBindingConfig::default();
        config
            .bindings
            .insert("transactions.filter".to_string(), "F".to_string());
        let page = Page::with_keys(&config);

        let hints: Vec<(String, String)> = page
            .view()
            .hints
            .into_iter()
            .map(|hint| (hint.chord, hint.action))
            .collect();
        let pair = |chord: &str, action: &str| (chord.to_string(), action.to_string());
        assert_eq!(
            hints,
            [
                pair("j/k", "row"),
                pair("enter", "open"),
                pair("e", "edit"),
                pair("n", "add"),
                pair("/", "search"),
                pair("F", "filter"),
            ]
        );
    }

    #[test]
    fn a_chip_opened_over_the_search_leaves_it_with_its_text_kept() {
        let mut page = Page::new();
        page.search("rent");

        page.send(TransactionsPageEvent::OpenFilter(Some(ChipField::Payee)));
        page.send(TransactionsPageEvent::Filter(FilterEvent::Cancel));

        let view = page.view();
        assert!(!view.searching, "the popover took the Search's place");
        assert_eq!(view.search, "rent");
        assert_eq!(page.ids(), [RENT]);
    }

    #[test]
    fn entering_the_page_again_closes_a_popover_left_open() {
        let mut page = Page::new();
        page.send(TransactionsPageEvent::OpenFilter(None));
        page.send(TransactionsPageEvent::Filter(FilterEvent::Focus(
            FilterField::Payee,
        )));
        page.send(TransactionsPageEvent::Filter(FilterEvent::Type('x')));

        page.send(TransactionsPageEvent::Entered);

        assert!(page.view().filter.is_none());
        assert!(!page.view().filters_changed, "its draft is discarded");
    }

    #[test]
    fn a_new_date_style_reaches_the_open_popover() {
        let mut page = Page::new();
        page.send(TransactionsPageEvent::OpenFilter(Some(ChipField::Date)));

        page.send(TransactionsPageEvent::SetDateStyle(Some(DateStyle::Iso)));

        let view = page.view();
        let filter = view.filter.expect("the popover stays open");
        assert_eq!(filter.from.text, "2026-01-01");
        assert_eq!(view.rows[0].date, "2026-09-18");
    }

    #[test]
    fn the_legend_is_read_without_the_rows() {
        let page = Page::new();

        assert_eq!(page.presenter.hints(), page.view().hints);
    }
}
