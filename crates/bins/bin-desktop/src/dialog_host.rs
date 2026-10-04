//! The Dialog host: the one slot a Dialog lives in while it is open, and the keyboard handling
//! every Dialog shares. `gpui`-free, so a Dialog's keys are unit-tested without a window.
//!
//! `Shell` holds `Option<OpenDialog>` and opens and closes it only through `Shell::open_dialog`
//! and `Shell::close_dialog`, which also enter and leave `InputMode::Dialog`, so the slot and
//! the mode can't disagree. A key reaches the open Dialog through [`handle_key`]: the Dialog's
//! own [`Dialog::handle_own_key`] sees it first (a list Dialog's `j`/`k`, a search box), then
//! the shared handling types into the focused [`TextField`], cycles fields on `Tab` and confirms
//! on `Enter` only while the form is valid. A Dialog never touches ledger data: it answers
//! [`DialogOutcome::Confirm`] and `Shell` applies the form (`Shell::confirm_open_dialog`), the
//! same path the confirm button's click takes.
//!
//! `Esc` is not a key here: the router turns it into `ClosePopupsAndExitMode`, and `Shell` asks
//! [`Dialog::close_open_select`] first, closing the Dialog only when no select was open.
//!
//! Drawing stays in `crate::dialog` (the chrome) and each feature's view; `Shell::render` matches
//! on the [`OpenDialog`] variant to draw it.
//!
//! Dialogs move into this slot one feature at a time; until they all have, the rest still live in
//! their own `Shell::*_dialog` fields.

use crate::{
    accounts::AccountsDialog, bills::BillsDialog, categories::CategoriesDialog,
    document_types::DocumentTypesDialog, field::TextField, inventory_form::InventoryDialog,
    payees::PayeesDialog, settings::SettingsDialog, tags::TagsDialog,
};

/// A keystroke as a Dialog sees it, already stripped of modifiers by `Shell`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DialogKey {
    Backspace,
    Tab,
    /// `Shift-Tab`; a Dialog without its own handling treats it as `Tab`.
    BackTab,
    Up,
    Down,
    Left,
    Right,
    Enter,
    /// A printable character, typed without Ctrl/Alt/Cmd/Fn.
    Char(char),
    /// Anything else (arrows, function keys, a modified chord).
    Other,
}

/// What `Shell` does after a Dialog has seen a key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DialogOutcome {
    /// Not the Dialog's key; `Shell` lets it fall through.
    Ignored,
    Handled,
    /// The form is valid and the user confirmed: `Shell` applies it and closes the Dialog.
    Confirm,
}

/// One feature's Dialogs. Every method has the default a plain confirm (no fields) wants.
pub trait Dialog {
    /// Keys this Dialog takes before the shared handling, `None` to leave the key to it.
    fn handle_own_key(&mut self, _key: DialogKey) -> Option<DialogOutcome> {
        None
    }

    /// The text field that typing and `Backspace` edit.
    fn focused_text(&mut self) -> Option<&mut TextField> {
        None
    }

    /// `Tab`: the next field in this Dialog's own focus ring. Never reaches the shell-wide zones.
    fn cycle_field(&mut self) {}

    /// Whether `Enter` (and the confirm button) may confirm.
    fn is_valid(&self) -> bool {
        true
    }

    /// The first `Esc` closes an open select only. `true` when one was open.
    fn close_open_select(&mut self) -> bool {
        false
    }
}

/// The open Dialog, one variant per feature.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OpenDialog {
    Settings(SettingsDialog),
    Accounts(AccountsDialog),
    Categories(CategoriesDialog),
    Payees(PayeesDialog),
    DocumentTypes(DocumentTypesDialog),
    Inventory(InventoryDialog),
    Tags(TagsDialog),
    /// Boxed: the plan form carries its select choices, which would swell every variant.
    Bills(Box<BillsDialog>),
}

impl OpenDialog {
    fn inner(&self) -> &dyn Dialog {
        match self {
            Self::Settings(dialog) => dialog,
            Self::Accounts(dialog) => dialog,
            Self::Categories(dialog) => dialog,
            Self::Payees(dialog) => dialog,
            Self::DocumentTypes(dialog) => dialog,
            Self::Inventory(dialog) => dialog,
            Self::Tags(dialog) => dialog,
            Self::Bills(dialog) => &**dialog,
        }
    }

    fn inner_mut(&mut self) -> &mut dyn Dialog {
        match self {
            Self::Settings(dialog) => dialog,
            Self::Accounts(dialog) => dialog,
            Self::Categories(dialog) => dialog,
            Self::Payees(dialog) => dialog,
            Self::DocumentTypes(dialog) => dialog,
            Self::Inventory(dialog) => dialog,
            Self::Tags(dialog) => dialog,
            Self::Bills(dialog) => &mut **dialog,
        }
    }
}

impl Dialog for OpenDialog {
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
}

/// Routes `key` to `dialog`: its own keys first, then typing, `Tab` and `Enter`.
pub fn handle_key(dialog: &mut impl Dialog, key: DialogKey) -> DialogOutcome {
    if let Some(outcome) = dialog.handle_own_key(key) {
        return outcome;
    }
    match key {
        DialogKey::Backspace => match dialog.focused_text() {
            Some(field) => {
                field.backspace();
                DialogOutcome::Handled
            }
            None => DialogOutcome::Ignored,
        },
        // Swallowed even with one field, so `Tab` never moves the shell's focus zones behind
        // the scrim.
        DialogKey::Tab | DialogKey::BackTab => {
            dialog.cycle_field();
            DialogOutcome::Handled
        }
        DialogKey::Enter if dialog.is_valid() => DialogOutcome::Confirm,
        DialogKey::Enter => DialogOutcome::Handled,
        DialogKey::Char(ch) => match dialog.focused_text() {
            Some(field) => {
                field.push(ch);
                DialogOutcome::Handled
            }
            None => DialogOutcome::Ignored,
        },
        DialogKey::Up | DialogKey::Down | DialogKey::Left | DialogKey::Right | DialogKey::Other => {
            DialogOutcome::Ignored
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::accounts::{AccountField, AccountForm, DeleteAccountForm};
    use crate::bill_form::BillPlanField;
    use crate::bills::AmountKind;
    use crate::categories::{BudgetLock, CategoryField, CategoryForm, DeleteCategoryForm};
    use crate::document_types::{
        DocumentTypeForm, DocumentTypesDialog, FormField, RemoveForm, TracksDate, default_types,
    };
    use crate::inventory_form::{
        InventoryDialog, PropertyField, PropertyForm, RemovePropertyForm, RemoveRoomForm,
    };
    use crate::payees::{DeleteAction, DeletePayeeForm, PayeeField, PayeeForm, default_payees};
    use crate::settings::{
        AccountType, AddInstitutionForm, AddUnitField, DeleteUnitForm, UnitForm, default_units,
    };

    fn type_text(dialog: &mut OpenDialog, text: &str) {
        for ch in text.chars() {
            assert_eq!(
                handle_key(dialog, DialogKey::Char(ch)),
                DialogOutcome::Handled
            );
        }
    }

    fn unit_form(dialog: &OpenDialog) -> &UnitForm {
        match dialog {
            OpenDialog::Settings(SettingsDialog::AddUnit(form)) => form,
            other => panic!("expected Add unit, got {other:?}"),
        }
    }

    #[test]
    fn typing_and_backspace_edit_the_focused_field_only() {
        let mut dialog = OpenDialog::Settings(SettingsDialog::AddUnit(UnitForm::default()));
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
        let mut dialog = OpenDialog::Settings(SettingsDialog::AddUnit(UnitForm::default()));
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
        let mut dialog =
            OpenDialog::Settings(SettingsDialog::DeleteUnit(0, DeleteUnitForm::new("btc")));
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
    fn add_institution_needs_a_name_an_account_type_and_a_unit() {
        let mut dialog = OpenDialog::Settings(SettingsDialog::AddInstitution(
            AddInstitutionForm::new(&default_units()),
        ));
        assert!(!dialog.is_valid());
        type_text(&mut dialog, "ANZ");
        assert!(dialog.is_valid());
        if let OpenDialog::Settings(SettingsDialog::AddInstitution(form)) = &mut dialog {
            form.toggle_account_type(AccountType::Savings);
        }
        assert!(!dialog.is_valid());
    }

    #[test]
    fn a_dialog_without_text_fields_ignores_typing_but_swallows_tab() {
        let mut dialog = OpenDialog::Settings(SettingsDialog::ClearLogs);
        assert_eq!(
            handle_key(&mut dialog, DialogKey::Char('x')),
            DialogOutcome::Ignored
        );
        assert_eq!(
            handle_key(&mut dialog, DialogKey::Backspace),
            DialogOutcome::Ignored
        );
        assert_eq!(
            handle_key(&mut dialog, DialogKey::Tab),
            DialogOutcome::Handled
        );
        assert_eq!(
            handle_key(&mut dialog, DialogKey::Enter),
            DialogOutcome::Confirm
        );
    }

    #[test]
    fn other_keys_fall_through() {
        let mut dialog = OpenDialog::Settings(SettingsDialog::AddUnit(UnitForm::default()));
        assert_eq!(
            handle_key(&mut dialog, DialogKey::Other),
            DialogOutcome::Ignored
        );
    }

    fn account_options() -> crate::accounts::AccountOptions {
        crate::accounts::AccountOptions::new(
            vec!["ANZ".to_string(), "CBA".to_string()],
            vec!["aud".to_string(), "btc".to_string()],
        )
    }

    fn add_account() -> OpenDialog {
        OpenDialog::Accounts(AccountsDialog::Add(AccountForm::new(
            &account_options(),
            Some("aud"),
        )))
    }

    fn account_form(dialog: &OpenDialog) -> &AccountForm {
        match dialog {
            OpenDialog::Accounts(AccountsDialog::Add(form) | AccountsDialog::Edit(_, form)) => form,
            other => panic!("expected an account form, got {other:?}"),
        }
    }

    #[test]
    fn account_typing_edits_the_focused_text_field_and_tab_moves_on() {
        let mut dialog = add_account();
        type_text(&mut dialog, "Everyday");
        handle_key(&mut dialog, DialogKey::Backspace);
        assert_eq!(account_form(&dialog).name.text(), "Everyda");

        // Name -> Institution (a select): typed characters are swallowed, not typed.
        handle_key(&mut dialog, DialogKey::Tab);
        assert_eq!(account_form(&dialog).focused, AccountField::Institution);
        type_text(&mut dialog, "x");
        assert_eq!(account_form(&dialog).name.text(), "Everyda");
        assert_eq!(account_form(&dialog).account_number.text(), "");

        handle_key(&mut dialog, DialogKey::BackTab);
        assert_eq!(account_form(&dialog).focused, AccountField::Name);
    }

    #[test]
    fn account_opening_balance_only_takes_a_decimal_amount() {
        let mut dialog = add_account();
        if let OpenDialog::Accounts(AccountsDialog::Add(form)) = &mut dialog {
            form.focus(AccountField::OpeningBalance);
        }
        type_text(&mut dialog, "-12a3.4.5e-");
        assert_eq!(account_form(&dialog).opening_balance.text(), "-123.45");
        handle_key(&mut dialog, DialogKey::Backspace);
        assert_eq!(account_form(&dialog).opening_balance.text(), "-123.4");
    }

    #[test]
    fn account_enter_confirms_only_with_a_name_and_activates_a_focused_select() {
        let mut dialog = add_account();
        assert_eq!(
            handle_key(&mut dialog, DialogKey::Enter),
            DialogOutcome::Handled
        );
        type_text(&mut dialog, "Everyday");
        assert_eq!(
            handle_key(&mut dialog, DialogKey::Enter),
            DialogOutcome::Confirm
        );

        // On a select, Enter opens its list instead of confirming.
        handle_key(&mut dialog, DialogKey::Tab);
        assert_eq!(
            handle_key(&mut dialog, DialogKey::Enter),
            DialogOutcome::Handled
        );
        assert!(account_form(&dialog).institution.is_open());
    }

    #[test]
    fn account_select_keys_step_and_the_first_esc_closes_only_the_open_list() {
        let mut dialog = add_account();
        handle_key(&mut dialog, DialogKey::Tab);
        assert_eq!(account_form(&dialog).institution.value(), Some("ANZ"));
        handle_key(&mut dialog, DialogKey::Down);
        assert_eq!(account_form(&dialog).institution.value(), Some("CBA"));

        handle_key(&mut dialog, DialogKey::Char(' '));
        assert!(account_form(&dialog).institution.is_open());
        assert!(dialog.close_open_select(), "first Esc closes the list");
        assert!(!dialog.close_open_select(), "second Esc closes the Dialog");
    }

    #[test]
    fn delete_account_confirms_only_on_the_exact_case_sensitive_name() {
        let mut dialog = OpenDialog::Accounts(AccountsDialog::Delete(
            1,
            DeleteAccountForm::new("Amex Platinum"),
        ));
        type_text(&mut dialog, "amex platinum");
        assert_eq!(
            handle_key(&mut dialog, DialogKey::Enter),
            DialogOutcome::Handled
        );
        for _ in 0.."amex platinum".len() {
            handle_key(&mut dialog, DialogKey::Backspace);
        }
        type_text(&mut dialog, "Amex Platinum");
        assert_eq!(
            handle_key(&mut dialog, DialogKey::Enter),
            DialogOutcome::Confirm
        );
        type_text(&mut dialog, " ");
        assert!(!dialog.is_valid(), "no trimming");
        assert_eq!(
            handle_key(&mut dialog, DialogKey::Tab),
            DialogOutcome::Handled
        );
    }

    fn add_category() -> OpenDialog {
        OpenDialog::Categories(CategoriesDialog::Add {
            parent_id: None,
            form: CategoryForm::default(),
        })
    }

    fn category_form(dialog: &OpenDialog) -> &CategoryForm {
        match dialog {
            OpenDialog::Categories(dialog) => dialog.form().expect("an Add or Edit dialog"),
            other => panic!("expected a Categories form, got {other:?}"),
        }
    }

    #[test]
    fn category_typing_edits_the_name_and_tab_cycles_through_the_fields() {
        let mut dialog = add_category();
        type_text(&mut dialog, "Pets");
        handle_key(&mut dialog, DialogKey::Backspace);
        assert_eq!(category_form(&dialog).name.text(), "Pet");

        // Parent and Type are click-driven selects: typing there is swallowed, not leaked.
        handle_key(&mut dialog, DialogKey::Tab);
        assert_eq!(category_form(&dialog).focused, CategoryField::Parent);
        type_text(&mut dialog, "x");
        assert_eq!(
            handle_key(&mut dialog, DialogKey::Backspace),
            DialogOutcome::Handled
        );
        assert_eq!(category_form(&dialog).name.text(), "Pet");

        handle_key(&mut dialog, DialogKey::Tab);
        handle_key(&mut dialog, DialogKey::Tab);
        assert_eq!(category_form(&dialog).focused, CategoryField::Budget);
        handle_key(&mut dialog, DialogKey::Tab);
        assert_eq!(category_form(&dialog).focused, CategoryField::Name);
    }

    #[test]
    fn category_budget_takes_only_numbers_and_nothing_while_locked() {
        let mut dialog = add_category();
        for _ in 0..3 {
            handle_key(&mut dialog, DialogKey::Tab);
        }
        type_text(&mut dialog, "1a2.5");
        assert_eq!(category_form(&dialog).budget.text(), "12.5");

        if let OpenDialog::Categories(CategoriesDialog::Add { form, .. }) = &mut dialog {
            form.budget_lock = Some(BudgetLock::Parent);
        }
        type_text(&mut dialog, "9");
        assert_eq!(category_form(&dialog).budget.text(), "12.5");
    }

    #[test]
    fn category_enter_confirms_only_with_a_name() {
        let mut dialog = add_category();
        assert_eq!(
            handle_key(&mut dialog, DialogKey::Enter),
            DialogOutcome::Handled
        );
        type_text(&mut dialog, "Pets");
        assert_eq!(
            handle_key(&mut dialog, DialogKey::Enter),
            DialogOutcome::Confirm
        );
    }

    #[test]
    fn category_dialogs_have_no_select_for_the_first_esc_to_close() {
        let mut dialog = add_category();
        assert!(!dialog.close_open_select(), "first Esc closes the Dialog");
    }

    #[test]
    fn delete_category_confirms_only_on_the_exact_name() {
        let mut dialog = OpenDialog::Categories(CategoriesDialog::Delete(
            4,
            DeleteCategoryForm::new("Groceries"),
        ));
        type_text(&mut dialog, "groceries");
        assert_eq!(
            handle_key(&mut dialog, DialogKey::Enter),
            DialogOutcome::Handled
        );
        for _ in 0.."groceries".len() {
            handle_key(&mut dialog, DialogKey::Backspace);
        }
        type_text(&mut dialog, "Groceries");
        assert_eq!(
            handle_key(&mut dialog, DialogKey::Enter),
            DialogOutcome::Confirm
        );
        assert_eq!(
            handle_key(&mut dialog, DialogKey::Tab),
            DialogOutcome::Handled,
            "Tab is swallowed, the confirmation being the only field"
        );
    }

    fn payee_options() -> crate::payees::PayeeOptions {
        crate::payees::PayeeOptions::new(&crate::categories::default_categories(), "none".into())
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

    fn add_document_type() -> OpenDialog {
        OpenDialog::DocumentTypes(DocumentTypesDialog::Add(DocumentTypeForm::new(
            &default_types(),
        )))
    }

    fn document_type_form(dialog: &OpenDialog) -> &DocumentTypeForm {
        match dialog {
            OpenDialog::DocumentTypes(
                DocumentTypesDialog::Add(form) | DocumentTypesDialog::Edit(_, form),
            ) => form,
            other => panic!("expected a document type form, got {other:?}"),
        }
    }

    #[test]
    fn document_type_typing_edits_the_name_and_tab_skips_a_disabled_remind() {
        let mut dialog = add_document_type();
        type_text(&mut dialog, "Leasex");
        handle_key(&mut dialog, DialogKey::Backspace);
        assert_eq!(document_type_form(&dialog).name.text(), "Lease");
        assert!(document_type_form(&dialog).touched);

        handle_key(&mut dialog, DialogKey::Tab);
        assert_eq!(document_type_form(&dialog).focused, FormField::TracksDate);
        // Typing on a control is swallowed, not appended to the name.
        assert_eq!(
            handle_key(&mut dialog, DialogKey::Char('x')),
            DialogOutcome::Handled
        );
        assert_eq!(document_type_form(&dialog).name.text(), "Lease");
        handle_key(&mut dialog, DialogKey::Tab);
        assert_eq!(
            document_type_form(&dialog).focused,
            FormField::FinancialYear
        );
        handle_key(&mut dialog, DialogKey::Char(' '));
        assert!(document_type_form(&dialog).financial_year);
        handle_key(&mut dialog, DialogKey::BackTab);
        assert_eq!(document_type_form(&dialog).focused, FormField::TracksDate);
        handle_key(&mut dialog, DialogKey::Right);
        assert_eq!(
            document_type_form(&dialog).tracks_date,
            Some(TracksDate::Renews)
        );
    }

    #[test]
    fn document_type_enter_confirms_only_with_a_free_name() {
        let mut dialog = add_document_type();
        assert_eq!(
            handle_key(&mut dialog, DialogKey::Enter),
            DialogOutcome::Handled
        );
        type_text(&mut dialog, "tax");
        assert_eq!(
            handle_key(&mut dialog, DialogKey::Enter),
            DialogOutcome::Handled
        );
        type_text(&mut dialog, "es");
        assert_eq!(
            handle_key(&mut dialog, DialogKey::Enter),
            DialogOutcome::Confirm
        );
    }

    #[test]
    fn remove_document_type_select_walks_commits_and_first_esc_closes_it() {
        let mut dialog = OpenDialog::DocumentTypes(DocumentTypesDialog::Remove(
            3,
            RemoveForm::new(&default_types(), 3),
        ));
        let destination = |dialog: &OpenDialog| match dialog {
            OpenDialog::DocumentTypes(DocumentTypesDialog::Remove(_, form)) => {
                form.select.value().map(str::to_string)
            }
            other => panic!("expected a remove dialog, got {other:?}"),
        };
        assert_eq!(destination(&dialog).as_deref(), Some("Other"));
        // Space opens the list; Esc closes it first and keeps the value and the dialog.
        handle_key(&mut dialog, DialogKey::Char(' '));
        handle_key(&mut dialog, DialogKey::Up);
        assert!(dialog.close_open_select());
        assert!(!dialog.close_open_select());
        assert_eq!(destination(&dialog).as_deref(), Some("Other"));
        // Enter on an open list commits it rather than confirming; on a closed one it confirms.
        handle_key(&mut dialog, DialogKey::Char(' '));
        handle_key(&mut dialog, DialogKey::Up);
        assert_eq!(
            handle_key(&mut dialog, DialogKey::Enter),
            DialogOutcome::Handled
        );
        assert_eq!(destination(&dialog).as_deref(), Some("Bills"));
        assert_eq!(
            handle_key(&mut dialog, DialogKey::Enter),
            DialogOutcome::Confirm
        );
    }

    #[test]
    fn the_default_document_type_notice_confirms_with_enter_and_swallows_tab() {
        let mut dialog = OpenDialog::DocumentTypes(DocumentTypesDialog::DefaultNotice);
        assert_eq!(
            handle_key(&mut dialog, DialogKey::Tab),
            DialogOutcome::Handled
        );
        assert_eq!(
            handle_key(&mut dialog, DialogKey::Enter),
            DialogOutcome::Confirm
        );
    }

    fn property_context() -> crate::inventory_form::PropertyContext {
        crate::inventory_form::PropertyContext {
            inventory: crate::inventory::Inventory::default(),
            today: chrono::NaiveDate::from_ymd_opt(2026, 10, 2).expect("a valid date"),
            date_style: None,
            unit_choices: vec![
                ("AUD".to_string(), "aud".to_string()),
                ("NZD".to_string(), "nzd".to_string()),
            ],
        }
    }

    fn add_property() -> OpenDialog {
        OpenDialog::Inventory(InventoryDialog::Add(PropertyForm::for_add(
            Some("AUD".to_string()),
            property_context(),
        )))
    }

    fn property_form(dialog: &OpenDialog) -> &PropertyForm {
        match dialog {
            OpenDialog::Inventory(InventoryDialog::Add(form) | InventoryDialog::Edit(_, form)) => {
                form
            }
            other => panic!("expected a property form, got {other:?}"),
        }
    }

    #[test]
    fn property_typing_tab_and_enter_only_when_valid() {
        let mut dialog = add_property();
        assert_eq!(
            handle_key(&mut dialog, DialogKey::Enter),
            DialogOutcome::Handled
        );
        type_text(&mut dialog, "Beach housex");
        handle_key(&mut dialog, DialogKey::Backspace);
        assert_eq!(property_form(&dialog).name.text(), "Beach house");
        handle_key(&mut dialog, DialogKey::Tab);
        assert_eq!(property_form(&dialog).focused, PropertyField::Address);
        type_text(&mut dialog, "1 Beach Rd");
        assert_eq!(property_form(&dialog).address.text(), "1 Beach Rd");
        handle_key(&mut dialog, DialogKey::BackTab);
        assert_eq!(property_form(&dialog).focused, PropertyField::Name);
        assert_eq!(
            handle_key(&mut dialog, DialogKey::Enter),
            DialogOutcome::Confirm
        );
    }

    #[test]
    fn property_unit_select_first_esc_closes_it_and_keeps_the_dialog() {
        let mut dialog = add_property();
        handle_key(&mut dialog, DialogKey::Tab);
        handle_key(&mut dialog, DialogKey::Tab);
        assert_eq!(property_form(&dialog).focused, PropertyField::Unit);
        handle_key(&mut dialog, DialogKey::Char(' '));
        assert!(property_form(&dialog).unit.is_open());
        assert!(dialog.close_open_select());
        assert!(!dialog.close_open_select());
        assert_eq!(property_form(&dialog).unit.value(), Some("AUD"));
    }

    #[test]
    fn remove_property_with_holdings_confirms_only_on_the_typed_name() {
        let mut dialog = OpenDialog::Inventory(InventoryDialog::Remove(
            1,
            RemovePropertyForm::new("Elm St", true),
        ));
        type_text(&mut dialog, "Elm");
        assert_eq!(
            handle_key(&mut dialog, DialogKey::Enter),
            DialogOutcome::Handled
        );
        type_text(&mut dialog, " St");
        assert_eq!(
            handle_key(&mut dialog, DialogKey::Enter),
            DialogOutcome::Confirm
        );
    }

    #[test]
    fn remove_room_select_first_esc_closes_it_and_enter_needs_a_destination() {
        let mut dialog = OpenDialog::Inventory(InventoryDialog::RemoveRoom(
            1,
            RemoveRoomForm::new(vec!["Kitchen".to_string()], None, true),
        ));
        assert_eq!(
            handle_key(&mut dialog, DialogKey::Enter),
            DialogOutcome::Handled
        );
        handle_key(&mut dialog, DialogKey::Char(' '));
        assert!(dialog.close_open_select());
        assert!(!dialog.close_open_select());
        handle_key(&mut dialog, DialogKey::Down);
        assert_eq!(
            handle_key(&mut dialog, DialogKey::Enter),
            DialogOutcome::Confirm
        );
    }

    #[test]
    fn the_room_blocked_notice_confirms_with_enter_and_swallows_tab() {
        let mut dialog = OpenDialog::Inventory(InventoryDialog::RoomBlocked(1));
        assert_eq!(
            handle_key(&mut dialog, DialogKey::Tab),
            DialogOutcome::Handled
        );
        assert_eq!(
            handle_key(&mut dialog, DialogKey::Enter),
            DialogOutcome::Confirm
        );
    }

    fn tag_options() -> Vec<crate::tags::MergeOption> {
        ["a", "b", "c"]
            .iter()
            .zip(1..)
            .map(|(label, id)| crate::tags::MergeOption {
                id,
                label: (*label).to_string(),
            })
            .collect()
    }

    fn add_tag() -> OpenDialog {
        OpenDialog::Tags(TagsDialog::Add(crate::tags::TagForm::new(
            crate::tags::default_tags(),
        )))
    }

    fn tag_form(dialog: &OpenDialog) -> &crate::tags::TagForm {
        match dialog {
            OpenDialog::Tags(TagsDialog::Add(form) | TagsDialog::Edit(_, form)) => form,
            other => panic!("expected a tag form, got {other:?}"),
        }
    }

    #[test]
    fn tag_typing_tab_and_enter_only_when_valid() {
        let mut dialog = add_tag();
        assert_eq!(
            handle_key(&mut dialog, DialogKey::Enter),
            DialogOutcome::Handled
        );
        type_text(&mut dialog, "campingx");
        handle_key(&mut dialog, DialogKey::Backspace);
        assert_eq!(tag_form(&dialog).name.text(), "camping");
        // The swatch row takes no text; the hex box stops at seven characters.
        handle_key(&mut dialog, DialogKey::Tab);
        assert_eq!(
            handle_key(&mut dialog, DialogKey::Char('x')),
            DialogOutcome::Handled
        );
        assert_eq!(tag_form(&dialog).name.text(), "camping");
        handle_key(&mut dialog, DialogKey::Tab);
        type_text(&mut dialog, "#12abef99");
        assert_eq!(tag_form(&dialog).hex.text(), "#12abef");
        handle_key(&mut dialog, DialogKey::BackTab);
        assert_eq!(tag_form(&dialog).focused, crate::tags::TagField::Swatches);
        assert_eq!(
            handle_key(&mut dialog, DialogKey::Enter),
            DialogOutcome::Confirm
        );
        handle_key(&mut dialog, DialogKey::Tab);
        handle_key(&mut dialog, DialogKey::Backspace);
        assert_eq!(
            handle_key(&mut dialog, DialogKey::Enter),
            DialogOutcome::Handled
        );
    }

    #[test]
    fn remove_tag_with_transactions_confirms_only_on_the_typed_name() {
        let mut dialog = OpenDialog::Tags(TagsDialog::Remove(
            1,
            crate::tags::RemoveTagForm::new("travel", 3),
        ));
        type_text(&mut dialog, "trav");
        assert_eq!(
            handle_key(&mut dialog, DialogKey::Enter),
            DialogOutcome::Handled
        );
        type_text(&mut dialog, "el");
        assert_eq!(
            handle_key(&mut dialog, DialogKey::Enter),
            DialogOutcome::Confirm
        );
    }

    #[test]
    fn merge_tags_first_esc_closes_the_select_and_enter_needs_a_pair() {
        let mut dialog = OpenDialog::Tags(TagsDialog::Merge(crate::tags::MergeTagsForm::new(
            tag_options(),
            Some(1),
            None,
        )));
        assert_eq!(
            handle_key(&mut dialog, DialogKey::Enter),
            DialogOutcome::Handled
        );
        handle_key(&mut dialog, DialogKey::Char(' '));
        assert!(dialog.close_open_select());
        assert!(!dialog.close_open_select());
        handle_key(&mut dialog, DialogKey::Down);
        assert_eq!(
            handle_key(&mut dialog, DialogKey::Enter),
            DialogOutcome::Confirm
        );
    }

    fn bill_plan_form() -> crate::bill_form::BillPlanForm {
        use crate::{accounts::default_accounts, bill_form, categories::default_categories};
        let today = chrono::NaiveDate::from_ymd_opt(2026, 9, 19).unwrap();
        let source = bill_form::BillPlanSource::new(
            &default_categories(),
            &default_accounts(),
            &default_payees(),
            None,
            "none".to_string(),
            |recurrence| format!("{recurrence:?}"),
            today,
            None,
        );
        bill_form::BillPlanForm::new(source)
    }

    fn bill_plan_form_of(dialog: &OpenDialog) -> &crate::bill_form::BillPlanForm {
        match dialog {
            OpenDialog::Bills(inner) => match &**inner {
                BillsDialog::Add(form) => form,
                other => panic!("expected Add bill plan, got {other:?}"),
            },
            other => panic!("expected Bills, got {other:?}"),
        }
    }

    #[test]
    fn bill_plan_typing_tab_and_the_amount_filter() {
        let mut dialog = OpenDialog::Bills(Box::new(BillsDialog::Add(bill_plan_form())));
        type_text(&mut dialog, "Water");
        handle_key(&mut dialog, DialogKey::Backspace);
        assert_eq!(bill_plan_form_of(&dialog).name.text(), "Wate");

        // Name, then the Category, Unit, Account, Payee selects, to Amount.
        for _ in 0..5 {
            handle_key(&mut dialog, DialogKey::Tab);
        }
        assert_eq!(bill_plan_form_of(&dialog).focused, BillPlanField::Amount);
        type_text(&mut dialog, "1a2.3.4-");
        assert_eq!(bill_plan_form_of(&dialog).amount.text(), "12.34");
        handle_key(&mut dialog, DialogKey::BackTab);
        assert_eq!(bill_plan_form_of(&dialog).focused, BillPlanField::Payee);
        type_text(&mut dialog, "x");
        assert_eq!(
            bill_plan_form_of(&dialog).name.text(),
            "Wate",
            "selects take no text"
        );
    }

    #[test]
    fn bill_plan_enter_confirms_only_once_valid_and_a_select_takes_enter_first() {
        let mut dialog = OpenDialog::Bills(Box::new(BillsDialog::Add(bill_plan_form())));
        assert_eq!(
            handle_key(&mut dialog, DialogKey::Enter),
            DialogOutcome::Handled
        );
        type_text(&mut dialog, "Water");
        for _ in 0..5 {
            handle_key(&mut dialog, DialogKey::Tab);
        }
        type_text(&mut dialog, "90");
        assert_eq!(
            handle_key(&mut dialog, DialogKey::Enter),
            DialogOutcome::Confirm
        );

        // On a select `Enter` opens its list instead, and a first Esc closes it.
        handle_key(&mut dialog, DialogKey::BackTab);
        handle_key(&mut dialog, DialogKey::BackTab);
        assert_eq!(
            handle_key(&mut dialog, DialogKey::Enter),
            DialogOutcome::Handled
        );
        assert!(dialog.close_open_select());
        assert!(!dialog.close_open_select());
    }

    #[test]
    fn bill_plan_space_toggles_fixed_estimated_and_a_unit_change_keeps_the_account_in_it() {
        let mut dialog = OpenDialog::Bills(Box::new(BillsDialog::Add(bill_plan_form())));
        for _ in 0..6 {
            handle_key(&mut dialog, DialogKey::Tab);
        }
        assert_eq!(
            bill_plan_form_of(&dialog).focused,
            BillPlanField::AmountKind
        );
        handle_key(&mut dialog, DialogKey::Char(' '));
        assert_eq!(
            bill_plan_form_of(&dialog).amount_kind,
            AmountKind::Estimated
        );
        handle_key(&mut dialog, DialogKey::Left);
        assert_eq!(bill_plan_form_of(&dialog).amount_kind, AmountKind::Fixed);

        let OpenDialog::Bills(inner) = &mut dialog else {
            panic!("expected Bills");
        };
        let BillsDialog::Add(form) = &mut **inner else {
            panic!("expected Add bill plan");
        };
        form.focus(BillPlanField::Unit);
        let before = form.account.value().map(str::to_string);
        if form.options.units.len() > 1 {
            handle_key(form, DialogKey::Down);
            assert_ne!(form.account.value().map(str::to_string), before);
            assert!(
                form.options
                    .account_labels
                    .iter()
                    .any(|a| Some(a.as_str()) == form.account.value())
            );
        }
    }

    fn pay_form(candidates: usize) -> crate::pay_form::PayForm {
        use crate::{
            accounts::default_accounts,
            bills::{EntryId, SplitRef, default_bills},
            categories::default_categories,
            tags::default_tags,
            transactions::default_transactions,
        };
        let today = chrono::NaiveDate::from_ymd_opt(2026, 9, 19).unwrap();
        let (accounts, categories, payees) =
            (default_accounts(), default_categories(), default_payees());
        let mut transactions =
            default_transactions(&accounts, &categories, &payees, &default_tags(), today);
        let plan = default_bills(&accounts, &categories, &payees, &mut transactions, today)
            .plans
            .into_iter()
            .next()
            .unwrap();
        let entry = EntryId {
            plan_id: plan.id,
            due: today,
        };
        let candidates = (0..candidates as u32)
            .map(|transaction_id| SplitRef {
                transaction_id,
                split_index: 0,
            })
            .collect();
        crate::pay_form::PayForm::new(entry, &plan, candidates, None, today, None)
    }

    fn pay_form_of(dialog: &OpenDialog) -> &crate::pay_form::PayForm {
        match dialog {
            OpenDialog::Bills(inner) => match &**inner {
                BillsDialog::Pay(form) => form,
                other => panic!("expected Pay, got {other:?}"),
            },
            other => panic!("expected Bills, got {other:?}"),
        }
    }

    #[test]
    fn pay_directly_types_into_amount_and_date_and_enter_needs_both_valid() {
        let mut dialog = OpenDialog::Bills(Box::new(BillsDialog::Pay(pay_form(0))));
        assert_eq!(
            handle_key(&mut dialog, DialogKey::Enter),
            DialogOutcome::Confirm,
            "prefilled from the plan, dated today"
        );
        for _ in 0..pay_form_of(&dialog).amount.text().len() {
            handle_key(&mut dialog, DialogKey::Backspace);
        }
        assert_eq!(
            handle_key(&mut dialog, DialogKey::Enter),
            DialogOutcome::Handled,
            "an empty amount refuses"
        );
        type_text(&mut dialog, "1x2.5.");
        assert_eq!(pay_form_of(&dialog).amount.text(), "12.5");
        handle_key(&mut dialog, DialogKey::Tab);
        assert_eq!(
            pay_form_of(&dialog).focused,
            crate::pay_form::PayField::Date
        );
        type_text(&mut dialog, "!");
        assert!(pay_form_of(&dialog).date_error().is_some());
        assert_eq!(
            handle_key(&mut dialog, DialogKey::Enter),
            DialogOutcome::Handled
        );
    }

    #[test]
    fn pay_match_steps_rows_and_none_of_these_switches_panel_instead_of_confirming() {
        use crate::pay_form::{MatchChoice, PayMode};
        let mut dialog = OpenDialog::Bills(Box::new(BillsDialog::Pay(pay_form(1))));
        assert_eq!(pay_form_of(&dialog).mode, PayMode::Match);
        assert_eq!(
            handle_key(&mut dialog, DialogKey::Enter),
            DialogOutcome::Handled,
            "nothing chosen yet"
        );
        handle_key(&mut dialog, DialogKey::Char('j'));
        assert_eq!(
            handle_key(&mut dialog, DialogKey::Enter),
            DialogOutcome::Confirm
        );
        handle_key(&mut dialog, DialogKey::Down);
        assert_eq!(pay_form_of(&dialog).choice, Some(MatchChoice::NoneOfThese));
        assert_eq!(
            handle_key(&mut dialog, DialogKey::Enter),
            DialogOutcome::Handled
        );
        assert_eq!(pay_form_of(&dialog).mode, PayMode::Direct);
        handle_key(&mut dialog, DialogKey::Right);
        assert_eq!(pay_form_of(&dialog).mode, PayMode::Match);
        handle_key(&mut dialog, DialogKey::Char('k'));
        assert_eq!(pay_form_of(&dialog).choice, Some(MatchChoice::Candidate(0)));
    }

    #[test]
    fn skip_confirms_on_enter_and_swallows_tab() {
        let entry = crate::bills::EntryId {
            plan_id: 1,
            due: chrono::NaiveDate::from_ymd_opt(2026, 9, 19).unwrap(),
        };
        let mut dialog = OpenDialog::Bills(Box::new(BillsDialog::Skip(entry)));
        assert_eq!(
            handle_key(&mut dialog, DialogKey::Tab),
            DialogOutcome::Handled
        );
        assert_eq!(
            handle_key(&mut dialog, DialogKey::Enter),
            DialogOutcome::Confirm
        );
        assert!(!dialog.close_open_select());
    }
}
