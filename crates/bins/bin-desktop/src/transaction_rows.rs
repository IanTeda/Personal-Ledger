//! Turns the filter engine's visible rows into display-ready table rows: every cell already
//! formatted (dates, amounts, glyphs per the Display preferences) and every multi-Split summary
//! already decided, so the table view only paints. `gpui`-free and unit-tested.
//!
//! The multi-Split cells follow the Desktop Transactions map's Splits decision: CATEGORY reads
//! `split · N`; PAYEE is the first Split's Payee with `+N` when other Splits carry different
//! Payees (`—` if none has one); TAGS is the first chip of the Splits' combined, deduplicated Tags
//! with `+N` when there are more (`—` if none). A single-Split row reads exactly as the mockup's.

use lib_core::DateStyle;

use crate::{
    categories, format,
    payees::Payee,
    settings::StatusGlyphs,
    tags::Tag,
    transaction_query::{Ledger, Visible},
    transactions::Transaction,
};

/// The mark for an empty PAYEE or TAGS cell.
pub const EMPTY_CELL: &str = "\u{2014}";

/// The Display preferences a row is formatted with (density is a layout matter, not a text one).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DisplayPrefs {
    pub date_style: Option<DateStyle>,
    pub glyphs: StatusGlyphs,
}

/// The TAGS cell.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TagsCell {
    /// No Split carries a Tag: the cell shows `—`.
    None,
    /// The first Tag as a chip, and how many further Tags follow it.
    Chips { first: String, more: usize },
}

/// One table row, ready to paint.
#[derive(Debug, Clone, PartialEq)]
pub struct RowView {
    pub status_glyph: &'static str,
    /// `Some` (⚑ or `!`) only while the Transaction is flagged.
    pub flag_glyph: Option<&'static str>,
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

/// The PAYEE cell: the first Payee among the Splits, then `+N` for each further *different* Payee.
pub fn payee_summary(transaction: &Transaction, payees: &[Payee]) -> String {
    let ids: Vec<u32> = transaction
        .splits
        .iter()
        .filter_map(|s| s.payee_id)
        .collect();
    let Some(&first) = ids.first() else {
        return EMPTY_CELL.to_string();
    };
    let name = payees
        .iter()
        .find(|payee| payee.id == first)
        .map(|payee| payee.name.as_str())
        .unwrap_or(EMPTY_CELL);
    let mut others: Vec<u32> = ids.into_iter().filter(|id| *id != first).collect();
    others.sort_unstable();
    others.dedup();
    if others.is_empty() {
        name.to_string()
    } else {
        format!("{name} +{}", others.len())
    }
}

/// The CATEGORY cell: the leaf's own name for one Split, `split · N` for several.
pub fn category_summary(transaction: &Transaction, all: &[categories::Category]) -> String {
    match transaction.splits.as_slice() {
        [only] => all
            .iter()
            .find(|category| category.id == only.category_id)
            .map(|category| category.name.clone())
            .unwrap_or_else(|| EMPTY_CELL.to_string()),
        several => {
            crate::msg::desktop_transactions_split_summary(several.len().to_string().as_str())
        }
    }
}

/// The TAGS cell: the union of every Split's Tags in first-seen order.
pub fn tags_summary(transaction: &Transaction, tags: &[Tag]) -> TagsCell {
    let mut ids: Vec<u32> = Vec::new();
    for split in &transaction.splits {
        for id in &split.tag_ids {
            if !ids.contains(id) {
                ids.push(*id);
            }
        }
    }
    let names: Vec<&str> = ids
        .iter()
        .filter_map(|id| tags.iter().find(|tag| tag.id == *id))
        .map(|tag| tag.name.as_str())
        .collect();
    match names.split_first() {
        None => TagsCell::None,
        Some((first, rest)) => TagsCell::Chips {
            first: (*first).to_string(),
            more: rest.len(),
        },
    }
}

/// Formats every visible row per the Display preferences.
pub fn build_rows(
    visible: &Visible<'_>,
    ledger: &Ledger<'_>,
    prefs: &DisplayPrefs,
) -> Vec<RowView> {
    visible
        .rows
        .iter()
        .map(|row| {
            let transaction = row.transaction;
            let account = ledger
                .accounts
                .iter()
                .find(|account| account.id == transaction.account_id)
                .map(|account| account.name.clone())
                .unwrap_or_default();
            let (amount_negative, amount) = format::signed_amount(&row.amount);
            let (running_negative, running) = match &row.running {
                Some(running) => {
                    let (negative, text) = format::amount(running);
                    (negative, Some(text))
                }
                None => (false, None),
            };
            RowView {
                status_glyph: format::status_glyph(&transaction.status, prefs.glyphs),
                flag_glyph: transaction
                    .is_flagged
                    .then(|| format::flag_glyph(prefs.glyphs)),
                date: format::date(transaction.date, prefs.date_style),
                account,
                payee: payee_summary(transaction, ledger.payees),
                category: category_summary(transaction, ledger.categories),
                tags: tags_summary(transaction, ledger.tags),
                amount,
                amount_negative,
                running,
                running_negative,
            }
        })
        .collect()
}

/// Keeps a selection (a position in the visible rows) in range: `0` when there are none.
pub fn clamp_selection(selected: usize, len: usize) -> usize {
    selected.min(len.saturating_sub(1))
}

/// How many rows a half-page move steps, from the list's measured viewport height. Before the first
/// paint the height is zero, so it falls back to a small fixed step rather than moving nowhere.
pub fn half_page_rows(viewport_px: f32, row_px: f32) -> usize {
    const FALLBACK: usize = 5;
    if viewport_px <= 0.0 || row_px <= 0.0 {
        return FALLBACK;
    }
    (((viewport_px / row_px).floor() as usize) / 2).max(1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        accounts::default_accounts,
        categories::default_categories,
        payees::default_payees,
        tags::default_tags,
        transaction_query::{TransactionFilters, query},
        transactions::default_transactions,
    };
    use chrono::NaiveDate;
    use lib_core::TransactionStatus;

    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 9, 19).unwrap()
    }

    fn prefs() -> DisplayPrefs {
        DisplayPrefs {
            date_style: None,
            glyphs: StatusGlyphs::Unicode,
        }
    }

    struct World {
        accounts: Vec<crate::accounts::Account>,
        categories: Vec<categories::Category>,
        payees: Vec<Payee>,
        tags: Vec<Tag>,
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

    fn special<'a>(world: &'a World, description: &str) -> &'a Transaction {
        world
            .transactions
            .iter()
            .find(|t| t.description.as_deref() == Some(description))
            .unwrap_or_else(|| panic!("no special {description:?}"))
    }

    fn rows_for(world: &World, filters: &TransactionFilters, search: &str) -> Vec<RowView> {
        let ledger = Ledger {
            accounts: &world.accounts,
            categories: &world.categories,
            payees: &world.payees,
            tags: &world.tags,
        };
        let visible = query(&ledger, &world.transactions, filters, search);
        build_rows(&visible, &ledger, &prefs())
    }

    fn open_filters() -> TransactionFilters {
        TransactionFilters {
            from: None,
            to: None,
            ..TransactionFilters::defaults(today())
        }
    }

    #[test]
    fn a_single_split_row_reads_like_the_mockups() {
        let w = world();
        let bill = special(&w, "Bill looks high");
        assert_eq!(payee_summary(bill, &w.payees), "Origin Energy");
        assert_eq!(category_summary(bill, &w.categories), "Electricity");
        assert_eq!(tags_summary(bill, &w.tags), TagsCell::None);
    }

    #[test]
    fn a_multi_split_category_reads_split_and_the_count() {
        let w = world();
        let receipt = special(&w, "Weekly shop and cleaning supplies");
        assert_eq!(category_summary(receipt, &w.categories), "split \u{b7} 2");
    }

    #[test]
    fn splits_sharing_one_payee_show_it_without_a_plus() {
        let w = world();
        let receipt = special(&w, "Weekly shop and cleaning supplies");
        assert_eq!(payee_summary(receipt, &w.payees), "Woolworths");
    }

    #[test]
    fn splits_with_different_payees_show_the_first_and_a_plus_count() {
        let w = world();
        let airport = special(&w, "Airport");
        assert_eq!(payee_summary(airport, &w.payees), "Hudson News +1");
    }

    #[test]
    fn a_transaction_with_no_payee_shows_the_empty_mark() {
        let w = world();
        let mut bare = special(&w, "Bill looks high").clone();
        bare.splits[0].payee_id = None;
        assert_eq!(payee_summary(&bare, &w.payees), EMPTY_CELL);
    }

    #[test]
    fn the_first_payee_is_the_first_split_that_has_one() {
        let w = world();
        let mut receipt = special(&w, "Airport").clone();
        receipt.splits[0].payee_id = None;
        assert_eq!(payee_summary(&receipt, &w.payees), "Kmart");
    }

    #[test]
    fn tags_show_the_first_and_count_the_rest_from_every_split_deduplicated() {
        let w = world();
        let tokyo = special(&w, "Tokyo dinner and souvenirs");
        assert_eq!(
            tags_summary(tokyo, &w.tags),
            TagsCell::Chips {
                first: "Japan Trip 2026".to_string(),
                more: 0
            }
        );
        let mut many = tokyo.clone();
        many.splits[1].tag_ids = vec![2, 1, 3];
        assert_eq!(
            tags_summary(&many, &w.tags),
            TagsCell::Chips {
                first: "Japan Trip 2026".to_string(),
                more: 2
            },
            "the duplicated Japan tag counts once"
        );
    }

    #[test]
    fn rows_follow_the_display_preferences() {
        let w = world();
        let ledger = Ledger {
            accounts: &w.accounts,
            categories: &w.categories,
            payees: &w.payees,
            tags: &w.tags,
        };
        let filters = open_filters();
        let visible = query(
            &ledger,
            &w.transactions,
            &filters,
            "Weekly shop and cleaning",
        );
        assert_eq!(visible.rows.len(), 1);
        let day = visible.rows[0].transaction.date;

        let comma = build_rows(&visible, &ledger, &prefs());
        assert_eq!(comma[0].amount, "\u{2212}214.30");
        assert!(comma[0].amount_negative);
        assert_eq!(comma[0].date, format::date(day, None));

        let iso = DisplayPrefs {
            date_style: Some(DateStyle::Iso),
            glyphs: StatusGlyphs::AsciiFallback,
        };
        let rows = build_rows(&visible, &ledger, &iso);
        assert_eq!(rows[0].date, day.format("%Y-%m-%d").to_string());
        assert_eq!(rows[0].status_glyph, "o", "cleared, ascii");
        assert_eq!(rows[0].amount, "\u{2212}214.30");
    }

    #[test]
    fn income_is_signed_positive_and_not_negative() {
        let w = world();
        let rows = rows_for(&w, &open_filters(), "Sunrise Payroll");
        let pay = rows
            .iter()
            .find(|r| r.amount == "+4,210.00")
            .expect("the salary row");
        assert!(!pay.amount_negative);
    }

    #[test]
    fn the_flag_glyph_appears_only_on_flagged_rows_and_is_separate_from_the_status() {
        let w = world();
        let flagged = rows_for(&w, &open_filters(), "Bill looks high");
        assert_eq!(flagged[0].flag_glyph, Some("\u{2691}"));
        assert_eq!(flagged[0].status_glyph, "\u{25cb}", "still Open");
        let plain = rows_for(&w, &open_filters(), "Weekly shop and cleaning");
        assert_eq!(plain[0].flag_glyph, None);
    }

    #[test]
    fn the_account_name_and_running_total_are_filled_in() {
        let w = world();
        let rows = rows_for(&w, &TransactionFilters::defaults(today()), "");
        assert!(!rows.is_empty());
        assert!(rows.iter().all(|r| !r.account.is_empty()));
        assert!(
            rows.iter().all(|r| r.running.is_some()),
            "every seeded account is aud"
        );
    }

    #[test]
    fn mixed_units_leave_the_running_cell_blank() {
        let mut w = world();
        for account in &mut w.accounts {
            if account.name == "Amex Platinum" {
                account.unit = "usd".to_string();
            }
        }
        let rows = rows_for(&w, &open_filters(), "");
        assert!(rows.iter().all(|r| r.running.is_none()));
        assert!(
            rows.iter().all(|r| !r.amount.is_empty()),
            "amounts still show"
        );
    }

    #[test]
    fn a_reconciled_row_uses_the_filled_circle() {
        let w = world();
        let salary = w
            .transactions
            .iter()
            .find(|t| t.status == TransactionStatus::Reconciled)
            .unwrap();
        assert_eq!(
            format::status_glyph(&salary.status, StatusGlyphs::Unicode),
            "\u{25cf}"
        );
    }

    #[test]
    fn selection_is_clamped_and_an_empty_list_selects_zero() {
        assert_eq!(clamp_selection(5, 10), 5);
        assert_eq!(clamp_selection(50, 10), 9);
        assert_eq!(clamp_selection(3, 0), 0);
    }

    #[test]
    fn a_half_page_is_half_the_rows_that_fit_and_never_zero() {
        assert_eq!(half_page_rows(680.0, 34.0), 10);
        assert_eq!(half_page_rows(60.0, 34.0), 1);
        assert_eq!(half_page_rows(0.0, 34.0), 5, "before the first paint");
        assert_eq!(half_page_rows(680.0, 0.0), 5);
    }
}
