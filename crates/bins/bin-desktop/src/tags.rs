//! Shared stub Tags, seeded from the Tags handoff's sample list (`docs/ux/desktop/19-tags/`) plus the
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

use crate::{
    accounts::Account,
    dialog_host::{Dialog, DialogKey, DialogOutcome},
    field::TextField,
    select::SelectState,
    transaction_query::Total,
    transactions::Transaction,
};

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

/// The Add and Edit tag dialogs' fields, in `Tab` order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TagField {
    #[default]
    Name,
    /// The swatch row: "none" then the [`swatches`] presets.
    Swatches,
    /// The free hex field beside the swatches.
    Hex,
    /// The Edit dialog's Active checkbox (#353: a Tag is reactivated from Edit); Add has none.
    Active,
}

impl TagField {
    const ADD_ORDER: [TagField; 3] = [Self::Name, Self::Swatches, Self::Hex];
    const EDIT_ORDER: [TagField; 4] = [Self::Name, Self::Swatches, Self::Hex, Self::Active];
}

/// The Add and Edit tag dialogs' live form state -- pure, `gpui`-free. The hex field is the one
/// source of the colour: picking a swatch writes its value there and "none" empties it, so a typed
/// value and a picked one can never disagree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TagForm {
    pub name: TextField,
    pub hex: TextField,
    pub is_active: bool,
    pub focused: TagField,
    /// An Edit form, which adds the Active checkbox to the `Tab` order.
    pub editing: bool,
    /// Every Tag, copied in when the Dialog opens so the name check needs nothing from `Shell`.
    tags: Vec<Tag>,
    /// The Tag being edited; `None` on Add.
    own_id: Option<u32>,
}

impl TagForm {
    /// The picker's choices: "none" at 0, then the [`swatches`] presets.
    pub const PICKS: usize = 7;

    /// A fresh Add form: no name and no colour (#352: a new Tag has none).
    pub fn new(tags: Vec<Tag>) -> Self {
        Self {
            name: TextField::default(),
            hex: TextField::default(),
            is_active: true,
            focused: TagField::default(),
            editing: false,
            tags,
            own_id: None,
        }
    }

    /// An Edit form pre-filled from `tag`; `tags` is every Tag, `tag` included.
    pub fn for_edit(tag: &Tag, tags: Vec<Tag>) -> Self {
        Self {
            name: TextField::new(tag.name.clone()),
            hex: TextField::new(
                tag.color
                    .as_ref()
                    .map(|colour| colour.as_str().to_string())
                    .unwrap_or_default(),
            ),
            is_active: tag.is_active,
            focused: TagField::Name,
            editing: true,
            tags,
            own_id: Some(tag.id),
        }
    }

    /// `Space` on, or a click of, the Active checkbox.
    pub fn toggle_active(&mut self) {
        self.is_active = !self.is_active;
    }

    /// The colour the hex field holds: `Ok(None)` when empty, `Err` while it isn't `#RRGGBB`.
    pub fn colour(&self) -> Result<Option<HexColor>, lib_core::HexColorError> {
        if self.hex.is_blank() {
            return Ok(None);
        }
        HexColor::parse(self.hex.text()).map(Some)
    }

    /// Whether the hex field holds something that isn't a colour.
    pub fn hex_invalid(&self) -> bool {
        self.colour().is_err()
    }

    /// The highlighted pick: 0 for "none", `1..=6` for a preset, `None` for any other colour.
    pub fn picked(&self) -> Option<usize> {
        match self.colour() {
            Ok(None) => Some(0),
            Ok(Some(colour)) => swatches()
                .iter()
                .position(|preset| *preset == colour)
                .map(|index| index + 1),
            Err(_) => None,
        }
    }

    /// Picks "none" (0) or a preset (`1..=6`); anything else is ignored.
    pub fn pick(&mut self, index: usize) {
        match index {
            0 => self.hex = TextField::default(),
            _ => {
                if let Some(preset) = swatches().get(index - 1) {
                    self.hex = TextField::new(preset.as_str());
                }
            }
        }
    }

    /// `←`/`→` on the swatch row: steps the pick, stopping at either end. From a custom colour it
    /// starts over at "none".
    pub fn step_pick(&mut self, forward: bool) {
        let next = match (self.picked(), forward) {
            (None, _) => 0,
            (Some(index), true) => (index + 1).min(Self::PICKS - 1),
            (Some(index), false) => index.saturating_sub(1),
        };
        self.pick(next);
    }

    /// What the dialog submits, or `None` while the hex field is invalid.
    pub fn draft(&self) -> Option<TagDraft> {
        Some(TagDraft {
            name: self.name.text().to_string(),
            color: self.colour().ok()?,
        })
    }

    /// The name's problem, checked live against every other Tag. An empty name is not reported,
    /// only kept from submitting.
    pub fn name_error(&self) -> Option<TagError> {
        if self.name.is_blank() {
            return None;
        }
        name_error(&self.tags, self.own_id, self.name.text())
    }

    pub fn is_valid(&self) -> bool {
        name_error(&self.tags, self.own_id, self.name.text()).is_none() && !self.hex_invalid()
    }

    pub fn focus(&mut self, field: TagField) {
        self.focused = field;
    }

    /// `Tab` / `Shift-Tab`.
    pub fn cycle_focus(&mut self, backward: bool) {
        let order: &[TagField] = if self.editing {
            &TagField::EDIT_ORDER
        } else {
            &TagField::ADD_ORDER
        };
        let count = order.len();
        let index = order
            .iter()
            .position(|field| *field == self.focused)
            .unwrap_or(0);
        let next = if backward {
            (index + count - 1) % count
        } else {
            (index + 1) % count
        };
        self.focused = order[next];
    }
}

impl Dialog for TagForm {
    /// `Shift-Tab` goes back a field, `←`/`→` step the swatch row and `Space` toggles Active. The
    /// swatch row and the checkbox take no text, and the hex box stops at `#` plus six digits (any
    /// more can only be a typo).
    fn handle_own_key(&mut self, key: DialogKey) -> Option<DialogOutcome> {
        match (key, self.focused) {
            (DialogKey::BackTab, _) => self.cycle_focus(true),
            (DialogKey::Char(' '), TagField::Active) => self.toggle_active(),
            (DialogKey::Left, TagField::Swatches) => self.step_pick(false),
            (DialogKey::Right, TagField::Swatches) => self.step_pick(true),
            (DialogKey::Char(_) | DialogKey::Backspace, TagField::Swatches | TagField::Active) => {}
            (DialogKey::Char(_), TagField::Hex) if self.hex.text().chars().count() >= 7 => {}
            _ => return None,
        }
        Some(DialogOutcome::Handled)
    }

    fn focused_text(&mut self) -> Option<&mut TextField> {
        match self.focused {
            TagField::Name => Some(&mut self.name),
            TagField::Hex => Some(&mut self.hex),
            TagField::Swatches | TagField::Active => None,
        }
    }

    fn cycle_field(&mut self) {
        self.cycle_focus(false);
    }

    fn is_valid(&self) -> bool {
        TagForm::is_valid(self)
    }
}

/// The 7d Remove dialog's typed-name confirm (#353): only a used Tag asks for it, since removing
/// an unused one loses nothing but the name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoveTagForm {
    pub confirmation_name: TextField,
    /// The Tag's name and Transaction count, copied in when the Dialog opens.
    name: String,
    transactions: usize,
}

impl RemoveTagForm {
    pub fn new(name: impl Into<String>, transactions: usize) -> Self {
        Self {
            confirmation_name: TextField::default(),
            name: name.into(),
            transactions,
        }
    }

    /// Whether **Remove tag** is live: always for an unused Tag, else once the name is typed
    /// exactly (case-sensitive, as the Payees dialog's).
    pub fn allows(&self) -> bool {
        self.transactions == 0 || self.confirmation_name.text() == self.name
    }
}

impl Dialog for RemoveTagForm {
    fn focused_text(&mut self) -> Option<&mut TextField> {
        Some(&mut self.confirmation_name)
    }

    fn is_valid(&self) -> bool {
        self.allows()
    }
}

/// The 7e selects' options in usage order, each labelled by `label(name, transactions)`.
pub fn merge_options(
    tags: &[Tag],
    transactions: &[Transaction],
    label: impl Fn(&str, usize) -> String,
) -> Vec<MergeOption> {
    sorted_by_usage(tags, transactions)
        .into_iter()
        .map(|tag| MergeOption {
            id: tag.id,
            label: label(&tag.name, transaction_count(transactions, tag.id)),
        })
        .collect()
}

/// Which Tags dialog is open on the Tags page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TagsDialog {
    Add(TagForm),
    /// Editing the Tag with this [`Tag::id`].
    Edit(u32, TagForm),
    /// Removing the Tag with this [`Tag::id`].
    Remove(u32, RemoveTagForm),
    /// Merging one Tag into another; either select is empty until chosen (the palette's
    /// `tags merge` opens with neither, `m` on an unflagged Tag with only the source).
    Merge(MergeTagsForm),
}

impl TagsDialog {
    fn inner(&self) -> &dyn Dialog {
        match self {
            Self::Add(form) | Self::Edit(_, form) => form,
            Self::Remove(_, form) => form,
            Self::Merge(form) => form,
        }
    }

    fn inner_mut(&mut self) -> &mut dyn Dialog {
        match self {
            Self::Add(form) | Self::Edit(_, form) => form,
            Self::Remove(_, form) => form,
            Self::Merge(form) => form,
        }
    }
}

impl Dialog for TagsDialog {
    fn handle_own_key(&mut self, key: DialogKey) -> Option<DialogOutcome> {
        self.inner_mut().handle_own_key(key)
    }

    fn focused_text(&mut self) -> Option<&mut TextField> {
        self.inner_mut().focused_text()
    }

    fn cycle_field(&mut self) {
        self.inner_mut().cycle_field();
    }

    fn is_valid(&self) -> bool {
        self.inner().is_valid()
    }

    fn close_open_select(&mut self) -> bool {
        self.inner_mut().close_open_select()
    }
}

/// A 7e select's option: a Tag and its label, `Shared (9 txns)`. Labels are unique because names
/// are, so the selects key their value on the label and map it back to the id here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MergeOption {
    pub id: u32,
    pub label: String,
}

/// Which 7e select has focus.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MergeField {
    #[default]
    Source,
    Target,
}

/// The 7e Merge dialog's two selects (#354). Target's options leave out the chosen source, so the
/// two can never be equal; choosing as source the Tag already picked as target clears the target.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MergeTagsForm {
    pub source: SelectState,
    pub target: SelectState,
    pub focused: MergeField,
    /// Every Tag and its label in usage order, copied in when the Dialog opens.
    options: Vec<MergeOption>,
}

fn label_of(options: &[MergeOption], id: Option<u32>) -> Option<String> {
    let id = id?;
    options
        .iter()
        .find(|option| option.id == id)
        .map(|option| option.label.clone())
}

fn id_of(options: &[MergeOption], label: Option<&str>) -> Option<u32> {
    let label = label?;
    options
        .iter()
        .find(|option| option.label == label)
        .map(|option| option.id)
}

impl MergeTagsForm {
    /// Opens on `source` and `target` (ignored if equal), focusing the first one still empty.
    pub fn new(options: Vec<MergeOption>, source: Option<u32>, target: Option<u32>) -> Self {
        let target = target.filter(|target| Some(*target) != source);
        let focused = if source.is_some() && target.is_none() {
            MergeField::Target
        } else {
            MergeField::Source
        };
        Self {
            source: SelectState::new(label_of(&options, source)),
            target: SelectState::new(label_of(&options, target)),
            focused,
            options,
        }
    }

    pub fn source_id(&self) -> Option<u32> {
        id_of(&self.options, self.source.value())
    }

    pub fn target_id(&self) -> Option<u32> {
        id_of(&self.options, self.target.value())
    }

    /// The labels `field`'s list offers: every Tag for the source, every Tag but the source for
    /// the target.
    pub fn labels(&self, field: MergeField) -> Vec<String> {
        let source = self.source_id();
        self.options
            .iter()
            .filter(|option| field == MergeField::Source || Some(option.id) != source)
            .map(|option| option.label.clone())
            .collect()
    }

    /// Whether **Merge** is live: both chosen, and different.
    pub fn pair(&self) -> Option<(u32, u32)> {
        let source = self.source_id()?;
        let target = self.target_id()?;
        (source != target).then_some((source, target))
    }

    fn state_mut(&mut self, field: MergeField) -> &mut SelectState {
        match field {
            MergeField::Source => &mut self.source,
            MergeField::Target => &mut self.target,
        }
    }

    /// Clears the target once the source has moved onto it.
    fn settle(&mut self) {
        if self.source_id().is_some() && self.source_id() == self.target_id() {
            self.target = SelectState::default();
        }
    }

    /// Closes whichever list is open, returning whether one was (`Esc`'s first press).
    fn close_lists(&mut self) -> bool {
        let was_open = self.is_open();
        self.source.cancel();
        self.target.cancel();
        was_open
    }

    pub fn is_open(&self) -> bool {
        self.source.is_open() || self.target.is_open()
    }

    /// `Tab` / `Shift-Tab`: commits an open list's highlight and moves to the other select.
    pub fn cycle_focus(&mut self) {
        let field = self.focused;
        let labels = self.labels(field);
        self.state_mut(field).commit(&labels);
        self.settle();
        self.focused = match field {
            MergeField::Source => MergeField::Target,
            MergeField::Target => MergeField::Source,
        };
    }

    /// `Up`/`Down` on the focused select: moves an open list's highlight, else steps the value.
    pub fn step(&mut self, delta: isize) {
        let field = self.focused;
        let labels = self.labels(field);
        let state = self.state_mut(field);
        if state.is_open() {
            state.move_highlight(&labels, delta);
        } else {
            state.step(&labels, delta);
        }
        self.settle();
    }

    /// `Space`, or a click on the closed field: opens the list, or commits the open one.
    pub fn toggle(&mut self, field: MergeField) {
        if field != self.focused {
            self.close_lists();
            self.focused = field;
        }
        let labels = self.labels(field);
        let state = self.state_mut(field);
        if state.is_open() {
            state.commit(&labels);
        } else {
            state.open(&labels);
        }
        self.settle();
    }

    /// A click on row `index` of `field`'s open list.
    pub fn choose(&mut self, field: MergeField, index: usize) {
        let labels = self.labels(field);
        self.focused = field;
        self.state_mut(field).choose(&labels, index);
        self.settle();
    }
}

impl Dialog for MergeTagsForm {
    /// `Tab` commits an open list and moves to the other select (`Shift-Tab` does the same, via
    /// the shared handling), `↑`/`↓` step the value or an open list's highlight, `Space` opens or
    /// commits the list and `Enter` commits an open list, else merges. Typing goes nowhere.
    fn handle_own_key(&mut self, key: DialogKey) -> Option<DialogOutcome> {
        match key {
            DialogKey::Up => self.step(-1),
            DialogKey::Down => self.step(1),
            DialogKey::Char(' ') => self.toggle(self.focused),
            DialogKey::Enter if self.is_open() => self.toggle(self.focused),
            DialogKey::Char(_) | DialogKey::Backspace => {}
            _ => return None,
        }
        Some(DialogOutcome::Handled)
    }

    fn cycle_field(&mut self) {
        self.cycle_focus();
    }

    fn is_valid(&self) -> bool {
        self.pair().is_some()
    }

    fn close_open_select(&mut self) -> bool {
        self.close_lists()
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn sorted_by_name_ignores_case() {
        let tags = default_tags();
        let names: Vec<String> = sorted_by_name(&tags)
            .iter()
            .map(|tag| tag.name.to_lowercase())
            .collect();
        let mut expected = names.clone();
        expected.sort();
        assert_eq!(names, expected);
    }

    use lib_core::TransactionStatus;

    use super::*;
    use crate::dialog_host::handle_key;
    use crate::{
        accounts::default_accounts,
        categories::default_categories,
        payees::default_payees,
        transactions::{Split, default_transactions},
    };

    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 9, 19).unwrap()
    }

    fn seeded() -> (Vec<Account>, Vec<Tag>, Vec<Transaction>) {
        let accounts = default_accounts();
        let tags = default_tags();
        let transactions = default_transactions(
            &accounts,
            &default_categories(),
            &default_payees(),
            &tags,
            today(),
        );
        (accounts, tags, transactions)
    }

    fn id(tags: &[Tag], name: &str) -> u32 {
        tags.iter().find(|t| t.name == name).unwrap().id
    }

    fn draft(name: &str) -> TagDraft {
        TagDraft {
            name: name.to_string(),
            color: None,
        }
    }

    fn split(cents: i64, tag_ids: &[u32]) -> Split {
        Split {
            amount: Money(BigDecimal::new(cents.into(), 2)),
            category_id: 7,
            payee_id: None,
            tag_ids: tag_ids.to_vec(),
        }
    }

    fn transaction(id: u32, account_id: u32, date: NaiveDate, splits: Vec<Split>) -> Transaction {
        Transaction {
            id,
            date,
            account_id,
            status: TransactionStatus::Open,
            is_flagged: false,
            description: None,
            splits,
        }
    }

    fn account_id(accounts: &[Account], name: &str) -> u32 {
        accounts.iter().find(|a| a.name == name).unwrap().id
    }

    #[test]
    fn normalise_lower_cases_and_keeps_only_letters_and_digits() {
        assert_eq!(normalise("Work-Trip"), "worktrip");
        assert_eq!(normalise(" work_trip 2 "), "worktrip2");
        assert_eq!(normalise("Überweisung"), "überweisung");
        assert_eq!(normalise("--- !"), "");
    }

    #[test]
    fn seeded_tags_break_the_rule_only_for_the_one_duplicate_pair() {
        let (_, tags, transactions) = seeded();
        let groups = duplicate_groups(&tags, &transactions);
        assert_eq!(
            groups,
            vec![DuplicateGroup {
                target: id(&tags, "work-trip"),
                duplicates: vec![id(&tags, "Work Trip")],
            }]
        );
    }

    #[test]
    fn the_seed_has_one_inactive_tag_left_out_of_the_count() {
        let (_, tags, _) = seeded();
        assert!(!get(&tags, id(&tags, "Bali 2025")).unwrap().is_active);
        assert_eq!(active_count(&tags), tags.len() - 1);
    }

    #[test]
    fn seeded_usage_keeps_the_mockups_order() {
        let (_, tags, transactions) = seeded();
        let order: Vec<&str> = sorted_by_usage(&tags, &transactions)
            .iter()
            .map(|t| t.name.as_str())
            .take(6)
            .collect();
        assert_eq!(
            order,
            [
                "shared",
                "reimbursable",
                "tax-deductible",
                "work-trip",
                "gift",
                "one-off"
            ]
        );
        assert_eq!(transaction_count(&transactions, id(&tags, "one-off")), 3);
        assert!(transaction_count(&transactions, id(&tags, "Work Trip")) > 0);
        assert!(transaction_count(&transactions, id(&tags, "Bali 2025")) > 0);
    }

    #[test]
    fn find_by_name_ignores_case_but_not_punctuation() {
        let tags = default_tags();
        assert_eq!(find_by_name(&tags, "SHARED"), Some(id(&tags, "shared")));
        assert_eq!(find_by_name(&tags, "tax deductible"), None);
        assert_eq!(find_by_name(&tags, "missing"), None);
    }

    #[test]
    fn insert_trims_the_name_and_starts_active() {
        let mut tags = default_tags();
        let new = insert_tag(
            &mut tags,
            &TagDraft {
                name: "  holiday ".to_string(),
                color: Some(swatches()[3].clone()),
            },
        )
        .unwrap();
        let tag = get(&tags, new).unwrap();
        assert_eq!(tag.name, "holiday");
        assert_eq!(tag.color, Some(swatches()[3].clone()));
        assert!(tag.is_active);
        assert_eq!(new, tags.iter().map(|t| t.id).max().unwrap());
    }

    #[test]
    fn insert_refuses_empty_punctuation_only_and_normalised_duplicates() {
        let mut tags = default_tags();
        assert_eq!(
            insert_tag(&mut tags, &draft("   ")),
            Err(TagError::NameRequired)
        );
        assert_eq!(
            insert_tag(&mut tags, &draft("- !")),
            Err(TagError::NoLetterOrDigit)
        );
        assert_eq!(
            insert_tag(&mut tags, &draft("Tax Deductible")),
            Err(TagError::DuplicateName("tax-deductible".to_string()))
        );
        // An inactive Tag's name stays taken.
        assert_eq!(
            insert_tag(&mut tags, &draft("bali-2025")),
            Err(TagError::DuplicateName("Bali 2025".to_string()))
        );
    }

    #[test]
    fn edit_renames_and_recolours_and_may_keep_its_own_name() {
        let mut tags = default_tags();
        let shared = id(&tags, "shared");
        edit_tag(&mut tags, shared, &draft("Shared")).unwrap();
        assert_eq!(get(&tags, shared).unwrap().name, "Shared");
        assert_eq!(get(&tags, shared).unwrap().color, None);
        assert_eq!(
            edit_tag(&mut tags, shared, &draft("gift!")),
            Err(TagError::DuplicateName("gift".to_string()))
        );
        assert_eq!(
            edit_tag(&mut tags, 999, &draft("x")),
            Err(TagError::NotFound)
        );
    }

    #[test]
    fn set_active_toggles() {
        let mut tags = default_tags();
        let gift = id(&tags, "gift");
        set_active(&mut tags, gift, false).unwrap();
        assert!(!get(&tags, gift).unwrap().is_active);
        set_active(&mut tags, gift, true).unwrap();
        assert!(get(&tags, gift).unwrap().is_active);
        assert_eq!(set_active(&mut tags, 999, false), Err(TagError::NotFound));
    }

    #[test]
    fn remove_untags_every_split_and_keeps_the_transactions() {
        let (_, mut tags, mut transactions) = seeded();
        let before = transactions.clone();
        let shared = id(&tags, "shared");
        remove_tag(&mut tags, &mut transactions, shared).unwrap();
        assert!(get(&tags, shared).is_none());
        assert_eq!(transaction_count(&transactions, shared), 0);
        assert_eq!(transactions.len(), before.len());
        for (after, before) in transactions.iter().zip(&before) {
            assert_eq!(after.total(), before.total());
            for (a, b) in after.splits.iter().zip(&before.splits) {
                assert_eq!((a.category_id, a.payee_id), (b.category_id, b.payee_id));
            }
        }
        assert_eq!(
            remove_tag(&mut tags, &mut transactions, shared),
            Err(TagError::NotFound)
        );
    }

    #[test]
    fn remove_deletes_an_unused_tag_too() {
        let (_, mut tags, mut transactions) = seeded();
        let new = insert_tag(&mut tags, &draft("unused")).unwrap();
        remove_tag(&mut tags, &mut transactions, new).unwrap();
        assert!(get(&tags, new).is_none());
    }

    #[test]
    fn merge_retags_without_doubling_and_deletes_the_source() {
        let mut tags = default_tags();
        let (source, target, other) = (id(&tags, "Work Trip"), id(&tags, "work-trip"), 1);
        let day = today();
        let mut transactions = vec![
            transaction(1, 2, day, vec![split(-100, &[source])]),
            transaction(
                2,
                2,
                day,
                vec![split(-200, &[source, target]), split(-50, &[other])],
            ),
            transaction(3, 2, day, vec![split(-300, &[target])]),
        ];
        merge_tags(&mut tags, &mut transactions, source, target).unwrap();
        assert!(get(&tags, source).is_none());
        assert_eq!(transactions[0].splits[0].tag_ids, vec![target]);
        assert_eq!(transactions[1].splits[0].tag_ids, vec![target]);
        assert_eq!(transactions[1].splits[1].tag_ids, vec![other]);
        assert_eq!(transaction_count(&transactions, target), 3);
        // The target keeps its own name, colour and state.
        let kept = get(&tags, target).unwrap();
        assert_eq!(kept.name, "work-trip");
        assert_eq!(kept.color, Some(swatches()[4].clone()));
    }

    #[test]
    fn merge_refuses_itself_and_missing_tags_and_allows_inactive_ones() {
        let (_, mut tags, mut transactions) = seeded();
        let gift = id(&tags, "gift");
        let bali = id(&tags, "Bali 2025");
        assert_eq!(
            merge_tags(&mut tags, &mut transactions, gift, gift),
            Err(TagError::MergeIntoItself)
        );
        assert_eq!(
            merge_tags(&mut tags, &mut transactions, 999, gift),
            Err(TagError::NotFound)
        );
        let expected =
            transaction_count(&transactions, gift) + transaction_count(&transactions, bali);
        merge_tags(&mut tags, &mut transactions, bali, gift).unwrap();
        assert_eq!(transaction_count(&transactions, gift), expected);
        assert!(get(&tags, gift).unwrap().is_active);
    }

    #[test]
    fn a_split_reuses_an_exact_match_and_refuses_a_normalised_one() {
        let tags = default_tags();
        assert_eq!(
            resolve_split_tag(&tags, " SHARED "),
            Ok(SplitTag::Existing(id(&tags, "shared")))
        );
        assert_eq!(
            resolve_split_tag(&tags, "one off"),
            Err(TagError::DuplicateName("one-off".to_string()))
        );
        assert_eq!(resolve_split_tag(&tags, "camping"), Ok(SplitTag::New));
        assert_eq!(resolve_split_tag(&tags, ""), Err(TagError::NameRequired));
    }

    #[test]
    fn usage_counts_distinct_transactions_sums_the_tagged_splits_and_takes_the_newest_date() {
        let accounts = default_accounts();
        let everyday = account_id(&accounts, "ANZ Everyday");
        let day = today();
        let transactions = vec![
            transaction(
                1,
                everyday,
                day - chrono::Duration::days(3),
                vec![split(-1_000, &[5]), split(-250, &[5]), split(-9_999, &[])],
            ),
            // Future-dated still counts as last used.
            transaction(
                2,
                everyday,
                day + chrono::Duration::days(2),
                vec![split(-500, &[5])],
            ),
            transaction(3, everyday, day, vec![split(-700, &[6])]),
        ];
        let used = usage(&transactions, &accounts, 5);
        assert_eq!(used.transactions, 2);
        assert_eq!(
            used.total,
            Total::Single {
                unit: "aud".to_string(),
                amount: Money(BigDecimal::new((-1_750).into(), 2)),
            }
        );
        assert_eq!(used.last_used, Some(day + chrono::Duration::days(2)));
    }

    #[test]
    fn usage_across_units_is_mixed_and_an_unused_tag_is_empty() {
        let accounts = default_accounts();
        let day = today();
        let transactions = vec![
            transaction(
                1,
                account_id(&accounts, "ANZ Everyday"),
                day,
                vec![split(-1_000, &[5])],
            ),
            transaction(
                2,
                account_id(&accounts, "Bitcoin"),
                day,
                vec![split(-500, &[5])],
            ),
        ];
        assert_eq!(usage(&transactions, &accounts, 5).total, Total::Mixed);
        let unused = usage(&transactions, &accounts, 6);
        assert_eq!(
            (unused.transactions, unused.total, unused.last_used),
            (0, Total::Empty, None)
        );
    }

    #[test]
    fn sort_ties_fall_back_to_the_name() {
        let tags = vec![
            Tag {
                id: 1,
                name: "zeta".to_string(),
                color: None,
                is_active: true,
            },
            Tag {
                id: 2,
                name: "Alpha".to_string(),
                color: None,
                is_active: true,
            },
            Tag {
                id: 3,
                name: "beta".to_string(),
                color: None,
                is_active: true,
            },
        ];
        let transactions = vec![transaction(1, 2, today(), vec![split(-1, &[3])])];
        let order: Vec<u32> = sorted_by_usage(&tags, &transactions)
            .iter()
            .map(|t| t.id)
            .collect();
        assert_eq!(order, [3, 2, 1]);
    }

    #[test]
    fn the_suggested_target_is_active_then_more_used_then_older() {
        let tag = |id, name: &str, is_active| Tag {
            id,
            name: name.to_string(),
            color: None,
            is_active,
        };
        let day = today();
        let used = |id, tag_id| transaction(id, 2, day, vec![split(-1, &[tag_id])]);

        // More Transactions wins among active Tags.
        let tags = vec![tag(1, "Shared", true), tag(2, "shared", true)];
        let transactions = vec![used(1, 2)];
        let groups = duplicate_groups(&tags, &transactions);
        assert_eq!(
            groups[0],
            DuplicateGroup {
                target: 2,
                duplicates: vec![1]
            }
        );
        assert_eq!(duplicate_of(&groups, 1), Some(2));
        assert_eq!(duplicate_of(&groups, 2), None);

        // Active beats more used.
        let tags = vec![tag(1, "Shared", true), tag(2, "shared", false)];
        assert_eq!(duplicate_groups(&tags, &transactions)[0].target, 1);

        // A tie goes to the older (lower id); every other member is flagged against it.
        let tags = vec![
            tag(4, "work trip", true),
            tag(3, "Work-Trip", true),
            tag(5, "WORK_TRIP", false),
        ];
        assert_eq!(
            duplicate_groups(&tags, &[]),
            vec![DuplicateGroup {
                target: 3,
                duplicates: vec![4, 5]
            }]
        );
    }

    #[test]
    fn a_new_form_has_no_colour_and_picks_none() {
        let form = TagForm::new(default_tags());
        assert_eq!(form.colour(), Ok(None));
        assert_eq!(form.picked(), Some(0));
        assert_eq!(form.focused, TagField::Name);
    }

    #[test]
    fn picking_a_swatch_fills_the_hex_field_and_none_empties_it() {
        let mut form = TagForm::new(default_tags());
        form.pick(4);
        assert_eq!(form.hex.text(), "#4A7C9E");
        assert_eq!(form.colour(), Ok(Some(swatches()[3].clone())));
        assert_eq!(form.picked(), Some(4));
        form.pick(0);
        assert_eq!(form.hex.text(), "");
        assert_eq!(form.picked(), Some(0));
        form.pick(99);
        assert_eq!(form.picked(), Some(0));
    }

    #[test]
    fn stepping_the_pick_stops_at_either_end_and_restarts_from_a_custom_colour() {
        let mut form = TagForm::new(default_tags());
        form.step_pick(false);
        assert_eq!(form.picked(), Some(0));
        for _ in 0..10 {
            form.step_pick(true);
        }
        assert_eq!(form.picked(), Some(TagForm::PICKS - 1));
        form.hex = TextField::new("#123456");
        assert_eq!(form.picked(), None);
        form.step_pick(true);
        assert_eq!(form.picked(), Some(0));
    }

    #[test]
    fn a_typed_hex_is_any_colour_and_an_unfinished_one_blocks_submit() {
        let mut form = TagForm::new(default_tags());
        for ch in "camping".chars() {
            type_char(&mut form, ch);
        }
        form.focus(TagField::Hex);
        for ch in "#12ab".chars() {
            type_char(&mut form, ch);
        }
        assert!(form.hex_invalid());
        assert!(!form.is_valid());
        assert_eq!(form.draft(), None);
        for ch in "ef99".chars() {
            type_char(&mut form, ch);
        }
        // Capped at `#` plus six digits.
        assert_eq!(form.hex.text(), "#12abef");
        assert!(form.is_valid());
        assert_eq!(
            form.draft(),
            Some(TagDraft {
                name: "camping".to_string(),
                color: Some(HexColor::from_rgb(0x12, 0xab, 0xef)),
            })
        );
        handle_key(&mut form, DialogKey::Backspace);
        assert!(form.hex_invalid());
    }

    #[test]
    fn the_name_error_is_live_but_silent_while_empty() {
        let mut form = TagForm::new(default_tags());
        assert_eq!(form.name_error(), None);
        assert!(!form.is_valid());
        form.name = TextField::new("Work Trip!");
        assert_eq!(
            form.name_error(),
            Some(TagError::DuplicateName("work-trip".to_string()))
        );
        form.name = TextField::new("--");
        assert_eq!(form.name_error(), Some(TagError::NoLetterOrDigit));
        form.name = TextField::new("camping");
        assert!(form.is_valid());
    }

    #[test]
    fn tab_cycles_the_fields_and_the_swatch_row_takes_no_text() {
        let mut form = TagForm::new(default_tags());
        form.cycle_focus(false);
        assert_eq!(form.focused, TagField::Swatches);
        type_char(&mut form, 'x');
        handle_key(&mut form, DialogKey::Backspace);
        assert_eq!((form.name.text(), form.hex.text()), ("", ""));
        form.cycle_focus(false);
        assert_eq!(form.focused, TagField::Hex);
        form.cycle_focus(false);
        assert_eq!(form.focused, TagField::Name);
        form.cycle_focus(true);
        assert_eq!(form.focused, TagField::Hex);
    }

    #[test]
    fn an_edit_form_is_pre_filled_and_only_it_tabs_to_active() {
        let tag = Tag {
            id: 7,
            name: "gift".to_string(),
            color: Some(swatches()[2].clone()),
            is_active: false,
        };
        let mut form = TagForm::for_edit(&tag, default_tags());
        assert_eq!(form.name.text(), "gift");
        assert_eq!(form.picked(), Some(3));
        assert!(!form.is_active);
        for _ in 0..3 {
            form.cycle_focus(false);
        }
        assert_eq!(form.focused, TagField::Active);
        type_char(&mut form, 'x');
        handle_key(&mut form, DialogKey::Backspace);
        assert_eq!(form.name.text(), "gift");
        form.toggle_active();
        assert!(form.is_active);
        form.cycle_focus(false);
        assert_eq!(form.focused, TagField::Name);
        form.cycle_focus(true);
        assert_eq!(form.focused, TagField::Active);
    }

    #[test]
    fn an_edit_may_keep_its_own_name_but_not_take_another() {
        let tags = default_tags();
        let shared = find_by_name(&tags, "shared").unwrap();
        let mut form = TagForm::for_edit(get(&tags, shared).unwrap(), tags.clone());
        assert!(form.is_valid());
        form.name = TextField::new("Shared!");
        assert!(form.is_valid());
        form.name = TextField::new("GIFT");
        assert_eq!(
            form.name_error(),
            Some(TagError::DuplicateName("gift".to_string()))
        );
    }

    #[test]
    fn removing_an_unused_tag_needs_no_confirm_but_a_used_one_needs_its_exact_name() {
        let mut unused = RemoveTagForm::new("travel", 0);
        assert!(unused.allows());
        assert!(handle_key(&mut unused, DialogKey::Enter) == DialogOutcome::Confirm);

        let mut form = RemoveTagForm::new("travel", 3);
        assert!(!form.allows());
        for ch in "Travel".chars() {
            type_char(&mut form, ch);
        }
        assert!(!form.allows());
        form.confirmation_name = TextField::default();
        for ch in "travel".chars() {
            type_char(&mut form, ch);
        }
        assert!(form.allows());
        handle_key(&mut form, DialogKey::Backspace);
        assert!(!form.allows());
    }

    fn type_char(dialog: &mut impl Dialog, ch: char) {
        handle_key(dialog, DialogKey::Char(ch));
    }

    fn merge_options_of(ids: &[u32]) -> Vec<MergeOption> {
        ids.iter()
            .map(|id| MergeOption {
                id: *id,
                label: format!("tag {id}"),
            })
            .collect()
    }

    #[test]
    fn merge_options_follow_usage_order_with_their_counts() {
        let (_, tags, transactions) = seeded();
        let options = merge_options(&tags, &transactions, |name, count| {
            format!("{name} ({count})")
        });
        let sorted = sorted_by_usage(&tags, &transactions);
        assert_eq!(options.len(), tags.len());
        for (option, tag) in options.iter().zip(sorted) {
            assert_eq!(option.id, tag.id);
            assert_eq!(
                option.label,
                format!(
                    "{} ({})",
                    tag.name,
                    transaction_count(&transactions, tag.id)
                )
            );
        }
    }

    #[test]
    fn a_merge_form_opens_on_its_pair_and_focuses_the_first_empty_select() {
        let options = merge_options_of(&[1, 2, 3]);
        let both = MergeTagsForm::new(options.clone(), Some(2), Some(1));
        assert_eq!(both.pair(), Some((2, 1)));
        assert_eq!(both.focused, MergeField::Source);

        let source_only = MergeTagsForm::new(options.clone(), Some(2), None);
        assert_eq!(source_only.pair(), None);
        assert_eq!(source_only.focused, MergeField::Target);

        let same = MergeTagsForm::new(options.clone(), Some(2), Some(2));
        assert_eq!(same.target_id(), None);
    }

    #[test]
    fn the_target_list_leaves_out_the_source() {
        let options = merge_options_of(&[1, 2, 3]);
        let form = MergeTagsForm::new(options.clone(), Some(2), None);
        assert_eq!(form.labels(MergeField::Source).len(), 3);
        assert_eq!(form.labels(MergeField::Target), ["tag 1", "tag 3"]);
    }

    #[test]
    fn moving_the_source_onto_the_target_clears_the_target() {
        let options = merge_options_of(&[1, 2, 3]);
        let mut form = MergeTagsForm::new(options.clone(), Some(1), Some(2));
        form.step(1);
        assert_eq!(form.source_id(), Some(2));
        assert_eq!(form.target_id(), None);
        assert_eq!(form.pair(), None);
    }

    #[test]
    fn toggling_opens_then_commits_and_choose_picks_a_row() {
        let options = merge_options_of(&[1, 2, 3]);
        let mut form = MergeTagsForm::new(options.clone(), Some(1), None);
        form.toggle(MergeField::Target);
        assert!(form.target.is_open());
        form.step(1);
        form.toggle(MergeField::Target);
        assert!(!form.is_open());
        assert_eq!(form.pair(), Some((1, 3)));

        form.toggle(MergeField::Source);
        form.choose(MergeField::Source, 2);
        assert_eq!(form.pair(), None);
        assert_eq!(form.source_id(), Some(3));
    }

    #[test]
    fn tab_commits_the_open_list_and_moves_on() {
        let options = merge_options_of(&[1, 2, 3]);
        let mut form = MergeTagsForm::new(options.clone(), None, None);
        form.toggle(MergeField::Source);
        form.cycle_focus();
        assert_eq!(form.source_id(), Some(1));
        assert_eq!(form.focused, MergeField::Target);
        assert!(!Dialog::close_open_select(&mut form));
    }
}
