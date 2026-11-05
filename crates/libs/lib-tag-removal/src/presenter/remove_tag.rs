//! The Remove tag Dialog (`docs/ux/desktop-mockups/19-tags/README.md`'s 7d), at the handoff's
//! narrower width. #353 settled that removing always deletes the Tag, used or not, and keeps the
//! neutral (not red) styling the handoff gives it: nothing but a label is lost. A used Tag still
//! sits behind the typed-back name (exact, case-sensitive), since the untagging can't be undone;
//! an unused one needs only **Remove tag**.
//!
//! Its confirm runs [`RemoveTag`] through the removal port, which untags the Splits carrying the
//! Tag and deletes it in the one commit of the input; a refusal stays in the Dialog. It raises its
//! own Toast on landing, so the page only learns what it did.

use lib_core::TagId;
use lib_locale::Segment;
use lib_tags::{TagChange, TagsOutcome};
use lib_toast::{Notice, ToastKind};
use lib_viper::dialog::{DialogInput, DialogRouter};
use lib_viper::view::{DialogFrame, TextFieldView, count};

use super::refusal_text;
use crate::interactor::RemoveTag;
use crate::msg;
use crate::protocols::{SharedTagRemoval, TagRemovalReads};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RemoveTagEvent {
    /// The confirmation field's whole new text.
    ConfirmText(String),
    Cancel,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoveTagViewModel {
    pub frame: DialogFrame,
    /// What removing does, the name and the transaction count tagged `strong`.
    pub warning: Vec<Segment>,
    /// `Type travel to confirm`; `None` for an unused Tag, which needs no typed name.
    pub confirm: Option<TextFieldView>,
    /// Why the last confirm was refused.
    pub refusal: Option<String>,
}

pub struct RemoveTagPresenter {
    removal: SharedTagRemoval,
    id: TagId,
    name: String,
    /// The Transactions carrying the Tag, read when the Dialog opens.
    transactions: usize,
    confirm_text: String,
    refusal: Option<String>,
}

impl RemoveTagPresenter {
    /// The Dialog for Tag `id`, its usage read from `reads` when it opens, or `None` when it is
    /// gone.
    pub fn new(removal: SharedTagRemoval, reads: &dyn TagRemovalReads, id: TagId) -> Option<Self> {
        let offered = reads.tag_count(id)?;
        Some(Self {
            removal,
            id,
            name: offered.tag.name,
            transactions: offered.transactions,
            confirm_text: String::new(),
            refusal: None,
        })
    }

    pub fn handle(&mut self, event: RemoveTagEvent, router: &mut dyn DialogRouter<TagsOutcome>) {
        match event {
            RemoveTagEvent::ConfirmText(text) => self.confirm_text = text,
            RemoveTagEvent::Cancel => router.close(None),
        }
    }

    /// Reads `input`: `select` and `submit` confirm once the Dialog allows it, and typing goes to
    /// the typed-back name, the only field, which an unused Tag doesn't have.
    pub fn input(&mut self, input: DialogInput, router: &mut dyn DialogRouter<TagsOutcome>) {
        match input {
            DialogInput::Select | DialogInput::Submit => self.confirm(router),
            DialogInput::Char(ch) if self.transactions > 0 => self.confirm_text.push(ch),
            DialogInput::Backspace if self.transactions > 0 => {
                self.confirm_text.pop();
            }
            _ => {}
        }
    }

    /// Removes the Tag once the Dialog allows it, raises the Toast and closes the Dialog; a
    /// refusal stays in the Dialog.
    pub fn confirm(&mut self, router: &mut dyn DialogRouter<TagsOutcome>) {
        if !self.can_confirm() {
            return;
        }
        let outcome = RemoveTag { id: self.id }.run(&mut *self.removal.borrow_mut());
        match outcome {
            Ok(removed) => {
                router.notify(Notice::Toast(
                    ToastKind::Success,
                    lib_locale::msg::toast_tag_deleted(&removed.name),
                ));
                router.close(Some(TagsOutcome {
                    change: TagChange::Removed,
                    id: removed.id,
                    name: removed.name,
                }));
            }
            Err(refusal) => self.refusal = Some(refusal_text(refusal)),
        }
    }

    pub fn view_model(&self) -> RemoveTagViewModel {
        let name = self.name.as_str();
        RemoveTagViewModel {
            frame: DialogFrame {
                title: msg::tags_remove_title(name),
                submit: msg::tags_remove_submit(),
                can_confirm: self.can_confirm(),
                destructive: false,
                hints: Vec::new(),
            },
            warning: if self.transactions == 0 {
                msg::tags_remove_unused(name)
            } else {
                msg::tags_remove_used(name, count(self.transactions))
            },
            confirm: (self.transactions > 0).then(|| TextFieldView {
                label: msg::tags_remove_confirm_label(name),
                text: self.confirm_text.clone(),
                placeholder: self.name.clone(),
                focused: true,
            }),
            refusal: self.refusal.clone(),
        }
    }

    /// Always for an unused Tag, else once the name is typed exactly (case-sensitive, as the
    /// Payees Dialog's).
    fn can_confirm(&self) -> bool {
        self.transactions == 0 || self.confirm_text == self.name
    }
}

#[cfg(test)]
mod tests {
    use lib_tags::TagRepository as _;
    use lib_tags::fake::{RecordingRouter, RouterCall};

    use super::super::test_support::shared_world;
    use super::*;
    use crate::fake::world::{ONE_OFF, SHARED};
    use crate::protocols::TagRemoval as _;

    fn typed(dialog: &mut RemoveTagPresenter, text: &str) {
        dialog.handle(
            RemoveTagEvent::ConfirmText(text.to_string()),
            &mut RecordingRouter::default(),
        );
    }

    fn strong(segments: &[Segment]) -> Vec<&str> {
        segments
            .iter()
            .filter(|segment| segment.tag.as_deref() == Some("strong"))
            .map(|segment| segment.text.as_str())
            .collect()
    }

    #[test]
    fn a_used_tag_quotes_its_transactions_and_asks_for_its_name() {
        let (rows, removal) = shared_world();
        let view = RemoveTagPresenter::new(removal.clone(), &*rows.borrow(), SHARED)
            .expect("a world Tag")
            .view_model();

        assert_eq!(view.frame.title, "Remove tag \u{2014} shared");
        assert_eq!(view.frame.submit, "Remove tag");
        assert!(!view.frame.destructive, "nothing but a label is lost");
        assert!(!view.frame.can_confirm);
        assert_eq!(strong(&view.warning), ["shared", "2 transactions"]);
        let confirm = view.confirm.expect("a used Tag asks for its name");
        assert_eq!(confirm.label, "Type shared to confirm");
        assert_eq!(confirm.placeholder, "shared");
        assert!(RemoveTagPresenter::new(removal, &*rows.borrow(), TagId::seed(999)).is_none());
    }

    #[test]
    fn an_unused_tag_needs_no_typed_name() {
        let (rows, removal) = shared_world();
        let view = RemoveTagPresenter::new(removal, &*rows.borrow(), ONE_OFF)
            .expect("a world Tag")
            .view_model();

        assert!(view.frame.can_confirm);
        assert_eq!(view.confirm, None);
        assert_eq!(strong(&view.warning), ["one-off"]);
    }

    #[test]
    fn confirm_waits_for_the_exact_name_then_removes_and_raises_its_toast() {
        let (rows, removal) = shared_world();
        let mut dialog =
            RemoveTagPresenter::new(removal, &*rows.borrow(), SHARED).expect("a world Tag");
        let mut router = RecordingRouter::default();

        typed(&mut dialog, "Shared");
        dialog.confirm(&mut router);
        assert_eq!(router.take(), []);
        assert!(rows.borrow().tags.tag(SHARED).is_some());

        typed(&mut dialog, "shared");
        dialog.confirm(&mut router);

        assert_eq!(
            router.take(),
            [
                RouterCall::Notice(Notice::Toast(
                    ToastKind::Success,
                    "Deleted tag shared".into()
                )),
                RouterCall::CloseDialog(Some(TagsOutcome {
                    change: TagChange::Removed,
                    id: SHARED,
                    name: "shared".into(),
                })),
            ]
        );
        assert!(rows.borrow().tags.tag(SHARED).is_none());
        assert!(
            rows.borrow().transactions_carrying(SHARED).is_empty(),
            "its Splits are untagged"
        );
    }

    #[test]
    fn typing_builds_the_name_and_select_removes_only_once_it_is_exact() {
        let (rows, removal) = shared_world();
        let mut dialog =
            RemoveTagPresenter::new(removal, &*rows.borrow(), SHARED).expect("a world Tag");
        let mut router = RecordingRouter::default();
        let mut press = |dialog: &mut RemoveTagPresenter, input| {
            dialog.input(input, &mut router);
        };

        for ch in "shareX".chars() {
            press(&mut dialog, DialogInput::Char(ch));
        }
        press(&mut dialog, DialogInput::Select);
        press(&mut dialog, DialogInput::NextField);
        assert!(rows.borrow().tags.tag(SHARED).is_some(), "not the name yet");
        press(&mut dialog, DialogInput::Backspace);
        press(&mut dialog, DialogInput::Char('d'));
        assert_eq!(
            dialog
                .view_model()
                .confirm
                .map(|field| field.text)
                .as_deref(),
            Some("shared")
        );
        press(&mut dialog, DialogInput::Submit);

        assert!(rows.borrow().tags.tag(SHARED).is_none());
        assert!(matches!(
            router.take().last(),
            Some(RouterCall::CloseDialog(Some(_)))
        ));
    }

    #[test]
    fn an_unused_tag_takes_no_typing_and_select_removes_it() {
        let (rows, removal) = shared_world();
        let mut dialog =
            RemoveTagPresenter::new(removal, &*rows.borrow(), ONE_OFF).expect("a world Tag");
        let mut router = RecordingRouter::default();

        dialog.input(DialogInput::Char('x'), &mut router);
        assert_eq!(dialog.view_model().confirm, None);
        dialog.input(DialogInput::Select, &mut router);

        assert!(rows.borrow().tags.tag(ONE_OFF).is_none());
    }

    #[test]
    fn an_unused_tag_is_removed_on_confirm_alone() {
        let (rows, removal) = shared_world();
        let mut dialog =
            RemoveTagPresenter::new(removal, &*rows.borrow(), ONE_OFF).expect("a world Tag");

        dialog.confirm(&mut RecordingRouter::default());

        assert!(rows.borrow().tags.tag(ONE_OFF).is_none());
    }

    #[test]
    fn a_tag_gone_under_the_dialog_is_refused_and_cancel_closes() {
        let (rows, removal) = shared_world();
        let mut dialog =
            RemoveTagPresenter::new(removal, &*rows.borrow(), ONE_OFF).expect("a world Tag");
        rows.borrow_mut().tags.tags.retain(|tag| tag.id != ONE_OFF);
        let mut router = RecordingRouter::default();

        dialog.confirm(&mut router);

        assert_eq!(router.take(), []);
        assert_eq!(
            dialog.view_model().refusal.as_deref(),
            Some("This tag has since been removed.")
        );
        dialog.handle(RemoveTagEvent::Cancel, &mut router);
        assert_eq!(router.take(), [RouterCall::CloseDialog(None)]);
    }
}
