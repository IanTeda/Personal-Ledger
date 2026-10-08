//! Shared stub Payees, seeded from the Payees handoff's sample list (`docs/ux/desktop/20-payees/`)
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

pub(crate) mod form;

use bigdecimal::BigDecimal;
use lib_core::Money;

use crate::{accounts::Account, transactions::Transaction};
use form::DeleteAction;

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
        // The Bills handoff's sample Bill Plans' Payees.
        payee(20, "Telstra", Some(HOUSEHOLD), &["TELSTRA"]),
        payee(21, "Fitness First", Some(HOUSEHOLD), &[]),
        payee(22, "AAMI", Some(TRANSPORT), &[]),
        payee(23, "Brisbane City Council", Some(HOUSEHOLD), &[]),
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

/// The Payee owning `alias` (as stored), if any.
pub fn alias_owner<'a>(payees: &'a [Payee], alias: &str) -> Option<&'a Payee> {
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

/// Whether a new Payee could take `name`: present, and neither another Payee's name nor alias.
pub fn name_available(payees: &[Payee], name: &str) -> bool {
    clean_name(payees, None, name).is_ok()
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
/// Payees A–Z, ignoring case: the Settings page's order (it has no usage columns to rank by).
pub fn sorted_by_name(payees: &[Payee]) -> Vec<&Payee> {
    let mut sorted: Vec<&Payee> = payees.iter().collect();
    sorted.sort_by(|a, b| {
        a.name
            .to_lowercase()
            .cmp(&b.name.to_lowercase())
            .then_with(|| a.id.cmp(&b.id))
    });
    sorted
}

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

/// Applies the dialog's [`DeleteAction`] to Payee `id`.
pub fn apply_delete_action(
    payees: &mut Vec<Payee>,
    transactions: &[Transaction],
    id: u32,
    action: DeleteAction,
) -> Result<(), PayeeError> {
    match action {
        DeleteAction::Delete => delete_payee(payees, transactions, id),
        DeleteAction::Deactivate => set_active(payees, id, false),
        DeleteAction::Reactivate => set_active(payees, id, true),
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
    fn the_delete_action_follows_references_and_activity() {
        let mut payees = default_payees();
        let transactions = seeded_transactions(&payees);
        let netflix = id_of(&payees, "Netflix");
        let j_smith = id_of(&payees, "J Smith");
        let action =
            |payees: &[Payee], id| DeleteAction::for_payee(get(payees, id).unwrap(), &transactions);

        assert_eq!(action(&payees, netflix), DeleteAction::Delete);
        assert_eq!(action(&payees, j_smith), DeleteAction::Deactivate);
        apply_delete_action(
            &mut payees,
            &transactions,
            j_smith,
            DeleteAction::Deactivate,
        )
        .unwrap();
        assert!(!get(&payees, j_smith).unwrap().is_active);
        assert_eq!(action(&payees, j_smith), DeleteAction::Reactivate);
        apply_delete_action(
            &mut payees,
            &transactions,
            j_smith,
            DeleteAction::Reactivate,
        )
        .unwrap();
        assert!(get(&payees, j_smith).unwrap().is_active);
        apply_delete_action(&mut payees, &transactions, netflix, DeleteAction::Delete).unwrap();
        assert_eq!(get(&payees, netflix), None);
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
}
