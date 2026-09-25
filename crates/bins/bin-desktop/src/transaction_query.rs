//! The pure engine behind the Transactions table (`docs/ux/desktop/Transactions/README.md`):
//! applies a [`TransactionFilters`] value and a search string to the stub data and returns the
//! visible rows newest-first with each row's amount, its running total and the footer figure.
//! `gpui`-free and side-effect free, so every rule is unit-tested without a window.
//!
//! The semantics are the ones decided on the Desktop Transactions map:
//!
//! - **Transaction-level** filters: account, status, and the date range (inclusive; an empty side
//!   is unbounded). **Split-level** filters: category (a parent matches its descendants), payee and
//!   tag (case-insensitive substrings of the Split's Payee current name and its Tag names).
//! - A Transaction matches a Split-level filter when **any** Split matches, and with several
//!   Split-level filters active **one Split must satisfy all of them**: a Split is one line.
//! - While any Split-level filter is active a row's amount is the **sum of its matching Splits**
//!   (a groceries filter yields a groceries total, not a total of whole receipts); otherwise it is
//!   the Transaction total. A row where some Split was left out is [`VisibleRow::partial`].
//! - **Search** is Transaction-level: a case-insensitive substring of the description or of any
//!   Split's Payee name. It combines with the filters by AND and never changes an amount.
//! - The running total accumulates down the visible rows (newest first) and, under the no-cross-Unit
//!   rule, exists only when every visible row shares one Unit; otherwise it is blank and the total
//!   is [`Total::Mixed`].
//! - The default filter set is **this year**: 1 January to `today`, with everything else empty.
//!   `clear filters` and the popover's `reset` both mean "the defaults".

use std::collections::HashMap;

use bigdecimal::BigDecimal;
use chrono::{Datelike, NaiveDate};
use lib_core::{Money, TransactionStatus};
use lib_locale::Label;

use crate::{
    accounts::Account,
    categories::{self, Category},
    payees::Payee,
    tags::Tag,
    transactions::{Split, Transaction},
};

/// The Status filter: all, or exactly one of the three Transaction Statuses. (Flagged is not a
/// status and has no filter here; see the map's fog.)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StatusFilter {
    #[default]
    All,
    Open,
    Cleared,
    Reconciled,
}

impl StatusFilter {
    pub const ALL: [StatusFilter; 4] = [Self::All, Self::Open, Self::Cleared, Self::Reconciled];

    /// The word for this filter in the Locale in effect: the shared Status label, or "All".
    pub fn label(self) -> String {
        match self {
            Self::All => crate::msg::desktop_transactions_status_all(),
            Self::Open => TransactionStatus::Open.label(),
            Self::Cleared => TransactionStatus::Cleared.label(),
            Self::Reconciled => TransactionStatus::Reconciled.label(),
        }
    }

    pub fn matches(self, status: &TransactionStatus) -> bool {
        match self {
            Self::All => true,
            Self::Open => *status == TransactionStatus::Open,
            Self::Cleared => *status == TransactionStatus::Cleared,
            Self::Reconciled => *status == TransactionStatus::Reconciled,
        }
    }
}

/// Every filter dimension the chip row and the popover edit. Text fields are matched as typed
/// (case-insensitively, trimmed); an empty one means "any".
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransactionFilters {
    /// An [`Account::id`].
    pub account: Option<u32>,
    /// A [`Category::id`]; a parent matches its descendants.
    pub category: Option<u32>,
    pub payee: String,
    pub tag: String,
    /// Inclusive lower bound; `None` is unbounded.
    pub from: Option<NaiveDate>,
    /// Inclusive upper bound; `None` is unbounded.
    pub to: Option<NaiveDate>,
    pub status: StatusFilter,
}

/// 1 January of `today`'s year through `today` -- the `this year` range.
pub fn this_year(today: NaiveDate) -> (NaiveDate, NaiveDate) {
    let start = NaiveDate::from_ymd_opt(today.year(), 1, 1).unwrap_or(today);
    (start, today)
}

impl TransactionFilters {
    /// The defaults: this year, everything else empty.
    pub fn defaults(today: NaiveDate) -> Self {
        let (from, to) = this_year(today);
        Self {
            account: None,
            category: None,
            payee: String::new(),
            tag: String::new(),
            from: Some(from),
            to: Some(to),
            status: StatusFilter::All,
        }
    }

    /// The defaults narrowed to one account: what "open ledger" on the Accounts page hands over.
    pub fn for_account(today: NaiveDate, account: u32) -> Self {
        Self {
            account: Some(account),
            ..Self::defaults(today)
        }
    }

    /// Whether these are exactly the defaults (so no chip is an "active" accent chip).
    pub fn is_default(&self, today: NaiveDate) -> bool {
        *self == Self::defaults(today)
    }

    /// Whether any Split-level filter (category, payee, tag) is set -- what switches a row's amount
    /// to the sum of its matching Splits.
    pub fn has_split_level(&self) -> bool {
        self.category.is_some() || !self.payee.trim().is_empty() || !self.tag.trim().is_empty()
    }
}

/// The reference data the engine reads. Borrowed, so a caller builds it from whatever `Shell` owns.
pub struct Ledger<'a> {
    pub accounts: &'a [Account],
    pub categories: &'a [Category],
    pub payees: &'a [Payee],
    pub tags: &'a [Tag],
}

/// One row of the table.
#[derive(Debug, Clone, PartialEq)]
pub struct VisibleRow<'a> {
    pub transaction: &'a Transaction,
    /// The Transaction total, or -- while a Split-level filter is active -- the sum of the Splits
    /// that matched it.
    pub amount: Money,
    /// Some of the Transaction's Splits were left out of `amount`.
    pub partial: bool,
    /// The running total down to and including this row; `None` whenever the total is
    /// [`Total::Mixed`] (or empty).
    pub running: Option<Money>,
}

/// The footer's grand total under the no-cross-Unit rule.
#[derive(Debug, Clone, PartialEq)]
pub enum Total {
    /// No rows are visible.
    Empty,
    /// Every visible row is in one Unit: the sum of the rows' amounts, in that Unit's code.
    Single { unit: String, amount: Money },
    /// The visible rows span more than one Unit, so no sum exists.
    Mixed,
}

/// The result of a query.
#[derive(Debug, Clone, PartialEq)]
pub struct Visible<'a> {
    pub rows: Vec<VisibleRow<'a>>,
    pub total: Total,
    /// How many Transactions exist in all -- the `M` of the footer's `N of M shown` (`N` is
    /// `rows.len()`).
    pub ledger_size: usize,
}

fn zero() -> Money {
    Money(BigDecimal::new(0.into(), 2))
}

fn add(a: &Money, b: &Money) -> Money {
    Money(a.0.clone() + b.0.clone())
}

/// Case-folded lookups built once per query, so matching a Split is a map read.
struct Names {
    payees: HashMap<u32, String>,
    tags: HashMap<u32, String>,
}

impl Names {
    fn new(ledger: &Ledger<'_>) -> Self {
        Self {
            payees: ledger
                .payees
                .iter()
                .map(|payee| (payee.id, payee.name.to_lowercase()))
                .collect(),
            tags: ledger
                .tags
                .iter()
                .map(|tag| (tag.id, tag.name.to_lowercase()))
                .collect(),
        }
    }
}

/// The active Split-level conditions, ready to test a Split against.
struct SplitFilter {
    /// The chosen category and everything nested under it.
    categories: Option<Vec<u32>>,
    payee: String,
    tag: String,
}

impl SplitFilter {
    fn new(ledger: &Ledger<'_>, filters: &TransactionFilters) -> Self {
        Self {
            categories: filters
                .category
                .map(|id| categories::descendants_inclusive(ledger.categories, id)),
            payee: filters.payee.trim().to_lowercase(),
            tag: filters.tag.trim().to_lowercase(),
        }
    }

    fn is_active(&self) -> bool {
        self.categories.is_some() || !self.payee.is_empty() || !self.tag.is_empty()
    }

    /// Whether this one Split satisfies **every** active condition.
    fn matches(&self, names: &Names, split: &Split) -> bool {
        if let Some(ids) = &self.categories
            && !ids.contains(&split.category_id)
        {
            return false;
        }
        if !self.payee.is_empty() {
            let payee_matches = split
                .payee_id
                .and_then(|id| names.payees.get(&id))
                .is_some_and(|name| name.contains(&self.payee));
            if !payee_matches {
                return false;
            }
        }
        if !self.tag.is_empty() {
            let tag_matches = split
                .tag_ids
                .iter()
                .filter_map(|id| names.tags.get(id))
                .any(|name| name.contains(&self.tag));
            if !tag_matches {
                return false;
            }
        }
        true
    }
}

fn transaction_level_matches(transaction: &Transaction, filters: &TransactionFilters) -> bool {
    if filters
        .account
        .is_some_and(|id| transaction.account_id != id)
    {
        return false;
    }
    if !filters.status.matches(&transaction.status) {
        return false;
    }
    if filters.from.is_some_and(|from| transaction.date < from) {
        return false;
    }
    if filters.to.is_some_and(|to| transaction.date > to) {
        return false;
    }
    true
}

fn search_matches(names: &Names, transaction: &Transaction, needle: &str) -> bool {
    if needle.is_empty() {
        return true;
    }
    let in_description = transaction
        .description
        .as_deref()
        .is_some_and(|text| text.to_lowercase().contains(needle));
    in_description
        || transaction.splits.iter().any(|split| {
            split
                .payee_id
                .and_then(|id| names.payees.get(&id))
                .is_some_and(|name| name.contains(needle))
        })
}

/// Applies `filters` and `search` to `transactions` (which are already newest first, and stay in
/// that order), returning the visible rows with amounts, the running total and the footer total.
pub fn query<'a>(
    ledger: &Ledger<'_>,
    transactions: &'a [Transaction],
    filters: &TransactionFilters,
    search: &str,
) -> Visible<'a> {
    let names = Names::new(ledger);
    let split_filter = SplitFilter::new(ledger, filters);
    let needle = search.trim().to_lowercase();

    let mut rows: Vec<VisibleRow<'a>> = Vec::new();
    for transaction in transactions {
        if !transaction_level_matches(transaction, filters)
            || !search_matches(&names, transaction, &needle)
        {
            continue;
        }
        let (amount, partial) = if split_filter.is_active() {
            let matching: Vec<&Split> = transaction
                .splits
                .iter()
                .filter(|split| split_filter.matches(&names, split))
                .collect();
            if matching.is_empty() {
                continue;
            }
            let amount = matching
                .iter()
                .fold(zero(), |sum, split| add(&sum, &split.amount));
            (amount, matching.len() < transaction.splits.len())
        } else {
            (transaction.total(), false)
        };
        rows.push(VisibleRow {
            transaction,
            amount,
            partial,
            running: None,
        });
    }

    let unit_of = |account_id: u32| -> &str {
        ledger
            .accounts
            .iter()
            .find(|account| account.id == account_id)
            .map(|account| account.unit.as_str())
            .unwrap_or("")
    };
    let first_unit = rows.first().map(|row| unit_of(row.transaction.account_id));
    let single_unit = first_unit
        .filter(|unit| {
            rows.iter()
                .all(|row| unit_of(row.transaction.account_id) == *unit)
        })
        .map(str::to_string);

    let total = match (&single_unit, rows.is_empty()) {
        (_, true) => Total::Empty,
        (Some(unit), false) => {
            let mut running = zero();
            for row in &mut rows {
                running = add(&running, &row.amount);
                row.running = Some(running.clone());
            }
            Total::Single {
                unit: unit.clone(),
                amount: running,
            }
        }
        (None, false) => Total::Mixed,
    };

    Visible {
        rows,
        total,
        ledger_size: transactions.len(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        accounts::default_accounts,
        categories::{self, default_categories},
        payees::{self, default_payees},
        tags::{self, default_tags},
        transactions::default_transactions,
    };
    use chrono::Duration;

    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 9, 19).unwrap()
    }

    struct World {
        accounts: Vec<Account>,
        categories: Vec<Category>,
        payees: Vec<Payee>,
        tags: Vec<Tag>,
        transactions: Vec<Transaction>,
    }

    impl World {
        fn seeded() -> Self {
            let accounts = default_accounts();
            let categories = default_categories();
            let payees = default_payees();
            let tags = default_tags();
            let transactions =
                default_transactions(&accounts, &categories, &payees, &tags, today());
            Self {
                accounts,
                categories,
                payees,
                tags,
                transactions,
            }
        }

        fn ledger(&self) -> Ledger<'_> {
            Ledger {
                accounts: &self.accounts,
                categories: &self.categories,
                payees: &self.payees,
                tags: &self.tags,
            }
        }

        fn run(&self, filters: &TransactionFilters, search: &str) -> Visible<'_> {
            query(&self.ledger(), &self.transactions, filters, search)
        }

        /// Filters with no bounds and nothing set, so a test isolates one dimension.
        fn open(&self) -> TransactionFilters {
            TransactionFilters {
                from: None,
                to: None,
                ..TransactionFilters::defaults(today())
            }
        }

        fn account(&self, name: &str) -> u32 {
            self.accounts.iter().find(|a| a.name == name).unwrap().id
        }

        fn category(&self, name: &str) -> u32 {
            categories::find_by_name(&self.categories, name).unwrap()
        }
    }

    fn money(text: &str) -> Money {
        text.parse().unwrap()
    }

    #[test]
    fn the_defaults_are_this_year_with_everything_else_empty() {
        let filters = TransactionFilters::defaults(today());
        assert_eq!(filters.from, NaiveDate::from_ymd_opt(2026, 1, 1));
        assert_eq!(filters.to, Some(today()));
        assert_eq!(filters.account, None);
        assert_eq!(filters.category, None);
        assert!(filters.payee.is_empty() && filters.tag.is_empty());
        assert_eq!(filters.status, StatusFilter::All);
        assert!(filters.is_default(today()));
        assert!(!filters.has_split_level());
    }

    #[test]
    fn this_year_runs_from_the_first_of_january_to_today() {
        assert_eq!(
            this_year(today()),
            (NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(), today())
        );
        let new_years_day = NaiveDate::from_ymd_opt(2027, 1, 1).unwrap();
        assert_eq!(this_year(new_years_day), (new_years_day, new_years_day));
    }

    #[test]
    fn changing_any_default_makes_the_filters_non_default() {
        let mut filters = TransactionFilters::defaults(today());
        filters.status = StatusFilter::Open;
        assert!(!filters.is_default(today()));
        let mut filters = TransactionFilters::defaults(today());
        filters.from = None;
        assert!(!filters.is_default(today()), "all dates is not the default");
        assert!(!TransactionFilters::defaults(today()).is_default(today() + Duration::days(1)));
    }

    #[test]
    fn the_default_view_shows_exactly_this_years_transactions_newest_first() {
        let world = World::seeded();
        let visible = world.run(&TransactionFilters::defaults(today()), "");
        let expected = world
            .transactions
            .iter()
            .filter(|t| t.date >= NaiveDate::from_ymd_opt(2026, 1, 1).unwrap())
            .count();
        assert_eq!(visible.rows.len(), expected);
        assert!(visible.rows.len() < world.transactions.len());
        assert_eq!(visible.ledger_size, 700);
        for pair in visible.rows.windows(2) {
            assert!(pair[0].transaction.date >= pair[1].transaction.date);
        }
    }

    #[test]
    fn no_bounds_shows_everything() {
        let world = World::seeded();
        assert_eq!(world.run(&world.open(), "").rows.len(), 700);
    }

    #[test]
    fn the_date_range_is_inclusive_and_an_empty_side_is_unbounded() {
        let world = World::seeded();
        let day = today() - Duration::days(9);
        let mut filters = world.open();
        filters.from = Some(day);
        filters.to = Some(day);
        let on_the_day = world.run(&filters, "");
        assert!(!on_the_day.rows.is_empty());
        assert!(on_the_day.rows.iter().all(|r| r.transaction.date == day));

        let mut only_from = world.open();
        only_from.from = Some(day);
        assert!(
            world
                .run(&only_from, "")
                .rows
                .iter()
                .all(|r| r.transaction.date >= day)
        );
        let mut only_to = world.open();
        only_to.to = Some(day);
        assert!(
            world
                .run(&only_to, "")
                .rows
                .iter()
                .all(|r| r.transaction.date <= day)
        );
    }

    #[test]
    fn a_range_that_ends_before_it_starts_matches_nothing() {
        let world = World::seeded();
        let mut filters = world.open();
        filters.from = Some(today());
        filters.to = Some(today() - Duration::days(30));
        let visible = world.run(&filters, "");
        assert!(visible.rows.is_empty());
        assert_eq!(visible.total, Total::Empty);
    }

    #[test]
    fn the_account_filter_keeps_only_that_account() {
        let world = World::seeded();
        let amex = world.account("Amex Platinum");
        let mut filters = world.open();
        filters.account = Some(amex);
        let visible = world.run(&filters, "");
        assert_eq!(visible.rows.len(), 204);
        assert!(
            visible
                .rows
                .iter()
                .all(|r| r.transaction.account_id == amex)
        );
    }

    #[test]
    fn the_status_filter_keeps_only_that_status() {
        let world = World::seeded();
        for (filter, status) in [
            (StatusFilter::Open, TransactionStatus::Open),
            (StatusFilter::Cleared, TransactionStatus::Cleared),
            (StatusFilter::Reconciled, TransactionStatus::Reconciled),
        ] {
            let mut filters = world.open();
            filters.status = filter;
            let visible = world.run(&filters, "");
            assert!(!visible.rows.is_empty());
            assert!(visible.rows.iter().all(|r| r.transaction.status == status));
        }
    }

    #[test]
    fn a_parent_category_matches_its_descendants_and_a_leaf_only_itself() {
        let world = World::seeded();
        let (food, groceries, dining, household) = (
            world.category("Food"),
            world.category("Groceries"),
            world.category("Dining"),
            world.category("Household"),
        );
        let mut filters = world.open();
        filters.category = Some(food);
        let food_rows = world.run(&filters, "");
        assert!(!food_rows.rows.is_empty());
        for row in &food_rows.rows {
            assert!(
                row.transaction
                    .splits
                    .iter()
                    .any(|s| s.category_id == groceries || s.category_id == dining)
            );
        }

        filters.category = Some(groceries);
        let grocery_rows = world.run(&filters, "");
        assert!(grocery_rows.rows.len() < food_rows.rows.len());
        for row in &grocery_rows.rows {
            assert!(
                row.transaction
                    .splits
                    .iter()
                    .any(|s| s.category_id == groceries)
            );
        }

        // A Transaction with only Household Splits is in neither.
        assert!(!grocery_rows.rows.iter().any(|r| {
            r.transaction
                .splits
                .iter()
                .all(|s| s.category_id == household)
        }));
    }

    #[test]
    fn payee_and_tag_filters_are_case_insensitive_substrings_of_names() {
        let world = World::seeded();
        let mut filters = world.open();
        filters.payee = "  WOOL ".to_string();
        let woolworths = payees::find_by_name(&world.payees, "Woolworths").unwrap();
        let visible = world.run(&filters, "");
        assert!(!visible.rows.is_empty());
        for row in &visible.rows {
            assert!(
                row.transaction
                    .splits
                    .iter()
                    .any(|s| s.payee_id == Some(woolworths))
            );
        }

        let mut filters = world.open();
        filters.tag = "JAPAN".to_string();
        let japan = tags::find_by_name(&world.tags, "Japan Trip 2026").unwrap();
        let visible = world.run(&filters, "");
        assert!(!visible.rows.is_empty());
        for row in &visible.rows {
            assert!(
                row.transaction
                    .splits
                    .iter()
                    .any(|s| s.tag_ids.contains(&japan))
            );
        }
    }

    #[test]
    fn a_payee_filter_matches_current_names_not_aliases() {
        let world = World::seeded();
        let mut filters = world.open();
        filters.payee = "woolies".to_string();
        assert!(world.run(&filters, "").rows.is_empty());
    }

    #[test]
    fn several_split_level_filters_must_be_satisfied_by_the_same_split() {
        let world = World::seeded();
        // The Tokyo receipt: a tagged Dining Split and an untagged Household Split.
        let mut filters = world.open();
        filters.tag = "japan".to_string();
        filters.category = Some(world.category("Dining"));
        let dining = world.run(&filters, "");
        assert!(
            dining
                .rows
                .iter()
                .any(|r| r.transaction.description.as_deref() == Some("Tokyo dinner and souvenirs")),
            "the tagged Split is a Dining Split"
        );

        filters.category = Some(world.category("Household"));
        let household = world.run(&filters, "");
        assert!(
            !household
                .rows
                .iter()
                .any(|r| r.transaction.description.as_deref() == Some("Tokyo dinner and souvenirs")),
            "the Household Split is untagged: the tag and the category sit on different Splits"
        );
    }

    fn receipt<'a>(visible: &'a Visible<'a>, description: &str) -> &'a VisibleRow<'a> {
        visible
            .rows
            .iter()
            .find(|r| r.transaction.description.as_deref() == Some(description))
            .unwrap_or_else(|| panic!("{description:?} is not visible"))
    }

    #[test]
    fn a_split_level_filter_shows_the_matching_splits_sum_and_marks_the_row_partial() {
        let world = World::seeded();
        let mut filters = world.open();
        filters.category = Some(world.category("Groceries"));
        let visible = world.run(&filters, "");
        let row = receipt(&visible, "Weekly shop and cleaning supplies");
        assert_eq!(row.amount, money("-180.00"));
        assert!(row.partial);
        assert_eq!(row.transaction.total(), money("-214.30"));
    }

    #[test]
    fn without_a_split_level_filter_the_amount_is_the_transaction_total() {
        let world = World::seeded();
        let visible = world.run(&world.open(), "");
        let row = receipt(&visible, "Weekly shop and cleaning supplies");
        assert_eq!(row.amount, money("-214.30"));
        assert!(!row.partial);
    }

    #[test]
    fn transaction_level_filters_never_make_a_row_partial() {
        let world = World::seeded();
        let mut filters = world.open();
        filters.status = StatusFilter::Cleared;
        filters.account = Some(world.account("ANZ Everyday"));
        let visible = world.run(&filters, "");
        let row = receipt(&visible, "Weekly shop and cleaning supplies");
        assert_eq!(row.amount, money("-214.30"));
        assert!(!row.partial);
    }

    #[test]
    fn a_filter_that_matches_every_split_is_not_partial() {
        let world = World::seeded();
        let mut filters = world.open();
        filters.payee = "woolworths".to_string();
        let visible = world.run(&filters, "");
        let row = receipt(&visible, "Weekly shop and cleaning supplies");
        assert_eq!(row.amount, money("-214.30"));
        assert!(!row.partial, "both Splits are Woolworths");
    }

    #[test]
    fn search_matches_the_description_or_any_splits_payee_case_insensitively() {
        let world = World::seeded();
        let by_memo = world.run(&world.open(), "tokyo DINNER");
        assert!(by_memo
            .rows
            .iter()
            .any(|r| r.transaction.description.as_deref() == Some("Tokyo dinner and souvenirs")));
        // Kmart is the payee of the *second* Split of the airport receipt.
        let by_payee = world.run(&world.open(), "kmart");
        assert!(
            by_payee
                .rows
                .iter()
                .any(|r| r.transaction.description.as_deref() == Some("Airport"))
        );
        assert!(
            world
                .run(&world.open(), "zzzz-no-such-thing")
                .rows
                .is_empty()
        );
    }

    #[test]
    fn search_combines_by_and_and_never_changes_an_amount() {
        let world = World::seeded();
        let mut filters = world.open();
        filters.account = Some(world.account("Amex Platinum"));
        let visible = world.run(&filters, "kmart");
        assert!(!visible.rows.is_empty());
        assert!(
            visible
                .rows
                .iter()
                .all(|r| r.transaction.account_id == world.account("Amex Platinum"))
        );
        let row = receipt(&visible, "Airport");
        assert_eq!(row.amount, row.transaction.total());
        assert!(!row.partial);
    }

    #[test]
    fn search_is_not_bound_by_the_same_split_rule() {
        let world = World::seeded();
        let mut filters = world.open();
        filters.category = Some(world.category("Dining"));
        // "kmart" is the Household Split's payee, the Dining Split is Hudson News: search reads the
        // whole Transaction while the category filter picks the Dining Split's amount.
        let visible = world.run(&filters, "kmart");
        let row = receipt(&visible, "Airport");
        assert_eq!(row.amount, money("-18.50"));
        assert!(row.partial);
    }

    fn one_account_world(units: [&str; 2]) -> World {
        let mut world = World::seeded();
        for account in &mut world.accounts {
            match account.name.as_str() {
                "ANZ Everyday" => account.unit = units[0].to_string(),
                "Amex Platinum" => account.unit = units[1].to_string(),
                _ => {}
            }
        }
        world
    }

    #[test]
    fn the_running_total_accumulates_down_the_rows_and_the_total_is_the_last_figure() {
        let world = World::seeded();
        let mut filters = world.open();
        filters.account = Some(world.account("ANZ Everyday"));
        let visible = world.run(&filters, "");
        let mut expected = money("0.00");
        for row in &visible.rows {
            expected = Money(expected.0 + row.amount.0.clone());
            assert_eq!(row.running.as_ref(), Some(&expected));
        }
        match &visible.total {
            Total::Single { unit, amount } => {
                assert_eq!(unit, "aud");
                assert_eq!(Some(amount), visible.rows.last().unwrap().running.as_ref());
            }
            other => panic!("expected a single-Unit total, got {other:?}"),
        }
    }

    #[test]
    fn the_running_total_under_a_split_filter_sums_the_matching_splits() {
        let world = World::seeded();
        let mut filters = world.open();
        filters.category = Some(world.category("Groceries"));
        let visible = world.run(&filters, "");
        let summed = visible
            .rows
            .iter()
            .fold(money("0.00"), |sum, r| Money(sum.0 + r.amount.0.clone()));
        match visible.total {
            Total::Single { amount, .. } => assert_eq!(amount, summed),
            other => panic!("expected a single-Unit total, got {other:?}"),
        }
    }

    #[test]
    fn rows_in_two_units_have_no_running_total_and_the_total_is_mixed() {
        let world = one_account_world(["aud", "usd"]);
        let mut filters = world.open();
        filters.account = None;
        let visible = world.run(&filters, "");
        assert_eq!(visible.total, Total::Mixed);
        assert!(visible.rows.iter().all(|r| r.running.is_none()));
        assert!(!visible.rows.is_empty(), "the rows still show");
    }

    #[test]
    fn narrowing_to_one_unit_brings_the_running_total_back() {
        let world = one_account_world(["aud", "usd"]);
        let mut filters = world.open();
        filters.account = Some(world.account("Amex Platinum"));
        let visible = world.run(&filters, "");
        match visible.total {
            Total::Single { unit, .. } => assert_eq!(unit, "usd"),
            other => panic!("expected a single-Unit total, got {other:?}"),
        }
        assert!(visible.rows.iter().all(|r| r.running.is_some()));
    }

    #[test]
    fn two_accounts_sharing_a_unit_still_get_one_running_total() {
        let world = one_account_world(["aud", "aud"]);
        let visible = world.run(&world.open(), "");
        assert!(matches!(visible.total, Total::Single { .. }));
    }

    #[test]
    fn no_visible_rows_means_an_empty_total_and_no_running_figures() {
        let world = World::seeded();
        let visible = world.run(&world.open(), "zzzz-no-such-thing");
        assert!(visible.rows.is_empty());
        assert_eq!(visible.total, Total::Empty);
        assert_eq!(visible.ledger_size, 700);
    }

    #[test]
    fn a_split_filter_that_matches_no_split_drops_the_transaction() {
        let world = World::seeded();
        let mut filters = world.open();
        filters.category = Some(world.category("Salary"));
        filters.tag = "japan".to_string();
        assert!(world.run(&filters, "").rows.is_empty());
    }

    #[test]
    fn the_status_filter_labels_and_matching() {
        crate::locale::init_for_tests();
        assert_eq!(
            StatusFilter::ALL.map(StatusFilter::label),
            ["All", "Open", "Cleared", "Reconciled"]
        );
        assert!(StatusFilter::All.matches(&TransactionStatus::Open));
        assert!(StatusFilter::Cleared.matches(&TransactionStatus::Cleared));
        assert!(!StatusFilter::Cleared.matches(&TransactionStatus::Reconciled));
    }

    #[test]
    fn the_engine_reads_the_data_it_is_given_and_leaves_it_alone() {
        let world = World::seeded();
        let before = world.transactions.clone();
        let _ = world.run(&TransactionFilters::defaults(today()), "shop");
        assert_eq!(world.transactions, before);
    }
}
