//! The Add and Edit tag Dialogs (`docs/ux/desktop-mockups/19-tags/README.md`'s 7b and 7c): one
//! form, Edit pre-filled from the Tag.
//!
//! - The fields, in `Tab` order: Name, the Colour swatch row, the free hex box and, on Edit only,
//!   the Active checkbox (#353: a Tag is deactivated and reactivated from Edit).
//! - The hex box is the one source of the colour. Picking a swatch writes its value there and
//!   "none" empties it, so a typed value and a picked one can never disagree. It holds at most
//!   `#` and six digits.
//! - The name is required, needs a letter or a digit, and is unique once case, spaces and
//!   punctuation are ignored (ADR-0015), checked live against every other Tag. An empty name is
//!   not reported, only kept from confirming.
//!
//! The Tag's usage is read when an Edit opens, as the Dialog is modal. Saving raises no Toast: the
//! page selects the Tag instead.

use lib_core::{HexColor, TagId};
use lib_viper::view::{DialogFrame, TextFieldView};

use super::{count, name_error_text};
use crate::interactor::{AddTag, DeactivateTag, EditTag, ReactivateTag};
use crate::protocols::{SharedPorts, TagChange, TagsOutcome, TagsRouter};
use crate::{Tag, TagDraft, TagError, msg, name_error, swatches};

/// The form's fields, in `Tab` order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TagField {
    #[default]
    Name,
    /// The swatch row: "none" then the [`swatches`] presets.
    Swatches,
    /// The free hex field beside the swatches.
    Hex,
    /// The Edit Dialog's Active checkbox; Add has none.
    Active,
}

impl TagField {
    const ADD_ORDER: [Self; 3] = [Self::Name, Self::Swatches, Self::Hex];
    const EDIT_ORDER: [Self; 4] = [Self::Name, Self::Swatches, Self::Hex, Self::Active];
}

/// The picker's choices: "none" at 0, then the six presets.
const PICKS: usize = 7;

/// The longest the hex box holds: `#` and six digits, since any more can only be a typo.
const HEX_LEN: usize = 7;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TagFormEvent {
    /// The name's whole new text, as its View's input holds it.
    Name(String),
    /// The hex box's whole new text.
    Hex(String),
    /// A click on a swatch: 0 for "none", `1..=6` for a preset.
    Pick(usize),
    /// `←`/`→` on the swatch row, stopping at either end. From a custom colour it starts at "none".
    StepPick {
        forward: bool,
    },
    /// `Space` on, or a click of, the Active checkbox.
    ToggleActive,
    /// `Tab`/`Shift-Tab`, wrapping.
    NextField,
    PrevField,
    /// A click on a field.
    Focus(TagField),
    Cancel,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TagFormViewModel {
    pub frame: DialogFrame,
    pub name: TextFieldView,
    /// The name's problem, checked live.
    pub name_error: Option<String>,
    pub colour: ColourField,
    /// The Edit Dialog's Active checkbox.
    pub active: Option<ActiveField>,
    /// The note under the fields.
    pub callout: String,
    /// Why the last confirm was refused.
    pub refusal: Option<String>,
    /// Edit, rather than Add.
    pub editing: bool,
    pub focused: TagField,
}

/// The Colour field as drawn: a "none" chip, the preset swatches and the hex box.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColourField {
    pub label: String,
    pub none: String,
    pub swatches: Vec<Swatch>,
    /// The highlighted pick: 0 for "none", `1..=6` for a preset, `None` for any other colour.
    pub picked: Option<usize>,
    /// Whether the swatch row has keyboard focus.
    pub row_focused: bool,
    pub hex: TextFieldView,
    /// The colour the hex box holds, once it parses.
    pub preview: Option<HexColor>,
    /// Set while the hex box holds something that isn't a colour.
    pub error: Option<String>,
}

/// One preset swatch: its pick index (`1..=6`) and colour.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Swatch {
    pub pick: usize,
    pub color: HexColor,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActiveField {
    pub label: String,
    pub hint: String,
    pub checked: bool,
    pub focused: bool,
}

/// The Tag an Edit saves, as it opened.
struct Editing {
    id: TagId,
    /// The Transactions carrying it, for the callout.
    transactions: usize,
}

pub struct TagFormPresenter {
    ports: SharedPorts,
    editing: Option<Editing>,
    name: String,
    hex: String,
    is_active: bool,
    focused: TagField,
    refusal: Option<String>,
}

impl TagFormPresenter {
    /// A fresh Add form: no name and no colour (#352: a new Tag has none).
    pub fn add(ports: SharedPorts) -> Self {
        Self {
            ports,
            editing: None,
            name: String::new(),
            hex: String::new(),
            is_active: true,
            focused: TagField::Name,
            refusal: None,
        }
    }

    /// The Edit form pre-filled from Tag `id`, or `None` when it is gone.
    pub fn edit(ports: SharedPorts, id: TagId) -> Option<Self> {
        let (tag, transactions) = {
            let ports = ports.borrow();
            (ports.tag(id)?, ports.tag_usage(id).transactions)
        };
        Some(Self {
            ports,
            editing: Some(Editing { id, transactions }),
            name: tag.name,
            hex: tag
                .color
                .map(|colour| colour.as_str().to_string())
                .unwrap_or_default(),
            is_active: tag.is_active,
            focused: TagField::Name,
            refusal: None,
        })
    }

    pub fn handle(&mut self, event: TagFormEvent, router: &mut dyn TagsRouter) {
        match event {
            TagFormEvent::Name(text) => {
                self.name = text;
                self.refusal = None;
            }
            TagFormEvent::Hex(text) => self.hex = text.chars().take(HEX_LEN).collect(),
            TagFormEvent::Pick(index) => self.pick(index),
            TagFormEvent::StepPick { forward } => {
                let next = match (self.picked(), forward) {
                    (None, _) => 0,
                    (Some(index), true) => (index + 1).min(PICKS - 1),
                    (Some(index), false) => index.saturating_sub(1),
                };
                self.pick(next);
            }
            TagFormEvent::ToggleActive => {
                if self.editing.is_some() {
                    self.is_active = !self.is_active;
                }
            }
            TagFormEvent::NextField => self.cycle(false),
            TagFormEvent::PrevField => self.cycle(true),
            TagFormEvent::Focus(field) => {
                if self.order().contains(&field) {
                    self.focused = field;
                }
            }
            TagFormEvent::Cancel => router.close_dialog(None),
        }
    }

    /// Saves the form through the Interactors and closes the Dialog; a refusal stays on the form.
    /// A form that can't confirm does nothing.
    pub fn confirm(&mut self, router: &mut dyn TagsRouter) {
        let Some(draft) = self.draft().filter(|_| self.can_confirm()) else {
            return;
        };
        let saved = {
            let mut ports = self.ports.borrow_mut();
            let tags: &mut dyn crate::TagRepository = &mut *ports;
            match &self.editing {
                Some(editing) => EditTag {
                    id: editing.id,
                    draft,
                }
                .run(&mut *tags)
                .and_then(|tag| match (tag.is_active, self.is_active) {
                    (true, false) => DeactivateTag(tag.id).run(&mut *tags),
                    (false, true) => ReactivateTag(tag.id).run(&mut *tags),
                    _ => Ok(tag),
                }),
                None => AddTag(draft).run(&mut *tags),
            }
        };
        match saved {
            Ok(tag) => router.close_dialog(Some(TagsOutcome {
                change: TagChange::Saved,
                id: tag.id,
                name: tag.name,
            })),
            Err(refusal) => self.refusal = Some(name_error_text(&refusal)),
        }
    }

    pub fn view_model(&self) -> TagFormViewModel {
        let editing = self.editing.is_some();
        let (title, submit) = match &self.editing {
            Some(_) => (
                msg::tags_edit_title(&self.original_name()),
                msg::tags_edit_submit(),
            ),
            None => (msg::tags_add_title(), msg::tags_add_submit()),
        };
        let colour = self.colour();
        TagFormViewModel {
            frame: DialogFrame {
                title,
                submit,
                can_confirm: self.can_confirm(),
                destructive: false,
                hints: Vec::new(),
            },
            name: TextFieldView {
                label: lib_locale::msg::column_name(),
                text: self.name.clone(),
                placeholder: msg::tags_name_placeholder(),
                focused: self.focused == TagField::Name,
            },
            name_error: self.name_problem().map(|error| name_error_text(&error)),
            colour: ColourField {
                label: msg::tags_field_colour(),
                none: msg::tags_colour_none(),
                swatches: swatches()
                    .into_iter()
                    .zip(1..)
                    .map(|(color, pick)| Swatch { pick, color })
                    .collect(),
                picked: self.picked(),
                row_focused: self.focused == TagField::Swatches,
                hex: TextFieldView {
                    label: msg::tags_field_colour(),
                    text: self.hex.clone(),
                    placeholder: msg::tags_hex_placeholder(),
                    focused: self.focused == TagField::Hex,
                },
                preview: colour.clone().ok().flatten(),
                error: colour.is_err().then(msg::tags_error_hex),
            },
            active: editing.then(|| ActiveField {
                label: msg::tags_field_active(),
                hint: msg::tags_active_hint(),
                checked: self.is_active,
                focused: self.focused == TagField::Active,
            }),
            callout: match &self.editing {
                Some(editing) => msg::tags_edit_callout(count(editing.transactions)),
                None => msg::tags_add_callout(),
            },
            refusal: self.refusal.clone(),
            editing,
            focused: self.focused,
        }
    }

    /// The Tag's name before this edit, for the title.
    fn original_name(&self) -> String {
        self.editing
            .as_ref()
            .and_then(|editing| self.ports.borrow().tag(editing.id))
            .map(|tag: Tag| tag.name)
            .unwrap_or_else(|| self.name.clone())
    }

    fn order(&self) -> &'static [TagField] {
        if self.editing.is_some() {
            &TagField::EDIT_ORDER
        } else {
            &TagField::ADD_ORDER
        }
    }

    fn cycle(&mut self, backward: bool) {
        let order = self.order();
        let at = order
            .iter()
            .position(|field| *field == self.focused)
            .unwrap_or(0);
        let next = if backward {
            at + order.len() - 1
        } else {
            at + 1
        };
        self.focused = order[next % order.len()];
    }

    /// The colour the hex box holds: `Ok(None)` when blank, `Err` while it isn't `#RRGGBB`.
    fn colour(&self) -> Result<Option<HexColor>, lib_core::HexColorError> {
        if self.hex.trim().is_empty() {
            return Ok(None);
        }
        HexColor::parse(&self.hex).map(Some)
    }

    /// The highlighted pick: 0 for "none", `1..=6` for a preset, `None` for any other colour.
    fn picked(&self) -> Option<usize> {
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
    fn pick(&mut self, index: usize) {
        match index {
            0 => self.hex.clear(),
            _ => {
                if let Some(preset) = swatches().get(index - 1) {
                    self.hex = preset.as_str().to_string();
                }
            }
        }
    }

    /// What the Dialog submits, or `None` while the hex box is invalid.
    fn draft(&self) -> Option<TagDraft> {
        Some(TagDraft {
            name: self.name.clone(),
            color: self.colour().ok()?,
        })
    }

    fn own_id(&self) -> Option<TagId> {
        self.editing.as_ref().map(|editing| editing.id)
    }

    /// The name's problem against every other Tag, an empty name included.
    fn problem(&self) -> Option<TagError> {
        name_error(&self.ports.borrow().tags(), self.own_id(), &self.name)
    }

    /// The name's problem to show: none while it is empty.
    fn name_problem(&self) -> Option<TagError> {
        if self.name.trim().is_empty() {
            return None;
        }
        self.problem()
    }

    fn can_confirm(&self) -> bool {
        self.problem().is_none() && self.colour().is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_support::stub;
    use super::*;
    use crate::TagRepository as _;
    use crate::fake::{RecordingRouter, RouterCall};

    const SHARED: TagId = TagId::seed(2);
    const BALI: TagId = TagId::seed(8);

    fn send(form: &mut TagFormPresenter, events: Vec<TagFormEvent>) -> Vec<RouterCall> {
        let mut router = RecordingRouter::default();
        for event in events {
            form.handle(event, &mut router);
        }
        router.calls
    }

    fn name(text: &str) -> TagFormEvent {
        TagFormEvent::Name(text.to_string())
    }

    fn confirm(form: &mut TagFormPresenter) -> Vec<RouterCall> {
        let mut router = RecordingRouter::default();
        form.confirm(&mut router);
        router.calls
    }

    #[test]
    fn a_new_form_has_no_name_no_colour_and_cannot_confirm() {
        let (_, shared) = stub();
        let view = TagFormPresenter::add(shared).view_model();

        assert_eq!(view.frame.title, "Add tag");
        assert_eq!(view.frame.submit, "Add tag");
        assert!(!view.frame.can_confirm);
        assert!(!view.frame.destructive);
        assert_eq!(view.name_error, None, "an empty name is kept, not reported");
        assert_eq!(view.colour.picked, Some(0));
        assert_eq!(view.colour.swatches.len(), 6);
        assert_eq!(view.active, None);
        assert!(view.callout.contains("matched ignoring case"));
        assert_eq!(view.focused, TagField::Name);
    }

    #[test]
    fn the_name_rule_is_a_form_error_checked_live() {
        let (_, shared) = stub();
        let mut form = TagFormPresenter::add(shared);
        let error = |form: &TagFormPresenter| form.view_model().name_error;

        send(&mut form, vec![name("Tax Deductible")]);
        assert_eq!(
            error(&form).as_deref(),
            Some("tax-deductible already exists")
        );
        assert!(!form.view_model().frame.can_confirm);
        send(&mut form, vec![name("- !")]);
        assert_eq!(
            error(&form).as_deref(),
            Some("A tag name needs a letter or a digit")
        );
        assert!(!form.view_model().frame.can_confirm);
        send(&mut form, vec![name("bali-2025")]);
        assert_eq!(
            error(&form).as_deref(),
            Some("Bali 2025 already exists"),
            "an inactive Tag's name stays taken"
        );
        send(&mut form, vec![name("holiday")]);
        assert_eq!(error(&form), None);
        assert!(form.view_model().frame.can_confirm);
    }

    #[test]
    fn a_swatch_fills_the_hex_box_and_none_empties_it() {
        let (_, shared) = stub();
        let mut form = TagFormPresenter::add(shared);

        send(&mut form, vec![TagFormEvent::Pick(4)]);
        let view = form.view_model();
        assert_eq!(view.colour.hex.text, "#4A7C9E");
        assert_eq!(view.colour.picked, Some(4));
        assert_eq!(view.colour.preview, Some(swatches()[3].clone()));
        send(&mut form, vec![TagFormEvent::Pick(0)]);
        assert_eq!(form.view_model().colour.hex.text, "");
        send(&mut form, vec![TagFormEvent::Pick(99)]);
        assert_eq!(form.view_model().colour.picked, Some(0), "no such pick");
    }

    #[test]
    fn stepping_the_pick_stops_at_either_end_and_restarts_from_a_custom_colour() {
        let (_, shared) = stub();
        let mut form = TagFormPresenter::add(shared);
        let step = |form: &mut TagFormPresenter, forward| {
            send(form, vec![TagFormEvent::StepPick { forward }]);
            form.view_model().colour.picked
        };

        assert_eq!(step(&mut form, false), Some(0));
        for _ in 0..10 {
            step(&mut form, true);
        }
        assert_eq!(form.view_model().colour.picked, Some(PICKS - 1));
        send(&mut form, vec![TagFormEvent::Hex("#123456".into())]);
        assert_eq!(form.view_model().colour.picked, None);
        assert_eq!(step(&mut form, true), Some(0));
    }

    #[test]
    fn a_typed_hex_is_any_colour_and_an_unfinished_one_blocks_confirming() {
        let (_, shared) = stub();
        let mut form = TagFormPresenter::add(shared);
        send(&mut form, vec![name("camping")]);

        send(&mut form, vec![TagFormEvent::Hex("#12".into())]);
        let view = form.view_model();
        assert_eq!(
            view.colour.error.as_deref(),
            Some("A colour is # and six hex digits, e.g. #4A7C9E")
        );
        assert!(!view.frame.can_confirm);
        send(&mut form, vec![TagFormEvent::Hex("#123456".into())]);
        let view = form.view_model();
        assert_eq!(view.colour.error, None);
        assert!(view.colour.preview.is_some());
        assert!(view.frame.can_confirm);
        send(&mut form, vec![TagFormEvent::Hex("#1234567890".into())]);
        assert_eq!(
            form.view_model().colour.hex.text,
            "#123456",
            "the box stops at # and six digits"
        );
    }

    #[test]
    fn tab_cycles_the_fields_and_only_edit_reaches_active() {
        let (_, shared) = stub();
        let mut add = TagFormPresenter::add(shared.clone());
        let mut edit = TagFormPresenter::edit(shared, SHARED).expect("a stub Tag");
        let walk = |form: &mut TagFormPresenter, event: TagFormEvent, times| {
            let mut seen = Vec::new();
            for _ in 0..times {
                send(form, vec![event.clone()]);
                seen.push(form.view_model().focused);
            }
            seen
        };

        assert_eq!(
            walk(&mut add, TagFormEvent::NextField, 4),
            [
                TagField::Swatches,
                TagField::Hex,
                TagField::Name,
                TagField::Swatches
            ]
        );
        assert_eq!(
            walk(&mut edit, TagFormEvent::NextField, 4),
            [
                TagField::Swatches,
                TagField::Hex,
                TagField::Active,
                TagField::Name
            ]
        );
        assert_eq!(
            walk(&mut edit, TagFormEvent::PrevField, 1),
            [TagField::Active]
        );
        send(&mut add, vec![TagFormEvent::Focus(TagField::Active)]);
        assert_eq!(
            add.view_model().focused,
            TagField::Swatches,
            "Add has no Active"
        );
        send(&mut add, vec![TagFormEvent::ToggleActive]);
        assert_eq!(add.view_model().active, None);
    }

    #[test]
    fn an_edit_is_pre_filled_and_may_keep_its_own_name_but_not_take_anothers() {
        let (ports, shared) = stub();
        ports.borrow_mut().usage.uses.insert(
            SHARED,
            crate::TagUse {
                transactions: 9,
                ..crate::TagUse::default()
            },
        );
        let mut form = TagFormPresenter::edit(shared.clone(), SHARED).expect("a stub Tag");

        let view = form.view_model();
        assert_eq!(view.frame.title, "Edit tag \u{2014} shared");
        assert_eq!(view.frame.submit, "Save");
        assert_eq!(view.name.text, "shared");
        assert_eq!(view.colour.hex.text, "#4A7C9E");
        assert_eq!(view.colour.picked, Some(4));
        assert_eq!(
            view.active,
            Some(ActiveField {
                label: "Active".into(),
                hint: "inactive tags aren't offered when tagging a split".into(),
                checked: true,
                focused: false,
            })
        );
        assert!(view.callout.starts_with("Used by 9 transactions."));
        assert!(view.frame.can_confirm);
        send(&mut form, vec![name("Shared")]);
        assert_eq!(form.view_model().name_error, None);
        send(&mut form, vec![name("gift!")]);
        assert_eq!(
            form.view_model().name_error.as_deref(),
            Some("gift already exists")
        );
        assert!(TagFormPresenter::edit(shared, TagId::seed(999)).is_none());
    }

    #[test]
    fn confirming_an_add_writes_the_tag_active_and_closes_with_it() {
        let (ports, shared) = stub();
        let mut form = TagFormPresenter::add(shared);
        send(
            &mut form,
            vec![
                name("  holiday "),
                TagFormEvent::Pick(2),
                TagFormEvent::Cancel,
            ],
        );

        let calls = confirm(&mut form);

        let added = ports
            .borrow()
            .tags
            .tags()
            .into_iter()
            .find(|tag| tag.name == "holiday")
            .expect("the Tag was written");
        assert!(added.is_active);
        assert_eq!(added.color, Some(swatches()[1].clone()));
        assert_eq!(
            calls,
            [RouterCall::CloseDialog(Some(TagsOutcome {
                change: TagChange::Saved,
                id: added.id,
                name: "holiday".into(),
            }))],
            "a save raises no Toast"
        );
    }

    #[test]
    fn a_form_that_cannot_confirm_writes_nothing() {
        let (ports, shared) = stub();
        let before = ports.borrow().tags.clone();
        let mut form = TagFormPresenter::add(shared);

        assert_eq!(confirm(&mut form), []);
        send(
            &mut form,
            vec![name("holiday"), TagFormEvent::Hex("#12".into())],
        );
        assert_eq!(confirm(&mut form), []);
        assert_eq!(ports.borrow().tags, before);
    }

    #[test]
    fn confirming_an_edit_renames_recolours_and_deactivates_together() {
        let (ports, shared) = stub();
        let mut form = TagFormPresenter::edit(shared, SHARED).expect("a stub Tag");
        send(
            &mut form,
            vec![
                name("Shared costs"),
                TagFormEvent::Pick(0),
                TagFormEvent::ToggleActive,
            ],
        );

        let calls = confirm(&mut form);

        let saved = ports.borrow().tags.tag(SHARED).expect("still there");
        assert_eq!(
            (saved.name.as_str(), saved.color, saved.is_active),
            ("Shared costs", None, false)
        );
        assert_eq!(
            calls,
            [RouterCall::CloseDialog(Some(TagsOutcome {
                change: TagChange::Saved,
                id: SHARED,
                name: "Shared costs".into(),
            }))]
        );
    }

    #[test]
    fn an_edit_reactivates_an_inactive_tag() {
        let (ports, shared) = stub();
        let mut form = TagFormPresenter::edit(shared, BALI).expect("a stub Tag");
        assert!(
            !form
                .view_model()
                .active
                .is_some_and(|active| active.checked)
        );

        send(&mut form, vec![TagFormEvent::ToggleActive]);
        confirm(&mut form);

        assert!(
            ports
                .borrow()
                .tags
                .tag(BALI)
                .is_some_and(|tag| tag.is_active)
        );
    }

    #[test]
    fn a_tag_gone_under_the_dialog_is_refused_and_cancel_closes() {
        let (ports, shared) = stub();
        let mut form = TagFormPresenter::edit(shared, SHARED).expect("a stub Tag");
        ports.borrow_mut().tags.tags.retain(|tag| tag.id != SHARED);

        assert_eq!(confirm(&mut form), []);
        assert_eq!(
            form.view_model().refusal.as_deref(),
            Some("This tag has since been removed.")
        );
        assert_eq!(
            send(&mut form, vec![TagFormEvent::Cancel]),
            [RouterCall::CloseDialog(None)]
        );
    }
}
