//! The Units page's Dialogs: Add, Edit and Delete unit. [`UnitsDialog`] is the value `Shell`'s
//! Dialog host holds while one is open (`OpenDialog::Units`); [`UnitsDialog::render`] draws
//! whichever is open, and every click inside it reaches the form as a [`Change`].

pub mod add;
pub mod delete;
pub mod edit;

use std::rc::Rc;

use gpui::{AnyElement, App, Window, div, prelude::*};
use lib_units::{UnitId, UnitKind, UnitRow};

use super::form::{AddUnitField, DeleteUnitForm, UnitForm};
use crate::{chrome::dialog_host::Dialog, dialog, form::field::TextField};

/// The open Units Dialog. Edit and Delete carry the row's [`UnitId`], so the confirm knows which
/// row to overwrite or remove even if the table shifts under the open Dialog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnitsDialog {
    Add(UnitForm),
    Edit(UnitId, UnitForm),
    Delete(UnitId, DeleteUnitForm),
}

/// Applies a [`Change`] to the open Units Dialog where the Dialog host holds it
/// ([`UnitsDialog::apply_change`]), then repaints.
pub type OnChange = Rc<dyn Fn(Change, &mut Window, &mut App)>;

/// What a click inside a Units Dialog changes on its form.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Change {
    Field(AddUnitField),
    Kind(UnitKind),
}

/// The handlers a Units Dialog binds: changes to the open form, cancel and confirm.
#[derive(Clone)]
pub struct UnitsDialogHandlers {
    pub on_change: OnChange,
    pub on_cancel: dialog::OnClick,
    pub on_confirm: dialog::OnClick,
}

impl UnitsDialog {
    /// Draws the open Dialog. Delete reads its row from `units`; a row that has gone draws
    /// nothing rather than panicking mid-render.
    pub fn render(&self, units: &[UnitRow], handlers: UnitsDialogHandlers, cx: &App) -> AnyElement {
        match self {
            Self::Add(form) => add::render(form, handlers, cx),
            Self::Edit(_, form) => edit::render(form, handlers, cx),
            Self::Delete(id, form) => match units.iter().find(|row| row.id == *id) {
                Some(row) => delete::render(row, form, handlers, cx),
                None => div().into_any_element(),
            },
        }
    }

    /// Applies a click's change to the Add or Edit form; Delete has no field to click into and
    /// no Type.
    pub fn apply_change(&mut self, change: Change) {
        if let Self::Add(form) | Self::Edit(_, form) = self {
            match change {
                Change::Field(field) => form.focused_field = field,
                Change::Kind(kind) => form.kind = kind,
            }
        }
    }
}

impl Dialog for UnitsDialog {
    fn focused_text(&mut self) -> Option<&mut TextField> {
        match self {
            Self::Add(form) | Self::Edit(_, form) => Some(form.focused_mut()),
            Self::Delete(_, form) => Some(&mut form.confirm_input),
        }
    }

    fn cycle_field(&mut self) {
        if let Self::Add(form) | Self::Edit(_, form) = self {
            form.cycle_field();
        }
    }

    fn is_valid(&self) -> bool {
        match self {
            Self::Add(form) | Self::Edit(_, form) => form.is_valid(),
            Self::Delete(_, form) => form.is_valid(),
        }
    }
}

/// A click handler that applies `change` to the open Dialog.
fn on_change(handlers: &UnitsDialogHandlers, change: Change) -> dialog::OnClick {
    let on_change = handlers.on_change.clone();
    Rc::new(move |window: &mut Window, cx: &mut App| on_change(change, window, cx))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chrome::dialog_host::{DialogKey, DialogOutcome, OpenDialog, handle_key, type_text};

    fn unit_form(dialog: &OpenDialog) -> &UnitForm {
        match dialog {
            OpenDialog::Units(UnitsDialog::Add(form)) => form,
            other => panic!("expected Add unit, got {other:?}"),
        }
    }

    #[test]
    fn typing_and_backspace_edit_the_focused_field_only() {
        let mut dialog = OpenDialog::Units(UnitsDialog::Add(UnitForm::default()));
        type_text(&mut dialog, "aud");
        handle_key(&mut dialog, DialogKey::Backspace);
        assert_eq!(unit_form(&dialog).code.text(), "au");

        handle_key(&mut dialog, DialogKey::Tab);
        assert_eq!(unit_form(&dialog).focused_field, AddUnitField::Name);
        type_text(&mut dialog, "x");
        assert_eq!(unit_form(&dialog).name.text(), "x");
        assert_eq!(unit_form(&dialog).code.text(), "au");
    }

    #[test]
    fn enter_confirms_only_once_the_form_is_valid() {
        let mut dialog = OpenDialog::Units(UnitsDialog::Add(UnitForm::default()));
        assert_eq!(
            handle_key(&mut dialog, DialogKey::Enter),
            DialogOutcome::Handled
        );
        type_text(&mut dialog, "aud");
        handle_key(&mut dialog, DialogKey::Tab);
        type_text(&mut dialog, "Australian dollar");
        assert_eq!(
            handle_key(&mut dialog, DialogKey::Enter),
            DialogOutcome::Confirm
        );
    }

    #[test]
    fn delete_unit_confirms_only_on_the_exact_case_sensitive_code() {
        let mut dialog = OpenDialog::Units(UnitsDialog::Delete(
            UnitId::SEED_BTC,
            DeleteUnitForm::new("btc"),
        ));
        type_text(&mut dialog, "BTC");
        assert_eq!(
            handle_key(&mut dialog, DialogKey::Enter),
            DialogOutcome::Handled
        );
        for _ in 0..3 {
            handle_key(&mut dialog, DialogKey::Backspace);
        }
        type_text(&mut dialog, "btc");
        assert_eq!(
            handle_key(&mut dialog, DialogKey::Enter),
            DialogOutcome::Confirm
        );
    }

    #[test]
    fn a_click_change_reaches_add_and_edit_but_not_delete() {
        let mut dialog = UnitsDialog::Edit(UnitId::SEED_AUD, UnitForm::default());
        dialog.apply_change(Change::Field(AddUnitField::Name));
        dialog.apply_change(Change::Kind(UnitKind::Crypto));
        let UnitsDialog::Edit(_, form) = &dialog else {
            unreachable!()
        };
        assert_eq!(form.focused_field, AddUnitField::Name);
        assert_eq!(form.kind, UnitKind::Crypto);

        let mut delete = UnitsDialog::Delete(UnitId::SEED_BTC, DeleteUnitForm::new("btc"));
        let before = delete.clone();
        delete.apply_change(Change::Kind(UnitKind::Crypto));
        assert_eq!(delete, before);
    }
}
