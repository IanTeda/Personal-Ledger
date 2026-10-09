//! The Dialog host plumbing of `Shell`: opening, closing, confirming and the typed per-domain
//! accessors. An `impl Shell` block, not the Dialog host itself (`chrome::dialog_host`).

use super::Shell;
use crate::{
    accounts::form::AccountsDialog,
    bills, budgets, categories,
    chrome::dialog_host::{Dialog, OpenDialog},
    documents::types::DocumentTypesDialog,
    inventory::form::InventoryDialog,
    navigation::nav::InputMode,
    payees,
    settings::SettingsDialog,
    tags,
};

impl Shell {
    /// Opens `dialog` in the Dialog host, entering `InputMode::Dialog` with it.
    pub(super) fn open_dialog(&mut self, dialog: OpenDialog) {
        self.chrome.dialog = Some(dialog);
        self.nav.enter_mode(InputMode::Dialog);
    }

    /// Closes the Dialog host's Dialog, if one is open, without applying it.
    pub(super) fn close_dialog(&mut self) {
        if self.chrome.dialog.take().is_some() {
            self.nav.exit_mode();
        }
    }

    /// `Enter` and the confirm button both land here: applies the open Dialog if its form is
    /// valid, then closes it. A no-op while the form is invalid, leaving the Dialog open.
    pub(super) fn confirm_open_dialog(&mut self) {
        if !self.chrome.dialog.as_ref().is_some_and(Dialog::is_valid) {
            return;
        }
        let Some(dialog) = self.chrome.dialog.take() else {
            return;
        };
        self.nav.exit_mode();
        match dialog {
            OpenDialog::Settings(dialog) => self.apply_settings_dialog(dialog),
            OpenDialog::Accounts(dialog) => self.apply_accounts_dialog(dialog),
            OpenDialog::Categories(dialog) => self.apply_categories_dialog(dialog),
            OpenDialog::Payees(dialog) => self.apply_payees_dialog(dialog),
            OpenDialog::DocumentTypes(dialog) => self.apply_document_types_dialog(dialog),
            OpenDialog::Inventory(dialog) => self.apply_inventory_dialog(dialog),
            OpenDialog::Tags(dialog) => self.apply_tags_dialog(dialog),
            OpenDialog::Bills(dialog) => self.apply_bills_dialog(*dialog),
            OpenDialog::Budgets(dialog) => self.apply_budgets_dialog(*dialog),
            OpenDialog::Documents(dialog) => self.apply_documents_dialog(dialog),
            // Read-only: it has nothing to apply.
            OpenDialog::ToastHistory(_) => {}
        }
    }

    pub(super) fn settings_dialog(&self) -> Option<&SettingsDialog> {
        match self.chrome.dialog.as_ref()? {
            OpenDialog::Settings(dialog) => Some(dialog),
            _ => None,
        }
    }

    pub(super) fn settings_dialog_mut(&mut self) -> Option<&mut SettingsDialog> {
        match self.chrome.dialog.as_mut()? {
            OpenDialog::Settings(dialog) => Some(dialog),
            _ => None,
        }
    }

    pub(super) fn categories_dialog(&self) -> Option<&categories::form::CategoriesDialog> {
        match self.chrome.dialog.as_ref()? {
            OpenDialog::Categories(dialog) => Some(dialog),
            _ => None,
        }
    }

    pub(super) fn categories_dialog_mut(
        &mut self,
    ) -> Option<&mut categories::form::CategoriesDialog> {
        match self.chrome.dialog.as_mut()? {
            OpenDialog::Categories(dialog) => Some(dialog),
            _ => None,
        }
    }

    pub(super) fn document_types_dialog(&self) -> Option<&DocumentTypesDialog> {
        match self.chrome.dialog.as_ref()? {
            OpenDialog::DocumentTypes(dialog) => Some(dialog),
            _ => None,
        }
    }

    pub(super) fn inventory_dialog(&self) -> Option<&InventoryDialog> {
        match self.chrome.dialog.as_ref()? {
            OpenDialog::Inventory(dialog) => Some(dialog),
            _ => None,
        }
    }

    pub(super) fn inventory_dialog_mut(&mut self) -> Option<&mut InventoryDialog> {
        match self.chrome.dialog.as_mut()? {
            OpenDialog::Inventory(dialog) => Some(dialog),
            _ => None,
        }
    }

    pub(super) fn document_types_dialog_mut(&mut self) -> Option<&mut DocumentTypesDialog> {
        match self.chrome.dialog.as_mut()? {
            OpenDialog::DocumentTypes(dialog) => Some(dialog),
            _ => None,
        }
    }

    pub(super) fn bills_dialog(&self) -> Option<&bills::BillsDialog> {
        match self.chrome.dialog.as_ref()? {
            OpenDialog::Bills(dialog) => Some(dialog),
            _ => None,
        }
    }

    pub(super) fn bills_dialog_mut(&mut self) -> Option<&mut bills::BillsDialog> {
        match self.chrome.dialog.as_mut()? {
            OpenDialog::Bills(dialog) => Some(dialog),
            _ => None,
        }
    }

    pub(super) fn budgets_dialog(&self) -> Option<&budgets::BudgetsDialog> {
        match self.chrome.dialog.as_ref()? {
            OpenDialog::Budgets(dialog) => Some(dialog),
            _ => None,
        }
    }

    pub(super) fn budgets_dialog_mut(&mut self) -> Option<&mut budgets::BudgetsDialog> {
        match self.chrome.dialog.as_mut()? {
            OpenDialog::Budgets(dialog) => Some(dialog),
            _ => None,
        }
    }

    pub(super) fn tags_dialog(&self) -> Option<&tags::form::TagsDialog> {
        match self.chrome.dialog.as_ref()? {
            OpenDialog::Tags(dialog) => Some(dialog),
            _ => None,
        }
    }

    pub(super) fn tags_dialog_mut(&mut self) -> Option<&mut tags::form::TagsDialog> {
        match self.chrome.dialog.as_mut()? {
            OpenDialog::Tags(dialog) => Some(dialog),
            _ => None,
        }
    }

    pub(super) fn payees_dialog(&self) -> Option<&payees::form::PayeesDialog> {
        match self.chrome.dialog.as_ref()? {
            OpenDialog::Payees(dialog) => Some(dialog),
            _ => None,
        }
    }

    pub(super) fn payees_dialog_mut(&mut self) -> Option<&mut payees::form::PayeesDialog> {
        match self.chrome.dialog.as_mut()? {
            OpenDialog::Payees(dialog) => Some(dialog),
            _ => None,
        }
    }

    pub(super) fn accounts_dialog(&self) -> Option<&AccountsDialog> {
        match self.chrome.dialog.as_ref()? {
            OpenDialog::Accounts(dialog) => Some(dialog),
            _ => None,
        }
    }

    pub(super) fn accounts_dialog_mut(&mut self) -> Option<&mut AccountsDialog> {
        match self.chrome.dialog.as_mut()? {
            OpenDialog::Accounts(dialog) => Some(dialog),
            _ => None,
        }
    }
}
