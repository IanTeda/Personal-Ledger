//! The Settings wiring `Shell` still owns: opening a page from a command or a hand-off, the
//! Documents and Inventory pages, the Settings dialogs' clicks and confirm, and the `?`
//! cheat-sheet's Settings group. The index, the focus rules and the pages without Ledger data
//! are the `SettingsView` Entity's own (ADR-0037); the pages with Ledger data move into it one
//! ticket at a time.

use super::Shell;
use crate::chrome::dialog_apply;
use crate::navigation::command::AccountsVerb;
use crate::navigation::key_router::Movement;
use crate::navigation::nav::{FocusZone, Noun};
use crate::settings::accounts::{AccountsPage, hints::accounts_hints};
use crate::settings::apply::{SettingsApply, SettingsOutcome};
use crate::settings::categories::{
    apply::{CategoriesApply, CategoryOutcome},
    dialogs::CategoriesDialog,
    form::CategoryField,
};
use crate::settings::hints::{
    settings_categories_hints, settings_colour_grid_hints, settings_display_hints,
    settings_documents_hints, settings_index_hints, settings_inventory_hints,
    settings_payees_hints, settings_plain_page_hints, settings_tags_hints, settings_tracing_hints,
};
use crate::settings::inventory::{self as inventory_view, InventoryRow};
use crate::settings::{SettingsDialog, SettingsFocus, SettingsPage};
use gpui::{App, Context, Entity, Keystroke};
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

    /// Whether a Settings page with Ledger data, which `Shell` still draws, owns the keyboard:
    /// the page, not the index, has focus in the View zone.
    pub(super) fn settings_data_page_has_focus(&self, section: SettingsPage, cx: &App) -> bool {
        self.nav.noun() == Noun::Settings
            && self.nav.focus() == FocusZone::View
            && self.views.settings.read(cx).focus() == SettingsFocus::Page
            && self.views.settings.read(cx).selected_section() == section
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
            SettingsPage::Payees => settings_payees_hints(),
            SettingsPage::Documents => settings_documents_hints(),
            SettingsPage::Inventory => {
                settings_inventory_hints(self.settings_inventory_selected_row(cx))
            }
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

    /// The Documents page's selected type: the stored id while it still exists, else the first
    /// row, so the cursor is never lost.
    pub(super) fn settings_documents_selected_id(&self, cx: &App) -> Option<u32> {
        lib_list_cursor::resolve(
            self.document_types(cx).iter().map(|row| row.id),
            self.views.settings.read(cx).documents_selected(),
        )
    }

    /// `j`/`k`/`g`/`G`/`Ctrl-d`/`Ctrl-u` walk the types in the user's order.
    pub(super) fn apply_settings_documents_movement(&mut self, movement: Movement, cx: &mut App) {
        let Some(step) = movement.step() else {
            return;
        };
        let current = self.settings_documents_selected_id(cx);
        let ids: Vec<u32> = self.document_types(cx).iter().map(|row| row.id).collect();
        let next = lib_list_cursor::step_id(&ids, current, step);
        self.views
            .settings
            .update(cx, |v, cx| v.set_documents_selected(cx, next));
    }

    /// `J`/`K` on the Documents page move the selected type a place, which is also its place in
    /// the Documents Type filter.
    pub(super) fn handle_settings_documents_reorder_key(
        &mut self,
        keystroke: &Keystroke,
        cx: &mut App,
    ) -> bool {
        if !self.settings_data_page_has_focus(SettingsPage::Documents, cx) {
            return false;
        }
        let modifiers = &keystroke.modifiers;
        if modifiers.control || modifiers.alt || modifiers.platform || !modifiers.shift {
            return false;
        }
        match keystroke.key.as_str() {
            "j" => self.move_selected_document_type(1, cx),
            "k" => self.move_selected_document_type(-1, cx),
            _ => return false,
        }
        true
    }

    /// The Documents page's own `n`/`e`/`x`, which open the Add, Edit and Remove dialogs.
    pub(super) fn handle_settings_documents_key(
        &mut self,
        keystroke: &Keystroke,
        cx: &mut App,
    ) -> bool {
        if !self.settings_data_page_has_focus(SettingsPage::Documents, cx) {
            return false;
        }
        let modifiers = &keystroke.modifiers;
        if modifiers.control || modifiers.alt || modifiers.platform {
            return false;
        }
        match (keystroke.key.as_str(), modifiers.shift) {
            ("x", false) => self.remove_selected_document_type(cx),
            ("n", false) => self.open_add_document_type_dialog(cx),
            ("e", false) => {
                if let Some(id) = self.settings_documents_selected_id(cx) {
                    self.open_edit_document_type_dialog(id, cx);
                }
            }
            _ => return false,
        }
        true
    }

    /// The Inventory page's selected row: the stored one while it is still on screen, else the
    /// first row, so the cursor is never lost. `None` only with no Properties.
    pub(super) fn settings_inventory_selected_row(&self, cx: &App) -> Option<InventoryRow> {
        let rows = inventory_view::visible_rows(
            self.inventory(cx),
            self.views.settings.read(cx).inventory_expanded(),
        );
        lib_list_cursor::resolve(rows, self.views.settings.read(cx).inventory_selected())
    }

    /// Whether `left` has something to do on the Inventory page: close an open Property, or
    /// climb from a Room to its Property. Anything else hands it back to the index.
    pub(super) fn settings_inventory_left_is_local(&self, cx: &App) -> bool {
        match self.settings_inventory_selected_row(cx) {
            Some(InventoryRow::Room(_)) => true,
            Some(InventoryRow::Property(id)) => self
                .views
                .settings
                .read(cx)
                .inventory_expanded()
                .contains(&id),
            None => false,
        }
    }

    /// `j`/`k`/`g`/`G`/`Ctrl-d`/`Ctrl-u` walk the visible Property and Room rows as one list.
    pub(super) fn apply_settings_inventory_movement(&mut self, movement: Movement, cx: &mut App) {
        let rows = inventory_view::visible_rows(
            self.inventory(cx),
            self.views.settings.read(cx).inventory_expanded(),
        );
        let Some(step) = movement.step() else {
            return;
        };
        if rows.is_empty() {
            return;
        }
        let current = self.settings_inventory_selected_row(cx);
        let next = lib_list_cursor::step_id(&rows, current, step);
        self.views
            .settings
            .update(cx, |v, cx| v.set_inventory_selected(cx, next));
    }

    /// `J`/`K` on a Room move it a place within its Property; inert on a Property row.
    pub(super) fn handle_settings_inventory_reorder_key(
        &mut self,
        keystroke: &Keystroke,
        cx: &mut App,
    ) -> bool {
        if !self.settings_data_page_has_focus(SettingsPage::Inventory, cx) {
            return false;
        }
        let modifiers = &keystroke.modifiers;
        if modifiers.control || modifiers.alt || modifiers.platform || !modifiers.shift {
            return false;
        }
        let delta = match keystroke.key.as_str() {
            "j" => 1,
            "k" => -1,
            _ => return false,
        };
        if let Some(InventoryRow::Room(id)) = self.settings_inventory_selected_row(cx) {
            self.ledger
                .update(cx, |store, cx| {
                    store.apply(cx, lib_ledger::MoveRoom { room: id, delta })
                })
                .ok();
        }
        true
    }

    /// The Inventory page's own keys: right and left open, close and climb, and `n`/`r`/`e`/`x`
    /// ask for the Add, Edit and Remove dialogs, which are still placeholders.
    pub(super) fn handle_settings_inventory_key(
        &mut self,
        keystroke: &Keystroke,
        cx: &mut App,
    ) -> bool {
        if !self.settings_data_page_has_focus(SettingsPage::Inventory, cx) {
            return false;
        }
        let modifiers = &keystroke.modifiers;
        if modifiers.control || modifiers.alt || modifiers.platform || modifiers.shift {
            return false;
        }
        match keystroke.key.as_str() {
            "n" => self.open_add_property_dialog(cx),
            "r" => self.add_inventory_room_for_selection(cx),
            "e" => {
                if let Some(row) = self.settings_inventory_selected_row(cx) {
                    self.open_edit_inventory_row(row, cx);
                }
            }
            "x" => {
                if let Some(row) = self.settings_inventory_selected_row(cx) {
                    self.open_remove_inventory_row(row, cx);
                }
            }
            "right" => self.step_settings_inventory_in(cx),
            "left" => self.step_settings_inventory_out(cx),
            _ => return false,
        }
        true
    }

    /// `r`: Add room for the selected Property (or the selected Room's), opening it so the new
    /// Room shows. With no Property it is a Toast.
    pub(super) fn add_inventory_room_for_selection(&mut self, cx: &mut App) {
        let property = match self.settings_inventory_selected_row(cx) {
            Some(InventoryRow::Property(id)) => id,
            Some(InventoryRow::Room(id)) => match self.inventory(cx).room(id) {
                Some((property, _)) => property.id,
                None => return,
            },
            None => {
                self.raise_toast(
                    ToastKind::Info,
                    crate::msg::desktop_inventory_toast_no_property(),
                );
                return;
            }
        };
        self.views
            .settings
            .update(cx, |v, cx| v.expand(cx, property));
        self.open_add_room_dialog(property, cx);
    }

    /// `right`: open a closed Property, or step from an open one to its first Room.
    pub(super) fn step_settings_inventory_in(&mut self, cx: &mut App) {
        let Some(InventoryRow::Property(id)) = self.settings_inventory_selected_row(cx) else {
            return;
        };
        if self.views.settings.update(cx, |v, cx| v.expand(cx, id)) {
            return;
        }
        let first_room = self
            .inventory(cx)
            .property(id)
            .and_then(|property| property.rooms.first().map(|room| room.id));
        if let Some(room) = first_room {
            self.views.settings.update(cx, |v, cx| {
                v.set_inventory_selected(cx, Some(InventoryRow::Room(room)))
            });
        }
    }

    /// `left`: climb from a Room to its Property, or close an open Property.
    pub(super) fn step_settings_inventory_out(&mut self, cx: &mut App) {
        match self.settings_inventory_selected_row(cx) {
            Some(InventoryRow::Room(id)) => {
                let property = self.inventory(cx).room(id).map(|(property, _)| property.id);
                if let Some(property) = property {
                    self.views.settings.update(cx, |v, cx| {
                        v.set_inventory_selected(cx, Some(InventoryRow::Property(property)))
                    });
                }
            }
            Some(InventoryRow::Property(id)) => {
                self.views.settings.update(cx, |v, cx| v.collapse(cx, id));
            }
            None => {}
        }
    }

    /// A click on a row of Settings' Documents table: selects it and moves focus into the page.
    pub(super) fn handle_settings_documents_row_click(
        &mut self,
        id: u32,
        cx: &mut Context<'_, Self>,
    ) {
        self.views
            .settings
            .update(cx, |v, cx| v.set_documents_selected(cx, Some(id)));
        self.focus_settings_page(cx);
        cx.notify();
    }

    /// **edit** on a Documents row: selects it and opens the Edit dialog.
    pub(super) fn handle_settings_documents_edit_click(
        &mut self,
        id: u32,
        cx: &mut Context<'_, Self>,
    ) {
        self.views
            .settings
            .update(cx, |v, cx| v.set_documents_selected(cx, Some(id)));
        self.focus_settings_page(cx);
        self.open_edit_document_type_dialog(id, cx);
        cx.notify();
    }

    /// **remove** on a Documents row: selects it and opens the Remove dialog.
    pub(super) fn handle_settings_documents_remove_click(
        &mut self,
        id: u32,
        cx: &mut Context<'_, Self>,
    ) {
        self.views
            .settings
            .update(cx, |v, cx| v.set_documents_selected(cx, Some(id)));
        self.focus_settings_page(cx);
        self.open_remove_document_type_dialog(id, cx);
        cx.notify();
    }

    /// **+ Add document type**: opens the Add dialog.
    pub(super) fn handle_settings_documents_add_click(&mut self, cx: &mut Context<'_, Self>) {
        self.focus_settings_page(cx);
        self.open_add_document_type_dialog(cx);
        cx.notify();
    }

    /// A click on a row of Settings' Inventory table: selects it and moves focus into the page.
    pub(super) fn handle_settings_inventory_row_click(
        &mut self,
        row: InventoryRow,
        cx: &mut Context<'_, Self>,
    ) {
        self.views
            .settings
            .update(cx, |v, cx| v.set_inventory_selected(cx, Some(row)));
        self.focus_settings_page(cx);
        cx.notify();
    }

    /// The disclosure control of a Property row: opens or closes it. Closing keeps the selection
    /// on screen by moving a hidden Room's selection up to its Property.
    pub(super) fn handle_settings_inventory_toggle_click(
        &mut self,
        id: u32,
        cx: &mut Context<'_, Self>,
    ) {
        self.focus_settings_page(cx);
        if self.views.settings.update(cx, |v, cx| v.collapse(cx, id)) {
            if let Some(InventoryRow::Room(room)) = self.settings_inventory_selected_row(cx)
                && self
                    .inventory(cx)
                    .room(room)
                    .is_some_and(|(property, _)| property.id == id)
            {
                self.views.settings.update(cx, |v, cx| {
                    v.set_inventory_selected(cx, Some(InventoryRow::Property(id)))
                });
            }
        } else {
            self.views.settings.update(cx, |v, cx| v.expand(cx, id));
        }
        cx.notify();
    }

    /// **edit** on an Inventory row: selects it and asks for the Edit dialog.
    pub(super) fn handle_settings_inventory_edit_click(
        &mut self,
        row: InventoryRow,
        cx: &mut Context<'_, Self>,
    ) {
        self.views
            .settings
            .update(cx, |v, cx| v.set_inventory_selected(cx, Some(row)));
        self.focus_settings_page(cx);
        self.open_edit_inventory_row(row, cx);
        cx.notify();
    }

    /// **remove** on an Inventory row: selects it and asks for the Remove dialog.
    pub(super) fn handle_settings_inventory_remove_click(
        &mut self,
        row: InventoryRow,
        cx: &mut Context<'_, Self>,
    ) {
        self.views
            .settings
            .update(cx, |v, cx| v.set_inventory_selected(cx, Some(row)));
        self.focus_settings_page(cx);
        self.open_remove_inventory_row(row, cx);
        cx.notify();
    }

    /// **+ Add property**.
    pub(super) fn handle_settings_inventory_add_property_click(
        &mut self,
        cx: &mut Context<'_, Self>,
    ) {
        self.focus_settings_page(cx);
        self.open_add_property_dialog(cx);
        cx.notify();
    }

    /// **+ Add room** under an open Property.
    pub(super) fn handle_settings_inventory_add_room_click(
        &mut self,
        id: u32,
        cx: &mut Context<'_, Self>,
    ) {
        self.focus_settings_page(cx);
        self.open_add_room_dialog(id, cx);
        cx.notify();
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
