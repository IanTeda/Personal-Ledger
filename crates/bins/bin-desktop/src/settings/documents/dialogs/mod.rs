//! The Document type dialogs: Add, Edit and Remove a type, and the notice Other gets instead of a
//! Remove. [`DocumentTypesDialog`] is the value `Shell`'s Dialog host holds while one is open
//! (`OpenDialog::DocumentTypes`). It draws itself (`Dialog::render`), every click reaching the form
//! through the host's one edit callback, reads itself back for the UI tests and confirms itself
//! through `apply.rs`.

pub mod add_edit;
pub mod remove;

use std::rc::Rc;

use gpui::{AnyElement, App, Window};
use lib_core::DocumentTypeId;
use lib_documents::types::{RemindLead, TracksDate};

use super::form::{DocumentTypeForm, FormField, RemoveForm};
use crate::{
    chrome::dialog_host::{
        Dialog, DialogActions, DialogKey, DialogOutcome, DialogReadback, DialogScope,
    },
    dialog::OnClick,
    form::{field::TextField, fields::OnSegmentClick, select_field::OnOptionClick},
};
use add_edit::OnFieldClick;

/// The dialog open over the Document Types page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DocumentTypesDialog {
    Add(DocumentTypeForm),
    Edit(DocumentTypeId, DocumentTypeForm),
    /// Removing the type with this id; the select is where its Filed files move to.
    Remove(DocumentTypeId, RemoveForm),
    /// Other has no remove action, so this only says why.
    DefaultNotice,
}

impl DocumentTypesDialog {
    pub fn form_mut(&mut self) -> Option<&mut DocumentTypeForm> {
        match self {
            Self::Add(form) | Self::Edit(_, form) => Some(form),
            _ => None,
        }
    }

    /// A click on a field of the Add/Edit form: focuses it, and a Financial year chip also
    /// toggles.
    pub fn click_field(&mut self, field: FormField) {
        if let Some(form) = self.form_mut() {
            form.focus(field);
            if field == FormField::FinancialYear {
                form.toggle_financial_year();
            }
        }
    }

    pub fn click_tracks(&mut self, option: Option<TracksDate>) {
        if let Some(form) = self.form_mut() {
            form.focus(FormField::TracksDate);
            form.set_tracks_date(option);
        }
    }

    /// Remind is inert without a Tracks date, so it only takes focus while enabled.
    pub fn click_remind(&mut self, option: Option<RemindLead>) {
        if let Some(form) = self.form_mut() {
            if form.remind_enabled() {
                form.focus(FormField::Remind);
            }
            form.set_remind(option);
        }
    }

    /// A click on the Remove dialog's destination field opens or closes its select.
    pub fn click_destination_field(&mut self) {
        if let Self::Remove(_, form) = self {
            form.click_select();
        }
    }

    pub fn click_destination_option(&mut self, index: usize) {
        if let Self::Remove(_, form) = self {
            form.choose(index);
        }
    }

    /// Draws the open dialog. Edit and Remove read their type from the Ledger; a type that has gone
    /// draws nothing rather than panicking mid-render.
    fn draw(&self, scope: &DialogScope, actions: &DialogActions, cx: &App) -> Option<AnyElement> {
        let types = scope.ledger.read(cx).ledger().document_types();
        let on_field_click: OnFieldClick = {
            let actions = actions.clone();
            Rc::new(move |field, _window: &mut Window, cx: &mut App| {
                actions.edit(cx, |dialog: &mut Self, _| {
                    dialog.click_field(field);
                    None
                });
            })
        };
        let on_tracks_click: OnSegmentClick<Option<TracksDate>> = {
            let actions = actions.clone();
            Rc::new(move |option, _window: &mut Window, cx: &mut App| {
                actions.edit(cx, |dialog: &mut Self, _| {
                    dialog.click_tracks(option);
                    None
                });
            })
        };
        let on_remind_click: OnSegmentClick<Option<RemindLead>> = {
            let actions = actions.clone();
            Rc::new(move |option, _window: &mut Window, cx: &mut App| {
                actions.edit(cx, |dialog: &mut Self, _| {
                    dialog.click_remind(option);
                    None
                });
            })
        };
        let on_destination_field_click: OnClick = {
            let actions = actions.clone();
            Rc::new(move |_window: &mut Window, cx: &mut App| {
                actions.edit(cx, |dialog: &mut Self, _| {
                    dialog.click_destination_field();
                    None
                });
            })
        };
        let on_destination_option_click: OnOptionClick = {
            let actions = actions.clone();
            Rc::new(move |index, _window: &mut Window, cx: &mut App| {
                actions.edit(cx, |dialog: &mut Self, _| {
                    dialog.click_destination_option(index);
                    None
                });
            })
        };
        let form_handlers = || add_edit::FormHandlers {
            on_field_click: on_field_click.clone(),
            on_tracks_click: on_tracks_click.clone(),
            on_remind_click: on_remind_click.clone(),
            on_cancel: actions.cancel.clone(),
            on_confirm: actions.confirm.clone(),
        };
        match self {
            Self::Add(form) => Some(add_edit::render_form(
                add_edit::FormProps {
                    form,
                    edit: None,
                    error: form.name_error(),
                    valid: form.is_valid(),
                    handlers: form_handlers(),
                },
                cx,
            )),
            Self::Edit(id, form) => {
                let row = types.iter().find(|row| row.id == *id)?;
                Some(add_edit::render_form(
                    add_edit::FormProps {
                        form,
                        edit: Some(row),
                        error: form.name_error(),
                        valid: form.is_valid(),
                        handlers: form_handlers(),
                    },
                    cx,
                ))
            }
            Self::Remove(id, form) => {
                let row = types.iter().find(|row| row.id == *id)?;
                Some(remove::render_remove(
                    remove::RemoveProps {
                        row,
                        destinations: &form.destinations,
                        destination: &form.select,
                        error: form.error,
                        handlers: remove::RemoveHandlers {
                            on_field_click: on_destination_field_click,
                            on_option_click: on_destination_option_click,
                            on_cancel: actions.cancel.clone(),
                            on_confirm: actions.confirm.clone(),
                        },
                    },
                    cx,
                ))
            }
            Self::DefaultNotice => Some(remove::render_default_notice(actions.cancel.clone(), cx)),
        }
    }
}

impl Dialog for DocumentTypesDialog {
    fn handle_own_key(&mut self, key: DialogKey) -> Option<DialogOutcome> {
        match self {
            Self::Add(form) | Self::Edit(_, form) => form.handle_own_key(key),
            Self::Remove(_, form) => form.handle_own_key(key),
            Self::DefaultNotice => None,
        }
    }

    fn focused_text(&mut self) -> Option<&mut TextField> {
        match self {
            Self::Add(form) | Self::Edit(_, form) if form.focused == FormField::Name => {
                Some(&mut form.name)
            }
            _ => None,
        }
    }

    fn cycle_field(&mut self) {
        if let Some(form) = self.form_mut() {
            form.cycle_focus(false);
        }
    }

    fn is_valid(&self) -> bool {
        match self {
            Self::Add(form) | Self::Edit(_, form) => form.is_valid(),
            Self::Remove(_, form) => form.is_valid(),
            Self::DefaultNotice => true,
        }
    }

    fn close_open_select(&mut self) -> bool {
        match self {
            Self::Remove(_, form) => {
                let was_open = form.select.is_open();
                form.select.cancel();
                was_open
            }
            _ => false,
        }
    }

    /// The legend: Remove and the notice have no fields to `tab` through.
    fn hints(&self) -> Option<Vec<crate::chrome::statusline::Hint>> {
        let mut hints = vec![
            ("esc", crate::msg::desktop_hint_cancel()),
            ("enter", crate::msg::desktop_hint_confirm()),
        ];
        if matches!(self, Self::Add(_) | Self::Edit(..)) {
            hints.push(("tab", crate::msg::desktop_hint_next_field()));
        }
        Some(crate::chrome::statusline::owned_hints(hints))
    }

    fn render(&self, scope: &DialogScope, actions: &DialogActions, cx: &App) -> Option<AnyElement> {
        self.draw(scope, actions, cx)
    }

    /// Reads back an Add or Edit form's name; the Remove and notice have no typed value.
    fn readback(&self, _scope: &DialogScope, _cx: &App) -> Option<DialogReadback> {
        let (kind, name) = match self {
            Self::Add(form) => ("AddDocumentType", Some(form.name.text())),
            Self::Edit(_, form) => ("EditDocumentType", Some(form.name.text())),
            Self::Remove(..) => ("RemoveDocumentType", None),
            Self::DefaultNotice => ("DefaultNotice", None),
        };
        Some(DialogReadback {
            kind,
            values: name
                .map(|name| ("name", name.to_string()))
                .into_iter()
                .collect(),
        })
    }
}
