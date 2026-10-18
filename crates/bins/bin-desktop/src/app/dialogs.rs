//! The Dialog host plumbing of `Shell`: opening, closing and confirming, the Bills, Budgets and
//! Documents Dialogs it still confirms by variant, and the page accessors the palette commands
//! and snapshots use. An `impl Shell` block, not the Dialog host itself (`chrome::dialog_host`).

use super::Shell;
use crate::navigation::nav::Noun;
use crate::settings::inventory::InventoryPage;
use crate::view::help::HelpDialog;
use crate::{
    chrome::{
        dialog_apply::{self, Apply, DialogForm},
        dialog_host::{Dialog, DialogReadback, DialogScope, DialogVariant, HostedOpen, OpenDialog},
    },
    navigation::nav::InputMode,
    settings::accounts::AccountsPage,
};
use gpui::Context;

impl Shell {
    /// Opens `dialog` in the Dialog host, entering `InputMode::Dialog` with it.
    pub(super) fn open_dialog(&mut self, dialog: OpenDialog) {
        self.chrome.dialog = Some(dialog);
        self.chrome.dialog_confirm = None;
        self.nav.enter_mode(InputMode::Dialog);
    }

    /// Opens a hosted Dialog, keeping the confirm that hands its outcome back to its page.
    pub(super) fn open_hosted_dialog(&mut self, open: HostedOpen) {
        self.open_dialog(open.dialog);
        self.chrome.dialog_confirm = Some(open.confirm);
    }

    /// What a hosted Dialog reads and writes through.
    pub(super) fn dialog_scope(&self) -> DialogScope {
        DialogScope {
            ledger: self.ledger.clone(),
            today: self.today,
        }
    }

    /// The open hosted Dialog's state as labelled values, for a test; `None` when no Dialog or
    /// one not yet hosted is open.
    #[doc(hidden)]
    pub fn dialog_readback(&self, cx: &gpui::App) -> Option<DialogReadback> {
        self.chrome
            .dialog
            .as_ref()?
            .readback(&self.dialog_scope(), cx)
    }

    /// Opens the Help Dialog over the active page, carrying the shortcuts that page adds to the
    /// global ones.
    pub(super) fn open_help(&mut self, cx: &mut Context<'_, Self>) {
        let mut shortcuts = if self.nav.noun() == Noun::Settings {
            self.views.settings.read(cx).cheat_sheet(cx)
        } else {
            Vec::new()
        };
        shortcuts.extend(self.documents_cheat_sheet(cx));
        self.open_dialog(OpenDialog::Help(HelpDialog { shortcuts }));
    }

    /// Whether the Help Dialog is the open Dialog.
    #[doc(hidden)]
    pub fn help_open(&self) -> bool {
        matches!(self.chrome.dialog, Some(OpenDialog::Help(_)))
    }

    /// Closes the Dialog host's Dialog, if one is open, without applying it.
    pub(super) fn close_dialog(&mut self) {
        self.chrome.dialog_confirm = None;
        if self.chrome.dialog.take().is_some() {
            self.nav.exit_mode();
        }
    }

    /// `Enter` and the confirm button both land here: applies the open Dialog if its form is
    /// valid, then closes it. A no-op while the form is invalid, leaving the Dialog open.
    pub(super) fn confirm_open_dialog(&mut self, cx: &mut Context<'_, Self>) {
        if !self.chrome.dialog.as_ref().is_some_and(Dialog::is_valid) {
            return;
        }
        // A hosted Dialog confirms itself and has already handed its outcome to its page.
        if let Some(confirm) = self.chrome.dialog_confirm.clone() {
            let scope = self.dialog_scope();
            let Some(dialog) = self.chrome.dialog.as_mut() else {
                return;
            };
            if let Some(landed) = confirm(dialog, &scope, cx) {
                self.close_dialog();
                if let Some((kind, message)) = landed.toast {
                    self.raise_toast(kind, message);
                }
            }
            return;
        }
        // Every domain's Dialog runs through the host's cycle (`DialogForm`/`Apply`, ADR-0037).
        match self.chrome.dialog {
            Some(OpenDialog::Bills(_)) => self.confirm_bills_dialog(cx),
            Some(OpenDialog::Budgets(_)) => self.confirm_budgets_dialog(cx),
            Some(OpenDialog::Documents(_)) => self.confirm_documents_dialog(cx),
            // Read-only: it has nothing to apply.
            Some(OpenDialog::ToastHistory(_) | OpenDialog::Help(_)) => self.close_dialog(),
            // Hosted: confirmed above through the opener's confirm.
            Some(
                OpenDialog::Tracing(_)
                | OpenDialog::Tags(_)
                | OpenDialog::Categories(_)
                | OpenDialog::Units(_)
                | OpenDialog::Institutions(_)
                | OpenDialog::Accounts(_)
                | OpenDialog::Payees(_)
                | OpenDialog::DocumentTypes(_)
                | OpenDialog::Inventory(_),
            )
            | None => {}
        }
    }

    /// The Inventory page's Entity, which the Settings View owns.
    pub(super) fn inventory_page(&self, cx: &gpui::App) -> gpui::Entity<InventoryPage> {
        self.views.settings.read(cx).inventory().clone()
    }

    /// The Accounts page's Entity, which the Settings View owns.
    pub(super) fn accounts_page(&self, cx: &gpui::App) -> gpui::Entity<AccountsPage> {
        self.views.settings.read(cx).accounts().clone()
    }

    /// Confirms the open `T` Dialog through the host's cycle (ADR-0037): the form's command runs
    /// through `apply`, and a landed one closes the Dialog. `None` while the form is invalid or
    /// the command was refused, leaving the Dialog open.
    fn confirm_dialog<T, A>(&mut self, apply: &mut A, cx: &mut gpui::App) -> Option<A::Outcome>
    where
        T: DialogVariant + DialogForm,
        A: Apply<T::Command, Refusal = T::Refusal>,
    {
        let outcome = dialog_apply::confirm(self.dialog_mut::<T>()?, apply, cx)?;
        self.close_dialog();
        Some(outcome)
    }

    /// The cancel button: closes the Dialog without applying it.
    pub(super) fn handle_dialog_cancel(&mut self, cx: &mut Context<'_, Self>) {
        self.close_dialog();
        cx.notify();
    }

    /// The confirm button, the click twin of `Enter`.
    pub(super) fn handle_dialog_confirm(&mut self, cx: &mut Context<'_, Self>) {
        self.confirm_open_dialog(cx);
        cx.notify();
    }

    /// A click inside the open `T` Dialog: runs `change` on it and repaints. A no-op when another
    /// feature's Dialog is open.
    pub(super) fn edit_dialog<T: DialogVariant>(
        &mut self,
        cx: &mut Context<'_, Self>,
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
