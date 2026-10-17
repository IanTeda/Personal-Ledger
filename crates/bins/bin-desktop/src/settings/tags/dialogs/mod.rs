//! The Tags dialogs: Add, Edit, Remove and Merge a tag. [`TagsDialog`] is the value `Shell`'s Dialog
//! host holds while one is open (`OpenDialog::Tags`); [`TagsDialog::render`] draws whichever is
//! open, and every click inside it reaches the form through the handlers the Tags page builds.

pub mod add;
pub mod edit;
pub mod merge;
pub mod remove;

use gpui::{AnyElement, App};
use lib_core::TagId;
use lib_tags::{Tag, get, transaction_count};

use super::form::{MergeTagsForm, RemoveTagForm, TagForm};
use crate::{
    chrome::dialog_host::{Dialog, DialogKey, DialogOutcome},
    dialog::OnClick,
    form::colour_field::OnPick,
    form::field::TextField,
    settings::hints::{confirm_dialog_hints, merge_tags_dialog_hints, tag_dialog_hints},
    transactions::Transaction,
};
use add::OnFieldClick;
use merge::{OnFieldClick as OnMergeFieldClick, OnOptionClick};

/// Which Tags dialog is open on the Tags page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TagsDialog {
    Add(TagForm),
    /// Editing the Tag with this [`Tag::id`].
    Edit(TagId, TagForm),
    /// Removing the Tag with this [`Tag::id`].
    Remove(TagId, RemoveTagForm),
    /// Merging one Tag into another; either select is empty until chosen (the palette's
    /// `tags merge` opens with neither, `m` on an unflagged Tag with only the source).
    Merge(MergeTagsForm),
}

/// Every click the open Tags dialog can make. The page builds it once and each dialog takes the
/// part it draws.
pub struct TagsDialogHandlers {
    pub on_field_click: OnFieldClick,
    pub on_pick: OnPick,
    pub on_toggle_active: OnClick,
    pub on_merge_field_click: OnMergeFieldClick,
    pub on_merge_option_click: OnOptionClick,
    pub on_cancel: OnClick,
    pub on_confirm: OnClick,
}

impl TagsDialogHandlers {
    /// The Add and Edit dialogs' share of the handlers.
    fn name_dialog(&self) -> add::TagDialogHandlers {
        add::TagDialogHandlers {
            on_field_click: self.on_field_click.clone(),
            on_pick: self.on_pick.clone(),
            on_cancel: self.on_cancel.clone(),
            on_confirm: self.on_confirm.clone(),
        }
    }
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

    /// The Add or Edit form, which the Name field and the swatches edit.
    pub fn name_form_mut(&mut self) -> Option<&mut TagForm> {
        match self {
            Self::Add(form) | Self::Edit(_, form) => Some(form),
            Self::Remove(..) | Self::Merge(_) => None,
        }
    }

    /// Draws the open Dialog. Edit and Remove read their Tag from `tags` and count its Splits in
    /// `transactions`; a Tag that has gone draws nothing rather than panicking mid-render.
    pub fn render(
        &self,
        tags: &[Tag],
        transactions: &[Transaction],
        handlers: TagsDialogHandlers,
        cx: &App,
    ) -> Option<AnyElement> {
        match self {
            Self::Add(form) => Some(add::render(
                form,
                form.name_error(),
                form.is_valid(),
                handlers.name_dialog(),
                cx,
            )),
            Self::Edit(id, form) => {
                let tag = get(tags, *id)?;
                Some(edit::render(
                    edit::EditTagProps {
                        original_name: &tag.name,
                        form,
                        name_error: form.name_error(),
                        valid: form.is_valid(),
                        transactions: transaction_count(transactions, *id),
                        on_toggle_active: handlers.on_toggle_active.clone(),
                    },
                    handlers.name_dialog(),
                    cx,
                ))
            }
            Self::Remove(id, form) => {
                let tag = get(tags, *id)?;
                Some(remove::render(
                    remove::RemoveTagProps {
                        tag,
                        form,
                        transactions: transaction_count(transactions, *id),
                        on_cancel: handlers.on_cancel.clone(),
                        on_confirm: handlers.on_confirm.clone(),
                    },
                    cx,
                ))
            }
            Self::Merge(form) => {
                let source = form.source_id();
                Some(merge::render(
                    merge::MergeTagsProps {
                        form,
                        source: source.and_then(|id| get(tags, id)),
                        target: form.target_id().and_then(|id| get(tags, id)),
                        transactions: source.map_or(0, |id| transaction_count(transactions, id)),
                        on_field_click: handlers.on_merge_field_click,
                        on_option_click: handlers.on_merge_option_click,
                        on_cancel: handlers.on_cancel,
                        on_confirm: handlers.on_confirm,
                    },
                    cx,
                ))
            }
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

    fn hints(&self) -> Option<Vec<(&'static str, String)>> {
        Some(match self {
            Self::Add(_) => tag_dialog_hints(false),
            Self::Edit(..) => tag_dialog_hints(true),
            Self::Remove(..) => confirm_dialog_hints(),
            Self::Merge(_) => merge_tags_dialog_hints(),
        })
    }
}
