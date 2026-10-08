//! The stubbed **6e** Import "match payees" step (`docs/ux/desktop-mockups/20-payees/README.md`), as #284
//! settled it: a seeded mock statement (no real CSV upload or parsing), each line matched to a
//! Payee by its Payee Aliases, the rest given a cleaned-up name to create, and **continue**
//! committing the lot to the in-memory stubs. `gpui`-free, so every rule is unit-tested without a
//! window; `Shell` owns an [`ImportState`] and `view::import` draws it.
//!
//! **Row status** follows the handoff's Behaviour section: a row *needs review* while its Payee or
//! its Category is unresolved, and **continue** is refused while any row does. A resolved row is a
//! *rule match* (its Payee came from an alias), a *new payee* (it creates one) or *matched* (a
//! Payee picked by hand).

use bigdecimal::BigDecimal;
use chrono::{Duration, NaiveDate};
use lib_core::{Money, TransactionStatus};

use crate::{
    form::select::SelectState,
    payees::{self, Payee, PayeeDraft, form::PayeeOptions},
    transactions::{Split, Transaction},
};

/// The statement's file name, shown in the title-bar context and the subline.
pub const STATEMENT_FILE: &str = "statement.csv";

/// The Account the seeded statement imports into (the Accounts stub's "ANZ Everyday").
pub const EVERYDAY_ACCOUNT_ID: u32 = 2;

/// The seeded statement: the handoff's seven visible lines plus eleven more, 18 in all -- 14 an
/// alias matches and 4 that need a Payee (`(raw description, amount in cents, days ago)`).
const STATEMENT: [(&str, i64, i64); 18] = [
    ("WOOLWORTHS 2137 SYDNEY AU", -8_640, 1),
    ("COLES 0332 BRISBANE AU", -4_190, 1),
    ("NETFLIX.COM 866-579-7172", -2_299, 2),
    ("BP SERVICE STN 4471", -6_820, 2),
    ("SP AUSSIE CANDLE CO", -3_400, 3),
    ("TFR TO J SMITH REF PAYMENT", -64_000, 3),
    ("ORIGIN ENERGY DIRECT DEBIT", -11_020, 4),
    ("WW SUPERMARKET 1044 PARRAMATTA", -5_215, 5),
    ("EMPLOYER PTY LTD PAY 0925", 325_000, 6),
    ("UBER *TRIP HELP.UBER.COM", -1_860, 6),
    ("BUNNINGS 7731 ALEXANDRIA", -6_495, 7),
    ("KMART 1123 BONDI JUNCTION", -2_900, 8),
    ("OFFICEWORKS 0561 SYDNEY", -4_580, 9),
    ("SYDNEY WATER CORP BPAY", -9_635, 10),
    ("SQ *BLUE BOTTLE CAFE SYDNEY", -1_150, 11),
    ("COLES ONLINE 8842", -12_870, 12),
    ("ANZ INTEREST CREDIT", 412, 13),
    ("PAYPAL *HANDMADE SOAPS 4029357733", -2_700, 14),
];

/// Leading payment-channel words that say how, not who (`SP`, `TFR TO`, ...), longest first.
const PREFIXES: [&[&str]; 8] = [
    &["TFR", "FROM"],
    &["TFR", "TO"],
    &["VISA", "PURCHASE"],
    &["EFTPOS"],
    &["PAYPAL"],
    &["TFR"],
    &["SP"],
    &["SQ"],
];

/// Tokens that end the useful part of a description: everything from one on is reference text.
const REFERENCE_MARKERS: [&str; 3] = ["REF", "BPAY", "RECEIPT"];

/// Trailing location words stripped from a description (cities, states, the country code).
const LOCATIONS: [&str; 17] = [
    "SYDNEY",
    "MELBOURNE",
    "BRISBANE",
    "PERTH",
    "ADELAIDE",
    "HOBART",
    "DARWIN",
    "CANBERRA",
    "NSW",
    "VIC",
    "QLD",
    "WA",
    "SA",
    "TAS",
    "NT",
    "ACT",
    "AU",
];

/// The words of `raw` that name who was paid: payment-channel prefixes, `*`, digit-bearing tokens
/// (store and phone numbers), reference text and trailing locations removed, upper-cased.
fn cleaned_words(raw: &str) -> Vec<String> {
    let mut words: Vec<String> = raw
        .replace('*', " ")
        .split_whitespace()
        .map(str::to_uppercase)
        .collect();
    if let Some(prefix) = PREFIXES.iter().find(|prefix| {
        words.len() > prefix.len() && words.iter().zip(prefix.iter()).all(|(w, p)| w == p)
    }) {
        words.drain(..prefix.len());
    }
    if let Some(end) = words
        .iter()
        .position(|word| REFERENCE_MARKERS.contains(&word.as_str()))
    {
        words.truncate(end);
    }
    words.retain(|word| !word.chars().any(|c| c.is_ascii_digit()));
    while words.len() > 1
        && words
            .last()
            .is_some_and(|word| LOCATIONS.contains(&word.as_str()))
    {
        words.pop();
    }
    words
}

/// The name suggested for a new Payee from a raw statement description, title-cased
/// (`SP AUSSIE CANDLE CO` → `Aussie Candle Co`); `None` when nothing is left once cleaned.
pub fn cleaned_name(raw: &str) -> Option<String> {
    let words = cleaned_words(raw);
    (!words.is_empty()).then(|| {
        words
            .iter()
            .map(|word| {
                let mut chars = word.chars();
                chars.next().map_or_else(String::new, |first| {
                    first
                        .to_uppercase()
                        .chain(chars.flat_map(char::to_lowercase))
                        .collect()
                })
            })
            .collect::<Vec<_>>()
            .join(" ")
    })
}

/// The Payee Alias "remember new payees' rules" adds for a raw description: its cleaned words as
/// an alias is stored (`AUSSIE CANDLE CO`).
pub fn cleaned_token(raw: &str) -> Option<String> {
    payees::normalise_alias(&cleaned_words(raw).join(" "))
}

/// What a row's Payee resolves to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RowPayee {
    /// An alias matched this active Payee.
    Rule(u32),
    /// An existing active Payee, picked by hand or by the cleaned name equalling its name.
    Existing(u32),
    /// A new Payee with this name, created on **continue**.
    Create(String),
}

impl RowPayee {
    fn id(&self) -> Option<u32> {
        match self {
            Self::Rule(id) | Self::Existing(id) => Some(*id),
            Self::Create(_) => None,
        }
    }
}

/// A row's status tag.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RowStatus {
    RuleMatch,
    NewPayee,
    Matched,
    NeedsReview,
}

/// One statement line and how it maps.
#[derive(Debug, Clone, PartialEq)]
pub struct ImportRow {
    pub raw: String,
    pub amount: Money,
    pub date: NaiveDate,
    pub payee: Option<RowPayee>,
    /// A leaf Category id.
    pub category: Option<u32>,
}

impl ImportRow {
    pub fn status(&self) -> RowStatus {
        match (&self.payee, self.category) {
            (None, _) | (_, None) => RowStatus::NeedsReview,
            (Some(RowPayee::Rule(_)), Some(_)) => RowStatus::RuleMatch,
            (Some(RowPayee::Create(_)), Some(_)) => RowStatus::NewPayee,
            (Some(RowPayee::Existing(_)), Some(_)) => RowStatus::Matched,
        }
    }

    pub fn needs_review(&self) -> bool {
        self.status() == RowStatus::NeedsReview
    }
}

fn cents_money(cents: i64) -> Money {
    Money(BigDecimal::new(cents.into(), 2))
}

/// The active Payee named `name`, case-insensitively.
fn active_named(payees: &[Payee], name: &str) -> Option<u32> {
    let folded = name.to_lowercase();
    payees
        .iter()
        .find(|p| p.is_active && p.name.to_lowercase() == folded)
        .map(|p| p.id)
}

/// Matches `raw`: an alias gives the Payee and its default Category; otherwise the cleaned name
/// picks the active Payee already so named, or else suggests creating it when that name is free.
pub fn match_row(payees: &[Payee], raw: &str, amount: Money, date: NaiveDate) -> ImportRow {
    if let Some(id) = payees::match_alias(payees, raw) {
        return ImportRow {
            raw: raw.to_string(),
            amount,
            date,
            payee: Some(RowPayee::Rule(id)),
            category: payees::get(payees, id).and_then(|p| p.default_category),
        };
    }
    let payee = cleaned_name(raw).and_then(|name| match active_named(payees, &name) {
        Some(id) => Some(RowPayee::Existing(id)),
        None => payees::name_available(payees, &name).then_some(RowPayee::Create(name)),
    });
    let category = payee
        .as_ref()
        .and_then(RowPayee::id)
        .and_then(|id| payees::get(payees, id))
        .and_then(|p| p.default_category);
    ImportRow {
        raw: raw.to_string(),
        amount,
        date,
        payee,
        category,
    }
}

/// The seeded statement, matched against `payees`, dated back from `today`.
pub fn seeded_rows(payees: &[Payee], today: NaiveDate) -> Vec<ImportRow> {
    STATEMENT
        .iter()
        .map(|&(raw, cents, days)| {
            match_row(
                payees,
                raw,
                cents_money(cents),
                today - Duration::days(days),
            )
        })
        .collect()
}

/// The footer's figures.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Summary {
    pub rows: usize,
    pub rule_matched: usize,
    /// Distinct new Payee names (case-insensitively), however many rows share one.
    pub new_payees: usize,
    pub needs_review: usize,
}

pub fn summary(rows: &[ImportRow]) -> Summary {
    let mut new_names: Vec<String> = rows
        .iter()
        .filter_map(|row| match &row.payee {
            Some(RowPayee::Create(name)) => Some(name.to_lowercase()),
            _ => None,
        })
        .collect();
    new_names.sort();
    new_names.dedup();
    Summary {
        rows: rows.len(),
        rule_matched: rows
            .iter()
            .filter(|r| matches!(r.payee, Some(RowPayee::Rule(_))))
            .count(),
        new_payees: new_names.len(),
        needs_review: rows.iter().filter(|r| r.needs_review()).count(),
    }
}

/// A row's Payee select: `+ create "<cleaned name>"` first (when that name is free), then every
/// active Payee alphabetically -- inactive ones are never offered (#283). `labels` is what the
/// select shows; `choices` runs parallel to it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PayeeChoices {
    pub labels: Vec<String>,
    pub choices: Vec<RowPayee>,
}

impl PayeeChoices {
    /// `create_label` renders the create option's label from the cleaned name.
    pub fn new(payees: &[Payee], raw: &str, create_label: impl Fn(&str) -> String) -> Self {
        let mut labels = Vec::new();
        let mut choices = Vec::new();
        if let Some(name) = cleaned_name(raw).filter(|name| payees::name_available(payees, name)) {
            labels.push(create_label(&name));
            choices.push(RowPayee::Create(name));
        }
        let mut active: Vec<&Payee> = payees.iter().filter(|p| p.is_active).collect();
        active.sort_by_key(|payee| payee.name.to_lowercase());
        for payee in active {
            labels.push(payee.name.clone());
            choices.push(RowPayee::Existing(payee.id));
        }
        Self { labels, choices }
    }

    /// The label showing `payee`: a rule match shows as its Payee's name.
    pub fn label_for(&self, payee: &RowPayee) -> Option<&str> {
        let wanted = match payee {
            RowPayee::Rule(id) => RowPayee::Existing(*id),
            other => other.clone(),
        };
        self.choices
            .iter()
            .position(|choice| *choice == wanted)
            .map(|index| self.labels[index].as_str())
    }

    fn choice_for(&self, label: &str) -> Option<RowPayee> {
        self.labels
            .iter()
            .position(|candidate| candidate == label)
            .map(|index| self.choices[index].clone())
    }
}

/// A key an open row select understands, parsed from a keystroke by `Shell`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectKey {
    Up,
    Down,
    /// `Enter` or `Space`.
    Commit,
    /// `Esc`.
    Cancel,
}

/// Which of a row's two selects.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RowSelect {
    Payee,
    Category,
}

/// The whole step's live state -- `Shell` holds it as `Some` while 6e is showing.
#[derive(Debug, Clone, PartialEq)]
pub struct ImportState {
    pub rows: Vec<ImportRow>,
    pub selected: usize,
    /// "remember new payees' rules for next time", ticked by default as in the handoff.
    pub remember: bool,
    /// The one open select, if any: which row, which select, and its state.
    pub open_select: Option<(usize, RowSelect, SelectState)>,
}

impl ImportState {
    pub fn new(payees: &[Payee], today: NaiveDate) -> Self {
        Self {
            rows: seeded_rows(payees, today),
            selected: 0,
            remember: true,
            open_select: None,
        }
    }

    pub fn can_continue(&self) -> bool {
        !self.rows.iter().any(ImportRow::needs_review)
    }

    /// `n`: the selected row creates a new Payee from its cleaned name, when that name is free.
    pub fn create_new_payee(&mut self, payees: &[Payee]) {
        if let Some(row) = self.rows.get_mut(self.selected)
            && let Some(name) =
                cleaned_name(&row.raw).filter(|name| payees::name_available(payees, name))
        {
            row.payee = Some(RowPayee::Create(name));
        }
    }

    /// Opens row `index`'s `select` on its current value (closing any other).
    pub fn open(
        &mut self,
        index: usize,
        select: RowSelect,
        payee_choices: &PayeeChoices,
        categories: &PayeeOptions,
    ) {
        let Some(row) = self.rows.get(index) else {
            return;
        };
        self.selected = index;
        let (value, options) = match select {
            RowSelect::Payee => (
                row.payee
                    .as_ref()
                    .and_then(|payee| payee_choices.label_for(payee))
                    .map(str::to_string),
                &payee_choices.labels,
            ),
            RowSelect::Category => (
                categories
                    .ids
                    .iter()
                    .position(|id| *id == row.category && row.category.is_some())
                    .map(|index| categories.labels[index].clone()),
                &categories.labels,
            ),
        };
        let mut state = SelectState::new(value);
        state.open(options);
        self.open_select = Some((index, select, state));
    }

    /// Keys while a select is open; `true` when the select took the key.
    pub fn handle_select_key(
        &mut self,
        key: SelectKey,
        payees: &[Payee],
        payee_choices: &PayeeChoices,
        categories: &PayeeOptions,
    ) -> bool {
        let Some((_, select, state)) = self.open_select.as_mut() else {
            return false;
        };
        let options = match select {
            RowSelect::Payee => &payee_choices.labels,
            RowSelect::Category => &categories.labels,
        };
        match key {
            SelectKey::Up => state.move_highlight(options, -1),
            SelectKey::Down => state.move_highlight(options, 1),
            SelectKey::Commit => {
                state.commit(options);
                self.apply_open_select(payees, payee_choices, categories);
            }
            SelectKey::Cancel => self.open_select = None,
        }
        true
    }

    /// A click on option `index` of the open select.
    pub fn choose(
        &mut self,
        index: usize,
        payees: &[Payee],
        payee_choices: &PayeeChoices,
        categories: &PayeeOptions,
    ) {
        let Some((_, select, state)) = self.open_select.as_mut() else {
            return;
        };
        let options = match select {
            RowSelect::Payee => &payee_choices.labels,
            RowSelect::Category => &categories.labels,
        };
        state.choose(options, index);
        self.apply_open_select(payees, payee_choices, categories);
    }

    /// Writes the closed select's value into its row. Picking an existing Payee pre-fills an empty
    /// Category with its default; picking "choose category…" clears it.
    fn apply_open_select(
        &mut self,
        payees: &[Payee],
        payee_choices: &PayeeChoices,
        categories: &PayeeOptions,
    ) {
        let Some((index, select, state)) = self.open_select.take() else {
            return;
        };
        let (Some(row), Some(value)) = (self.rows.get_mut(index), state.value()) else {
            return;
        };
        match select {
            RowSelect::Payee => {
                row.payee = payee_choices.choice_for(value);
                if row.category.is_none() {
                    row.category = row
                        .payee
                        .as_ref()
                        .and_then(RowPayee::id)
                        .and_then(|id| payees::get(payees, id))
                        .and_then(|p| p.default_category);
                }
            }
            RowSelect::Category => {
                row.category = categories
                    .labels
                    .iter()
                    .position(|label| label == value)
                    .and_then(|index| categories.ids[index]);
            }
        }
    }
}

/// What **continue** did, for its Toast.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Committed {
    pub transactions: usize,
    pub new_payees: usize,
}

/// Why **continue** refused.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ImportError {
    #[error("{0} rows still need review")]
    NeedsReview(usize),
    #[error(transparent)]
    Payee(#[from] payees::PayeeError),
}

/// **continue**: creates each new Payee once (with its rows' cleaned tokens as aliases when
/// `remember`), then appends one single-Split Transaction per row to `account_id`, newest first.
/// Nothing changes when any row still needs review.
pub fn commit(
    rows: &[ImportRow],
    remember: bool,
    account_id: u32,
    payees_list: &mut Vec<Payee>,
    transactions: &mut Vec<Transaction>,
) -> Result<Committed, ImportError> {
    let pending = rows.iter().filter(|r| r.needs_review()).count();
    if pending > 0 {
        return Err(ImportError::NeedsReview(pending));
    }

    let mut created: Vec<(String, u32)> = Vec::new();
    let mut splits_for: Vec<(u32, u32)> = Vec::with_capacity(rows.len());
    for row in rows {
        let (Some(payee), Some(category)) = (&row.payee, row.category) else {
            return Err(ImportError::NeedsReview(1));
        };
        let id = match payee {
            RowPayee::Rule(id) | RowPayee::Existing(id) => *id,
            RowPayee::Create(name) => {
                let folded = name.to_lowercase();
                match created.iter().find(|(n, _)| *n == folded) {
                    Some((_, id)) => *id,
                    None => {
                        let id = payees::insert_payee(
                            payees_list,
                            &PayeeDraft {
                                name: name.clone(),
                                default_category: None,
                                aliases: Vec::new(),
                            },
                        )?;
                        created.push((folded, id));
                        id
                    }
                }
            }
        };
        // A token another Payee already owns is skipped, keeping aliases unique (#282).
        if remember
            && matches!(payee, RowPayee::Create(_))
            && let Some(token) = cleaned_token(&row.raw)
            && payees::alias_owner(payees_list, &token).is_none_or(|owner| owner.id == id)
            && let Some(new_payee) = payees_list.iter_mut().find(|p| p.id == id)
            && !new_payee.aliases.contains(&token)
        {
            new_payee.aliases.push(token);
        }
        splits_for.push((id, category));
    }

    let first_id = transactions.iter().map(|t| t.id).max().unwrap_or(0) + 1;
    for (id, (row, (payee_id, category_id))) in (first_id..).zip(rows.iter().zip(splits_for)) {
        transactions.push(Transaction {
            id,
            date: row.date,
            account_id,
            status: TransactionStatus::Cleared,
            is_flagged: false,
            description: Some(row.raw.clone()),
            splits: vec![Split {
                amount: row.amount.clone(),
                category_id,
                payee_id: Some(payee_id),
                tag_ids: Vec::new(),
            }],
        });
    }
    transactions.sort_by(|a, b| b.date.cmp(&a.date).then(b.id.cmp(&a.id)));

    Ok(Committed {
        transactions: rows.len(),
        new_payees: created.len(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        accounts::default_accounts, categories::default_categories, payees::default_payees,
        tags::default_tags, transactions::default_transactions,
    };

    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 9, 28).unwrap()
    }

    fn state() -> (Vec<Payee>, ImportState) {
        let payees = default_payees();
        let state = ImportState::new(&payees, today());
        (payees, state)
    }

    fn choices(payees: &[Payee], raw: &str) -> PayeeChoices {
        PayeeChoices::new(payees, raw, |name| format!("+ create \"{name}\""))
    }

    fn category_options() -> PayeeOptions {
        PayeeOptions::new(&default_categories(), "choose category…".to_string())
    }

    #[test]
    fn cleaned_name_strips_prefixes_numbers_locations_and_references() {
        assert_eq!(
            cleaned_name("SP AUSSIE CANDLE CO").as_deref(),
            Some("Aussie Candle Co")
        );
        assert_eq!(
            cleaned_name("TFR TO J SMITH REF PAYMENT").as_deref(),
            Some("J Smith")
        );
        assert_eq!(
            cleaned_name("SQ *BLUE BOTTLE CAFE SYDNEY").as_deref(),
            Some("Blue Bottle Cafe")
        );
        assert_eq!(
            cleaned_name("PAYPAL *HANDMADE SOAPS 4029357733").as_deref(),
            Some("Handmade Soaps")
        );
        assert_eq!(
            cleaned_name("WOOLWORTHS 2137 SYDNEY AU").as_deref(),
            Some("Woolworths")
        );
    }

    #[test]
    fn cleaned_name_keeps_a_lone_location_word_and_rejects_empty_input() {
        assert_eq!(cleaned_name("SYDNEY").as_deref(), Some("Sydney"));
        assert_eq!(cleaned_name("  1234 5678 "), None);
        assert_eq!(cleaned_name(""), None);
        // A prefix alone is a name, not a prefix.
        assert_eq!(cleaned_name("SP").as_deref(), Some("Sp"));
    }

    #[test]
    fn cleaned_token_is_the_upper_cased_cleaned_words() {
        assert_eq!(
            cleaned_token("SP AUSSIE CANDLE CO").as_deref(),
            Some("AUSSIE CANDLE CO")
        );
        assert_eq!(cleaned_token("4471"), None);
    }

    #[test]
    fn the_seeded_statement_has_14_rule_matches_and_4_rows_to_review() {
        let (_, state) = state();
        let summary = summary(&state.rows);
        assert_eq!(summary.rows, 18);
        assert_eq!(summary.rule_matched, 14);
        assert_eq!(summary.new_payees, 3);
        assert_eq!(summary.needs_review, 4);
        assert!(!state.can_continue());
    }

    #[test]
    fn a_rule_match_takes_the_payee_and_its_default_category() {
        let (payees, state) = state();
        let row = &state.rows[0];
        let woolworths = payees::find_by_name(&payees, "Woolworths").unwrap();
        assert_eq!(row.payee, Some(RowPayee::Rule(woolworths)));
        assert_eq!(
            row.category,
            payees::get(&payees, woolworths).unwrap().default_category
        );
        assert_eq!(row.status(), RowStatus::RuleMatch);
    }

    #[test]
    fn an_unmatched_row_suggests_creating_its_cleaned_name() {
        let (_, state) = state();
        let row = state
            .rows
            .iter()
            .find(|r| r.raw == "SP AUSSIE CANDLE CO")
            .unwrap();
        assert_eq!(
            row.payee,
            Some(RowPayee::Create("Aussie Candle Co".to_string()))
        );
        assert_eq!(row.category, None);
        assert_eq!(row.status(), RowStatus::NeedsReview);
    }

    #[test]
    fn a_cleaned_name_equal_to_an_active_payee_picks_that_payee() {
        let (payees, state) = state();
        let row = state
            .rows
            .iter()
            .find(|r| r.raw.contains("J SMITH"))
            .unwrap();
        let j_smith = payees::find_by_name(&payees, "J Smith").unwrap();
        assert_eq!(row.payee, Some(RowPayee::Existing(j_smith)));
        // J Smith has no default Category, so it still needs review.
        assert!(row.needs_review());
    }

    #[test]
    fn inactive_payees_neither_match_nor_are_offered() {
        let mut payees = default_payees();
        payees::set_active(&mut payees, 1, false).unwrap();
        let row = match_row(&payees, "WOOLWORTHS 99", cents_money(-100), today());
        assert_eq!(row.payee, None);
        let offered = choices(&payees, "WOOLWORTHS 99");
        assert!(!offered.labels.iter().any(|l| l == "Woolworths"));
        // Its name is still taken, so no create is offered either.
        assert!(!offered.labels.iter().any(|l| l.starts_with('+')));
    }

    #[test]
    fn payee_choices_put_create_first_then_active_payees_alphabetically() {
        let payees = default_payees();
        let offered = choices(&payees, "SP AUSSIE CANDLE CO");
        assert_eq!(offered.labels[0], "+ create \"Aussie Candle Co\"");
        assert_eq!(
            offered.choices[0],
            RowPayee::Create("Aussie Candle Co".to_string())
        );
        let names = &offered.labels[1..];
        let mut sorted = names.to_vec();
        sorted.sort_by_key(|n| n.to_lowercase());
        assert_eq!(names, sorted.as_slice());
    }

    #[test]
    fn choosing_a_category_resolves_a_new_payee_row() {
        let (payees, mut state) = state();
        let index = state
            .rows
            .iter()
            .position(|r| r.raw == "SP AUSSIE CANDLE CO")
            .unwrap();
        let payee_choices = choices(&payees, &state.rows[index].raw);
        let categories = category_options();
        state.open(index, RowSelect::Category, &payee_choices, &categories);
        let household = categories
            .labels
            .iter()
            .position(|l| l == "Household")
            .unwrap();
        state.choose(household, &payees, &payee_choices, &categories);
        assert_eq!(state.rows[index].category, Some(10));
        assert_eq!(state.rows[index].status(), RowStatus::NewPayee);
        assert_eq!(state.open_select, None);
    }

    #[test]
    fn picking_an_existing_payee_by_key_fills_its_default_category() {
        let (payees, mut state) = state();
        let index = state
            .rows
            .iter()
            .position(|r| r.raw.starts_with("SQ"))
            .unwrap();
        let payee_choices = choices(&payees, &state.rows[index].raw);
        let categories = category_options();
        state.open(index, RowSelect::Payee, &payee_choices, &categories);
        let coles = payee_choices
            .labels
            .iter()
            .position(|l| l == "Coles")
            .unwrap();
        for _ in 0..coles {
            state.handle_select_key(SelectKey::Down, &payees, &payee_choices, &categories);
        }
        state.handle_select_key(SelectKey::Commit, &payees, &payee_choices, &categories);
        let row = &state.rows[index];
        assert_eq!(row.payee, Some(RowPayee::Existing(2)));
        assert_eq!(row.category, Some(7));
        assert_eq!(row.status(), RowStatus::Matched);
    }

    #[test]
    fn cancel_closes_the_select_without_changing_the_row() {
        let (payees, mut state) = state();
        let before = state.rows[4].clone();
        let payee_choices = choices(&payees, &before.raw);
        let categories = category_options();
        state.open(4, RowSelect::Payee, &payee_choices, &categories);
        state.handle_select_key(SelectKey::Down, &payees, &payee_choices, &categories);
        state.handle_select_key(SelectKey::Cancel, &payees, &payee_choices, &categories);
        assert_eq!(state.rows[4], before);
        assert_eq!(state.open_select, None);
    }

    #[test]
    fn create_new_payee_sets_the_selected_rows_cleaned_name() {
        let (payees, mut state) = state();
        state.selected = state
            .rows
            .iter()
            .position(|r| r.raw.starts_with("SQ"))
            .unwrap();
        state.rows[state.selected].payee = None;
        state.create_new_payee(&payees);
        assert_eq!(
            state.rows[state.selected].payee,
            Some(RowPayee::Create("Blue Bottle Cafe".to_string()))
        );
    }

    /// Resolves every row still needing review with Household.
    fn resolve_all(state: &mut ImportState) {
        for row in &mut state.rows {
            if row.category.is_none() {
                row.category = Some(10);
            }
        }
    }

    fn seeded_transactions(payees: &[Payee]) -> Vec<Transaction> {
        default_transactions(
            &default_accounts(),
            &default_categories(),
            payees,
            &default_tags(),
            today(),
        )
    }

    #[test]
    fn commit_refuses_while_rows_need_review_and_changes_nothing() {
        let (mut payees, state) = state();
        let mut transactions = seeded_transactions(&payees);
        let (payees_before, transactions_before) = (payees.clone(), transactions.len());
        let result = commit(
            &state.rows,
            true,
            EVERYDAY_ACCOUNT_ID,
            &mut payees,
            &mut transactions,
        );
        assert_eq!(result, Err(ImportError::NeedsReview(4)));
        assert_eq!(payees, payees_before);
        assert_eq!(transactions.len(), transactions_before);
    }

    #[test]
    fn commit_creates_payees_with_remembered_aliases_and_appends_transactions() {
        let (mut payees, mut state) = state();
        resolve_all(&mut state);
        let mut transactions = seeded_transactions(&payees);
        let before = transactions.len();
        let committed = commit(
            &state.rows,
            true,
            EVERYDAY_ACCOUNT_ID,
            &mut payees,
            &mut transactions,
        )
        .unwrap();
        assert_eq!(
            committed,
            Committed {
                transactions: 18,
                new_payees: 3
            }
        );
        assert_eq!(transactions.len(), before + 18);

        let candle = payees::find_by_name(&payees, "Aussie Candle Co").unwrap();
        assert_eq!(
            payees::get(&payees, candle).unwrap().aliases,
            vec!["AUSSIE CANDLE CO"]
        );
        // The next import matches it by rule.
        assert_eq!(
            payees::match_alias(&payees, "SP AUSSIE CANDLE CO"),
            Some(candle)
        );

        let imported = transactions
            .iter()
            .find(|t| t.description.as_deref() == Some("SP AUSSIE CANDLE CO"))
            .unwrap();
        assert_eq!(imported.account_id, EVERYDAY_ACCOUNT_ID);
        assert_eq!(imported.splits.len(), 1);
        assert_eq!(imported.splits[0].payee_id, Some(candle));
        assert_eq!(imported.splits[0].category_id, 10);
        // Still newest first.
        assert!(transactions.windows(2).all(|w| w[0].date >= w[1].date));
    }

    #[test]
    fn commit_without_remember_adds_no_aliases() {
        let (mut payees, mut state) = state();
        resolve_all(&mut state);
        let mut transactions = seeded_transactions(&payees);
        commit(
            &state.rows,
            false,
            EVERYDAY_ACCOUNT_ID,
            &mut payees,
            &mut transactions,
        )
        .unwrap();
        let candle = payees::find_by_name(&payees, "Aussie Candle Co").unwrap();
        assert!(payees::get(&payees, candle).unwrap().aliases.is_empty());
    }

    #[test]
    fn rows_sharing_a_new_name_create_one_payee() {
        let mut payees = default_payees();
        let rows: Vec<ImportRow> = ["SP AUSSIE CANDLE CO", "SP AUSSIE CANDLE CO 2"]
            .iter()
            .map(|raw| ImportRow {
                category: Some(10),
                ..match_row(&payees, raw, cents_money(-100), today())
            })
            .collect();
        let before = payees.len();
        let mut transactions = Vec::new();
        let committed = commit(
            &rows,
            true,
            EVERYDAY_ACCOUNT_ID,
            &mut payees,
            &mut transactions,
        )
        .unwrap();
        assert_eq!(committed.new_payees, 1);
        assert_eq!(payees.len(), before + 1);
        assert_eq!(summary(&rows).new_payees, 1);
    }
}
