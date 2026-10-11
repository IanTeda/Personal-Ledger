//! The Tags model, seeded service and rules, shared by the Desktop and TUI.
//!
//! Shared stub Tags, seeded from the Tags handoff's sample list (`docs/ux/desktop-mockups/19-tags/`) plus the
//! names the Transactions seed draws on. `gpui`-free and side-effect free, so every rule is
//! unit-tested without a window.
//!
//! The rules are ADR-0015's and its amendments (#352–#355):
//!
//! - A Tag's name is required and unique by its [`normalise`]d form (Unicode lower-cased, letters
//!   and digits only), so `work trip` is refused while `work-trip` exists. A name that normalises to
//!   nothing is refused too. Typing a Tag on a Split reuses an exact case-insensitive match.
//! - A Tag has an optional colour: the user's data, not a Colour Theme role.
//! - Removing a Tag always hard-deletes it, used or not: it untags every Split and never touches
//!   their amounts, Categories or Payees. `is_active` is a separate toggle; an inactive Tag drops
//!   out of the "N tags" count but keeps its name taken.
//! - Merging moves every Split from the source to the target (without doubling a Split that
//!   already carries the target) and deletes the source; the target keeps its name, colour and
//!   `is_active`. Likely duplicates exist only in data that broke the uniqueness rule (e.g. from
//!   Sync), so the seed carries one such pair on purpose.
//! - Usage is all time across every Account: distinct Transactions, the newest one's date, and a
//!   total that only sums when every Split carrying the Tag shares one Unit.

use std::collections::HashMap;

use bigdecimal::BigDecimal;
use chrono::NaiveDate;
use lib_core::{HexColor, Money};

use lib_accounts::Account;
use lib_core::Total;
use lib_transactions::Transaction;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tag {
    pub id: u32,
    pub name: String,
    /// `None` shows no swatch.
    pub color: Option<HexColor>,
    pub is_active: bool,
}

/// The picker's preset swatches, in the handoff's order; any other hex value is allowed too.
pub fn swatches() -> [HexColor; 6] {
    [
        HexColor::from_rgb(0xec, 0x30, 0x13),
        HexColor::from_rgb(0x7a, 0x8a, 0x6b),
        HexColor::from_rgb(0xc9, 0x92, 0x2f),
        HexColor::from_rgb(0x4a, 0x7c, 0x9e),
        HexColor::from_rgb(0x8a, 0x6b, 0xb5),
        HexColor::from_rgb(0x9b, 0x97, 0x97),
    ]
}

/// The handoff's sample Tags (with `tax-deductible` hyphenated as it shows them), the Tokyo
/// receipt's "Japan Trip 2026", one inactive Tag and `Work Trip`: a likely duplicate of
/// `work-trip` that bypasses the uniqueness rule on purpose, so 7a's flag and 7e's merge have
/// something to show.
pub fn default_tags() -> Vec<Tag> {
    let [red, sage, ochre, blue, violet, _grey] = swatches();
    [
        ("Japan Trip 2026", Some(red.clone()), true),
        ("shared", Some(blue), true),
        ("reimbursable", Some(sage), true),
        ("tax-deductible", Some(ochre), true),
        ("work-trip", Some(violet), true),
        ("gift", Some(red), true),
        ("one-off", None, true),
        ("Bali 2025", None, false),
        ("Work Trip", None, true),
    ]
    .into_iter()
    .zip(1u32..)
    .map(|((name, color, is_active), id)| Tag {
        id,
        name: name.to_string(),
        color,
        is_active,
    })
    .collect()
}

/// The form a Tag name is unique by: Unicode lower-cased, keeping only letters and digits.
pub fn normalise(name: &str) -> String {
    name.chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

/// The id of the Tag named `name`, ignoring case (not spaces or punctuation), if any.
pub fn find_by_name(tags: &[Tag], name: &str) -> Option<u32> {
    let folded = name.trim().to_lowercase();
    tags.iter()
        .find(|tag| tag.name.to_lowercase() == folded)
        .map(|tag| tag.id)
}

pub fn get(tags: &[Tag], id: u32) -> Option<&Tag> {
    tags.iter().find(|tag| tag.id == id)
}

/// Errors for Tag operations.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum TagError {
    #[error("a tag needs a name")]
    NameRequired,
    /// The name normalises to nothing: it has no letter or digit.
    #[error("a tag name needs a letter or a digit")]
    NoLetterOrDigit,
    /// Another Tag already has this name once normalised; carries that Tag's name.
    #[error("{0} already exists")]
    DuplicateName(String),
    #[error("a tag can't be merged into itself")]
    MergeIntoItself,
    #[error("tag not found")]
    NotFound,
}

/// What the Add and Edit dialogs submit.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TagDraft {
    pub name: String,
    pub color: Option<HexColor>,
}

/// Checks `name` is present, has a letter or digit and isn't another Tag's once normalised;
/// returns it trimmed. `own_id` is the Tag being edited.
fn clean_name(tags: &[Tag], own_id: Option<u32>, name: &str) -> Result<String, TagError> {
    let name = name.trim();
    if name.is_empty() {
        return Err(TagError::NameRequired);
    }
    let normalised = normalise(name);
    if normalised.is_empty() {
        return Err(TagError::NoLetterOrDigit);
    }
    if let Some(other) = tags
        .iter()
        .find(|t| Some(t.id) != own_id && normalise(&t.name) == normalised)
    {
        return Err(TagError::DuplicateName(other.name.clone()));
    }
    Ok(name.to_string())
}

/// The problem with `name` for Tag `own_id` (or a new Tag), if any; an empty name is reported too,
/// so a dialog decides for itself whether to show that one.
pub fn name_error(tags: &[Tag], own_id: Option<u32>, name: &str) -> Option<TagError> {
    clean_name(tags, own_id, name).err()
}

/// Adds a new, active Tag and returns its id.
pub fn insert_tag(tags: &mut Vec<Tag>, draft: &TagDraft) -> Result<u32, TagError> {
    let name = clean_name(tags, None, &draft.name)?;
    let id = tags.iter().map(|t| t.id).max().unwrap_or(0) + 1;
    tags.push(Tag {
        id,
        name,
        color: draft.color.clone(),
        is_active: true,
    });
    Ok(id)
}

/// Renames and recolours Tag `id` in place: Splits refer to it by id, so nothing is re-tagged.
pub fn edit_tag(tags: &mut [Tag], id: u32, draft: &TagDraft) -> Result<(), TagError> {
    get(tags, id).ok_or(TagError::NotFound)?;
    let name = clean_name(tags, Some(id), &draft.name)?;
    let tag = tags
        .iter_mut()
        .find(|t| t.id == id)
        .ok_or(TagError::NotFound)?;
    tag.name = name;
    tag.color = draft.color.clone();
    Ok(())
}

/// Deactivates or reactivates Tag `id`.
pub fn set_active(tags: &mut [Tag], id: u32, is_active: bool) -> Result<(), TagError> {
    let tag = tags
        .iter_mut()
        .find(|t| t.id == id)
        .ok_or(TagError::NotFound)?;
    tag.is_active = is_active;
    Ok(())
}

/// Untags every Split carrying Tag `id` and deletes it, used or not. Amounts, Categories and
/// Payees are untouched.
pub fn remove_tag(
    tags: &mut Vec<Tag>,
    transactions: &mut [Transaction],
    id: u32,
) -> Result<(), TagError> {
    get(tags, id).ok_or(TagError::NotFound)?;
    for split in transactions.iter_mut().flat_map(|t| &mut t.splits) {
        split.tag_ids.retain(|tag| *tag != id);
    }
    tags.retain(|t| t.id != id);
    Ok(())
}

/// Folds `source` into `target` as one operation: each Split carrying the source carries the
/// target instead (once, if it already had it), then the source is deleted. The target keeps its
/// name, colour and `is_active`; either may be inactive.
pub fn merge_tags(
    tags: &mut Vec<Tag>,
    transactions: &mut [Transaction],
    source: u32,
    target: u32,
) -> Result<(), TagError> {
    if source == target {
        return Err(TagError::MergeIntoItself);
    }
    get(tags, source).ok_or(TagError::NotFound)?;
    get(tags, target).ok_or(TagError::NotFound)?;
    for split in transactions.iter_mut().flat_map(|t| &mut t.splits) {
        if let Some(index) = split.tag_ids.iter().position(|tag| *tag == source) {
            if split.tag_ids.contains(&target) {
                split.tag_ids.remove(index);
            } else {
                split.tag_ids[index] = target;
            }
        }
    }
    tags.retain(|t| t.id != source);
    Ok(())
}

/// What typing a Tag name on a Split resolves to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SplitTag {
    /// An exact case-insensitive match: reuse that Tag.
    Existing(u32),
    /// Nothing takes the name: a new Tag would be created.
    New,
}

/// Resolves a Tag name typed on a Split: an exact case-insensitive match is reused, while a name
/// only equal once normalised (`work trip` beside `work-trip`) is refused as taken.
pub fn resolve_split_tag(tags: &[Tag], typed: &str) -> Result<SplitTag, TagError> {
    if let Some(id) = find_by_name(tags, typed) {
        return Ok(SplitTag::Existing(id));
    }
    clean_name(tags, None, typed).map(|_| SplitTag::New)
}

/// Active Tags: the sidebar badge and the "N tags" figure.
pub fn active_count(tags: &[Tag]) -> usize {
    tags.iter().filter(|t| t.is_active).count()
}

/// Whether any Split on `transaction` carries Tag `id`.
fn carries(transaction: &Transaction, id: u32) -> bool {
    transaction
        .splits
        .iter()
        .any(|split| split.tag_ids.contains(&id))
}

/// Distinct Transactions carrying Tag `id`: the TRANSACTIONS column, the figure the Remove and
/// Merge dialogs quote, and the sort key.
pub fn transaction_count(transactions: &[Transaction], id: u32) -> usize {
    transactions.iter().filter(|t| carries(t, id)).count()
}

/// A Tag's live figures for the TRANSACTIONS, TOTAL and LAST USED columns.
#[derive(Debug, Clone, PartialEq)]
pub struct TagUsage {
    /// Distinct Transactions: two Splits on one Transaction count once.
    pub transactions: usize,
    /// The sum of the Splits carrying the Tag, when they share one Unit; [`Total::Mixed`] when
    /// they don't, [`Total::Empty`] for an unused Tag.
    pub total: Total,
    /// The newest carrying Transaction's date, future-dated ones included.
    pub last_used: Option<NaiveDate>,
}

pub fn usage(transactions: &[Transaction], accounts: &[Account], id: u32) -> TagUsage {
    let unit_of = |account_id: u32| {
        accounts
            .iter()
            .find(|a| a.id == account_id)
            .map(|a| a.unit.as_str())
    };
    let mut count = 0;
    let mut last_used: Option<NaiveDate> = None;
    let mut unit: Option<&str> = None;
    let mut mixed = false;
    let mut sum = BigDecimal::new(0.into(), 2);
    for transaction in transactions.iter().filter(|t| carries(t, id)) {
        count += 1;
        last_used = last_used.max(Some(transaction.date));
        let this_unit = unit_of(transaction.account_id);
        match unit {
            None => unit = this_unit,
            Some(seen) if this_unit != Some(seen) => mixed = true,
            Some(_) => {}
        }
        for split in transaction
            .splits
            .iter()
            .filter(|s| s.tag_ids.contains(&id))
        {
            sum += split.amount.0.clone();
        }
    }
    let total = match unit {
        _ if count == 0 => Total::Empty,
        _ if mixed => Total::Mixed,
        Some(unit) => Total::Single {
            unit: unit.to_string(),
            amount: Money(sum),
        },
        // Carrying Transactions whose Account is gone: no Unit to name, so no sum either.
        None => Total::Mixed,
    };
    TagUsage {
        transactions: count,
        total,
        last_used,
    }
}

/// Every Tag in the page's order: most-used first (distinct Transactions), ties by name.
pub fn sorted_by_usage<'a>(tags: &'a [Tag], transactions: &[Transaction]) -> Vec<&'a Tag> {
    let counts = counts(tags, transactions);
    let mut sorted: Vec<&Tag> = tags.iter().collect();
    sorted.sort_by(|a, b| {
        counts[&b.id]
            .cmp(&counts[&a.id])
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
            .then_with(|| a.id.cmp(&b.id))
    });
    sorted
}

/// Tags A–Z, ignoring case: the Settings page's order (2j has no usage columns to rank by).
pub fn sorted_by_name(tags: &[Tag]) -> Vec<&Tag> {
    let mut sorted: Vec<&Tag> = tags.iter().collect();
    sorted.sort_by(|a, b| {
        a.name
            .to_lowercase()
            .cmp(&b.name.to_lowercase())
            .then_with(|| a.id.cmp(&b.id))
    });
    sorted
}

fn counts(tags: &[Tag], transactions: &[Transaction]) -> HashMap<u32, usize> {
    tags.iter()
        .map(|tag| (tag.id, transaction_count(transactions, tag.id)))
        .collect()
}

/// Tags whose names collide once normalised -- only possible in data that broke the rule. The
/// target is the suggested merge target; `duplicates` are the rest, each flagged "looks like a
/// duplicate of" it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DuplicateGroup {
    pub target: u32,
    pub duplicates: Vec<u32>,
}

/// Every group of two or more Tags sharing a normalised name, ordered by their lowest id.
pub fn duplicate_groups(tags: &[Tag], transactions: &[Transaction]) -> Vec<DuplicateGroup> {
    let counts = counts(tags, transactions);
    let mut by_name: Vec<(String, Vec<&Tag>)> = Vec::new();
    for tag in tags {
        let key = normalise(&tag.name);
        match by_name.iter_mut().find(|(name, _)| *name == key) {
            Some((_, members)) => members.push(tag),
            None => by_name.push((key, vec![tag])),
        }
    }
    by_name
        .into_iter()
        .filter(|(_, members)| members.len() > 1)
        .filter_map(|(_, mut members)| {
            // Suggested target first: active, then more Transactions, then older (lower id).
            members.sort_by(|a, b| {
                b.is_active
                    .cmp(&a.is_active)
                    .then_with(|| counts[&b.id].cmp(&counts[&a.id]))
                    .then_with(|| a.id.cmp(&b.id))
            });
            let (target, rest) = members.split_first()?;
            Some(DuplicateGroup {
                target: target.id,
                duplicates: rest.iter().map(|t| t.id).collect(),
            })
        })
        .collect()
}

/// The suggested merge target Tag `id` is flagged as a likely duplicate of, if it is flagged.
pub fn duplicate_of(groups: &[DuplicateGroup], id: u32) -> Option<u32> {
    groups
        .iter()
        .find(|group| group.duplicates.contains(&id))
        .map(|group| group.target)
}

#[cfg(test)]
mod tests {
    use lib_accounts::AccountService;
    use lib_core::{Money, Total, TransactionStatus};
    use lib_transactions::Split;

    use super::*;

    fn tag(id: u32, name: &str) -> Tag {
        Tag {
            id,
            name: name.to_string(),
            color: None,
            is_active: true,
        }
    }

    fn draft(name: &str) -> TagDraft {
        TagDraft {
            name: name.to_string(),
            color: None,
        }
    }

    fn transaction(id: u32, account_id: u32, tag_sets: &[&[u32]]) -> Transaction {
        Transaction {
            id,
            date: NaiveDate::from_ymd_opt(2026, 9, 1).expect("a fixed valid date"),
            account_id,
            status: TransactionStatus::default(),
            is_flagged: false,
            description: None,
            splits: tag_sets
                .iter()
                .map(|tags| Split {
                    amount: Money(BigDecimal::from(1)),
                    category_id: 1,
                    payee_id: None,
                    tag_ids: tags.to_vec(),
                })
                .collect(),
        }
    }

    #[test]
    fn normalise_ignores_case_spaces_and_punctuation() {
        assert_eq!(normalise("Work Trip"), normalise("work-trip"));
        assert_eq!(normalise("work-trip"), "worktrip");
    }

    #[test]
    fn insert_refuses_a_name_taken_once_normalised() {
        let mut tags = vec![tag(1, "work-trip")];

        assert_eq!(
            insert_tag(&mut tags, &draft("work trip")),
            Err(TagError::DuplicateName("work-trip".to_string()))
        );
        assert_eq!(tags.len(), 1);
    }

    #[test]
    fn edit_keeps_its_own_name_when_only_the_case_changes() {
        let mut tags = vec![tag(1, "work-trip")];

        assert_eq!(edit_tag(&mut tags, 1, &draft("Work-Trip")), Ok(()));
        assert_eq!(tags[0].name, "Work-Trip");
    }

    #[test]
    fn a_name_with_no_letter_or_digit_is_refused() {
        let mut tags = Vec::new();

        assert_eq!(
            insert_tag(&mut tags, &draft(" -- ")),
            Err(TagError::NoLetterOrDigit)
        );
    }

    #[test]
    fn merge_refuses_a_tag_into_itself() {
        let mut tags = vec![tag(1, "a")];

        assert_eq!(
            merge_tags(&mut tags, &mut [], 1, 1),
            Err(TagError::MergeIntoItself)
        );
    }

    #[test]
    fn merge_does_not_double_a_split_that_already_carries_the_target() {
        let mut tags = vec![tag(1, "source"), tag(2, "target")];
        let mut transactions = vec![transaction(1, 1, &[&[1, 2]])];

        merge_tags(&mut tags, &mut transactions, 1, 2).expect("both tags exist");

        assert_eq!(transactions[0].splits[0].tag_ids, vec![2]);
        assert_eq!(tags.len(), 1);
    }

    #[test]
    fn remove_untags_every_split_and_deletes_the_tag() {
        let mut tags = vec![tag(1, "gone"), tag(3, "kept")];
        let mut transactions = vec![transaction(1, 1, &[&[1, 3]])];

        remove_tag(&mut tags, &mut transactions, 1).expect("the tag exists");

        assert_eq!(transactions[0].splits[0].tag_ids, vec![3]);
        assert_eq!(tags, vec![tag(3, "kept")]);
    }

    #[test]
    fn transaction_count_counts_each_transaction_once() {
        let transactions = vec![transaction(1, 1, &[&[7], &[7]]), transaction(2, 1, &[&[8]])];

        assert_eq!(transaction_count(&transactions, 7), 1);
    }

    #[test]
    fn an_unused_tag_has_no_total_and_no_last_used_date() {
        let accounts = AccountService::seeded();
        let usage = usage(&[], accounts.accounts(), 1);

        assert_eq!(usage.transactions, 0);
        assert_eq!(usage.total, Total::Empty);
        assert_eq!(usage.last_used, None);
    }

    #[test]
    fn resolve_split_tag_reuses_an_exact_match_and_refuses_a_near_one() {
        let tags = vec![tag(1, "work-trip")];

        assert_eq!(
            resolve_split_tag(&tags, "WORK-TRIP"),
            Ok(SplitTag::Existing(1))
        );
        assert_eq!(
            resolve_split_tag(&tags, "work trip"),
            Err(TagError::DuplicateName("work-trip".to_string()))
        );
        assert_eq!(resolve_split_tag(&tags, "fresh"), Ok(SplitTag::New));
    }
}
