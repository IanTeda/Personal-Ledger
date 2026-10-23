//! The Dialog host plumbing of `Shell`: opening, closing and confirming, the Bills, Budgets and
//! Documents Dialogs it still confirms by variant, and the page accessors the palette commands
//! and snapshots use. An `impl Shell` block, not the Dialog host itself (`chrome::dialog_host`).

use super::Shell;
use crate::navigation::nav::Noun;
use crate::settings::inventory::InventoryPage;
use crate::view::help::HelpDialog;
use crate::{
    chrome::dialog_host::{
        Dialog, DialogReadback, DialogScope, DialogVariant, HostedOpen, OpenDialog,
    },
    navigation::nav::InputMode,
    settings::accounts::AccountsPage,
};
use gpui::Context;
use lib_shell::DialogInput;

use crate::chrome::dialog_entity::DialogHandle;

impl Shell {
    /// Opens `dialog` in the legacy Dialog host, entering `InputMode::Dialog` with it.
    pub(super) fn open_dialog(&mut self, dialog: OpenDialog) {
        self.close_dialog();
        self.chrome.dialog = Some(dialog);
        self.nav.enter_mode(InputMode::Dialog);
        self.presenter.dialog(DialogInput::Opened);
    }

    /// Opens a hosted Dialog, keeping the confirm that hands its outcome back to its page.
    pub(super) fn open_hosted_dialog(&mut self, open: HostedOpen) {
        self.open_dialog(open.dialog);
        self.chrome.dialog_confirm = Some(open.confirm);
    }

    /// Opens a module's Dialog Entity, closing the host once its Presenter closes it.
    pub(super) fn host_dialog(&mut self, dialog: DialogHandle, cx: &mut Context<'_, Self>) {
        self.close_dialog();
        let shell = cx.weak_entity();
        let id = dialog.id();
        let closed = dialog.on_closed(
            move |cx| {
                // A Shell gone with its window has no Dialog left to close.
                let _ = shell.update(cx, |shell, cx| {
                    if shell.hosted_dialog().is_some_and(|open| open.id() == id) {
                        shell.close_dialog();
                        cx.notify();
                    }
                });
            },
            cx,
        );
        self.chrome.hosted = Some((dialog, closed));
        self.nav.enter_mode(InputMode::Dialog);
        self.presenter.dialog(DialogInput::Opened);
    }

    /// The open module Dialog, if one is.
    pub(crate) fn hosted_dialog(&self) -> Option<&DialogHandle> {
        self.chrome.hosted.as_ref().map(|(dialog, _)| dialog)
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
        if let Some(dialog) = self.hosted_dialog() {
            return Some(dialog.readback(cx));
        }
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

    /// Closes the open Dialog, in either host, without applying it.
    pub(super) fn close_dialog(&mut self) {
        self.chrome.dialog_confirm = None;
        let legacy = self.chrome.dialog.take().is_some();
        if legacy || self.chrome.hosted.take().is_some() {
            self.nav.exit_mode();
            self.presenter.dialog(DialogInput::Closed);
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
        // Every domain's Dialog runs through the host's cycle (`DialogForm`/`confirm_ledger`, ADR-0037).
        match self.chrome.dialog {
            Some(OpenDialog::Budgets(_)) => self.confirm_budgets_dialog(cx),
            Some(OpenDialog::Documents(_)) => self.confirm_documents_dialog(cx),
            // Read-only: it has nothing to apply.
            Some(OpenDialog::ToastHistory(_) | OpenDialog::Help(_)) => self.close_dialog(),
            // Hosted: confirmed above through the opener's confirm.
            Some(
                OpenDialog::Tracing(_)
                | OpenDialog::Tags(_)
                | OpenDialog::Categories(_)
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

    /// The cancel button: closes the Dialog without applying it, as one input.
    pub(super) fn handle_dialog_cancel(&mut self, cx: &mut Context<'_, Self>) {
        crate::input::send(cx, |_| self.close_dialog());
    }

    /// The confirm button, the click twin of `Enter`: one input, whose commit saves what the
    /// Dialog and the page it hands its outcome to wrote.
    pub(super) fn handle_dialog_confirm(&mut self, cx: &mut Context<'_, Self>) {
        crate::input::send(cx, |cx| self.confirm_open_dialog(cx));
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
