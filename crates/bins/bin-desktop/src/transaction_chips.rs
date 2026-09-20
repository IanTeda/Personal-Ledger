//! The text and state behind the Transactions view's header and footer
//! (`docs/ux/desktop/Transactions/README.md`'s 4a): the filter chips, the count line, and the footer
//! bar's figures. `gpui`-free and unit-tested; the view only paints what it returns.
//!
//! Chip behaviour is the Desktop Transactions map's decision. There are six chips, one per filter
//! dimension. A chip at its **default** is an outline chip (`account: all accounts ▾`); one changed
//! from the default is an **accent** chip with a `✕` that resets just that filter. The default
//! this-year range is itself an outline chip (`this year`); a custom range reads as itself
//! (`12 aug – 12 sep`), and no bounds reads `all dates`. A chip click, its `▾`, or `f` opens the
//! filter popover (not built yet); `✕` and `clear filters` act directly on the one filter state.

use std::collections::HashSet;

use chrono::NaiveDate;

use crate::{
    categories, format,
    settings::{DateFormat, DecimalSeparator},
    transaction_query::{Ledger, Total, TransactionFilters, Visible, this_year},
    transactions::Transaction,
};

/// A filter dimension, in the order the chips appear.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FilterField {
    Account,
    Category,
    Payee,
    Tag,
    Date,
    Status,
}

impl FilterField {
    pub const ALL: [FilterField; 6] = [
        Self::Account,
        Self::Category,
        Self::Payee,
        Self::Tag,
        Self::Date,
        Self::Status,
    ];
}

/// One chip in the row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chip {
    pub field: FilterField,
    /// The text without its trailing `▾` or `✕`, which the view adds.
    pub label: String,
    /// Changed from the default: drawn as an accent chip with a `✕`.
    pub active: bool,
}

/// `12 aug – 12 sep`, `from 12 aug`, `until 12 sep`, `all dates`, or `this year` for the default
/// range. A range ending today reads `… – today`.
pub fn date_label(
    from: Option<NaiveDate>,
    to: Option<NaiveDate>,
    today: NaiveDate,
    date_format: DateFormat,
) -> String {
    let (default_from, default_to) = this_year(today);
    if from == Some(default_from) && to == Some(default_to) {
        return "this year".to_string();
    }
    let show = |date: NaiveDate| format::date_compact(date, date_format, today);
    let show_end = |date: NaiveDate| {
        if date == today {
            "today".to_string()
        } else {
            show(date)
        }
    };
    match (from, to) {
        (None, None) => "all dates".to_string(),
        (Some(from), None) => format!("from {}", show(from)),
        (None, Some(to)) => format!("until {}", show_end(to)),
        (Some(from), Some(to)) => format!("{} \u{2013} {}", show(from), show_end(to)),
    }
}

/// The six chips for `filters`, in order.
pub fn chips(
    filters: &TransactionFilters,
    ledger: &Ledger<'_>,
    today: NaiveDate,
    date_format: DateFormat,
) -> Vec<Chip> {
    let defaults = TransactionFilters::defaults(today);
    let account = filters.account.map(|id| {
        ledger
            .accounts
            .iter()
            .find(|account| account.id == id)
            .map(|account| account.name.clone())
            .unwrap_or_else(|| "unknown".to_string())
    });
    let category = filters
        .category
        .map(|id| categories::path(ledger.categories, id).unwrap_or_else(|| "unknown".to_string()));
    let payee = filters.payee.trim();
    let tag = filters.tag.trim();
    let date_active = filters.from != defaults.from || filters.to != defaults.to;

    let make = |field, label: String, active| Chip {
        field,
        label,
        active,
    };
    vec![
        make(
            FilterField::Account,
            format!("account: {}", account.as_deref().unwrap_or("all accounts")),
            account.is_some(),
        ),
        make(
            FilterField::Category,
            format!("category: {}", category.as_deref().unwrap_or("any")),
            category.is_some(),
        ),
        make(
            FilterField::Payee,
            format!("payee: {}", if payee.is_empty() { "any" } else { payee }),
            !payee.is_empty(),
        ),
        make(
            FilterField::Tag,
            format!("tag: {}", if tag.is_empty() { "any" } else { tag }),
            !tag.is_empty(),
        ),
        make(
            FilterField::Date,
            date_label(filters.from, filters.to, today, date_format),
            date_active,
        ),
        make(
            FilterField::Status,
            format!("status: {}", filters.status.label()),
            filters.status != defaults.status,
        ),
    ]
}

/// Resets one filter to its default (`✕` on an accent chip). The date range returns to this year,
/// not to "all dates": that is the default, and `all dates` is the non-default state.
pub fn clear_field(filters: &mut TransactionFilters, field: FilterField, today: NaiveDate) {
    let defaults = TransactionFilters::defaults(today);
    match field {
        FilterField::Account => filters.account = defaults.account,
        FilterField::Category => filters.category = defaults.category,
        FilterField::Payee => filters.payee = defaults.payee,
        FilterField::Tag => filters.tag = defaults.tag,
        FilterField::Date => {
            filters.from = defaults.from;
            filters.to = defaults.to;
        }
        FilterField::Status => filters.status = defaults.status,
    }
}

/// `700 transactions across 4 accounts`: the whole ledger, and how many accounts hold any of it.
pub fn count_line(transactions: &[Transaction]) -> String {
    let accounts: HashSet<u32> = transactions.iter().map(|t| t.account_id).collect();
    let noun = |count: usize, one: &str, many: &str| {
        if count == 1 {
            format!("{count} {one}")
        } else {
            format!("{count} {many}")
        }
    };
    format!(
        "{} across {}",
        noun(transactions.len(), "transaction", "transactions"),
        noun(accounts.len(), "account", "accounts")
    )
}

/// The footer's grand total, formatted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FooterTotal {
    /// No rows are visible.
    Empty,
    /// One Unit: the figure (`negative` selects the negative token) and the Unit's code.
    Single {
        unit: String,
        negative: bool,
        text: String,
    },
    /// The visible rows span more than one Unit, so there is no sum.
    Mixed,
}

/// The footer bar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Footer {
    /// Transactions visible.
    pub shown: usize,
    /// Transactions in all.
    pub of: usize,
    /// The chosen category's own name, lowercased, when a Category filter is set (the mockup's
    /// `8 of 96 groceries transactions shown`).
    pub category: Option<String>,
    pub total: FooterTotal,
}

impl Footer {
    /// `8 of 96 groceries transactions shown`.
    pub fn label(&self) -> String {
        let category = self
            .category
            .as_ref()
            .map(|name| format!("{name} "))
            .unwrap_or_default();
        format!("{} of {} {category}transactions shown", self.shown, self.of)
    }
}

/// The footer for a query result.
pub fn footer(
    visible: &Visible<'_>,
    filters: &TransactionFilters,
    ledger: &Ledger<'_>,
    separator: DecimalSeparator,
) -> Footer {
    let category = filters.category.and_then(|id| {
        ledger
            .categories
            .iter()
            .find(|category| category.id == id)
            .map(|category| category.name.to_lowercase())
    });
    let total = match &visible.total {
        Total::Empty => FooterTotal::Empty,
        Total::Mixed => FooterTotal::Mixed,
        Total::Single { unit, amount } => {
            let (negative, text) = format::amount(amount, separator);
            FooterTotal::Single {
                unit: unit.clone(),
                negative,
                text,
            }
        }
    };
    Footer {
        shown: visible.rows.len(),
        of: visible.ledger_size,
        category,
        total,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        accounts::default_accounts,
        categories::default_categories,
        payees::default_payees,
        tags::default_tags,
        transaction_query::{StatusFilter, query},
        transactions::default_transactions,
    };
    use chrono::Duration;

    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 9, 19).unwrap()
    }

    struct World {
        accounts: Vec<crate::accounts::Account>,
        categories: Vec<categories::Category>,
        payees: Vec<crate::payees::Payee>,
        tags: Vec<crate::tags::Tag>,
        transactions: Vec<Transaction>,
    }

    fn world() -> World {
        let accounts = default_accounts();
        let categories = default_categories();
        let payees = default_payees();
        let tags = default_tags();
        let transactions = default_transactions(&accounts, &categories, &payees, &tags, today());
        World {
            accounts,
            categories,
            payees,
            tags,
            transactions,
        }
    }

    impl World {
        fn ledger(&self) -> Ledger<'_> {
            Ledger {
                accounts: &self.accounts,
                categories: &self.categories,
                payees: &self.payees,
                tags: &self.tags,
            }
        }

        fn chips(&self, filters: &TransactionFilters) -> Vec<Chip> {
            chips(filters, &self.ledger(), today(), DateFormat::DayMonthYear)
        }
    }

    fn labels(chips: &[Chip]) -> Vec<&str> {
        chips.iter().map(|c| c.label.as_str()).collect()
    }

    #[test]
    fn an_account_ledger_hand_off_shows_only_the_account_chip_as_active() {
        let w = world();
        let account = &w.accounts[0];
        let filters = TransactionFilters::for_account(today(), account.id);
        let chips = w.chips(&filters);

        assert_eq!(chips[0].label, format!("account: {}", account.name));
        assert_eq!(
            chips.iter().map(|c| c.active).collect::<Vec<_>>(),
            [true, false, false, false, false, false]
        );
        assert_eq!(chips[4].label, "this year");
    }

    #[test]
    fn the_defaults_are_six_outline_chips_with_this_year_as_the_date_chip() {
        let w = world();
        let chips = w.chips(&TransactionFilters::defaults(today()));
        assert_eq!(
            labels(&chips),
            [
                "account: all accounts",
                "category: any",
                "payee: any",
                "tag: any",
                "this year",
                "status: all"
            ]
        );
        assert!(chips.iter().all(|c| !c.active));
        assert_eq!(
            chips.iter().map(|c| c.field).collect::<Vec<_>>(),
            FilterField::ALL
        );
    }

    #[test]
    fn a_changed_filter_is_an_accent_chip_that_reads_its_value() {
        let w = world();
        let groceries = categories::find_by_name(&w.categories, "Groceries").unwrap();
        let mut filters = TransactionFilters::defaults(today());
        filters.account = Some(w.accounts[1].id);
        filters.category = Some(groceries);
        filters.payee = "  wool ".to_string();
        filters.tag = "japan".to_string();
        filters.status = StatusFilter::Cleared;
        let chips = w.chips(&filters);
        assert_eq!(
            labels(&chips),
            [
                "account: ANZ Everyday",
                "category: Food \u{203a} Groceries",
                "payee: wool",
                "tag: japan",
                "this year",
                "status: cleared"
            ]
        );
        let active: Vec<bool> = chips.iter().map(|c| c.active).collect();
        assert_eq!(active, [true, true, true, true, false, true]);
    }

    #[test]
    fn the_date_chip_reads_this_year_a_range_or_all_dates() {
        let d = |m, day| NaiveDate::from_ymd_opt(2026, m, day).unwrap();
        let fmt = DateFormat::DayMonthYear;
        assert_eq!(
            date_label(Some(d(1, 1)), Some(today()), today(), fmt),
            "this year"
        );
        assert_eq!(date_label(None, None, today(), fmt), "all dates");
        assert_eq!(
            date_label(Some(d(8, 12)), Some(d(9, 12)), today(), fmt),
            "12 aug \u{2013} 12 sep"
        );
        assert_eq!(
            date_label(Some(d(8, 12)), Some(today()), today(), fmt),
            "12 aug \u{2013} today"
        );
        assert_eq!(
            date_label(Some(d(8, 12)), None, today(), fmt),
            "from 12 aug"
        );
        assert_eq!(
            date_label(None, Some(d(9, 12)), today(), fmt),
            "until 12 sep"
        );
    }

    #[test]
    fn a_custom_range_and_all_dates_are_accent_chips() {
        let w = world();
        let mut filters = TransactionFilters::defaults(today());
        filters.from = None;
        filters.to = None;
        let chips = w.chips(&filters);
        assert_eq!(chips[4].label, "all dates");
        assert!(chips[4].active);

        let mut filters = TransactionFilters::defaults(today());
        filters.from = Some(today() - Duration::days(30));
        assert!(w.chips(&filters)[4].active);
    }

    #[test]
    fn the_date_chip_follows_the_date_format_preference() {
        let from = NaiveDate::from_ymd_opt(2025, 1, 28).unwrap();
        let to = NaiveDate::from_ymd_opt(2026, 3, 2).unwrap();
        assert_eq!(
            date_label(Some(from), Some(to), today(), DateFormat::Iso),
            "2025-01-28 \u{2013} 2026-03-02"
        );
    }

    #[test]
    fn clearing_a_field_resets_just_that_filter_to_its_default() {
        let mut filters = TransactionFilters::defaults(today());
        filters.account = Some(3);
        filters.category = Some(7);
        filters.payee = "x".to_string();
        filters.tag = "y".to_string();
        filters.status = StatusFilter::Open;
        filters.from = None;

        clear_field(&mut filters, FilterField::Payee, today());
        assert!(filters.payee.is_empty());
        assert_eq!(filters.account, Some(3), "the others are untouched");
        clear_field(&mut filters, FilterField::Account, today());
        clear_field(&mut filters, FilterField::Category, today());
        clear_field(&mut filters, FilterField::Tag, today());
        clear_field(&mut filters, FilterField::Status, today());
        assert_eq!(filters.from, None, "the date is still as it was");
        clear_field(&mut filters, FilterField::Date, today());
        assert!(
            filters.is_default(today()),
            "all six are back to the defaults"
        );
    }

    #[test]
    fn clearing_the_date_returns_to_this_year_not_to_all_dates() {
        let mut filters = TransactionFilters::defaults(today());
        filters.from = None;
        filters.to = None;
        clear_field(&mut filters, FilterField::Date, today());
        assert_eq!(filters.from, NaiveDate::from_ymd_opt(2026, 1, 1));
        assert_eq!(filters.to, Some(today()));
    }

    #[test]
    fn the_count_line_counts_the_ledger_and_the_accounts_that_hold_it() {
        let w = world();
        assert_eq!(
            count_line(&w.transactions),
            "700 transactions across 4 accounts"
        );
        assert_eq!(
            count_line(&w.transactions[..1]),
            "1 transaction across 1 account"
        );
        assert_eq!(count_line(&[]), "0 transactions across 0 accounts");
    }

    #[test]
    fn the_footer_label_reads_n_of_m_and_names_a_category_filter() {
        let w = world();
        let filters = TransactionFilters::defaults(today());
        let visible = query(&w.ledger(), &w.transactions, &filters, "");
        let plain = footer(
            &visible,
            &filters,
            &w.ledger(),
            DecimalSeparator::CommaThousands,
        );
        assert_eq!(
            plain.label(),
            format!("{} of 700 transactions shown", visible.rows.len())
        );

        let groceries = categories::find_by_name(&w.categories, "Groceries").unwrap();
        let mut filters = TransactionFilters::defaults(today());
        filters.category = Some(groceries);
        let visible = query(&w.ledger(), &w.transactions, &filters, "");
        let named = footer(
            &visible,
            &filters,
            &w.ledger(),
            DecimalSeparator::CommaThousands,
        );
        assert_eq!(named.category.as_deref(), Some("groceries"));
        assert_eq!(
            named.label(),
            format!("{} of 700 groceries transactions shown", visible.rows.len())
        );
    }

    #[test]
    fn a_single_unit_footer_total_is_the_last_running_figure_in_the_chosen_separators() {
        let w = world();
        let filters = TransactionFilters::defaults(today());
        let visible = query(&w.ledger(), &w.transactions, &filters, "");
        let comma = footer(
            &visible,
            &filters,
            &w.ledger(),
            DecimalSeparator::CommaThousands,
        );
        let dot = footer(
            &visible,
            &filters,
            &w.ledger(),
            DecimalSeparator::DotThousands,
        );
        let (unit, text) = match (&comma.total, &dot.total) {
            (
                FooterTotal::Single { unit, text, .. },
                FooterTotal::Single { text: dot_text, .. },
            ) => {
                assert_ne!(text, dot_text, "the separators differ");
                (unit.clone(), text.clone())
            }
            other => panic!("expected single-Unit totals, got {other:?}"),
        };
        assert_eq!(unit, "aud");
        let last = visible.rows.last().unwrap().running.as_ref().unwrap();
        assert_eq!(
            text,
            format::amount(last, DecimalSeparator::CommaThousands).1
        );
    }

    #[test]
    fn a_negative_total_is_flagged_negative_for_the_accent_token() {
        let w = world();
        let mut filters = TransactionFilters::defaults(today());
        filters.status = StatusFilter::Open;
        let visible = query(&w.ledger(), &w.transactions, &filters, "");
        let f = footer(
            &visible,
            &filters,
            &w.ledger(),
            DecimalSeparator::CommaThousands,
        );
        match f.total {
            FooterTotal::Single { negative, text, .. } => {
                assert_eq!(negative, text.starts_with('\u{2212}'));
            }
            other => panic!("expected a single-Unit total, got {other:?}"),
        }
    }

    #[test]
    fn mixed_units_and_an_empty_result_have_no_figure() {
        let mut w = world();
        for account in &mut w.accounts {
            if account.name == "Amex Platinum" {
                account.unit = "usd".to_string();
            }
        }
        let filters = TransactionFilters::defaults(today());
        let visible = query(&w.ledger(), &w.transactions, &filters, "");
        let mixed = footer(
            &visible,
            &filters,
            &w.ledger(),
            DecimalSeparator::CommaThousands,
        );
        assert_eq!(mixed.total, FooterTotal::Mixed);
        assert!(mixed.shown > 0, "the rows still show");

        let none = query(&w.ledger(), &w.transactions, &filters, "zzzz-no-such-thing");
        let empty = footer(
            &none,
            &filters,
            &w.ledger(),
            DecimalSeparator::CommaThousands,
        );
        assert_eq!(empty.total, FooterTotal::Empty);
        assert_eq!(empty.shown, 0);
        assert_eq!(empty.label(), "0 of 700 transactions shown");
    }
}
