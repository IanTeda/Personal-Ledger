//! The text behind the Transactions view's count line and footer
//! (`docs/ux/desktop-mockups/03-transactions/README.md`'s 4a): the count line and the footer bar's
//! figures. `gpui`-free and unit-tested; the view only paints what it returns. The filter chips
//! are `lib_transactions`' (`FilterPresenter::chips`).

use std::collections::HashSet;

use lib_core::AccountId;

use crate::{
    transactions::Transaction,
    transactions::query::{Ledger, Total, TransactionFilters, Visible},
    view::format,
};

/// `700 transactions across 4 accounts`: the whole ledger, and how many accounts hold any of it.
pub fn count_line(transactions: &[Transaction]) -> String {
    let accounts: HashSet<AccountId> = transactions.iter().map(|t| t.account_id).collect();
    crate::msg::desktop_transactions_count_line(
        i64::try_from(transactions.len()).unwrap_or(i64::MAX),
        i64::try_from(accounts.len()).unwrap_or(i64::MAX),
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
        crate::msg::desktop_transactions_footer(
            i64::try_from(self.of).unwrap_or(i64::MAX),
            &self.shown.to_string(),
            &category,
        )
    }
}

/// The footer for a query result.
pub fn footer(visible: &Visible<'_>, filters: &TransactionFilters, ledger: &Ledger<'_>) -> Footer {
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
            let (negative, text) = format::amount(amount);
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
        transactions::default_transactions,
        transactions::query::{StatusFilter, query},
    };
    use chrono::NaiveDate;
    use lib_accounts::default_accounts;
    use lib_categories::default_categories;
    use lib_payees::default_payees;
    use lib_tags::default_tags;

    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 9, 19).unwrap()
    }

    struct World {
        accounts: Vec<lib_accounts::Account>,
        categories: Vec<lib_categories::Category>,
        payees: Vec<lib_payees::Payee>,
        tags: Vec<lib_tags::Tag>,
        transactions: Vec<Transaction>,
        units: Vec<lib_units::UnitRow>,
    }

    fn world() -> World {
        crate::locale::init_for_tests();
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
            units: lib_units::default_units(chrono::Utc::now()),
        }
    }

    impl World {
        fn ledger(&self) -> Ledger<'_> {
            Ledger {
                accounts: &self.accounts,
                categories: &self.categories,
                payees: &self.payees,
                tags: &self.tags,
                units: &self.units,
            }
        }
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
        let plain = footer(&visible, &filters, &w.ledger());
        assert_eq!(
            plain.label(),
            format!("{} of 700 transactions shown", visible.rows.len())
        );

        let groceries = lib_categories::find_by_name(&w.categories, "Groceries").unwrap();
        let mut filters = TransactionFilters::defaults(today());
        filters.category = Some(groceries);
        let visible = query(&w.ledger(), &w.transactions, &filters, "");
        let named = footer(&visible, &filters, &w.ledger());
        assert_eq!(named.category.as_deref(), Some("groceries"));
        assert_eq!(
            named.label(),
            format!("{} of 700 groceries transactions shown", visible.rows.len())
        );
    }

    #[test]
    fn a_single_unit_footer_total_is_the_last_running_figure() {
        let w = world();
        let filters = TransactionFilters::defaults(today());
        let visible = query(&w.ledger(), &w.transactions, &filters, "");
        let total = footer(&visible, &filters, &w.ledger());
        let (unit, text) = match &total.total {
            FooterTotal::Single { unit, text, .. } => (unit.clone(), text.clone()),
            other => panic!("expected single-Unit totals, got {other:?}"),
        };
        assert_eq!(unit, "aud");
        let last = visible.rows.last().unwrap().running.as_ref().unwrap();
        assert_eq!(text, format::amount(last).1);
    }

    #[test]
    fn a_negative_total_is_flagged_negative_for_the_accent_token() {
        let w = world();
        let mut filters = TransactionFilters::defaults(today());
        filters.status = StatusFilter::Open;
        let visible = query(&w.ledger(), &w.transactions, &filters, "");
        let f = footer(&visible, &filters, &w.ledger());
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
                account.unit = lib_core::UnitId::SEED_BTC;
            }
        }
        let filters = TransactionFilters::defaults(today());
        let visible = query(&w.ledger(), &w.transactions, &filters, "");
        let mixed = footer(&visible, &filters, &w.ledger());
        assert_eq!(mixed.total, FooterTotal::Mixed);
        assert!(mixed.shown > 0, "the rows still show");

        let none = query(&w.ledger(), &w.transactions, &filters, "zzzz-no-such-thing");
        let empty = footer(&none, &filters, &w.ledger());
        assert_eq!(empty.total, FooterTotal::Empty);
        assert_eq!(empty.shown, 0);
        assert_eq!(empty.label(), "0 of 700 transactions shown");
    }
}
