//! The Import's Entities and rules, moved from the Desktop's `import.rs`: a statement line, how it
//! is cleaned and matched to a Payee by its Payee Aliases, the Import row it becomes, its Row
//! status, a row's Payee choices and the summary.
//!
//! **Row status** follows the handoff's Behaviour section (`docs/ux/desktop-mockups/20-payees/`):
//! a row *needs review* while its Payee or its Category is unresolved. A resolved row is a *rule
//! match* (its Payee came from an alias), a *new payee* (it creates one) or *matched* (a Payee
//! picked by hand, or named by the cleaned description).

use chrono::NaiveDate;
use lib_core::{AccountId, CategoryId, Money, PayeeId};
use lib_payees::{self, Payee};

/// One line of a statement, as it came.
#[derive(Debug, Clone, PartialEq)]
pub struct StatementLine {
    /// The bank's description, kept as the Transaction's memo.
    pub raw: String,
    pub amount: Money,
    pub date: NaiveDate,
}

/// A statement: its lines and the Account they belong to. The Account is the statement's, not a
/// rule's, so a source names it.
#[derive(Debug, Clone, PartialEq)]
pub struct Statement {
    pub account_id: AccountId,
    /// The file the lines came from, which the step names.
    pub file_name: String,
    pub lines: Vec<StatementLine>,
}

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

/// The Payee Alias "remember new payees' rules" adds for a raw description: its cleaned words,
/// normalised by `lib-payees`' rule (`AUSSIE CANDLE CO`).
pub fn cleaned_token(raw: &str) -> Option<String> {
    lib_payees::normalise_alias(&cleaned_words(raw).join(" "))
}

/// What a row's Payee resolves to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RowPayee {
    /// An alias matched this active Payee.
    Rule(PayeeId),
    /// An existing active Payee, picked by hand or by the cleaned name equalling its name.
    Existing(PayeeId),
    /// A new Payee with this name, created on **continue**.
    Create(String),
}

impl RowPayee {
    /// The Payee it names, `None` for one still to create.
    pub fn id(&self) -> Option<PayeeId> {
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
    pub category: Option<CategoryId>,
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

/// The active Payee named `name`, case-insensitively.
fn active_named(payees: &[Payee], name: &str) -> Option<PayeeId> {
    let folded = name.to_lowercase();
    payees
        .iter()
        .find(|p| p.is_active && p.name.to_lowercase() == folded)
        .map(|p| p.id)
}

/// Matches `line` against every Payee: an alias gives the Payee and its default Category;
/// otherwise the cleaned name picks the active Payee already so named, or else suggests creating it
/// when that name is free. `payees` holds the inactive ones too, whose names stay taken.
pub fn match_line(payees: &[Payee], line: &StatementLine) -> ImportRow {
    let default_category =
        |id: PayeeId| lib_payees::get(payees, id).and_then(|payee| payee.default_category);
    let payee = match lib_payees::match_alias(payees, &line.raw) {
        Some(id) => Some(RowPayee::Rule(id)),
        None => cleaned_name(&line.raw).and_then(|name| match active_named(payees, &name) {
            Some(id) => Some(RowPayee::Existing(id)),
            None => lib_payees::name_available(payees, &name).then_some(RowPayee::Create(name)),
        }),
    };
    ImportRow {
        raw: line.raw.clone(),
        amount: line.amount.clone(),
        date: line.date,
        category: payee
            .as_ref()
            .and_then(RowPayee::id)
            .and_then(default_category),
        payee,
    }
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
        if let Some(name) =
            cleaned_name(raw).filter(|name| lib_payees::name_available(payees, name))
        {
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

    /// The choice labelled `label`.
    pub fn choice_for(&self, label: &str) -> Option<RowPayee> {
        self.labels
            .iter()
            .position(|candidate| candidate == label)
            .map(|index| self.choices[index].clone())
    }
}

#[cfg(test)]
mod tests {
    use lib_payees::default_payees;

    use super::*;
    use crate::statement::SeededStatement;
    use crate::{StatementSource, cents};

    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 9, 28).expect("a September day")
    }

    fn rows(payees: &[Payee]) -> Vec<ImportRow> {
        SeededStatement::stub(today())
            .statement()
            .lines
            .iter()
            .map(|line| match_line(payees, line))
            .collect()
    }

    fn line(raw: &str) -> StatementLine {
        StatementLine {
            raw: raw.to_string(),
            amount: cents(-100),
            date: today(),
        }
    }

    fn choices(payees: &[Payee], raw: &str) -> PayeeChoices {
        PayeeChoices::new(payees, raw, |name| format!("+ create \"{name}\""))
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
        let summary = summary(&rows(&default_payees()));
        assert_eq!(summary.rows, 18);
        assert_eq!(summary.rule_matched, 14);
        assert_eq!(summary.new_payees, 3);
        assert_eq!(summary.needs_review, 4);
    }

    #[test]
    fn a_rule_match_takes_the_payee_and_its_default_category() {
        let payees = default_payees();
        let row = &rows(&payees)[0];
        let woolworths = lib_payees::find_by_name(&payees, "Woolworths").expect("a stub Payee");
        assert_eq!(row.payee, Some(RowPayee::Rule(woolworths)));
        assert_eq!(
            row.category,
            lib_payees::get(&payees, woolworths)
                .expect("a stub Payee")
                .default_category
        );
        assert_eq!(row.status(), RowStatus::RuleMatch);
    }

    #[test]
    fn an_unmatched_row_suggests_creating_its_cleaned_name() {
        let row = match_line(&default_payees(), &line("SP AUSSIE CANDLE CO"));
        assert_eq!(
            row.payee,
            Some(RowPayee::Create("Aussie Candle Co".to_string()))
        );
        assert_eq!(row.category, None);
        assert_eq!(row.status(), RowStatus::NeedsReview);
    }

    #[test]
    fn a_cleaned_name_equal_to_an_active_payee_picks_that_payee() {
        let payees = default_payees();
        let row = match_line(&payees, &line("TFR TO J SMITH REF PAYMENT"));
        let j_smith = lib_payees::find_by_name(&payees, "J Smith").expect("a stub Payee");
        assert_eq!(row.payee, Some(RowPayee::Existing(j_smith)));
        // J Smith has no default Category, so it still needs review.
        assert!(row.needs_review());
    }

    #[test]
    fn a_resolved_row_is_a_new_payee_or_matched_by_its_payee() {
        let resolved = |payee| ImportRow {
            payee: Some(payee),
            category: Some(CategoryId::seed(10)),
            ..match_line(&[], &line("ANYTHING"))
        };
        assert_eq!(
            resolved(RowPayee::Create("New".to_string())).status(),
            RowStatus::NewPayee
        );
        assert_eq!(
            resolved(RowPayee::Existing(PayeeId::seed(2))).status(),
            RowStatus::Matched
        );
    }

    #[test]
    fn inactive_payees_neither_match_nor_are_offered() {
        let mut payees = default_payees();
        lib_payees::set_active(&mut payees, PayeeId::seed(1), false).expect("a stub Payee");
        let row = match_line(&payees, &line("WOOLWORTHS 99"));
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
    fn payee_choices_label_a_rule_match_by_its_payee_and_find_a_choice_by_label() {
        let payees = default_payees();
        let offered = choices(&payees, "SP AUSSIE CANDLE CO");
        assert_eq!(
            offered.label_for(&RowPayee::Rule(PayeeId::seed(2))),
            Some("Coles")
        );
        assert_eq!(
            offered.choice_for("Coles"),
            Some(RowPayee::Existing(PayeeId::seed(2)))
        );
        assert_eq!(offered.choice_for("Nobody"), None);
    }

    #[test]
    fn rows_sharing_a_new_name_count_as_one_new_payee() {
        let payees = default_payees();
        let rows: Vec<ImportRow> = ["SP AUSSIE CANDLE CO", "SP AUSSIE CANDLE CO 2"]
            .iter()
            .map(|raw| match_line(&payees, &line(raw)))
            .collect();
        assert_eq!(summary(&rows).new_payees, 1);
    }
}
