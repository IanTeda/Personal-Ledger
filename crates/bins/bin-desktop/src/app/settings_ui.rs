//! The Settings wiring `Shell` still owns: opening a page from a command or a hand-off, the
//! Settings dialogs' clicks and confirm, and the `?` cheat-sheet's Settings group. The index, the
//! focus rules and each page are the `SettingsView` Entity's and the page Entities' own
//! (ADR-0037).

use super::Shell;
use crate::chrome::dialog_apply;
use crate::navigation::command::AccountsVerb;
use crate::navigation::nav::Noun;
use crate::settings::accounts::{AccountsPage, hints::accounts_hints};
use crate::settings::apply::{SettingsApply, SettingsOutcome};
use crate::settings::categories::{
    apply::{CategoriesApply, CategoryOutcome},
    dialogs::CategoriesDialog,
    form::CategoryField,
};
use crate::settings::hints::{
    settings_categories_hints, settings_colour_grid_hints, settings_display_hints,
    settings_documents_hints, settings_index_hints, settings_payees_hints,
    settings_plain_page_hints, settings_tags_hints, settings_tracing_hints,
};
use crate::settings::{SettingsDialog, SettingsPage};
use gpui::{App, Context, Entity};
use lib_core::CategoryTypes;
use lib_toast::ToastKind;

impl Shell {
    /// Opens a Settings page with focus in it -- what `:settings <page>`, the old `:accounts` /
    /// `:categories` / `:payees` / `:tags` aliases and every hand-off to a moved noun land on.
    /// Leaves the page's scroll alone when Settings is already showing.
    pub(super) fn open_settings_page(&mut self, section: SettingsPage, cx: &mut App) {
        let noun_before = self.nav.noun();
        self.nav.set_noun(Noun::Settings);
        if noun_before != Noun::Settings {
            self.reset_view_scroll(cx);
        }
        self.views
            .settings
            .update(cx, |view, cx| view.open_page(section, cx));
    }

    /// Moves Settings' focus into the open page, as a click on one of its rows does.
    pub(super) fn focus_settings_page(&mut self, cx: &mut App) {
        self.views
            .settings
            .update(cx, |view, cx| view.focus_page_from_click(cx));
    }

    /// The `?` cheat-sheet's Settings group as `(action, keys)`: the index keys, then the open
    /// page's. Empty off the Settings noun, so the group is left out.
    pub(super) fn settings_cheat_sheet(&self, cx: &App) -> Vec<(String, &'static str)> {
        if self.nav.noun() != Noun::Settings {
            return Vec::new();
        }
        let page = match self.views.settings.read(cx).selected_section() {
            SettingsPage::Accounts => accounts_hints(),
            SettingsPage::Categories => settings_categories_hints(),
            SettingsPage::Tags => settings_tags_hints(),
            SettingsPage::Documents => settings_documents_hints(),
            SettingsPage::Payees => settings_payees_hints(),
            SettingsPage::Inventory => self.views.settings.read(cx).inventory().read(cx).hints(cx),
            SettingsPage::Display => {
                let mut keys = settings_display_hints();
                keys.extend(settings_colour_grid_hints().into_iter().take(2));
                keys
            }
            SettingsPage::Tracing => settings_tracing_hints(),
            _ => settings_plain_page_hints(),
        };
        settings_index_hints()
            .into_iter()
            .chain(page)
            .map(|(keys, action)| (action, keys))
            .collect()
    }

    /// Confirms the open Settings dialog through the host's cycle: the form's command runs
    /// through the Ledger, and a landed one closes the dialog. Clear logs toasts.
    pub(super) fn confirm_settings_dialog(&mut self, cx: &mut Context<'_, Self>) {
        let mut apply = SettingsApply;
        let Some(dialog) = self.dialog_mut::<SettingsDialog>() else {
            return;
        };
        let Some(outcome) = dialog_apply::confirm(dialog, &mut apply, cx) else {
            return;
        };
        self.close_dialog();
        match outcome {
            SettingsOutcome::LogsConfirmed => {
                self.views
                    .settings
                    .update(cx, |view, cx| view.clear_log(cx));
                self.raise_toast(
                    ToastKind::Info,
                    crate::msg::desktop_settings_tracing_toast_cleared(),
                );
            }
        }
    }
}

impl Shell {
    /// The Accounts page's Entity, which the Settings View owns.
    pub(super) fn accounts_page(&self, cx: &App) -> Entity<AccountsPage> {
        self.views.settings.read(cx).accounts().clone()
    }

    /// `accounts new|edit|delete [<account name>]`: jumps to the Accounts page, whose Entity then
    /// opens the same dialog its `n`/`e`/`d` and buttons do.
    pub(super) fn run_accounts_command(
        &mut self,
        command_name: &str,
        verb: AccountsVerb,
        argument: &str,
        cx: &mut Context<'_, Self>,
    ) {
        self.open_settings_page(SettingsPage::Accounts, cx);
        self.accounts_page(cx).update(cx, |page, cx| {
            page.run_command(command_name, verb, argument, cx);
        });
    }

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
