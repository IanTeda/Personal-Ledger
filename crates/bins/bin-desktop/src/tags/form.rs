//! The Tags dialogs' form state: the Add, Edit, Remove and Merge tag forms, the merge options and
//! the dialog they sit behind. `gpui`-free, like the rest of this domain; `view::tags` renders it.

use lib_core::HexColor;

use super::{Tag, TagDraft, TagError, name_error, sorted_by_usage, swatches, transaction_count};
use crate::{
    chrome::dialog_host::{Dialog, DialogKey, DialogOutcome},
    form::field::TextField,
    form::select::SelectState,
    transactions::Transaction,
};

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
    use super::*;
    use crate::chrome::dialog_host::handle_key;
    use crate::tags::{default_tags, find_by_name, get, tests::seeded};

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
