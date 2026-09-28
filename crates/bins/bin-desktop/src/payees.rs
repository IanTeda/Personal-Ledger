//! Shared stub Payees, seeded from the Payees handoff's sample list (`docs/ux/desktop/Payees/`)
//! plus the names the Transactions seed data needs. `gpui`-free and side-effect free, so every
//! rule is unit-tested without a window.
//!
//! The rules are ADR-0012's and its two amendments (#282, #283):
//!
//! - A Payee's name is required and case-insensitively unique. Renaming one keeps the old name as
//!   a Payee Alias; renaming it to one of its own aliases drops that alias.
//! - A Payee Alias (the UI's "match rule") is stored trimmed and upper-cased, deduplicated, empty
//!   input ignored, and is unique across Payees. It matches raw text as a case-insensitive
//!   *contains*; the longest matching alias wins, then the Payee name alphabetically. Only active
//!   Payees match.
//! - A Payee no Split references may be hard-deleted with its aliases; a referenced one can only
//!   be deactivated (and reactivated). Inactive Payees drop out of the Payee counts.
//! - A default Category is an optional leaf that only pre-fills new Splits.

use bigdecimal::BigDecimal;
use lib_core::Money;

use crate::{
    accounts::{Account, SelectKey},
    categories::{self, Category},
    select::SelectState,
    transactions::Transaction,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Payee {
    pub id: u32,
    /// The current, canonical name.
    pub name: String,
    /// Payee Aliases: former names and hand-authored match rules, normalised by [`normalise_alias`].
    pub aliases: Vec<String>,
    /// A leaf [`crate::categories::Category::id`] that pre-fills new Splits for this Payee.
    pub default_category: Option<u32>,
    pub is_active: bool,
}

fn payee(id: u32, name: &str, default_category: Option<u32>, aliases: &[&str]) -> Payee {
    Payee {
        id,
        name: name.to_string(),
        aliases: aliases.iter().map(|alias| alias.to_string()).collect(),
        default_category,
        is_active: true,
    }
}

/// The handoff's sample Payees first (J Smith with no default Category), then the rest of the
/// names the Transactions seed draws on. Category ids are `categories::default_categories`' leaves.
pub fn default_payees() -> Vec<Payee> {
    const RENT: u32 = 2;
    const ELECTRICITY: u32 = 4;
    const WATER: u32 = 5;
    const GROCERIES: u32 = 7;
    const DINING: u32 = 8;
    const TRANSPORT: u32 = 9;
    const HOUSEHOLD: u32 = 10;
    const SALARY: u32 = 11;
    const INTEREST: u32 = 12;
    let mut payees = vec![
        payee(
            1,
            "Woolworths",
            Some(GROCERIES),
            &["WOOLWORTHS", "WW SUPERMARKET", "WOOLIES"],
        ),
        payee(2, "Coles", Some(GROCERIES), &["COLES", "COLES ONLINE"]),
        payee(3, "Netflix", Some(HOUSEHOLD), &["NETFLIX.COM"]),
        payee(4, "BP", Some(TRANSPORT), &["BP SERVICE STN", "BP CONNECT"]),
        payee(
            5,
            "Origin Energy",
            Some(ELECTRICITY),
            &["ORIGIN ENERGY", "ORIGIN DIRECT DEBIT"],
        ),
        payee(6, "Officeworks", Some(HOUSEHOLD), &["OFFICEWORKS"]),
        payee(
            7,
            "Employer Pty Ltd",
            Some(SALARY),
            &["EMPLOYER PTY LTD PAY"],
        ),
        payee(8, "J Smith", None, &[]),
        payee(9, "Aldi Kelvin Grove", Some(GROCERIES), &["ALDI"]),
        payee(10, "Kura Sushi", Some(DINING), &[]),
        payee(11, "Cafe Vittoria", Some(DINING), &[]),
        payee(12, "Uber", Some(TRANSPORT), &["UBER"]),
        payee(13, "Bunnings Warehouse", Some(HOUSEHOLD), &["BUNNINGS"]),
        payee(14, "Kmart", Some(HOUSEHOLD), &["KMART"]),
        payee(15, "Sydney Water", Some(WATER), &["SYDNEY WATER"]),
        payee(16, "ANZ Banking Group", Some(INTEREST), &["ANZ INTEREST"]),
        payee(17, "Hudson News", None, &[]),
        payee(18, "Don Quijote", None, &[]),
        payee(19, "Ray White Rentals", Some(RENT), &["RAY WHITE"]),
    ];
    // One inactive Payee, so the list's dimmed "inactive" state has something to show.
    if let Some(don_quijote) = payees.iter_mut().find(|p| p.id == 18) {
        don_quijote.is_active = false;
    }
    payees
}

/// The id of the Payee named `name` (exact, first match), if any.
pub fn find_by_name(payees: &[Payee], name: &str) -> Option<u32> {
    payees
        .iter()
        .find(|payee| payee.name == name)
        .map(|payee| payee.id)
}

pub fn get(payees: &[Payee], id: u32) -> Option<&Payee> {
    payees.iter().find(|payee| payee.id == id)
}

/// An alias as stored: trimmed and upper-cased, or `None` for empty or whitespace-only input.
pub fn normalise_alias(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_uppercase())
}

/// Errors for Payee operations.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PayeeError {
    #[error("a payee needs a name")]
    NameRequired,
    /// Another Payee already has this name (case-insensitively); carries that Payee's name.
    #[error("{0} already exists")]
    DuplicateName(String),
    /// Another Payee already owns this alias.
    #[error("{alias} is already a match rule on {owner}")]
    AliasTaken { alias: String, owner: String },
    /// A Split references the Payee, so it can only be deactivated.
    #[error("a payee in use can only be deactivated")]
    InUse,
    #[error("payee not found")]
    NotFound,
}

/// What the Add and Edit dialogs submit: the aliases as typed, normalised on the way in.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PayeeDraft {
    pub name: String,
    pub default_category: Option<u32>,
    pub aliases: Vec<String>,
}

/// Normalises and dedupes `aliases` (keeping first-seen order) and checks none belongs to another
/// Payee than `own_id`.
fn clean_aliases(
    payees: &[Payee],
    own_id: Option<u32>,
    aliases: &[String],
) -> Result<Vec<String>, PayeeError> {
    let mut cleaned: Vec<String> = Vec::new();
    for alias in aliases.iter().filter_map(|raw| normalise_alias(raw)) {
        if cleaned.contains(&alias) {
            continue;
        }
        if let Some(owner) = alias_owner(payees, &alias).filter(|p| Some(p.id) != own_id) {
            return Err(PayeeError::AliasTaken {
                alias,
                owner: owner.name.clone(),
            });
        }
        cleaned.push(alias);
    }
    Ok(cleaned)
}

fn alias_owner<'a>(payees: &'a [Payee], alias: &str) -> Option<&'a Payee> {
    payees
        .iter()
        .find(|payee| payee.aliases.iter().any(|a| a == alias))
}

/// Checks `name` is present, not another Payee's name and not another Payee's alias; returns it
/// trimmed.
fn clean_name(payees: &[Payee], own_id: Option<u32>, name: &str) -> Result<String, PayeeError> {
    let name = name.trim();
    if name.is_empty() {
        return Err(PayeeError::NameRequired);
    }
    let folded = name.to_lowercase();
    if let Some(other) = payees
        .iter()
        .find(|p| Some(p.id) != own_id && p.name.to_lowercase() == folded)
    {
        return Err(PayeeError::DuplicateName(other.name.clone()));
    }
    let as_alias = name.to_uppercase();
    if let Some(owner) = alias_owner(payees, &as_alias).filter(|p| Some(p.id) != own_id) {
        return Err(PayeeError::AliasTaken {
            alias: as_alias,
            owner: owner.name.clone(),
        });
    }
    Ok(name.to_string())
}

/// Adds a new, active Payee and returns its id.
pub fn insert_payee(payees: &mut Vec<Payee>, draft: &PayeeDraft) -> Result<u32, PayeeError> {
    let name = clean_name(payees, None, &draft.name)?;
    let aliases = clean_aliases(payees, None, &draft.aliases)?;
    let id = payees.iter().map(|p| p.id).max().unwrap_or(0) + 1;
    payees.push(Payee {
        id,
        name,
        aliases,
        default_category: draft.default_category,
        is_active: true,
    });
    Ok(id)
}

/// Replaces Payee `id`'s name, default Category and aliases with `draft`'s. A rename keeps the old
/// name as an alias, and drops the alias the new name equals.
pub fn edit_payee(payees: &mut [Payee], id: u32, draft: &PayeeDraft) -> Result<(), PayeeError> {
    let current = get(payees, id).ok_or(PayeeError::NotFound)?;
    let name = clean_name(payees, Some(id), &draft.name)?;
    let mut typed = draft.aliases.clone();
    if name != current.name {
        typed.push(current.name.clone());
    }
    let new_name_alias = name.to_uppercase();
    let mut aliases = clean_aliases(payees, Some(id), &typed)?;
    aliases.retain(|alias| *alias != new_name_alias);

    let payee = payees
        .iter_mut()
        .find(|p| p.id == id)
        .ok_or(PayeeError::NotFound)?;
    payee.name = name;
    payee.aliases = aliases;
    payee.default_category = draft.default_category;
    Ok(())
}

/// Hard-deletes Payee `id` and its aliases, only if no Split references it.
pub fn delete_payee(
    payees: &mut Vec<Payee>,
    transactions: &[Transaction],
    id: u32,
) -> Result<(), PayeeError> {
    get(payees, id).ok_or(PayeeError::NotFound)?;
    if is_referenced(transactions, id) {
        return Err(PayeeError::InUse);
    }
    payees.retain(|p| p.id != id);
    Ok(())
}

/// Deactivates or reactivates Payee `id`.
pub fn set_active(payees: &mut [Payee], id: u32, is_active: bool) -> Result<(), PayeeError> {
    let payee = payees
        .iter_mut()
        .find(|p| p.id == id)
        .ok_or(PayeeError::NotFound)?;
    payee.is_active = is_active;
    Ok(())
}

/// Whether any Split carries Payee `id`.
pub fn is_referenced(transactions: &[Transaction], id: u32) -> bool {
    transactions
        .iter()
        .flat_map(|t| &t.splits)
        .any(|split| split.payee_id == Some(id))
}

/// The active Payee whose alias `raw` contains (case-insensitively): the longest alias wins, then
/// the Payee name alphabetically.
pub fn match_alias(payees: &[Payee], raw: &str) -> Option<u32> {
    let haystack = raw.to_uppercase();
    payees
        .iter()
        .filter(|payee| payee.is_active)
        .flat_map(|payee| {
            payee
                .aliases
                .iter()
                .filter(|alias| haystack.contains(alias.as_str()))
                .map(move |alias| (alias.len(), payee))
        })
        .max_by(|(a_len, a), (b_len, b)| a_len.cmp(b_len).then_with(|| b.name.cmp(&a.name)))
        .map(|(_, payee)| payee.id)
}

/// A Payee's live figures for the TRANSACTIONS and TOTAL columns.
#[derive(Debug, Clone, PartialEq)]
pub struct PayeeUsage {
    /// Splits carrying the Payee, in any Unit and all time (what decides delete versus deactivate).
    pub splits: usize,
    /// The sum of those Splits whose Account is in the base Unit; other Units never mix in.
    pub total: Money,
}

pub fn usage(
    transactions: &[Transaction],
    accounts: &[Account],
    base_unit: Option<&str>,
    id: u32,
) -> PayeeUsage {
    let in_base_unit = |account_id: u32| {
        accounts
            .iter()
            .find(|a| a.id == account_id)
            .is_some_and(|a| base_unit == Some(a.unit.as_str()))
    };
    let mut splits = 0;
    let mut total = BigDecimal::new(0.into(), 2);
    for transaction in transactions {
        let counts = in_base_unit(transaction.account_id);
        for split in transaction.splits.iter().filter(|s| s.payee_id == Some(id)) {
            splits += 1;
            if counts {
                total += split.amount.0.clone();
            }
        }
    }
    PayeeUsage {
        splits,
        total: Money(total),
    }
}

/// Active Payees: the sidebar badge and the "N payees" figure.
pub fn active_count(payees: &[Payee]) -> usize {
    payees.iter().filter(|p| p.is_active).count()
}

/// Active Payees with no default Category: the "N without a default category" figure.
pub fn without_default_category_count(payees: &[Payee]) -> usize {
    payees
        .iter()
        .filter(|p| p.is_active && p.default_category.is_none())
        .count()
}

/// The Default category select's options: "none" first, then every leaf Category. `labels` are
/// what the select shows and stores; `ids` runs parallel to it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PayeeOptions {
    pub labels: Vec<String>,
    pub ids: Vec<Option<u32>>,
}

impl PayeeOptions {
    /// `none_label` heads the list. A leaf whose name another leaf shares is shown under its
    /// parent (`Parent › Leaf`), so the stored label always names exactly one Category.
    pub fn new(categories: &[Category], none_label: String) -> Self {
        let leaves: Vec<&Category> = categories
            .iter()
            .filter(|category| categories::is_leaf(categories, category.id))
            .collect();
        let mut labels = vec![none_label];
        let mut ids = vec![None];
        for leaf in &leaves {
            let shared = leaves
                .iter()
                .filter(|other| other.name == leaf.name)
                .count()
                > 1;
            let parent = leaf
                .parent
                .and_then(|id| categories.iter().find(|c| c.id == id));
            labels.push(match parent {
                Some(parent) if shared => format!("{} \u{203a} {}", parent.name, leaf.name),
                _ => leaf.name.clone(),
            });
            ids.push(Some(leaf.id));
        }
        Self { labels, ids }
    }

    fn label_for(&self, id: Option<u32>) -> Option<String> {
        self.ids
            .iter()
            .position(|candidate| *candidate == id)
            .map(|index| self.labels[index].clone())
    }

    fn id_for(&self, label: &str) -> Option<u32> {
        self.labels
            .iter()
            .position(|candidate| candidate == label)
            .and_then(|index| self.ids[index])
    }
}

/// The Add and Edit payee dialogs' fields, in `Tab` order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PayeeField {
    #[default]
    Name,
    DefaultCategory,
    /// The match-rule input beside **+ add**.
    Rule,
}

impl PayeeField {
    const ORDER: [PayeeField; 3] = [Self::Name, Self::DefaultCategory, Self::Rule];
}

/// The Add and Edit payee dialogs' live form state -- pure, `gpui`-free. Name and the rule input
/// are typed into (append/pop only, like the other dialogs); Default category is a
/// [`SelectState`]; the rules are chips, added from the input and removed by their `✕`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PayeeForm {
    pub name: String,
    pub default_category: SelectState,
    /// The chips, normalised by [`normalise_alias`] as they are added.
    pub rules: Vec<String>,
    pub rule_input: String,
    pub focused: PayeeField,
    /// The last rejected rule or submit, shown inline until the offending text changes.
    pub error: Option<PayeeError>,
}

impl PayeeForm {
    /// A fresh Add form: no name, no default Category, no rules.
    pub fn new(options: &PayeeOptions) -> Self {
        Self {
            name: String::new(),
            default_category: SelectState::new(options.label_for(None)),
            rules: Vec::new(),
            rule_input: String::new(),
            focused: PayeeField::Name,
            error: None,
        }
    }

    /// A form pre-filled from `payee`, for the Edit dialog.
    pub fn from_payee(payee: &Payee, options: &PayeeOptions) -> Self {
        Self {
            name: payee.name.clone(),
            default_category: SelectState::new(
                options
                    .label_for(payee.default_category)
                    .or_else(|| options.label_for(None)),
            ),
            rules: payee.aliases.clone(),
            ..Self::new(options)
        }
    }

    /// What the dialog submits.
    pub fn draft(&self, options: &PayeeOptions) -> PayeeDraft {
        PayeeDraft {
            name: self.name.clone(),
            default_category: self
                .default_category
                .value()
                .and_then(|label| options.id_for(label)),
            aliases: self.rules.clone(),
        }
    }

    /// The name's problem, if any, checked live against every other Payee: `own_id` is the Payee
    /// being edited. An empty name is not reported, only kept from submitting.
    pub fn name_error(&self, payees: &[Payee], own_id: Option<u32>) -> Option<PayeeError> {
        if self.name.trim().is_empty() {
            return None;
        }
        clean_name(payees, own_id, &self.name).err()
    }

    pub fn is_valid(&self, payees: &[Payee], own_id: Option<u32>) -> bool {
        clean_name(payees, own_id, &self.name).is_ok()
    }

    /// Turns the rule input into a chip: normalised, ignored when blank or already a chip, and
    /// refused (kept in the input, with [`Self::error`] set) when another Payee owns it.
    pub fn add_rule(&mut self, payees: &[Payee], own_id: Option<u32>) {
        let Some(rule) = normalise_alias(&self.rule_input) else {
            self.rule_input.clear();
            return;
        };
        if let Some(owner) = alias_owner(payees, &rule).filter(|p| Some(p.id) != own_id) {
            self.error = Some(PayeeError::AliasTaken {
                alias: rule,
                owner: owner.name.clone(),
            });
            return;
        }
        if !self.rules.contains(&rule) {
            self.rules.push(rule);
        }
        self.rule_input.clear();
        self.error = None;
    }

    pub fn remove_rule(&mut self, index: usize) {
        if index < self.rules.len() {
            self.rules.remove(index);
        }
    }

    /// Moves focus to `field`, closing the select's list when leaving it.
    pub fn focus(&mut self, field: PayeeField) {
        if field != self.focused {
            self.default_category.cancel();
        }
        self.focused = field;
    }

    /// `Tab` / `Shift-Tab`: commits an open list's highlight, then moves to the next field.
    pub fn cycle_focus(&mut self, backward: bool, options: &PayeeOptions) {
        self.default_category.commit(&options.labels);
        let count = PayeeField::ORDER.len();
        let index = PayeeField::ORDER
            .iter()
            .position(|field| *field == self.focused)
            .unwrap_or(0);
        let next = if backward {
            (index + count - 1) % count
        } else {
            (index + 1) % count
        };
        self.focused = PayeeField::ORDER[next];
    }

    /// A key on the Default category select. Returns whether it is focused (and so took the key).
    pub fn handle_select_key(&mut self, key: SelectKey, options: &PayeeOptions) -> bool {
        if self.focused != PayeeField::DefaultCategory {
            return false;
        }
        let list = &options.labels;
        let state = &mut self.default_category;
        match (key, state.is_open()) {
            (SelectKey::Up, true) => state.move_highlight(list, -1),
            (SelectKey::Down, true) => state.move_highlight(list, 1),
            (SelectKey::Up, false) => state.step(list, -1),
            (SelectKey::Down, false) => state.step(list, 1),
            (SelectKey::Activate, true) => state.commit(list),
            (SelectKey::Activate, false) => state.open(list),
        }
        true
    }

    /// A click on the select's closed field: focuses it and toggles its list.
    pub fn click_select(&mut self, options: &PayeeOptions) {
        self.focus(PayeeField::DefaultCategory);
        if self.default_category.is_open() {
            self.default_category.cancel();
        } else {
            self.default_category.open(&options.labels);
        }
    }

    /// Closes the select's list if open -- the first `Esc`. Returns whether it was open.
    pub fn close_open_select(&mut self) -> bool {
        let was_open = self.default_category.is_open();
        self.default_category.cancel();
        was_open
    }

    /// Types `ch` into the focused text field.
    pub fn push_char(&mut self, ch: char) {
        if ch.is_control() {
            return;
        }
        match self.focused {
            PayeeField::Name => self.name.push(ch),
            PayeeField::Rule => self.rule_input.push(ch),
            PayeeField::DefaultCategory => return,
        }
        self.error = None;
    }

    /// Deletes from the focused text field; in an empty rule input, removes the last chip.
    pub fn backspace(&mut self) {
        match self.focused {
            PayeeField::Name => {
                self.name.pop();
            }
            PayeeField::Rule => {
                if self.rule_input.pop().is_none() {
                    self.rules.pop();
                }
            }
            PayeeField::DefaultCategory => return,
        }
        self.error = None;
    }
}

/// Which Payees dialog is open, following `AccountsDialog`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PayeesDialog {
    Add(PayeeForm),
    /// Editing the Payee with this [`Payee::id`].
    Edit(u32, PayeeForm),
    /// Deleting (or, when referenced, deactivating) the Payee with this [`Payee::id`].
    Delete(u32),
}

impl PayeesDialog {
    /// The form behind the Add and Edit dialogs.
    pub fn form_mut(&mut self) -> Option<&mut PayeeForm> {
        match self {
            Self::Add(form) | Self::Edit(_, form) => Some(form),
            Self::Delete(..) => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;

    use super::*;
    use crate::{
        accounts::default_accounts, categories, categories::default_categories, tags::default_tags,
        transactions::default_transactions,
    };

    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 9, 19).unwrap()
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

    fn draft(name: &str, aliases: &[&str]) -> PayeeDraft {
        PayeeDraft {
            name: name.to_string(),
            default_category: None,
            aliases: aliases.iter().map(|a| a.to_string()).collect(),
        }
    }

    fn id_of(payees: &[Payee], name: &str) -> u32 {
        find_by_name(payees, name).unwrap()
    }

    #[test]
    fn ids_names_and_aliases_are_unique() {
        let payees = default_payees();
        let mut ids: Vec<_> = payees.iter().map(|p| p.id).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), payees.len());
        let mut names: Vec<_> = payees.iter().map(|p| p.name.to_lowercase()).collect();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), payees.len());
        let mut aliases: Vec<_> = payees.iter().flat_map(|p| p.aliases.clone()).collect();
        let count = aliases.len();
        aliases.sort();
        aliases.dedup();
        assert_eq!(aliases.len(), count);
    }

    #[test]
    fn seeded_aliases_are_normalised_and_default_categories_are_leaves() {
        let categories = default_categories();
        for payee in default_payees() {
            for alias in &payee.aliases {
                assert_eq!(normalise_alias(alias).as_ref(), Some(alias));
            }
            if let Some(category) = payee.default_category {
                assert!(categories::is_leaf(&categories, category), "{}", payee.name);
            }
        }
    }

    #[test]
    fn find_by_name_is_exact_and_aliases_are_not_names() {
        let payees = default_payees();
        assert_eq!(find_by_name(&payees, "Woolworths"), Some(1));
        assert_eq!(find_by_name(&payees, "WOOLIES"), None);
        assert!(payees[0].aliases.contains(&"WOOLIES".to_string()));
    }

    #[test]
    fn normalise_alias_trims_upper_cases_and_ignores_blank() {
        assert_eq!(normalise_alias("  ww metro "), Some("WW METRO".to_string()));
        assert_eq!(normalise_alias("   "), None);
        assert_eq!(normalise_alias(""), None);
    }

    #[test]
    fn insert_normalises_and_dedupes_aliases_and_assigns_a_fresh_id() {
        let mut payees = default_payees();
        let max = payees.iter().map(|p| p.id).max().unwrap();
        let mut new = draft(
            " Aussie Candle Co ",
            &["aussie candle", "AUSSIE CANDLE ", " ", ""],
        );
        new.default_category = Some(10);
        let id = insert_payee(&mut payees, &new).unwrap();
        assert_eq!(id, max + 1);
        let payee = get(&payees, id).unwrap();
        assert_eq!(payee.name, "Aussie Candle Co");
        assert_eq!(payee.aliases, vec!["AUSSIE CANDLE".to_string()]);
        assert_eq!(payee.default_category, Some(10));
        assert!(payee.is_active);
    }

    #[test]
    fn insert_requires_a_name_unique_case_insensitively() {
        let mut payees = default_payees();
        assert_eq!(
            insert_payee(&mut payees, &draft("  ", &[])),
            Err(PayeeError::NameRequired)
        );
        assert_eq!(
            insert_payee(&mut payees, &draft("woolworths", &[])),
            Err(PayeeError::DuplicateName("Woolworths".to_string()))
        );
    }

    #[test]
    fn an_alias_another_payee_owns_is_rejected_naming_the_owner() {
        let mut payees = default_payees();
        assert_eq!(
            insert_payee(&mut payees, &draft("Woolies Metro", &["woolies"])),
            Err(PayeeError::AliasTaken {
                alias: "WOOLIES".to_string(),
                owner: "Woolworths".to_string(),
            })
        );
    }

    #[test]
    fn a_name_equal_to_another_payees_alias_is_rejected() {
        let mut payees = default_payees();
        assert_eq!(
            insert_payee(&mut payees, &draft("Woolies", &[])),
            Err(PayeeError::AliasTaken {
                alias: "WOOLIES".to_string(),
                owner: "Woolworths".to_string(),
            })
        );
    }

    #[test]
    fn edit_replaces_category_and_aliases() {
        let mut payees = default_payees();
        let coles = id_of(&payees, "Coles");
        let mut change = draft("Coles", &["coles express"]);
        change.default_category = Some(10);
        edit_payee(&mut payees, coles, &change).unwrap();
        let payee = get(&payees, coles).unwrap();
        assert_eq!(payee.aliases, vec!["COLES EXPRESS".to_string()]);
        assert_eq!(payee.default_category, Some(10));
    }

    #[test]
    fn a_rename_keeps_the_old_name_as_an_alias() {
        let mut payees = default_payees();
        let kmart = id_of(&payees, "Kmart");
        edit_payee(&mut payees, kmart, &draft("Kmart AU", &["KMART"])).unwrap();
        let payee = get(&payees, kmart).unwrap();
        assert_eq!(payee.name, "Kmart AU");
        // The old name normalises to the alias already there, so it isn't duplicated.
        assert_eq!(payee.aliases, vec!["KMART".to_string()]);

        let uber = id_of(&payees, "Uber");
        edit_payee(&mut payees, uber, &draft("Uber Eats", &[])).unwrap();
        assert_eq!(
            get(&payees, uber).unwrap().aliases,
            vec!["UBER".to_string()]
        );
    }

    #[test]
    fn renaming_to_an_own_alias_drops_that_alias() {
        let mut payees = default_payees();
        let woolworths = id_of(&payees, "Woolworths");
        let aliases: Vec<&str> = vec!["WOOLWORTHS", "WW SUPERMARKET", "WOOLIES"];
        edit_payee(&mut payees, woolworths, &draft("Woolies", &aliases)).unwrap();
        let payee = get(&payees, woolworths).unwrap();
        assert_eq!(payee.name, "Woolies");
        assert_eq!(
            payee.aliases,
            vec!["WOOLWORTHS".to_string(), "WW SUPERMARKET".to_string()]
        );
    }

    #[test]
    fn renaming_to_another_payees_name_or_alias_is_rejected() {
        let mut payees = default_payees();
        let coles = id_of(&payees, "Coles");
        assert_eq!(
            edit_payee(&mut payees, coles, &draft("bp", &[])),
            Err(PayeeError::DuplicateName("BP".to_string()))
        );
        assert_eq!(
            edit_payee(&mut payees, coles, &draft("Woolies", &[])),
            Err(PayeeError::AliasTaken {
                alias: "WOOLIES".to_string(),
                owner: "Woolworths".to_string(),
            })
        );
        assert_eq!(get(&payees, coles).unwrap().name, "Coles");
    }

    #[test]
    fn editing_keeping_the_same_name_in_another_case_is_allowed() {
        let mut payees = default_payees();
        let bp = id_of(&payees, "BP");
        edit_payee(&mut payees, bp, &draft("Bp", &[])).unwrap();
        // A change of case only is not a rename: the old name would equal the new one's alias.
        let payee = get(&payees, bp).unwrap();
        assert_eq!(payee.name, "Bp");
        assert!(payee.aliases.is_empty());
    }

    #[test]
    fn an_unreferenced_payee_hard_deletes_and_a_referenced_one_is_in_use() {
        let mut payees = default_payees();
        let transactions = seeded_transactions(&payees);
        let netflix = id_of(&payees, "Netflix");
        let j_smith = id_of(&payees, "J Smith");
        assert!(!is_referenced(&transactions, netflix));
        assert_eq!(
            delete_payee(&mut payees, &transactions, j_smith),
            Err(PayeeError::InUse)
        );
        delete_payee(&mut payees, &transactions, netflix).unwrap();
        assert_eq!(get(&payees, netflix), None);
        assert_eq!(
            delete_payee(&mut payees, &transactions, netflix),
            Err(PayeeError::NotFound)
        );
    }

    #[test]
    fn set_active_toggles_and_counts_leave_inactive_payees_out() {
        let mut payees = default_payees();
        let active = active_count(&payees);
        let without = without_default_category_count(&payees);
        assert_eq!(active, payees.len() - 1);
        assert_eq!(without, 2); // J Smith and Hudson News; Don Quijote is inactive.

        let j_smith = id_of(&payees, "J Smith");
        set_active(&mut payees, j_smith, false).unwrap();
        assert_eq!(active_count(&payees), active - 1);
        assert_eq!(without_default_category_count(&payees), without - 1);
        set_active(&mut payees, j_smith, true).unwrap();
        assert_eq!(active_count(&payees), active);
        assert_eq!(
            set_active(&mut payees, 999, true),
            Err(PayeeError::NotFound)
        );
    }

    #[test]
    fn match_alias_is_a_case_insensitive_contains() {
        let payees = default_payees();
        assert_eq!(
            match_alias(&payees, "woolworths 2137 sydney au"),
            Some(id_of(&payees, "Woolworths"))
        );
        assert_eq!(
            match_alias(&payees, "NETFLIX.COM 866-579-7172"),
            Some(id_of(&payees, "Netflix"))
        );
        assert_eq!(match_alias(&payees, "SP AUSSIE CANDLE CO"), None);
    }

    #[test]
    fn the_longest_alias_wins_then_the_name_alphabetically() {
        let mut payees = default_payees();
        // "BP SERVICE STN" beats a shorter alias another Payee holds.
        insert_payee(&mut payees, &draft("Bpay Biller", &["BP SERV"])).unwrap();
        assert_eq!(
            match_alias(&payees, "BP SERVICE STN 4471"),
            Some(id_of(&payees, "BP"))
        );
        // Equal lengths: the alphabetically first name wins.
        insert_payee(&mut payees, &draft("Zed", &["ZZZZ"])).unwrap();
        insert_payee(&mut payees, &draft("Abe", &["YYYY"])).unwrap();
        assert_eq!(
            match_alias(&payees, "ZZZZ YYYY"),
            Some(id_of(&payees, "Abe"))
        );
    }

    #[test]
    fn inactive_payees_do_not_match() {
        let mut payees = default_payees();
        let bp = id_of(&payees, "BP");
        set_active(&mut payees, bp, false).unwrap();
        assert_eq!(match_alias(&payees, "BP SERVICE STN 4471"), None);
    }

    #[test]
    fn usage_counts_every_split_and_totals_base_unit_ones() {
        let payees = default_payees();
        let accounts = default_accounts();
        let transactions = seeded_transactions(&payees);
        let j_smith = id_of(&payees, "J Smith");
        let j_smith_usage = usage(&transactions, &accounts, Some("aud"), j_smith);
        assert_eq!(j_smith_usage.splits, 4);
        assert_eq!(
            j_smith_usage.total,
            Money(BigDecimal::new((-64_000).into(), 2))
        );

        let other_unit = usage(&transactions, &accounts, Some("xyz"), j_smith);
        assert_eq!(other_unit.splits, 4);
        assert_eq!(other_unit.total, Money(BigDecimal::new(0.into(), 2)));
    }

    fn options() -> PayeeOptions {
        PayeeOptions::new(&default_categories(), "none".to_string())
    }

    #[test]
    fn options_lead_with_none_then_only_leaves() {
        let categories = default_categories();
        let options = options();
        assert_eq!(options.labels[0], "none");
        assert_eq!(options.ids[0], None);
        assert_eq!(options.labels.len(), options.ids.len());
        for id in options.ids.iter().skip(1).flatten() {
            assert!(categories::is_leaf(&categories, *id));
        }
        assert!(options.labels.contains(&"Groceries".to_string()));
    }

    #[test]
    fn a_fresh_form_drafts_no_category_and_no_rules() {
        let options = options();
        let mut form = PayeeForm::new(&options);
        assert_eq!(form.default_category.value(), Some("none"));
        form.name = "Aussie Candle Co".to_string();
        assert_eq!(
            form.draft(&options),
            PayeeDraft {
                name: "Aussie Candle Co".to_string(),
                default_category: None,
                aliases: Vec::new(),
            }
        );
    }

    #[test]
    fn the_select_picks_a_leaf_category_into_the_draft() {
        let options = options();
        let mut form = PayeeForm::new(&options);
        form.focus(PayeeField::DefaultCategory);
        assert!(form.handle_select_key(SelectKey::Down, &options));
        assert_eq!(form.draft(&options).default_category, options.ids[1]);
        assert!(form.handle_select_key(SelectKey::Activate, &options));
        assert!(form.default_category.is_open());
        assert!(form.close_open_select());
        assert!(!form.close_open_select());
    }

    #[test]
    fn select_keys_are_ignored_off_the_select() {
        let options = options();
        let mut form = PayeeForm::new(&options);
        assert!(!form.handle_select_key(SelectKey::Down, &options));
        assert_eq!(form.default_category.value(), Some("none"));
    }

    #[test]
    fn tab_cycles_the_three_fields_both_ways() {
        let options = options();
        let mut form = PayeeForm::new(&options);
        form.cycle_focus(false, &options);
        assert_eq!(form.focused, PayeeField::DefaultCategory);
        form.cycle_focus(false, &options);
        assert_eq!(form.focused, PayeeField::Rule);
        form.cycle_focus(false, &options);
        assert_eq!(form.focused, PayeeField::Name);
        form.cycle_focus(true, &options);
        assert_eq!(form.focused, PayeeField::Rule);
    }

    #[test]
    fn add_rule_normalises_dedupes_and_ignores_blank() {
        let payees = default_payees();
        let mut form = PayeeForm::new(&options());
        form.rule_input = " aussie candle ".to_string();
        form.add_rule(&payees, None);
        form.rule_input = "AUSSIE CANDLE".to_string();
        form.add_rule(&payees, None);
        form.rule_input = "   ".to_string();
        form.add_rule(&payees, None);
        assert_eq!(form.rules, vec!["AUSSIE CANDLE".to_string()]);
        assert!(form.rule_input.is_empty());
        assert_eq!(form.error, None);
    }

    #[test]
    fn add_rule_refuses_another_payees_alias_and_keeps_the_input() {
        let payees = default_payees();
        let mut form = PayeeForm::new(&options());
        form.rule_input = "woolies".to_string();
        form.add_rule(&payees, None);
        assert!(form.rules.is_empty());
        assert_eq!(form.rule_input, "woolies");
        assert_eq!(
            form.error,
            Some(PayeeError::AliasTaken {
                alias: "WOOLIES".to_string(),
                owner: "Woolworths".to_string(),
            })
        );
        form.push_char('!');
        assert_eq!(form.error, None);

        // Editing Woolworths itself, its own alias is fine.
        form.rule_input = "woolies".to_string();
        form.add_rule(&payees, Some(id_of(&payees, "Woolworths")));
        assert_eq!(form.rules, vec!["WOOLIES".to_string()]);
    }

    #[test]
    fn backspace_in_an_empty_rule_input_removes_the_last_chip() {
        let payees = default_payees();
        let mut form = PayeeForm::new(&options());
        form.focus(PayeeField::Rule);
        for rule in ["ONE", "TWO"] {
            form.rule_input = rule.to_string();
            form.add_rule(&payees, None);
        }
        form.push_char('x');
        form.backspace();
        assert_eq!(form.rules.len(), 2);
        form.backspace();
        assert_eq!(form.rules, vec!["ONE".to_string()]);
        form.remove_rule(0);
        form.remove_rule(5);
        assert!(form.rules.is_empty());
    }

    #[test]
    fn the_name_is_checked_live_but_blank_is_only_invalid() {
        let payees = default_payees();
        let mut form = PayeeForm::new(&options());
        assert_eq!(form.name_error(&payees, None), None);
        assert!(!form.is_valid(&payees, None));
        form.name = "coles".to_string();
        assert_eq!(
            form.name_error(&payees, None),
            Some(PayeeError::DuplicateName("Coles".to_string()))
        );
        assert!(!form.is_valid(&payees, None));
        assert!(form.is_valid(&payees, Some(id_of(&payees, "Coles"))));
        form.name = "Aussie Candle Co".to_string();
        assert!(form.is_valid(&payees, None));
    }

    #[test]
    fn from_payee_prefills_name_category_and_rules() {
        let payees = default_payees();
        let options = options();
        let woolworths = get(&payees, id_of(&payees, "Woolworths")).unwrap();
        let form = PayeeForm::from_payee(woolworths, &options);
        assert_eq!(form.name, "Woolworths");
        assert_eq!(form.default_category.value(), Some("Groceries"));
        assert_eq!(form.rules, woolworths.aliases);
        assert_eq!(form.draft(&options).default_category, Some(7));
    }
}
