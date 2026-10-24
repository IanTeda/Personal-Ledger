//! The Payees Dialogs: Add, Edit and Delete a payee. [`PayeesDialog`] is the value `Root`'s Dialog
//! host holds while one is open (`OpenDialog::Payees`). It draws itself (`Dialog::render`), every
//! click reaching the form through the host's one edit callback, reads itself back for the UI
//! tests and confirms itself through `apply.rs`.

pub mod add;
pub mod delete;
pub mod edit;
pub mod rules_field;

use std::rc::Rc;

use gpui::{AnyElement, App, Window};
use lib_core::PayeeId;
use lib_payees::PayeeUsage;

use super::form::{DeletePayeeForm, PayeeField, PayeeForm};
use crate::form::field::TextField;
use crate::shell::view::dialog_host::{
    Dialog, DialogActions, DialogKey, DialogOutcome, DialogReadback, DialogScope,
};
pub use add::PayeeDialogHandlers;

/// Which Payees dialog is open, following `AccountsDialog`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PayeesDialog {
    Add(PayeeForm),
    /// Editing the Payee with this [`Payee::id`].
    Edit(PayeeId, PayeeForm),
    /// Deleting (or, when referenced, deactivating) the Payee with this [`Payee::id`].
    Delete(PayeeId, DeletePayeeForm),
}

impl PayeesDialog {
    /// The form behind the Add and Edit dialogs.
    pub fn form_mut(&mut self) -> Option<&mut PayeeForm> {
        match self {
            Self::Add(form) | Self::Edit(_, form) => Some(form),
            Self::Delete(..) => None,
        }
    }
}

impl Dialog for PayeesDialog {
    fn handle_own_key(&mut self, key: DialogKey) -> Option<DialogOutcome> {
        self.form_mut()?.handle_own_key(key)
    }

    fn focused_text(&mut self) -> Option<&mut TextField> {
        match self {
            Self::Add(form) | Self::Edit(_, form) => form.focused_text(),
            Self::Delete(_, form) => Some(&mut form.confirmation_name),
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
            Self::Delete(_, form) => form.is_valid(),
        }
    }

    fn close_open_select(&mut self) -> bool {
        self.form_mut().is_some_and(PayeeForm::close_open_select)
    }

    fn hints(&self) -> Option<Vec<crate::shell::view::statusline::Hint>> {
        Some(crate::shell::view::statusline::owned_hints(match self {
            Self::Delete(..) => crate::settings::hints::confirm_dialog_hints(),
            Self::Add(_) | Self::Edit(..) => crate::settings::hints::payee_dialog_hints(),
        }))
    }

    fn render(&self, scope: &DialogScope, actions: &DialogActions, cx: &App) -> Option<AnyElement> {
        self.draw(scope, actions, cx)
    }

    /// Reads back an Add or Edit form's name; a Delete's typed-back `confirm`.
    fn readback(&self, _scope: &DialogScope, _cx: &App) -> Option<DialogReadback> {
        let (kind, name) = match self {
            Self::Add(form) => ("AddPayee", form.name.text()),
            Self::Edit(_, form) => ("EditPayee", form.name.text()),
            Self::Delete(_, form) => {
                return Some(DialogReadback {
                    kind: "DeletePayee",
                    values: vec![("confirm", form.confirmation_name.text().to_string())],
                });
            }
        };
        Some(DialogReadback {
            kind,
            values: vec![("name", name.to_string())],
        })
    }
}

impl PayeesDialog {
    /// A click on a field of the Add or Edit form: focuses a text field, or focuses the Default
    /// category select and toggles its list.
    pub fn click_field(&mut self, field: PayeeField) {
        if let Some(form) = self.form_mut() {
            if field == PayeeField::DefaultCategory {
                form.click_select();
            } else {
                form.focus(field);
            }
        }
    }

    /// Draws the open Dialog. Edit and Delete read their Payee from the Ledger and count its
    /// Splits there; a Payee that has gone draws nothing rather than panicking mid-render.
    fn draw(&self, scope: &DialogScope, actions: &DialogActions, cx: &App) -> Option<AnyElement> {
        let ledger = scope.ledger.read(cx).ledger();
        let splits = |id: PayeeId| ledger.payee_usage(id).splits;
        let handlers = PayeeDialogHandlers {
            on_field_click: {
                let actions = actions.clone();
                Rc::new(move |field, _window: &mut Window, cx| {
                    actions.edit(cx, |dialog: &mut Self, _| {
                        dialog.click_field(field);
                        None
                    });
                })
            },
            on_option_click: {
                let actions = actions.clone();
                Rc::new(move |index, _window: &mut Window, cx| {
                    actions.edit(cx, |dialog: &mut Self, _| {
                        if let Some(form) = dialog.form_mut() {
                            form.choose_category(index);
                        }
                        None
                    });
                })
            },
            on_add_rule: {
                let actions = actions.clone();
                Rc::new(move |_window: &mut Window, cx: &mut App| {
                    actions.edit(cx, |dialog: &mut Self, _| {
                        if let Some(form) = dialog.form_mut() {
                            form.focus(PayeeField::Rule);
                            form.add_rule();
                        }
                        None
                    });
                })
            },
            on_remove_rule: {
                let actions = actions.clone();
                Rc::new(move |index, _window: &mut Window, cx| {
                    actions.edit(cx, |dialog: &mut Self, _| {
                        if let Some(form) = dialog.form_mut() {
                            form.remove_rule(index);
                        }
                        None
                    });
                })
            },
            on_cancel: actions.cancel.clone(),
            on_confirm: actions.confirm.clone(),
        };
        match self {
            Self::Add(form) => Some(add::render(
                add::PayeeDialogMode::Add,
                form,
                form.options(),
                form.name_error(),
                form.is_valid(),
                handlers,
                cx,
            )),
            Self::Edit(id, form) => {
                let payee = lib_payees::get(ledger.payees(), *id)?;
                Some(edit::render(&payee.name, splits(*id), form, handlers, cx))
            }
            Self::Delete(id, form) => {
                let payee = lib_payees::get(ledger.payees(), *id)?;
                Some(delete::render(
                    delete::DeletePayeeProps {
                        payee,
                        action: form.action(),
                        form,
                        splits: splits(*id),
                        on_cancel: handlers.on_cancel,
                        on_confirm: handlers.on_confirm,
                    },
                    cx,
                ))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::payees::form::{DeleteAction, PayeeField, PayeeOptions};
    use lib_categories::default_categories;
    use lib_payees::default_payees;

    use crate::shell::view::dialog_host::{
        DialogKey, DialogOutcome, OpenDialog, handle_key, type_text,
    };

    fn payee_options() -> PayeeOptions {
        PayeeOptions::new(&default_categories(), "none".into())
    }

    fn add_payee() -> OpenDialog {
        OpenDialog::Payees(PayeesDialog::Add(PayeeForm::new(
            &payee_options(),
            &default_payees(),
        )))
    }

    fn payee_form(dialog: &OpenDialog) -> &PayeeForm {
        match dialog {
            OpenDialog::Payees(PayeesDialog::Add(form) | PayeesDialog::Edit(_, form)) => form,
            other => panic!("expected a payee form, got {other:?}"),
        }
    }

    #[test]
    fn payee_typing_edits_the_focused_text_field_and_tab_moves_on() {
        let mut dialog = add_payee();
        type_text(&mut dialog, "Aussie");
        handle_key(&mut dialog, DialogKey::Backspace);
        assert_eq!(payee_form(&dialog).name.text(), "Aussi");

        handle_key(&mut dialog, DialogKey::Tab);
        assert_eq!(payee_form(&dialog).focused, PayeeField::DefaultCategory);
        handle_key(&mut dialog, DialogKey::Tab);
        type_text(&mut dialog, "aussie");
        assert_eq!(payee_form(&dialog).rule_input.text(), "aussie");
        assert_eq!(payee_form(&dialog).name.text(), "Aussi");
        handle_key(&mut dialog, DialogKey::BackTab);
        assert_eq!(payee_form(&dialog).focused, PayeeField::DefaultCategory);
    }

    #[test]
    fn payee_enter_confirms_only_with_a_free_name() {
        let mut dialog = add_payee();
        assert_eq!(
            handle_key(&mut dialog, DialogKey::Enter),
            DialogOutcome::Handled
        );
        type_text(&mut dialog, "coles");
        assert_eq!(
            handle_key(&mut dialog, DialogKey::Enter),
            DialogOutcome::Handled
        );
        for _ in 0..5 {
            handle_key(&mut dialog, DialogKey::Backspace);
        }
        type_text(&mut dialog, "Aussie Candle Co");
        assert_eq!(
            handle_key(&mut dialog, DialogKey::Enter),
            DialogOutcome::Confirm
        );
    }

    #[test]
    fn payee_enter_in_the_rule_input_adds_a_chip_instead_of_confirming() {
        let mut dialog = add_payee();
        type_text(&mut dialog, "Aussie Candle Co");
        handle_key(&mut dialog, DialogKey::BackTab);
        assert_eq!(payee_form(&dialog).focused, PayeeField::Rule);
        type_text(&mut dialog, "candle");
        assert_eq!(
            handle_key(&mut dialog, DialogKey::Enter),
            DialogOutcome::Handled
        );
        assert_eq!(payee_form(&dialog).rules, vec!["CANDLE".to_string()]);
        // With the input empty again, Enter submits.
        assert_eq!(
            handle_key(&mut dialog, DialogKey::Enter),
            DialogOutcome::Confirm
        );
        // Backspace in the empty input removes the last chip.
        handle_key(&mut dialog, DialogKey::Backspace);
        assert!(payee_form(&dialog).rules.is_empty());
    }

    #[test]
    fn payee_typing_clears_a_refused_rules_error() {
        let mut dialog = add_payee();
        handle_key(&mut dialog, DialogKey::BackTab);
        type_text(&mut dialog, "woolies");
        handle_key(&mut dialog, DialogKey::Enter);
        assert!(payee_form(&dialog).error.is_some());
        type_text(&mut dialog, "!");
        assert_eq!(payee_form(&dialog).error, None);
    }

    #[test]
    fn payee_select_takes_arrows_and_swallows_typing_and_first_esc_closes_it() {
        let mut dialog = add_payee();
        handle_key(&mut dialog, DialogKey::Tab);
        assert_eq!(
            handle_key(&mut dialog, DialogKey::Char('x')),
            DialogOutcome::Handled
        );
        assert_eq!(payee_form(&dialog).name.text(), "");
        handle_key(&mut dialog, DialogKey::Enter);
        assert!(payee_form(&dialog).default_category.is_open());
        handle_key(&mut dialog, DialogKey::Down);
        assert!(dialog.close_open_select());
        assert!(!dialog.close_open_select());
        // The dialog itself is still there for the second Esc to close.
        assert!(matches!(dialog, OpenDialog::Payees(_)));
    }

    #[test]
    fn delete_payee_needs_the_exact_name_except_to_reactivate() {
        let payees = default_payees();
        let j_smith = payees.iter().find(|p| p.name == "J Smith").unwrap();
        let mut dialog = OpenDialog::Payees(PayeesDialog::Delete(
            j_smith.id,
            DeletePayeeForm::new(j_smith, DeleteAction::Delete),
        ));
        type_text(&mut dialog, "j smith");
        assert_eq!(
            handle_key(&mut dialog, DialogKey::Enter),
            DialogOutcome::Handled
        );
        for _ in 0..7 {
            handle_key(&mut dialog, DialogKey::Backspace);
        }
        type_text(&mut dialog, "J Smith");
        assert_eq!(
            handle_key(&mut dialog, DialogKey::Enter),
            DialogOutcome::Confirm
        );

        let mut reactivate = OpenDialog::Payees(PayeesDialog::Delete(
            j_smith.id,
            DeletePayeeForm::new(j_smith, DeleteAction::Reactivate),
        ));
        assert_eq!(
            handle_key(&mut reactivate, DialogKey::Enter),
            DialogOutcome::Confirm
        );
    }
}
