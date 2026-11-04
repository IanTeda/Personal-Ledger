//! The Merge tags Dialog (`docs/ux/desktop-mockups/19-tags/README.md`'s 7e, #354): two selects,
//! the Tag to fold in and the Tag to keep, over the irreversible callout with live counts.
//!
//! - Both selects list every Tag in usage order, most-used first, labelled `Shared (9 txns)`. The
//!   target's list leaves out the chosen source, so the two can never be equal; choosing as source
//!   the Tag already picked as target clears the target.
//! - The target keeps its name, colour and Active flag, so the callout names its colour, or says
//!   it has none (#352).
//!
//! The usage is read when the Dialog opens, as it is modal. Its confirm runs [`MergeTags`] through
//! the removal port, which retags the Splits and deletes the source in the one commit of the
//! input; a refusal stays in the Dialog. It raises its own Toast on landing, so the page only
//! learns what it did.

use lib_core::TagId;
use lib_locale::Segment;
use lib_tags::{Tag, TagChange, TagsOutcome};
use lib_toast::{Notice, ToastKind};
use lib_viper::dialog::{DialogInput, DialogRouter};
use lib_viper::view::{Choice, DialogFrame, SelectList, SelectMove, SelectView, count};

use super::refusal_text;
use crate::interactor::MergeTags;
use crate::msg;
use crate::protocols::{SharedTagRemoval, TagRemovalReads};

/// The Dialog's selects, in `Tab` order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MergeField {
    #[default]
    Source,
    Target,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MergeTagsEvent {
    /// A pick for the source, closing the lists.
    Source(TagId),
    /// A pick for the target, closing the lists.
    Target(TagId),
    /// A click on a select's closed field: focuses it, closes the other's list and opens or
    /// closes its own.
    Toggle(MergeField),
    /// A click on a field, which closes an open list.
    Focus(MergeField),
    /// Closes an open list without picking: the host's `back`.
    CloseLists,
    Cancel,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MergeTagsViewModel {
    pub frame: DialogFrame,
    pub source: SelectView<TagId>,
    pub target: SelectView<TagId>,
    /// The chosen Tags' names, without the counts the select labels carry.
    pub source_name: Option<String>,
    pub target_name: Option<String>,
    /// The irreversible callout: what the merge does once both Tags are chosen, else a prompt to
    /// choose them.
    pub callout: Vec<Segment>,
    /// Whether the callout is the prompt, rather than the live figures.
    pub choosing: bool,
    /// Why the last confirm was refused.
    pub refusal: Option<String>,
    pub focused: MergeField,
    /// The source's list: open, and its highlight.
    pub source_list: SelectList,
    /// The target's list: open, and its highlight.
    pub target_list: SelectList,
}

/// A Tag a select offers, with its Transactions read when the Dialog opened.
#[derive(Debug, Clone)]
struct Offered {
    tag: Tag,
    transactions: usize,
}

pub struct MergeTagsPresenter {
    removal: SharedTagRemoval,
    /// Every Tag in usage order.
    options: Vec<Offered>,
    source: Option<TagId>,
    target: Option<TagId>,
    focused: MergeField,
    source_list: SelectList,
    target_list: SelectList,
    refusal: Option<String>,
}

impl MergeTagsPresenter {
    /// The Dialog opened on `source` and `target` (the target ignored if equal to it), focusing
    /// the first select still empty, its options read from `reads` when it opens.
    pub fn new(
        removal: SharedTagRemoval,
        reads: &dyn TagRemovalReads,
        source: Option<TagId>,
        target: Option<TagId>,
    ) -> Self {
        let options: Vec<Offered> = reads
            .tags_by_usage()
            .into_iter()
            .map(|count| Offered {
                tag: count.tag,
                transactions: count.transactions,
            })
            .collect();
        let known = |id: &TagId| options.iter().any(|option| option.tag.id == *id);
        let source = source.filter(known);
        let target = target
            .filter(known)
            .filter(|target| Some(*target) != source);
        Self {
            removal,
            focused: if source.is_some() && target.is_none() {
                MergeField::Target
            } else {
                MergeField::Source
            },
            options,
            source,
            target,
            source_list: SelectList::default(),
            target_list: SelectList::default(),
            refusal: None,
        }
    }

    pub fn handle(&mut self, event: MergeTagsEvent, router: &mut dyn DialogRouter<TagsOutcome>) {
        match event {
            MergeTagsEvent::Source(id) => self.pick(MergeField::Source, id),
            MergeTagsEvent::Target(id) => self.pick(MergeField::Target, id),
            MergeTagsEvent::Toggle(field) => {
                self.focused = field;
                let other = match field {
                    MergeField::Source => MergeField::Target,
                    MergeField::Target => MergeField::Source,
                };
                self.list_mut(other).close();
                let choices = self.choices(field);
                let chosen = self.chosen(field);
                self.list_mut(field).toggle(&choices, chosen.as_ref());
            }
            MergeTagsEvent::Focus(field) => {
                self.close_lists();
                self.focused = field;
            }
            MergeTagsEvent::CloseLists => self.close_lists(),
            MergeTagsEvent::Cancel => router.close(None),
        }
    }

    /// Reads `input` for the focused select. `submit` merges a pair, as does `select` while the
    /// list is closed; `select` or `space` opens a closed list and picks from an open one, the
    /// arrows step a closed select's value or move an open list's highlight, and the field keys
    /// move to the other select, first committing an open list's highlight. Typing goes nowhere.
    pub fn input(&mut self, input: DialogInput, router: &mut dyn DialogRouter<TagsOutcome>) {
        let field = self.focused;
        match input {
            DialogInput::Submit => self.confirm(router),
            DialogInput::Select if !self.list(field).is_open() => self.confirm(router),
            DialogInput::NextField | DialogInput::PreviousField => {
                let choices = self.choices(field);
                if let Some(id) = self.list_mut(field).commit(&choices) {
                    self.pick(field, id);
                }
                self.focused = match field {
                    MergeField::Source => MergeField::Target,
                    MergeField::Target => MergeField::Source,
                };
            }
            DialogInput::Select
            | DialogInput::SelectNext
            | DialogInput::SelectPrevious
            | DialogInput::Char(' ') => {
                let choices = self.choices(field);
                let chosen = self.chosen(field);
                let moved = self.list_mut(field).input(input, &choices, chosen.as_ref());
                if let SelectMove::Chose(id) = moved {
                    self.pick(field, id);
                }
            }
            _ => {}
        }
    }

    fn list(&self, field: MergeField) -> SelectList {
        match field {
            MergeField::Source => self.source_list,
            MergeField::Target => self.target_list,
        }
    }

    fn list_mut(&mut self, field: MergeField) -> &mut SelectList {
        match field {
            MergeField::Source => &mut self.source_list,
            MergeField::Target => &mut self.target_list,
        }
    }

    fn close_lists(&mut self) {
        self.source_list.close();
        self.target_list.close();
    }

    /// `field`'s options.
    fn choices(&self, field: MergeField) -> Vec<Choice<TagId>> {
        self.select(String::new(), None, field).options
    }

    fn chosen(&self, field: MergeField) -> Option<TagId> {
        match field {
            MergeField::Source => self.source,
            MergeField::Target => self.target,
        }
    }

    /// Picks `id` for `field` and closes the lists. A Tag the select doesn't offer is ignored;
    /// choosing as source the Tag already the target clears the target.
    fn pick(&mut self, field: MergeField, id: TagId) {
        self.close_lists();
        if !self.offers(id) {
            return;
        }
        match field {
            MergeField::Source => {
                self.source = Some(id);
                if self.target == Some(id) {
                    self.target = None;
                }
            }
            MergeField::Target if self.source != Some(id) => self.target = Some(id),
            MergeField::Target => return,
        }
        self.refusal = None;
    }

    /// Merges the pair, raises the Toast and closes the Dialog; a refusal stays in the Dialog. A
    /// Dialog without a pair does nothing.
    pub fn confirm(&mut self, router: &mut dyn DialogRouter<TagsOutcome>) {
        let Some((source, target)) = self.pair() else {
            return;
        };
        let outcome = MergeTags { source, target }.run(&mut *self.removal.borrow_mut());
        match outcome {
            Ok(merged) => {
                router.notify(Notice::Toast(
                    ToastKind::Success,
                    lib_locale::msg::toast_tag_merged(
                        &merged.source_name,
                        &merged.target_name,
                        count(merged.retagged),
                    ),
                ));
                router.close(Some(TagsOutcome {
                    change: TagChange::Merged,
                    id: merged.target,
                    name: merged.target_name,
                }));
            }
            Err(refusal) => self.refusal = Some(refusal_text(refusal)),
        }
    }

    pub fn view_model(&self) -> MergeTagsViewModel {
        let pair = self.source_tag().zip(self.target_tag());
        let callout = match &pair {
            Some((source, target)) => {
                let transactions = count(self.transactions_of(source.id));
                if target.color.is_some() {
                    msg::tags_merge_callout(transactions, &source.name, &target.name)
                } else {
                    msg::tags_merge_callout_no_colour(transactions, &source.name, &target.name)
                }
            }
            None => vec![Segment {
                text: msg::tags_merge_choose(),
                tag: None,
                token: None,
            }],
        };
        MergeTagsViewModel {
            frame: DialogFrame {
                title: msg::tags_merge_title(),
                submit: self
                    .target_tag()
                    .map_or_else(msg::tags_merge_submit_empty, |target| {
                        msg::tags_merge_submit(&target.name)
                    }),
                can_confirm: pair.is_some(),
                destructive: false,
                hints: Vec::new(),
            },
            source: self.select(
                msg::tags_merge_source_label(),
                self.source,
                MergeField::Source,
            ),
            target: self.select(
                msg::tags_merge_target_label(),
                self.target,
                MergeField::Target,
            ),
            source_name: self.source_tag().map(|tag| tag.name.clone()),
            target_name: self.target_tag().map(|tag| tag.name.clone()),
            callout,
            choosing: pair.is_none(),
            refusal: self.refusal.clone(),
            focused: self.focused,
            source_list: self.source_list,
            target_list: self.target_list,
        }
    }

    /// The chosen source and target, once each is chosen and they differ.
    fn pair(&self) -> Option<(TagId, TagId)> {
        let (source, target) = (self.source?, self.target?);
        (source != target).then_some((source, target))
    }

    fn offers(&self, id: TagId) -> bool {
        self.options.iter().any(|option| option.tag.id == id)
    }

    fn source_tag(&self) -> Option<&Tag> {
        self.tag(self.source)
    }

    fn target_tag(&self) -> Option<&Tag> {
        self.tag(self.target)
    }

    fn tag(&self, id: Option<TagId>) -> Option<&Tag> {
        let tags: Vec<&Tag> = self.options.iter().map(|option| &option.tag).collect();
        let id = id?;
        tags.into_iter().find(|tag| tag.id == id)
    }

    fn transactions_of(&self, id: TagId) -> usize {
        self.options
            .iter()
            .find(|option| option.tag.id == id)
            .map_or(0, |option| option.transactions)
    }

    /// `field`'s select: every Tag for the source, every Tag but the source for the target.
    fn select(&self, label: String, chosen: Option<TagId>, field: MergeField) -> SelectView<TagId> {
        SelectView {
            label,
            options: self
                .options
                .iter()
                .filter(|option| field == MergeField::Source || Some(option.tag.id) != self.source)
                .map(|option| Choice {
                    value: option.tag.id,
                    label: msg::tags_merge_option(&option.tag.name, count(option.transactions)),
                })
                .collect(),
            chosen,
            focused: self.focused == field,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::rc::Rc;

    use lib_tags::TagRepository as _;
    use lib_tags::fake::{RecordingRouter, RouterCall};
    use lib_viper::dialog::DialogInput;

    use super::super::test_support::shared_world;
    use super::*;
    use crate::fake::InMemoryTagRemoval;
    use crate::fake::world::{GIFT, SHARED, WORK_TRIP, WORK_TRIP_DUPLICATE};

    type Rows = Rc<RefCell<InMemoryTagRemoval>>;

    /// The Dialog over the world, opened on `source` and `target`, with the rows it writes.
    fn open(source: Option<TagId>, target: Option<TagId>) -> (Rows, MergeTagsPresenter) {
        let (rows, removal) = shared_world();
        let dialog = MergeTagsPresenter::new(removal, &*rows.borrow(), source, target);
        (rows, dialog)
    }

    fn send(dialog: &mut MergeTagsPresenter, events: Vec<MergeTagsEvent>) -> Vec<RouterCall> {
        let mut router = RecordingRouter::default();
        for event in events {
            dialog.handle(event, &mut router);
        }
        router.calls
    }

    fn press(dialog: &mut MergeTagsPresenter, inputs: &[DialogInput]) -> Vec<RouterCall> {
        let mut router = RecordingRouter::default();
        for input in inputs {
            dialog.input(*input, &mut router);
        }
        router.calls
    }

    fn plain(segments: &[Segment]) -> String {
        segments
            .iter()
            .map(|segment| segment.text.as_str())
            .collect()
    }

    fn labels(view: &SelectView<TagId>) -> Vec<&str> {
        view.options
            .iter()
            .map(|choice| choice.label.as_str())
            .collect()
    }

    #[test]
    fn the_options_follow_usage_order_with_their_counts() {
        let (_, dialog) = open(None, None);
        let view = dialog.view_model();

        assert_eq!(
            labels(&view.source)[..2],
            ["shared (2 txns)", "Work Trip (2 txns)"]
        );
        assert!(labels(&view.source).contains(&"gift (1 txn)"));
        assert_eq!(view.source.options.len(), 9);
        assert_eq!(view.frame.title, "Merge tags");
        assert_eq!(view.frame.submit, "Merge");
        assert!(!view.frame.can_confirm);
        assert!(view.choosing);
        assert_eq!(
            plain(&view.callout),
            "Choose the tag to merge and the tag to keep."
        );
    }

    #[test]
    fn it_opens_on_its_pair_and_focuses_the_first_empty_select() {
        let (_, pair) = open(Some(WORK_TRIP_DUPLICATE), Some(WORK_TRIP));
        let (_, source_only) = open(Some(SHARED), None);
        let (_, equal) = open(Some(SHARED), Some(SHARED));

        let view = pair.view_model();
        assert_eq!(
            (view.source.chosen, view.target.chosen, view.focused),
            (
                Some(WORK_TRIP_DUPLICATE),
                Some(WORK_TRIP),
                MergeField::Source
            )
        );
        assert!(view.frame.can_confirm);
        assert_eq!(view.frame.submit, "Merge into \"work-trip\"");
        assert_eq!(source_only.view_model().focused, MergeField::Target);
        let view = equal.view_model();
        assert_eq!(view.target.chosen, None, "a Tag is not merged into itself");
        assert_eq!(view.focused, MergeField::Target);
    }

    #[test]
    fn the_target_list_leaves_out_the_source() {
        let (_, dialog) = open(Some(SHARED), None);
        let view = dialog.view_model();

        assert_eq!(view.target.options.len(), view.source.options.len() - 1);
        assert!(
            view.target
                .options
                .iter()
                .all(|choice| choice.value != SHARED)
        );
    }

    #[test]
    fn moving_the_source_onto_the_target_clears_the_target() {
        let (_, mut dialog) = open(Some(SHARED), Some(GIFT));

        send(&mut dialog, vec![MergeTagsEvent::Source(GIFT)]);
        let view = dialog.view_model();
        assert_eq!((view.source.chosen, view.target.chosen), (Some(GIFT), None));
        send(&mut dialog, vec![MergeTagsEvent::Target(GIFT)]);
        assert_eq!(
            dialog.view_model().target.chosen,
            None,
            "not the source itself"
        );
        send(&mut dialog, vec![MergeTagsEvent::Target(TagId::seed(999))]);
        assert_eq!(
            dialog.view_model().target.chosen,
            None,
            "not a Tag it offers"
        );
    }

    #[test]
    fn a_field_key_moves_to_the_other_select_and_a_click_focuses_one() {
        let (_, mut dialog) = open(None, None);

        press(&mut dialog, &[DialogInput::NextField]);
        assert_eq!(dialog.view_model().focused, MergeField::Target);
        assert!(dialog.view_model().target.focused);
        press(&mut dialog, &[DialogInput::PreviousField]);
        assert_eq!(dialog.view_model().focused, MergeField::Source);
        send(&mut dialog, vec![MergeTagsEvent::Focus(MergeField::Target)]);
        assert_eq!(dialog.view_model().focused, MergeField::Target);
    }

    #[test]
    fn select_opens_a_list_on_the_chosen_tag_and_picks_the_highlight() {
        let (_, mut dialog) = open(None, None);

        press(&mut dialog, &[DialogInput::Char(' ')]);
        assert!(dialog.view_model().source_list.is_open());
        assert_eq!(dialog.view_model().source_list.highlight(), Some(0));
        press(&mut dialog, &[DialogInput::SelectNext]);
        assert_eq!(dialog.view_model().source_list.highlight(), Some(1));
        assert_eq!(dialog.view_model().source.chosen, None, "only highlighted");
        let second = dialog.view_model().source.options[1].value;
        press(&mut dialog, &[DialogInput::Select]);

        let view = dialog.view_model();
        assert!(!view.source_list.is_open());
        assert_eq!(view.source.chosen, Some(second));
    }

    #[test]
    fn the_arrows_step_a_closed_select_and_select_then_merges_a_pair() {
        let (_, mut dialog) = open(None, None);

        press(&mut dialog, &[DialogInput::SelectNext]);
        assert!(dialog.view_model().source.chosen.is_some());
        assert_eq!(press(&mut dialog, &[DialogInput::Select]), [], "no pair");
        press(
            &mut dialog,
            &[DialogInput::NextField, DialogInput::SelectNext],
        );
        let calls = press(&mut dialog, &[DialogInput::Select]);

        assert!(
            matches!(
                calls.last(),
                Some(RouterCall::CloseDialog(Some(outcome))) if outcome.change == TagChange::Merged
            ),
            "{calls:?}"
        );
    }

    #[test]
    fn leaving_an_open_list_commits_its_highlight_and_typing_goes_nowhere() {
        let (_, mut dialog) = open(None, None);

        press(
            &mut dialog,
            &[
                DialogInput::Char(' '),
                DialogInput::SelectNext,
                DialogInput::Char('x'),
                DialogInput::Backspace,
                DialogInput::NextField,
            ],
        );

        let view = dialog.view_model();
        assert!(!view.source_list.is_open());
        assert!(view.source.chosen.is_some());
        assert_eq!(view.focused, MergeField::Target);
    }

    #[test]
    fn a_click_toggles_one_list_and_closes_the_other_and_close_lists_shuts_both() {
        let (_, mut dialog) = open(None, None);

        send(
            &mut dialog,
            vec![MergeTagsEvent::Toggle(MergeField::Source)],
        );
        assert!(dialog.view_model().source_list.is_open());
        send(
            &mut dialog,
            vec![MergeTagsEvent::Toggle(MergeField::Target)],
        );
        let view = dialog.view_model();
        assert!(!view.source_list.is_open() && view.target_list.is_open());
        assert_eq!(view.focused, MergeField::Target);
        send(&mut dialog, vec![MergeTagsEvent::CloseLists]);
        assert!(!dialog.view_model().target_list.is_open());
    }

    #[test]
    fn the_callout_bolds_the_count_and_names_the_targets_colour_or_its_lack() {
        let strong = |view: &MergeTagsViewModel| -> Vec<String> {
            view.callout
                .iter()
                .filter(|segment| segment.tag.as_deref() == Some("strong"))
                .map(|segment| segment.text.clone())
                .collect()
        };

        let coloured = open(Some(SHARED), Some(WORK_TRIP)).1.view_model();
        assert_eq!(strong(&coloured), ["2 transactions"]);
        assert!(plain(&coloured.callout).contains("using work-trip's colour"));
        assert!(!coloured.choosing);
        let plain_target = open(Some(SHARED), Some(WORK_TRIP_DUPLICATE)).1.view_model();
        assert!(plain(&plain_target.callout).contains("which has no colour"));
    }

    #[test]
    fn confirming_a_pair_merges_toasts_and_closes_on_the_target() {
        let (rows, mut dialog) = open(Some(WORK_TRIP_DUPLICATE), Some(WORK_TRIP));
        let mut router = RecordingRouter::default();

        dialog.confirm(&mut router);

        assert_eq!(
            router.take(),
            [
                RouterCall::Notice(Notice::Toast(
                    ToastKind::Success,
                    "Merged tag Work Trip into work-trip \u{b7} 2 transactions retagged".into()
                )),
                RouterCall::CloseDialog(Some(TagsOutcome {
                    change: TagChange::Merged,
                    id: WORK_TRIP,
                    name: "work-trip".into(),
                })),
            ]
        );
        assert!(rows.borrow().tags.tag(WORK_TRIP_DUPLICATE).is_none());
        assert!(rows.borrow().tags.tag(WORK_TRIP).is_some());
    }

    #[test]
    fn a_dialog_without_a_pair_does_nothing_and_a_gone_tag_is_refused() {
        let mut router = RecordingRouter::default();
        let (_, mut empty) = open(Some(SHARED), None);

        empty.confirm(&mut router);
        assert_eq!(router.take(), []);

        let (rows, mut dialog) = open(Some(SHARED), Some(GIFT));
        rows.borrow_mut().tags.tags.retain(|tag| tag.id != GIFT);
        dialog.confirm(&mut router);
        assert_eq!(router.take(), []);
        assert_eq!(
            dialog.view_model().refusal.as_deref(),
            Some("This tag has since been removed.")
        );
        assert_eq!(
            send(&mut dialog, vec![MergeTagsEvent::Cancel]),
            [RouterCall::CloseDialog(None)]
        );
    }
}
