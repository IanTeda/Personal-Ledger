//! The Tags Dialogs' confirm cycle (ADR-0037): the command each form describes and the [`Apply`]
//! that runs it against the Tags and Transactions stores. When `LedgerStore` replaces the
//! per-domain stores only [`TagsApply`] changes.

use gpui::{App, Entity};

use super::{
    TagDraft, TagError, edit_tag, form::TagsDialog, get, insert_tag, merge_tags, remove_tag,
    set_active, transaction_count,
};
use crate::{
    chrome::dialog_apply::{Apply, DialogForm, FormError},
    transactions::{TransactionsStore, edit_transactions},
    view::tags::state::TagsStore,
};

/// What confirming a Tags Dialog asks for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TagCommand {
    Add(TagDraft),
    /// Save the Tag's name and colour, then its Active flag.
    Edit(u32, TagDraft, bool),
    Remove(u32),
    Merge {
        source: u32,
        target: u32,
    },
}

/// What a landed [`TagCommand`] did, for the selection and Toast that follow it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TagOutcome {
    /// Added or edited: the Tag to select.
    Saved(u32),
    /// Removed; the name is read before the write removes it.
    Removed { name: String },
    /// Merged: both names, read before the write, and how many Transactions were retagged.
    Merged {
        source_name: String,
        target_name: String,
        retagged: usize,
        target: u32,
    },
}

impl DialogForm for TagsDialog {
    type Command = TagCommand;
    type Refusal = TagError;

    fn command(&self) -> Result<TagCommand, FormError> {
        match self {
            Self::Add(form) if form.is_valid() => {
                form.draft().map(TagCommand::Add).ok_or(FormError)
            }
            Self::Edit(id, form) if form.is_valid() => form
                .draft()
                .map(|draft| TagCommand::Edit(*id, draft, form.is_active))
                .ok_or(FormError),
            Self::Remove(id, form) if form.allows() => Ok(TagCommand::Remove(*id)),
            Self::Merge(form) => form
                .pair()
                .map(|(source, target)| TagCommand::Merge { source, target })
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

/// Runs [`TagCommand`]s against the Tags store, reading and rewriting the Transactions whose
/// Splits carry a Tag from theirs.
pub struct TagsApply {
    pub tags: Entity<TagsStore>,
    pub transactions: Entity<TransactionsStore>,
}

impl Apply<TagCommand> for TagsApply {
    type Outcome = TagOutcome;
    type Refusal = TagError;

    fn apply(&mut self, command: TagCommand, cx: &mut App) -> Result<TagOutcome, TagError> {
        match command {
            TagCommand::Add(draft) => self
                .tags
                .update(cx, |store, cx| {
                    store.mutate(cx, |rows| insert_tag(rows, &draft))
                })
                .map(TagOutcome::Saved),
            TagCommand::Edit(id, draft, is_active) => self
                .tags
                .update(cx, |store, cx| {
                    // The rules work on a copy, so a refused change never reaches the store.
                    let mut rows = store.tags().to_vec();
                    edit_tag(&mut rows, id, &draft)
                        .and_then(|()| set_active(&mut rows, id, is_active))?;
                    store.mutate(cx, |tags| *tags = rows);
                    Ok(())
                })
                .map(|()| TagOutcome::Saved(id)),
            TagCommand::Remove(id) => {
                let name = get(self.tags.read(cx).tags(), id)
                    .map(|tag| tag.name.clone())
                    .ok_or(TagError::NotFound)?;
                let mut rows = self.tags.read(cx).tags().to_vec();
                edit_transactions(&self.transactions, cx, |transactions| {
                    remove_tag(&mut rows, transactions, id)
                })?;
                self.tags.update(cx, |store, cx| {
                    store.mutate(cx, |tags| *tags = rows);
                });
                Ok(TagOutcome::Removed { name })
            }
            TagCommand::Merge { source, target } => {
                let rows = self.tags.read(cx).tags();
                let (Some(source_name), Some(target_name)) = (
                    get(rows, source).map(|tag| tag.name.clone()),
                    get(rows, target).map(|tag| tag.name.clone()),
                ) else {
                    return Err(TagError::NotFound);
                };
                let retagged = transaction_count(self.transactions.read(cx).transactions(), source);
                let mut rows = self.tags.read(cx).tags().to_vec();
                edit_transactions(&self.transactions, cx, |transactions| {
                    merge_tags(&mut rows, transactions, source, target)
                })?;
                self.tags.update(cx, |store, cx| {
                    store.mutate(cx, |tags| *tags = rows);
                });
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

#[cfg(test)]
mod tests {
    use gpui::{AppContext, TestAppContext};

    use super::*;
    use crate::{
        chrome::dialog_apply::confirm,
        tags::form::{MergeTagsForm, RemoveTagForm, TagForm},
    };

    fn apply(cx: &mut TestAppContext) -> TagsApply {
        let tags = cx.new(|_| TagsStore::new(lib_tags::default_tags()));
        let transactions = cx.new(|_| TransactionsStore::new(Vec::new()));
        TagsApply { tags, transactions }
    }

    #[gpui::test]
    fn an_invalid_form_is_not_a_command_and_a_free_name_lands(cx: &mut TestAppContext) {
        let mut apply = apply(cx);
        let rows = lib_tags::default_tags();
        let mut dialog = TagsDialog::Add(TagForm::new(rows));
        assert_eq!(dialog.command(), Err(FormError), "a blank name");

        if let TagsDialog::Add(form) = &mut dialog {
            form.name = crate::form::field::TextField::new("Groceries run");
        }
        let outcome = cx.update(|cx| confirm(&mut dialog, &mut apply, cx));
        assert!(matches!(outcome, Some(TagOutcome::Saved(_))));
    }

    #[gpui::test]
    fn a_vanished_tag_stays_open_with_the_refusal(cx: &mut TestAppContext) {
        let mut apply = apply(cx);
        let mut dialog = TagsDialog::Remove(999, RemoveTagForm::new("Gone", 0));
        let outcome = cx.update(|cx| confirm(&mut dialog, &mut apply, cx));
        assert_eq!(outcome, None);
        assert!(matches!(&dialog, TagsDialog::Remove(_, form) if form.error.is_some()));
    }

    #[gpui::test]
    fn a_merge_reports_both_names(cx: &mut TestAppContext) {
        let mut apply = apply(cx);
        let rows = lib_tags::default_tags();
        let options = crate::tags::form::merge_options(&rows, &[], |name, _| name.to_string());
        let mut dialog = TagsDialog::Merge(MergeTagsForm::new(
            options,
            Some(rows[1].id),
            Some(rows[2].id),
        ));
        let outcome = cx.update(|cx| confirm(&mut dialog, &mut apply, cx));
        assert!(matches!(
            outcome,
            Some(TagOutcome::Merged { ref source_name, ref target_name, .. })
                if *source_name == rows[1].name && *target_name == rows[2].name
        ));
    }
}
