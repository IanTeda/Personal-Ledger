//! The Dialog host plumbing of `Shell`: opening, closing, confirming and the typed per-domain
//! accessors. An `impl Shell` block, not the Dialog host itself (`chrome::dialog_host`).

use super::Shell;
use crate::navigation::nav::Noun;
use crate::settings::categories::{
    apply::{CategoriesApply, CategoryOutcome},
    dialogs::CategoriesDialog,
    form::CategoryField,
};
use crate::settings::documents::{
    DocumentsPage,
    apply::{DocumentTypeOutcome, DocumentTypesApply},
    dialogs::DocumentTypesDialog,
};
use crate::settings::inventory::{InventoryPage, apply::InventoryApply, dialogs::InventoryDialog};
use crate::settings::payees::PayeesPage;
use crate::settings::tags::{
    TagsPage,
    apply::{TagOutcome, TagsApply},
    dialogs::TagsDialog,
};
use crate::settings::tracing::{
    apply::{TracingApply, TracingOutcome},
    dialogs::TracingDialog,
};
use crate::view::help::HelpDialog;
use crate::{
    chrome::{
        dialog_apply::{self, Apply, DialogForm},
        dialog_host::{Dialog, DialogVariant, OpenDialog},
    },
    navigation::nav::InputMode,
    settings::{
        accounts::{
            AccountsPage,
            apply::{AccountOutcome, AccountsApply},
            dialogs::AccountsDialog,
        },
        institutions::{apply::InstitutionsApply, dialogs::InstitutionsDialog},
        payees::{
            apply::{PayeeOutcome, PayeesApply},
            dialogs::PayeesDialog,
        },
        units::{apply::UnitsApply, dialogs::UnitsDialog},
    },
};
use gpui::Context;
use lib_core::CategoryTypes;
use lib_toast::ToastKind;

impl Shell {
    /// Opens `dialog` in the Dialog host, entering `InputMode::Dialog` with it.
    pub(super) fn open_dialog(&mut self, dialog: OpenDialog) {
        self.chrome.dialog = Some(dialog);
        self.nav.enter_mode(InputMode::Dialog);
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
        // Every domain's Dialog runs through the host's cycle (`DialogForm`/`Apply`, ADR-0037).
        match self.chrome.dialog {
            Some(OpenDialog::Tracing(_)) => self.confirm_tracing_dialog(cx),
            Some(OpenDialog::Units(_)) => {
                let mut apply = UnitsApply {
                    ledger: self.ledger.clone(),
                };
                let toast = self
                    .confirm_dialog::<UnitsDialog, _>(&mut apply, cx)
                    .and_then(|outcome| outcome.toast());
                if let Some((kind, message)) = toast {
                    self.raise_toast(kind, message);
                }
            }
            Some(OpenDialog::Institutions(_)) => {
                let mut apply = InstitutionsApply {
                    ledger: self.ledger.clone(),
                };
                self.confirm_dialog::<InstitutionsDialog, _>(&mut apply, cx);
            }
            Some(OpenDialog::Accounts(_)) => self.confirm_accounts_dialog(cx),
            Some(OpenDialog::Categories(_)) => self.confirm_categories_dialog(cx),
            Some(OpenDialog::Payees(_)) => {
                let mut apply = PayeesApply {
                    ledger: self.ledger.clone(),
                };
                let outcome = self.confirm_dialog::<PayeesDialog, _>(&mut apply, cx);
                if let Some(PayeeOutcome::Saved(id)) = outcome {
                    self.payees_page(cx)
                        .update(cx, |page, cx| page.select_id(id, cx));
                }
                if let Some((kind, message)) = outcome.as_ref().and_then(PayeeOutcome::toast) {
                    self.raise_toast(kind, message);
                }
            }
            Some(OpenDialog::DocumentTypes(_)) => {
                let mut apply = DocumentTypesApply {
                    ledger: self.ledger.clone(),
                };
                let page = self.documents_page(cx);
                match self.confirm_dialog::<DocumentTypesDialog, _>(&mut apply, cx) {
                    Some(DocumentTypeOutcome::Saved(id)) => {
                        page.update(cx, |page, cx| page.select_id(id, cx));
                    }
                    // A removed type's Filed files have moved, and a Documents scope that was
                    // on the type drops back to All.
                    Some(DocumentTypeOutcome::Removed { id, selected }) => {
                        self.views
                            .documents
                            .update(cx, |view, cx| view.type_removed(id, cx));
                        if let Some(selected) = selected {
                            page.update(cx, |page, cx| page.select_id(selected, cx));
                        }
                    }
                    Some(DocumentTypeOutcome::Dismissed) | None => {}
                }
            }
            Some(OpenDialog::Inventory(_)) => {
                let mut apply = InventoryApply {
                    ledger: self.ledger.clone(),
                };
                // The page selects and opens what the Dialog left the cursor on.
                if let Some(outcome) = self.confirm_dialog::<InventoryDialog, _>(&mut apply, cx) {
                    self.inventory_page(cx)
                        .update(cx, |page, cx| page.land(&outcome, cx));
                    if let Some((kind, message)) = outcome.toast() {
                        self.raise_toast(kind, message);
                    }
                }
            }
            Some(OpenDialog::Tags(_)) => {
                let mut apply = TagsApply {
                    ledger: self.ledger.clone(),
                };
                let outcome = self.confirm_dialog::<TagsDialog, _>(&mut apply, cx);
                match outcome {
                    Some(TagOutcome::Saved(id)) => self
                        .tags_page(cx)
                        .update(cx, |page, cx| page.select_id(id, cx)),
                    Some(TagOutcome::Merged { target, .. }) => self
                        .tags_page(cx)
                        .update(cx, |page, cx| page.select_id(target, cx)),
                    // A removal keeps the selection in range: a removed Tag's id no longer
                    // resolves, so the page lands on its first row.
                    Some(TagOutcome::Removed { .. }) | None => {}
                }
                if let Some((kind, message)) = outcome.as_ref().and_then(TagOutcome::toast) {
                    self.raise_toast(kind, message);
                }
            }
            Some(OpenDialog::Bills(_)) => self.confirm_bills_dialog(cx),
            Some(OpenDialog::Budgets(_)) => self.confirm_budgets_dialog(cx),
            Some(OpenDialog::Documents(_)) => self.confirm_documents_dialog(cx),
            // Read-only: it has nothing to apply.
            Some(OpenDialog::ToastHistory(_) | OpenDialog::Help(_)) => self.close_dialog(),
            None => {}
        }
    }

    /// The Payees page's Entity, which the Settings View owns.
    pub(super) fn payees_page(&self, cx: &gpui::App) -> gpui::Entity<PayeesPage> {
        self.views.settings.read(cx).payees().clone()
    }

    /// The Document Types page's Entity, which the Settings View owns.
    pub(super) fn documents_page(&self, cx: &gpui::App) -> gpui::Entity<DocumentsPage> {
        self.views.settings.read(cx).documents().clone()
    }

    /// The Inventory page's Entity, which the Settings View owns.
    pub(super) fn inventory_page(&self, cx: &gpui::App) -> gpui::Entity<InventoryPage> {
        self.views.settings.read(cx).inventory().clone()
    }

    /// The Accounts page's Entity, which the Settings View owns.
    pub(super) fn accounts_page(&self, cx: &gpui::App) -> gpui::Entity<AccountsPage> {
        self.views.settings.read(cx).accounts().clone()
    }

    /// The Tags page's Entity, which the Settings View owns.
    pub(super) fn tags_page(&self, cx: &gpui::App) -> gpui::Entity<TagsPage> {
        self.views.settings.read(cx).tags().clone()
    }

    /// Confirms the open Tracing dialog through the host's cycle; Clear logs empties the capture
    /// and toasts.
    fn confirm_tracing_dialog(&mut self, cx: &mut Context<'_, Self>) {
        let Some(TracingOutcome::LogsConfirmed) =
            self.confirm_dialog::<TracingDialog, _>(&mut TracingApply, cx)
        else {
            return;
        };
        let tracing = self.views.settings.read(cx).tracing().clone();
        tracing.update(cx, |page, cx| page.clear_log(cx));
        self.raise_toast(
            ToastKind::Info,
            crate::msg::desktop_settings_tracing_toast_cleared(),
        );
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

impl Shell {
    /// Confirms the open Accounts dialog through the host's cycle: the form's command runs against
    /// the Ledger, and a landed one closes the dialog. Add and Edit select the account (an Edit
    /// regroups it if its Type changed); Delete toasts the count of Transactions that went with it
    /// and keeps the selection in range, so it lands on the account that slid into the deleted
    /// row's place (or the last one).
    pub(super) fn confirm_accounts_dialog(&mut self, cx: &mut Context<'_, Self>) {
        let mut apply = AccountsApply {
            ledger: self.ledger.clone(),
        };
        let Some(outcome) = self.confirm_dialog::<AccountsDialog, _>(&mut apply, cx) else {
            return;
        };
        let page = self.accounts_page(cx);
        match outcome {
            AccountOutcome::Saved(id) => page.update(cx, |page, cx| page.select_id(id, cx)),
            AccountOutcome::Deleted { name, transactions } => {
                self.raise_toast(
                    ToastKind::Success,
                    lib_locale::msg::toast_account_deleted(
                        &name,
                        i64::try_from(transactions).unwrap_or(i64::MAX),
                    ),
                );
                page.update(cx, |page, cx| page.clamp_selection(cx));
            }
        }
    }
}

impl Shell {
    // -----------------------------------------------------------------------------------------
    // The Categories Dialogs' clicks and confirm. The page opens them, and `Shell` runs the
    // Dialog host's cycle on them, as for every Dialog.
    // -----------------------------------------------------------------------------------------

    /// Raises the error Toast for a Categories write the store or Budgets refused, so the dialog
    /// closes with the reason on screen rather than dropping the write silently.
    fn raise_category_save_failed(&mut self, reason: &str) {
        self.raise_toast(
            ToastKind::Error,
            lib_locale::msg::toast_save_failed(&lib_locale::msg::toast_entity_category(), reason),
        );
    }

    /// Raises the Toast for a refused Categories store write, if it was refused.
    fn report_category_write(&mut self, result: Result<(), lib_categories::CategoryError>) {
        if let Err(error) = result {
            self.raise_category_save_failed(&error.to_string());
        }
    }

    pub(super) fn handle_categories_dialog_field_click(
        &mut self,
        field: CategoryField,
        cx: &mut Context<'_, Self>,
    ) {
        if let Some(dialog) = self.dialog_mut::<CategoriesDialog>()
            && let Some(form) = dialog.form_mut()
        {
            form.focus_field(field);
            cx.notify();
        }
    }

    pub(super) fn handle_categories_dialog_parent_change(
        &mut self,
        parent_id: Option<u32>,
        cx: &mut Context<'_, Self>,
    ) {
        // A child takes its parent's type.
        let parent_type = parent_id.and_then(|parent_id| {
            self.categories(cx)
                .iter()
                .find(|c| c.id == parent_id)
                .map(|parent| parent.category_type.clone())
        });
        if let Some(dialog) = self.dialog_mut::<CategoriesDialog>()
            && let Some(form) = dialog.form_mut()
        {
            form.parent_id = parent_id;
            if parent_type.is_some() {
                form.category_type = parent_type;
            }
            cx.notify();
        }
    }

    pub(super) fn handle_categories_dialog_type_change(
        &mut self,
        category_type: CategoryTypes,
        cx: &mut Context<'_, Self>,
    ) {
        let can_change_type = match self.dialog::<CategoriesDialog>() {
            Some(CategoriesDialog::Add { form, .. }) => form.parent_id.is_none(),
            Some(CategoriesDialog::Edit(id, _)) => self
                .categories(cx)
                .iter()
                .find(|c| c.id == *id)
                .is_some_and(|c| c.parent.is_none()),
            _ => false,
        };
        if !can_change_type {
            return;
        }
        let edited_id = match self.dialog::<CategoriesDialog>() {
            Some(CategoriesDialog::Edit(id, _)) => Some(*id),
            _ => None,
        };
        if let Some(dialog) = self.dialog_mut::<CategoriesDialog>()
            && let Some(form) = dialog.form_mut()
        {
            form.category_type = Some(category_type.clone());
        }
        // For Edit dialogs, cascade the type change to descendants
        if let Some(id) = edited_id {
            let changed = self.ledger.update(cx, |store, cx| {
                store.apply(cx, lib_ledger::ChangeCategoryType { id, category_type })
            });
            self.report_category_write(changed);
        }
        cx.notify();
    }

    /// Confirms the open Categories dialog through the host's cycle: the form's command runs
    /// against the Ledger, a refusal stays on the form, and a landed one closes the dialog. Delete
    /// toasts the Splits it moved.
    pub(super) fn confirm_categories_dialog(&mut self, cx: &mut Context<'_, Self>) {
        let mut apply = CategoriesApply {
            ledger: self.ledger.clone(),
            today: self.today,
        };
        let Some(dialog) = self.dialog_mut::<CategoriesDialog>() else {
            return;
        };
        let Some(outcome) = dialog_apply::confirm(dialog, &mut apply, cx) else {
            return;
        };
        self.close_dialog();
        match outcome {
            CategoryOutcome::Saved => {}
            CategoryOutcome::Deleted { name, moved } => {
                self.raise_toast(
                    ToastKind::Success,
                    lib_locale::msg::toast_category_deleted(&name, moved),
                );
            }
        }
    }
}
