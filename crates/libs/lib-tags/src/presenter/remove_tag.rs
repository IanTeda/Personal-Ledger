//! The Remove tag Dialog (`docs/ux/desktop-mockups/19-tags/README.md`'s 7d), at the handoff's
//! narrower width. #353 settled that removing always deletes the Tag, used or not, and keeps the
//! neutral (not red) styling the handoff gives it: nothing but a label is lost. A used Tag still
//! sits behind the typed-back name (exact, case-sensitive), since the untagging can't be undone;
//! an unused one needs only **Remove tag**.
//!
//! Its confirm is lent the reassign port, which untags the Splits carrying the Tag and deletes it
//! in the one commit of the input; a refusal stays in the Dialog. It raises its own Toast on
//! landing, so the page only learns what it did.

use lib_core::TagId;
use lib_locale::Segment;
use lib_toast::{Notice, ToastKind};
use lib_viper::view::{DialogFrame, TextFieldView};

use super::{count, refusal_text};
use crate::msg;
use crate::protocols::{SharedPorts, TagChange, TagReassign, TagsOutcome, TagsRouter};

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
    id: TagId,
    name: String,
    /// The Transactions carrying the Tag, read when the Dialog opens.
    transactions: usize,
    confirm_text: String,
    refusal: Option<String>,
}

impl RemoveTagPresenter {
    /// The Dialog for Tag `id`, its usage read when it opens, or `None` when it is gone.
    pub fn new(ports: &SharedPorts, id: TagId) -> Option<Self> {
        let ports = ports.borrow();
        let tag = ports.tag(id)?;
        Some(Self {
            id,
            name: tag.name,
            transactions: ports.tag_usage(id).transactions,
            confirm_text: String::new(),
            refusal: None,
        })
    }

    pub fn handle(&mut self, event: RemoveTagEvent, router: &mut dyn TagsRouter) {
        match event {
            RemoveTagEvent::ConfirmText(text) => self.confirm_text = text,
            RemoveTagEvent::Cancel => router.close_dialog(None),
        }
    }

    /// Removes the Tag through `reassign` once the Dialog allows it, raises the Toast and closes
    /// the Dialog; a refusal stays in the Dialog.
    pub fn confirm(&mut self, reassign: &mut dyn TagReassign, router: &mut dyn TagsRouter) {
        if !self.can_confirm() {
            return;
        }
        match reassign.remove_tag(self.id) {
            Ok(removed) => {
                router.notify(Notice::Toast(
                    ToastKind::Success,
                    lib_locale::msg::toast_tag_deleted(&removed.name),
                ));
                router.close_dialog(Some(TagsOutcome {
                    change: TagChange::Removed,
                    id: self.id,
                    name: removed.name,
                }));
            }
            Err(refusal) => self.refusal = Some(refusal_text(&refusal)),
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
    use super::super::test_support::stub;
    use super::*;
    use crate::TagRepository as _;
    use crate::TagUse;
    use crate::fake::{InMemoryReassign, RecordingRouter, RouterCall};

    const SHARED: TagId = TagId::seed(2);
    const ONE_OFF: TagId = TagId::seed(7);

    fn used(transactions: usize) -> TagUse {
        TagUse {
            transactions,
            ..TagUse::default()
        }
    }

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
        let (ports, shared) = stub();
        ports.borrow_mut().usage.uses.insert(SHARED, used(4));
        let view = RemoveTagPresenter::new(&shared, SHARED)
            .expect("a stub Tag")
            .view_model();

        assert_eq!(view.frame.title, "Remove tag \u{2014} shared");
        assert_eq!(view.frame.submit, "Remove tag");
        assert!(!view.frame.destructive, "nothing but a label is lost");
        assert!(!view.frame.can_confirm);
        assert_eq!(strong(&view.warning), ["shared", "4 transactions"]);
        let confirm = view.confirm.expect("a used Tag asks for its name");
        assert_eq!(confirm.label, "Type shared to confirm");
        assert_eq!(confirm.placeholder, "shared");
        assert!(RemoveTagPresenter::new(&shared, TagId::seed(999)).is_none());
    }

    #[test]
    fn an_unused_tag_needs_no_typed_name() {
        let (_, shared) = stub();
        let view = RemoveTagPresenter::new(&shared, ONE_OFF)
            .expect("a stub Tag")
            .view_model();

        assert!(view.frame.can_confirm);
        assert_eq!(view.confirm, None);
        assert_eq!(strong(&view.warning), ["one-off"]);
    }

    #[test]
    fn confirm_waits_for_the_exact_name_then_removes_and_raises_its_toast() {
        let (ports, shared) = stub();
        ports.borrow_mut().usage.uses.insert(SHARED, used(4));
        let mut reassign = InMemoryReassign::new(ports.clone());
        let mut dialog = RemoveTagPresenter::new(&shared, SHARED).expect("a stub Tag");
        let mut router = RecordingRouter::default();

        typed(&mut dialog, "Shared");
        dialog.confirm(&mut reassign, &mut router);
        assert_eq!(router.take(), []);
        assert!(ports.borrow().tags.tag(SHARED).is_some());

        typed(&mut dialog, "shared");
        dialog.confirm(&mut reassign, &mut router);

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
        assert!(ports.borrow().tags.tag(SHARED).is_none());
    }

    #[test]
    fn an_unused_tag_is_removed_on_confirm_alone() {
        let (ports, shared) = stub();
        let mut dialog = RemoveTagPresenter::new(&shared, ONE_OFF).expect("a stub Tag");

        dialog.confirm(
            &mut InMemoryReassign::new(ports.clone()),
            &mut RecordingRouter::default(),
        );

        assert!(ports.borrow().tags.tag(ONE_OFF).is_none());
    }

    #[test]
    fn a_tag_gone_under_the_dialog_is_refused_and_cancel_closes() {
        let (ports, shared) = stub();
        let mut dialog = RemoveTagPresenter::new(&shared, ONE_OFF).expect("a stub Tag");
        ports.borrow_mut().tags.tags.retain(|tag| tag.id != ONE_OFF);
        let mut router = RecordingRouter::default();

        dialog.confirm(&mut InMemoryReassign::new(ports), &mut router);

        assert_eq!(router.take(), []);
        assert_eq!(
            dialog.view_model().refusal.as_deref(),
            Some("This tag has since been removed.")
        );
        dialog.handle(RemoveTagEvent::Cancel, &mut router);
        assert_eq!(router.take(), [RouterCall::CloseDialog(None)]);
    }
}
