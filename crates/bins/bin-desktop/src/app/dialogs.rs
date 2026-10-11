//! The Dialog host plumbing of `Shell`: opening, closing, confirming and the typed per-domain
//! accessors. An `impl Shell` block, not the Dialog host itself (`chrome::dialog_host`).

use super::Shell;
use crate::{
    chrome::dialog_host::{Dialog, DialogVariant, OpenDialog},
    navigation::nav::InputMode,
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
    pub(super) fn confirm_open_dialog(&mut self, cx: &mut gpui::Context<'_, Self>) {
        if !self.chrome.dialog.as_ref().is_some_and(Dialog::is_valid) {
            return;
        }
        let Some(dialog) = self.chrome.dialog.take() else {
            return;
        };
        self.nav.exit_mode();
        match dialog {
            OpenDialog::Settings(dialog) => self.apply_settings_dialog(dialog, cx),
            OpenDialog::Accounts(dialog) => self.apply_accounts_dialog(dialog, cx),
            OpenDialog::Categories(dialog) => self.apply_categories_dialog(dialog, cx),
            OpenDialog::Payees(dialog) => self.apply_payees_dialog(dialog, cx),
            OpenDialog::DocumentTypes(dialog) => self.apply_document_types_dialog(dialog, cx),
            OpenDialog::Inventory(dialog) => self.apply_inventory_dialog(dialog, cx),
            OpenDialog::Tags(dialog) => self.apply_tags_dialog(dialog, cx),
            OpenDialog::Bills(dialog) => self.apply_bills_dialog(*dialog, cx),
            OpenDialog::Budgets(dialog) => self.apply_budgets_dialog(*dialog, cx),
            OpenDialog::Documents(dialog) => self.apply_documents_dialog(dialog, cx),
            // Read-only: it has nothing to apply.
            OpenDialog::ToastHistory(_) => {}
        }
    }

    /// The open Dialog as a `T`, `None` when no Dialog or another feature's is open.
    pub(super) fn dialog<T: DialogVariant>(&self) -> Option<&T> {
        T::from_open(self.chrome.dialog.as_ref()?)
    }

    pub(super) fn dialog_mut<T: DialogVariant>(&mut self) -> Option<&mut T> {
        T::from_open_mut(self.chrome.dialog.as_mut()?)
    }
}
