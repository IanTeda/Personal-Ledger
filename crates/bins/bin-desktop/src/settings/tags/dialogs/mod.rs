//! The Tags dialogs: Add, Edit, Remove and Merge a tag. [`TagsDialog`] is the value the Dialog
//! host holds while one is open (`OpenDialog::Tags`). It draws itself (`Dialog::render`), each
//! click editing it through the host's one edit callback, and reads itself back for the UI tests.

pub mod add;
pub mod edit;
pub mod merge;
pub mod remove;

use std::rc::Rc;

use gpui::{AnyElement, App};
use lib_core::TagId;
use lib_tags::{Tag, get, transaction_count};

use super::form::{MergeTagsForm, RemoveTagForm, TagForm};
use crate::{
    chrome::dialog_host::{
        Dialog, DialogActions, DialogKey, DialogOutcome, DialogReadback, DialogScope,
    },
    dialog::OnClick,
    form::colour_field::OnPick,
    form::field::TextField,
    settings::hints::{confirm_dialog_hints, merge_tags_dialog_hints, tag_dialog_hints},
    settings::tags::form::TagField,
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

/// Every click the open Tags dialog can make, built from the host's [`DialogActions`]; each dialog
/// takes the part it draws.
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
    /// Each click edits the open Tags Dialog through `actions`.
    fn new(actions: &DialogActions) -> Self {
        let on_field_click: OnFieldClick = {
            let actions = actions.clone();
            Rc::new(move |field, _window, cx| {
                actions.edit(cx, |dialog: &mut TagsDialog, _| {
                    if let Some(form) = dialog.name_form_mut() {
                        form.focus(field);
                    }
                    None
                });
            })
        };
        let on_pick: OnPick = {
            let actions = actions.clone();
            Rc::new(move |index, _window, cx| {
                actions.edit(cx, |dialog: &mut TagsDialog, _| {
                    if let Some(form) = dialog.name_form_mut() {
                        form.focus(TagField::Swatches);
                        form.pick(index);
                    }
                    None
                });
            })
        };
        let on_toggle_active: OnClick = {
            let actions = actions.clone();
            Rc::new(move |_window, cx| {
                actions.edit(cx, |dialog: &mut TagsDialog, _| {
                    if let Some(form) = dialog.name_form_mut() {
                        form.toggle_active();
                    }
                    None
                });
            })
        };
        let on_merge_field_click: OnMergeFieldClick = {
            let actions = actions.clone();
            Rc::new(move |field, _window, cx| {
                actions.edit(cx, |dialog: &mut TagsDialog, _| {
                    if let TagsDialog::Merge(form) = dialog {
                        form.toggle(field);
                    }
                    None
                });
            })
        };
        let on_merge_option_click: OnOptionClick = {
            let actions = actions.clone();
            Rc::new(move |field, index, _window, cx| {
                actions.edit(cx, |dialog: &mut TagsDialog, _| {
                    if let TagsDialog::Merge(form) = dialog {
                        form.choose(field, index);
                    }
                    None
                });
            })
        };
        Self {
            on_field_click,
            on_pick,
            on_toggle_active,
            on_merge_field_click,
            on_merge_option_click,
            on_cancel: actions.cancel.clone(),
            on_confirm: actions.confirm.clone(),
        }
    }

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
    fn draw(
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

    fn hints(&self) -> Option<Vec<crate::chrome::statusline::Hint>> {
        Some(crate::chrome::statusline::owned_hints(match self {
            Self::Add(_) => tag_dialog_hints(false),
            Self::Edit(..) => tag_dialog_hints(true),
            Self::Remove(..) => confirm_dialog_hints(),
            Self::Merge(_) => merge_tags_dialog_hints(),
        }))
    }

    fn render(&self, scope: &DialogScope, actions: &DialogActions, cx: &App) -> Option<AnyElement> {
        let ledger = scope.ledger.read(cx).ledger();
        self.draw(
            ledger.tags(),
            ledger.transactions(),
            TagsDialogHandlers::new(actions),
            cx,
        )
    }

    /// Add and Edit read back their Name, Remove its typed-back name, and Merge the names of the
    /// Tags it has chosen.
    fn readback(&self, scope: &DialogScope, cx: &App) -> Option<DialogReadback> {
        let (kind, values) = match self {
            Self::Add(form) => ("AddTag", vec![("name", form.name.text().to_string())]),
            Self::Edit(_, form) => ("EditTag", vec![("name", form.name.text().to_string())]),
            Self::Remove(_, form) => (
                "RemoveTag",
                vec![("confirm", form.confirmation_name.text().to_string())],
            ),
            Self::Merge(form) => {
                let tags = scope.ledger.read(cx).ledger().tags();
                let name = |id| get(tags, id).map(|tag: &Tag| tag.name.clone());
                let values = [("source", form.source_id()), ("target", form.target_id())]
                    .into_iter()
                    .filter_map(|(label, id)| Some((label, name(id?)?)))
                    .collect();
                ("MergeTags", values)
            }
        };
        Some(DialogReadback { kind, values })
    }
}
