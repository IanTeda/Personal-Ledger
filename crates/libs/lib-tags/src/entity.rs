//! The Tag entity, its stub seed and the rules over rows, shared by the Desktop and TUI.
//!
//! The stub Tags are seeded from the Tags handoff's sample list (`docs/ux/desktop-mockups/19-tags/`)
//! plus the names the Transactions seed draws on. `gpui`-free and side-effect free, so every rule
//! is unit-tested without a window.
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
//!   Sync), so the seed carries one such pair on purpose. Remove and merge rewrite Splits, so they
//!   are `lib-ledger`'s (`tag_commands`), not this crate's.
//! - Usage is all time across every Account: distinct Transactions, the newest one's date, and a
//!   total that only sums when every Split carrying the Tag shares one Unit. This crate reads it
//!   through the [`TagUsage`] port.

use std::collections::HashMap;

use lib_core::{HexColor, TagId};

use crate::protocols::TagUsage;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tag {
    pub id: TagId,
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
        id: TagId::seed(id),
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
pub fn find_by_name(tags: &[Tag], name: &str) -> Option<TagId> {
    let folded = name.trim().to_lowercase();
    tags.iter()
        .find(|tag| tag.name.to_lowercase() == folded)
        .map(|tag| tag.id)
}

pub fn get(tags: &[Tag], id: TagId) -> Option<&Tag> {
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
fn clean_name(tags: &[Tag], own_id: Option<TagId>, name: &str) -> Result<String, TagError> {
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
pub fn name_error(tags: &[Tag], own_id: Option<TagId>, name: &str) -> Option<TagError> {
    clean_name(tags, own_id, name).err()
}

/// Adds a new, active Tag and returns its id.
pub fn insert_tag(tags: &mut Vec<Tag>, draft: &TagDraft) -> Result<TagId, TagError> {
    let name = clean_name(tags, None, &draft.name)?;
    let id = TagId::new();
    tags.push(Tag {
        id,
        name,
        color: draft.color.clone(),
        is_active: true,
    });
    Ok(id)
}

/// Renames and recolours Tag `id` in place: Splits refer to it by id, so nothing is re-tagged.
pub fn edit_tag(tags: &mut [Tag], id: TagId, draft: &TagDraft) -> Result<(), TagError> {
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
pub fn set_active(tags: &mut [Tag], id: TagId, is_active: bool) -> Result<(), TagError> {
    let tag = tags
        .iter_mut()
        .find(|t| t.id == id)
        .ok_or(TagError::NotFound)?;
    tag.is_active = is_active;
    Ok(())
}

/// What typing a Tag name on a Split resolves to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SplitTag {
    /// An exact case-insensitive match: reuse that Tag.
    Existing(TagId),
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

/// Every Tag in the page's order: most-used first (distinct Transactions), ties by name.
pub fn sorted_by_usage<'a>(tags: &'a [Tag], usage: &dyn TagUsage) -> Vec<&'a Tag> {
    sorted_by_counts(tags, &usage_counts(tags, usage))
}

/// [`sorted_by_usage`] over counts already read, for a caller that shows them too.
pub fn sorted_by_counts<'a>(tags: &'a [Tag], counts: &HashMap<TagId, usize>) -> Vec<&'a Tag> {
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

/// Each Tag's distinct Transactions, from one read of `usage`.
pub fn usage_counts(tags: &[Tag], usage: &dyn TagUsage) -> HashMap<TagId, usize> {
    let ids: Vec<TagId> = tags.iter().map(|tag| tag.id).collect();
    usage.transactions_by_tag(&ids)
}

/// Tags whose names collide once normalised -- only possible in data that broke the rule. The
/// target is the suggested merge target; `duplicates` are the rest, each flagged "looks like a
/// duplicate of" it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DuplicateGroup {
    pub target: TagId,
    pub duplicates: Vec<TagId>,
}

/// Every group of two or more Tags sharing a normalised name, ordered by their lowest id.
pub fn duplicate_groups(tags: &[Tag], usage: &dyn TagUsage) -> Vec<DuplicateGroup> {
    let mut by_name: Vec<(String, Vec<&Tag>)> = Vec::new();
    for tag in tags {
        let key = normalise(&tag.name);
        match by_name.iter_mut().find(|(name, _)| *name == key) {
            Some((_, members)) => members.push(tag),
            None => by_name.push((key, vec![tag])),
        }
    }
    by_name.retain(|(_, members)| members.len() > 1);
    // Counts only break ties inside a group, so a page with none reads no usage at all.
    if by_name.is_empty() {
        return Vec::new();
    }
    let grouped: Vec<TagId> = by_name
        .iter()
        .flat_map(|(_, members)| members.iter().map(|tag| tag.id))
        .collect();
    let counts = usage.transactions_by_tag(&grouped);
    by_name
        .into_iter()
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
pub fn duplicate_of(groups: &[DuplicateGroup], id: TagId) -> Option<TagId> {
    groups
        .iter()
        .find(|group| group.duplicates.contains(&id))
        .map(|group| group.target)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fake::InMemoryTagUsage;
    use crate::protocols::TagUse;

    fn tag(id: TagId, name: &str) -> Tag {
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

    fn used(counts: &[(u32, usize)]) -> InMemoryTagUsage {
        InMemoryTagUsage {
            uses: counts
                .iter()
                .map(|(id, transactions)| {
                    (
                        TagId::seed(*id),
                        TagUse {
                            transactions: *transactions,
                            ..TagUse::default()
                        },
                    )
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
        let mut tags = vec![tag(TagId::seed(1), "work-trip")];

        assert_eq!(
            insert_tag(&mut tags, &draft("work trip")),
            Err(TagError::DuplicateName("work-trip".to_string()))
        );
        assert_eq!(tags.len(), 1);
    }

    #[test]
    fn edit_keeps_its_own_name_when_only_the_case_changes() {
        let mut tags = vec![tag(TagId::seed(1), "work-trip")];

        assert_eq!(
            edit_tag(&mut tags, TagId::seed(1), &draft("Work-Trip")),
            Ok(())
        );
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
    fn resolve_split_tag_reuses_an_exact_match_and_refuses_a_near_one() {
        let tags = vec![tag(TagId::seed(1), "work-trip")];

        assert_eq!(
            resolve_split_tag(&tags, "WORK-TRIP"),
            Ok(SplitTag::Existing(TagId::seed(1)))
        );
        assert_eq!(
            resolve_split_tag(&tags, "work trip"),
            Err(TagError::DuplicateName("work-trip".to_string()))
        );
        assert_eq!(resolve_split_tag(&tags, "fresh"), Ok(SplitTag::New));
    }

    /// Counts the reads made of it, to show the page asks only when it must.
    struct CountingUsage {
        inner: InMemoryTagUsage,
        reads: std::cell::Cell<usize>,
    }

    impl TagUsage for CountingUsage {
        fn tag_usage(&self, id: TagId) -> TagUse {
            self.reads.set(self.reads.get() + 1);
            self.inner.tag_usage(id)
        }

        fn transactions_by_tag(&self, ids: &[TagId]) -> HashMap<TagId, usize> {
            self.reads.set(self.reads.get() + 1);
            self.inner.transactions_by_tag(ids)
        }
    }

    fn counting(counts: &[(u32, usize)]) -> CountingUsage {
        CountingUsage {
            inner: used(counts),
            reads: std::cell::Cell::new(0),
        }
    }

    #[test]
    fn no_duplicates_read_no_usage() {
        let tags = vec![tag(TagId::seed(1), "a"), tag(TagId::seed(2), "b")];
        let usage = counting(&[(1, 5)]);

        assert!(duplicate_groups(&tags, &usage).is_empty());
        assert_eq!(usage.reads.get(), 0);
    }

    #[test]
    fn a_duplicate_group_reads_usage_once_for_only_its_members() {
        let tags = vec![
            tag(TagId::seed(1), "Shared"),
            tag(TagId::seed(2), "shared"),
            tag(TagId::seed(3), "other"),
        ];
        let usage = counting(&[(2, 1), (3, 9)]);

        let groups = duplicate_groups(&tags, &usage);

        assert_eq!(groups[0].target, TagId::seed(2));
        assert_eq!(usage.reads.get(), 1);
    }

    #[test]
    fn sort_ties_fall_back_to_the_name() {
        let tags = vec![
            tag(TagId::seed(1), "zeta"),
            tag(TagId::seed(2), "Alpha"),
            tag(TagId::seed(3), "beta"),
        ];
        let order: Vec<TagId> = sorted_by_usage(&tags, &used(&[(3, 1)]))
            .iter()
            .map(|t| t.id)
            .collect();
        assert_eq!(order, [TagId::seed(3), TagId::seed(2), TagId::seed(1)]);
    }

    #[test]
    fn the_suggested_target_is_active_then_more_used_then_older() {
        let named = |id, name: &str, is_active| Tag {
            is_active,
            ..tag(id, name)
        };

        // More Transactions wins among active Tags.
        let tags = vec![
            named(TagId::seed(1), "Shared", true),
            named(TagId::seed(2), "shared", true),
        ];
        let usage = used(&[(2, 1)]);
        let groups = duplicate_groups(&tags, &usage);
        assert_eq!(
            groups[0],
            DuplicateGroup {
                target: TagId::seed(2),
                duplicates: vec![TagId::seed(1)]
            }
        );
        assert_eq!(duplicate_of(&groups, TagId::seed(1)), Some(TagId::seed(2)));
        assert_eq!(duplicate_of(&groups, TagId::seed(2)), None);

        // Active beats more used.
        let tags = vec![
            named(TagId::seed(1), "Shared", true),
            named(TagId::seed(2), "shared", false),
        ];
        assert_eq!(duplicate_groups(&tags, &usage)[0].target, TagId::seed(1));

        // A tie goes to the older (lower id); every other member is flagged against it.
        let tags = vec![
            named(TagId::seed(4), "work trip", true),
            named(TagId::seed(3), "Work-Trip", true),
            named(TagId::seed(5), "WORK_TRIP", false),
        ];
        assert_eq!(
            duplicate_groups(&tags, &InMemoryTagUsage::default()),
            vec![DuplicateGroup {
                target: TagId::seed(3),
                duplicates: vec![TagId::seed(4), TagId::seed(5)]
            }]
        );
    }
}
