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
        // Payees, Tags, Accounts, Categories, Inventory and Document types run through the host's cycle (`DialogForm`/`Apply`); the other domains
        // still take their Dialog and apply it themselves until #633 lands.
        match self.chrome.dialog {
            Some(OpenDialog::Payees(_)) => return self.confirm_payees_dialog(cx),
            Some(OpenDialog::Tags(_)) => return self.confirm_tags_dialog(cx),
            Some(OpenDialog::Accounts(_)) => return self.confirm_accounts_dialog(cx),
            Some(OpenDialog::Categories(_)) => return self.confirm_categories_dialog(cx),
            Some(OpenDialog::Inventory(_)) => return self.confirm_inventory_dialog(cx),
            Some(OpenDialog::DocumentTypes(_)) => return self.confirm_document_types_dialog(cx),
            _ => {}
        }
        let Some(dialog) = self.chrome.dialog.take() else {
            return;
        };
        self.nav.exit_mode();
        match dialog {
            OpenDialog::Settings(dialog) => self.apply_settings_dialog(dialog, cx),
            OpenDialog::Accounts(_) => unreachable!("Accounts confirms through the host's cycle"),
            OpenDialog::Categories(_) => {
                unreachable!("Categories confirms through the host's cycle")
            }
            OpenDialog::Payees(_) => unreachable!("Payees confirms through the host's cycle"),
            OpenDialog::DocumentTypes(_) => {
                unreachable!("Document types confirm through the host's cycle")
            }
            OpenDialog::Inventory(_) => unreachable!("Inventory confirms through the host's cycle"),
            OpenDialog::Tags(_) => unreachable!("Tags confirms through the host's cycle"),
            OpenDialog::Bills(dialog) => self.apply_bills_dialog(*dialog, cx),
            OpenDialog::Budgets(dialog) => self.apply_budgets_dialog(*dialog, cx),
            OpenDialog::Documents(dialog) => self.apply_documents_dialog(dialog, cx),
            // Read-only: it has nothing to apply.
            OpenDialog::ToastHistory(_) => {}
        }
    }

    /// The cancel button: closes the Dialog without applying it.
    pub(super) fn handle_dialog_cancel(&mut self, cx: &mut gpui::Context<'_, Self>) {
        self.close_dialog();
        cx.notify();
    }

    /// The confirm button, the click twin of `Enter`.
    pub(super) fn handle_dialog_confirm(&mut self, cx: &mut gpui::Context<'_, Self>) {
        self.confirm_open_dialog(cx);
        cx.notify();
    }

    /// A click inside the open `T` Dialog: runs `change` on it and repaints. A no-op when another
    /// feature's Dialog is open.
    pub(super) fn edit_dialog<T: DialogVariant>(
        &mut self,
        cx: &mut gpui::Context<'_, Self>,
        change: impl FnOnce(&mut T),
    ) {
        if let Some(dialog) = self.dialog_mut::<T>() {
            change(dialog);
        }
        cx.notify();
    }

    /// The open Dialog as a `T`, `None` when no Dialog or another feature's is open.
    pub(super) fn dialog<T: DialogVariant>(&self) -> Option<&T> {
        T::from_open(self.chrome.dialog.as_ref()?)
    }

    pub(super) fn dialog_mut<T: DialogVariant>(&mut self) -> Option<&mut T> {
        T::from_open_mut(self.chrome.dialog.as_mut()?)
    }
}
