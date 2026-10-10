//! Shared stub Payees, seeded from the Payees handoff's sample list (`docs/ux/desktop-mockups/20-payees/`)
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

use lib_accounts::Account;
use lib_transactions::Transaction;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Payee {
    pub id: u32,
    /// The current, canonical name.
    pub name: String,
    /// Payee Aliases: former names and hand-authored match rules, normalised by [`normalise_alias`].
    pub aliases: Vec<String>,
    /// A leaf the Category id that pre-fills new Splits for this Payee.
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
pub fn clean_name(payees: &[Payee], own_id: Option<u32>, name: &str) -> Result<String, PayeeError> {
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

/// The Payees data behind the Desktop's Payees Entity: the rows, read through `payees` and changed
/// only through the rule-checked methods, so the rules above stay the way a name, alias or Split
/// reference changes. Each check runs before the rows are touched, so a refused change leaves them
/// as they were. Persistence is a later phase, so the rows are in memory.
pub struct PayeeService {
    payees: Vec<Payee>,
}

impl PayeeService {
    /// A service over `payees`, as the Desktop seeds it from [`default_payees`].
    pub fn new(payees: Vec<Payee>) -> Self {
        Self { payees }
    }

    pub fn payees(&self) -> &[Payee] {
        &self.payees
    }

    /// Adds a new, active Payee and returns its id; see [`insert_payee`].
    pub fn insert(&mut self, draft: &PayeeDraft) -> Result<u32, PayeeError> {
        insert_payee(&mut self.payees, draft)
    }

    /// Replaces Payee `id`'s fields with `draft`'s; see [`edit_payee`].
    pub fn edit(&mut self, id: u32, draft: &PayeeDraft) -> Result<(), PayeeError> {
        edit_payee(&mut self.payees, id, draft)
    }

    /// Hard-deletes Payee `id`, refused while a Split references it; see [`delete_payee`].
    pub fn delete(&mut self, id: u32, transactions: &[Transaction]) -> Result<(), PayeeError> {
        delete_payee(&mut self.payees, transactions, id)
    }

    /// Deactivates or reactivates Payee `id`; see [`set_active`].
    pub fn set_active(&mut self, id: u32, is_active: bool) -> Result<(), PayeeError> {
        set_active(&mut self.payees, id, is_active)
    }

    /// Replaces every row at once. Import's commit adds Payees and Aliases to a copy of the rows
    /// with the same rules, then writes the copy back in one step, since it also writes
    /// Transactions. This is the only bulk write, so keep new callers on the methods above.
    pub fn replace(&mut self, rows: Vec<Payee>) {
        self.payees = rows;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn service_hands_out_the_rows_it_was_seeded_with_and_keeps_writes() {
        let mut service = PayeeService::new(default_payees());
        let seeded = service.payees().len();
        assert!(seeded > 0);

        service.delete(1, &[]).unwrap();
        assert_eq!(service.payees().len(), seeded - 1);
        assert!(get(service.payees(), 1).is_none());
    }

    #[test]
    fn refused_writes_leave_the_rows_untouched() {
        let mut service = PayeeService::new(default_payees());
        let before = service.payees().to_vec();
        let taken = before[0].name.clone();
        let draft = PayeeDraft {
            name: taken,
            aliases: Vec::new(),
            default_category: None,
        };

        assert!(service.insert(&draft).is_err());
        assert!(service.edit(before[1].id, &draft).is_err());
        assert!(service.edit(u32::MAX, &draft).is_err());
        assert_eq!(service.payees(), before.as_slice());
    }
}
