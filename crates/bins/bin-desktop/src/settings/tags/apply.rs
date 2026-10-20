//! The Tags Dialogs' confirm cycle (ADR-0037): the page command each form describes, a
//! `lib_ledger` command in each variant, confirmed through [`confirm_ledger`].

use gpui::App;
use lib_core::TagId;
use lib_ledger::{AddTag, EditTag, Ledger, LedgerCommand, MergeTags, RemoveTag};
use lib_tags::{TagError, get};
use lib_toast::ToastKind;

use super::dialogs::TagsDialog;
use crate::chrome::{
    dialog_apply::{DialogForm, FormError, OutcomeToast, confirm_ledger},
    dialog_host::{ConfirmDialog, DialogScope, DialogToast, Landed},
};

/// What confirming a Tags Dialog asks the Ledger for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TagCommand {
    Add(AddTag),
    /// Save the Tag's name and colour, then its Active flag.
    Edit(EditTag),
    Remove(RemoveTag),
    Merge(MergeTags),
}

/// What a landed [`TagCommand`] did, for the selection and Toast that follow it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TagOutcome {
    /// Added or edited: the Tag to select.
    Saved(TagId),
    /// Removed; the name is read before the write removes it.
    Removed { name: String },
    /// Merged: both names, read before the write, and how many Transactions were retagged.
    Merged {
        source_name: String,
        target_name: String,
        retagged: usize,
        target: TagId,
    },
}

/// A removal or a merge raises its Toast; a save selects its Tag instead.
impl OutcomeToast for TagOutcome {
    fn toast(&self) -> Option<DialogToast> {
        match self {
            Self::Saved(_) => None,
            Self::Removed { name } => {
                Some((ToastKind::Success, lib_locale::msg::toast_tag_deleted(name)))
            }
            Self::Merged {
                source_name,
                target_name,
                retagged,
                ..
            } => Some((
                ToastKind::Success,
                lib_locale::msg::toast_tag_merged(
                    source_name,
                    target_name,
                    i64::try_from(*retagged).unwrap_or(i64::MAX),
                ),
            )),
        }
    }
}

/// Removing or merging a Tag rewrites the Transactions whose Splits carry it, which the Ledger
/// does in the same command. Names are read before the write removes them, for the Toast.
impl LedgerCommand for TagCommand {
    type Outcome = TagOutcome;
    type Refusal = TagError;

    fn run(self, ledger: &mut Ledger) -> Result<TagOutcome, TagError> {
        match self {
            Self::Add(add) => add.run(ledger).map(TagOutcome::Saved),
            Self::Edit(edit) => {
                let id = edit.id;
                edit.run(ledger).map(|()| TagOutcome::Saved(id))
            }
            Self::Remove(remove) => {
                let name = tag_name(ledger, remove.0)?;
                remove.run(ledger)?;
                Ok(TagOutcome::Removed { name })
            }
            Self::Merge(merge) => {
                let (source_name, target_name) = (
                    tag_name(ledger, merge.source),
                    tag_name(ledger, merge.target),
                );
                let (source_name, target_name) = (source_name?, target_name?);
                let target = merge.target;
                let retagged = merge.run(ledger)?;
                Ok(TagOutcome::Merged {
                    source_name,
                    target_name,
                    retagged,
                    target,
                })
            }
        }
    }
}

fn tag_name(ledger: &Ledger, id: TagId) -> Result<String, TagError> {
    get(ledger.tags(), id)
        .map(|tag| tag.name.clone())
        .ok_or(TagError::NotFound)
}

impl DialogForm for TagsDialog {
    type Command = TagCommand;
    type Refusal = TagError;

    fn command(&self) -> Result<TagCommand, FormError> {
        match self {
            Self::Add(form) if form.is_valid() => form
                .draft()
                .map(|draft| TagCommand::Add(AddTag(draft)))
                .ok_or(FormError),
            Self::Edit(id, form) if form.is_valid() => form
                .draft()
                .map(|draft| {
                    TagCommand::Edit(EditTag {
                        id: *id,
                        draft,
                        is_active: form.is_active,
                    })
                })
                .ok_or(FormError),
            Self::Remove(id, form) if form.allows() => Ok(TagCommand::Remove(RemoveTag(*id))),
            Self::Merge(form) => form
                .pair()
                .map(|(source, target)| TagCommand::Merge(MergeTags { source, target }))
                .ok_or(FormError),
            _ => Err(FormError),
        }
    }

    fn refuse(&mut self, refusal: TagError) {
        match self {
            Self::Add(form) | Self::Edit(_, form) => form.error = Some(refusal),
            Self::Remove(_, form) => form.error = Some(refusal),
            Self::Merge(form) => form.error = Some(refusal),
        }
    }
}

impl ConfirmDialog for TagsDialog {
    type Outcome = TagOutcome;

    fn confirm(&mut self, scope: &DialogScope, cx: &mut App) -> Option<Landed<TagOutcome>> {
        confirm_ledger(self, &scope.ledger, cx)
    }
}

#[cfg(test)]
mod tests {
    use gpui::{Entity, TestAppContext};
    use lib_ledger::LedgerParts;

    use super::*;
    use crate::{
        form::field::TextField,
        ledger_store::{LedgerStore, test_ledger},
        settings::tags::form::{MergeTagsForm, RemoveTagForm, TagForm, merge_options},
    };

    fn ledger(cx: &mut TestAppContext) -> Entity<LedgerStore> {
        test_ledger(
            cx,
            LedgerParts {
                tags: lib_tags::default_tags(),
                ..LedgerParts::default()
            },
        )
    }

    fn tags(ledger: &Entity<LedgerStore>, cx: &TestAppContext) -> Vec<lib_tags::Tag> {
        ledger.read_with(cx, |store, _| store.ledger().tags().to_vec())
    }

    #[gpui::test]
    fn a_blank_name_stays_open_and_a_free_name_lands(cx: &mut TestAppContext) {
        let ledger = ledger(cx);
        let before = tags(&ledger, cx);
        let mut dialog = TagsDialog::Add(TagForm::new(before.clone()));

        let landed = cx.update(|cx| confirm_ledger(&mut dialog, &ledger, cx));
        assert!(landed.is_none());
        assert_eq!(tags(&ledger, cx).len(), before.len());

        if let TagsDialog::Add(form) = &mut dialog {
            form.name = TextField::new("Groceries run");
        }
        let landed = cx
            .update(|cx| confirm_ledger(&mut dialog, &ledger, cx))
            .expect("a free name lands");
        assert_eq!(landed.toast, None);
        assert!(matches!(landed.outcome, TagOutcome::Saved(_)));
        assert_eq!(tags(&ledger, cx).len(), before.len() + 1);
    }

    #[gpui::test]
    fn a_vanished_tag_is_refused_onto_the_form_and_stays_open(cx: &mut TestAppContext) {
        let ledger = ledger(cx);
        let before = tags(&ledger, cx);
        let mut dialog = TagsDialog::Remove(TagId::seed(999), RemoveTagForm::new("Gone", 0));

        let landed = cx.update(|cx| confirm_ledger(&mut dialog, &ledger, cx));

        assert!(landed.is_none());
        assert!(matches!(&dialog, TagsDialog::Remove(_, form) if form.error.is_some()));
        assert_eq!(tags(&ledger, cx).len(), before.len());
    }

    #[gpui::test]
    fn a_merge_reports_both_names_and_toasts_them(cx: &mut TestAppContext) {
        let ledger = ledger(cx);
        let rows = tags(&ledger, cx);
        let options = merge_options(&rows, &[], |name, _| name.to_string());
        let mut dialog = TagsDialog::Merge(MergeTagsForm::new(
            options,
            Some(rows[1].id),
            Some(rows[2].id),
        ));

        let landed = cx
            .update(|cx| confirm_ledger(&mut dialog, &ledger, cx))
            .expect("a merge of two Tags lands");

        assert!(matches!(
            landed.outcome,
            TagOutcome::Merged { ref source_name, ref target_name, .. }
                if *source_name == rows[1].name && *target_name == rows[2].name
        ));
        assert!(matches!(landed.toast, Some((ToastKind::Success, _))));
        assert!(tags(&ledger, cx).iter().all(|tag| tag.id != rows[1].id));
    }
}
