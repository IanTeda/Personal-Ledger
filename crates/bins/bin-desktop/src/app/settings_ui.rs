//! The Settings destination's wiring: the page and focus rules, the Display colour grid, the
//! Documents and Inventory pages, the Units, Price Sources and Institutions dialogs and rows, the
//! Preferences, the Tracing page and the Settings dialogs. The pages themselves are drawn by
//! `view::settings`, and the View state is in the `SettingsView` Entity (ADR-0032).

use super::Shell;
use super::key_dispatch::PENDING_G_TIMEOUT;
use super::log_feed::LOG_LINE_STEP;
use crate::accounts::{self};
use crate::chrome::dialog_host::OpenDialog;
use crate::documents::{self};
use crate::institutions::AccountType;
use crate::institutions::form::AddInstitutionForm;
use crate::navigation::key_router::Movement;
use crate::navigation::nav::{FocusZone, InputMode, Noun};
use crate::settings::display::{DATE_STYLE_CHOICES, RowDensity, StatusGlyphs};
use crate::settings::tracing_log::TracingLevel;
use crate::settings::{
    DISPLAY_FIELD_COUNT, DISPLAY_FIELD_SIDEBAR, SettingsDialog, SettingsFocus, SettingsSection,
    step_choice,
};
use crate::theme::colours::ColourChange;
use crate::units::UnitKind;
use crate::units::form::{AddUnitField, DeleteUnitForm, UnitForm};
use crate::view::accounts::hints::accounts_hints;
use crate::view::settings as settings_view;
use crate::view::settings::hints::{
    confirm_dialog_hints, settings_categories_hints, settings_colour_grid_hints,
    settings_display_hints, settings_documents_hints, settings_index_hints,
    settings_inventory_hints, settings_payees_hints, settings_plain_page_hints,
    settings_tags_hints, settings_tracing_hints,
};
use crate::view::settings::inventory::{self as inventory_view, InventoryRow};
use gpui::{App, Context, Keystroke, ScrollStrategy};
use lib_core::DateStyle;
use lib_toast::ToastKind;

/// Settings' Documents page: rows per half page for `Ctrl-d`/`Ctrl-u`.
const SETTINGS_DOCUMENTS_HALF_PAGE: isize = 5;

/// Settings' Inventory page: rows per half page for `Ctrl-d`/`Ctrl-u`.
const SETTINGS_INVENTORY_HALF_PAGE: usize = 5;

impl Shell {
    /// Settings' Colour Theme grid (`docs/colour-themes-design.md` "Settings"): Focus arrives from
    /// the Display page (`l`/`enter` on the index) at the chosen card; arrows or `h`/`j`/`k`/`l` move focus,
    /// `Enter` selects, and `Esc` or `Tab` leaves it (`Tab` going on to the next zone). Moving
    /// focus never previews. `false` for any key the grid does not take.
    pub(super) fn handle_colour_theme_grid_key(
        &mut self,
        keystroke: &Keystroke,
        cx: &mut App,
    ) -> bool {
        let key = keystroke.key.as_str();
        let pending_g_active = self
            .pending_g
            .is_some_and(|since| since.elapsed() <= PENDING_G_TIMEOUT);
        if self.nav.mode() != InputMode::Normal
            || self.nav.noun() != Noun::Settings
            || self.nav.focus() != FocusZone::View
            || self.settings_view.read(cx).focus() != SettingsFocus::Page
            || self.settings_view.read(cx).selected_section() != SettingsSection::Display
            || keystroke.modifiers.control
            || pending_g_active
        {
            self.settings_view
                .update(cx, |v, cx| v.set_colour_theme_focus(cx, None));
            return false;
        }
        let Some(index) = self.settings_view.read(cx).colour_theme_focus() else {
            return false;
        };
        match key {
            "escape" => {
                self.chrome.status_message = None;
                self.leave_colour_theme_grid(cx);
                true
            }
            "tab" => {
                self.settings_view
                    .update(cx, |v, cx| v.set_colour_theme_focus(cx, None));
                false
            }
            "enter" => {
                if let Some(theme) = lib_colour_theme::ColourTheme::built_in().get(index) {
                    self.chrome.pending_colour_change = Some(ColourChange::Theme(theme.id));
                }
                true
            }
            _ => {
                let columns = settings_view::colour_theme::grid_columns(f32::from(
                    self.view_scroll_handle.bounds().size.width,
                ));
                let len = lib_colour_theme::ColourTheme::built_in().len();
                // `k` on the top row climbs back to the last control above the grid.
                if matches!(key, "k" | "up") && index < columns {
                    self.leave_colour_theme_grid(cx);
                    return true;
                }
                // `h` at the first column steps back out to the index.
                if matches!(key, "h" | "left") && index % columns.max(1) == 0 {
                    self.focus_settings_index(cx);
                    return true;
                }
                match settings_view::colour_theme::grid_move(index, len, columns, key) {
                    Some(next) => {
                        self.settings_view
                            .update(cx, |v, cx| v.set_colour_theme_focus(cx, Some(next)));
                        true
                    }
                    // Any other key leaves the grid and goes on to the usual handling.
                    None => {
                        self.settings_view
                            .update(cx, |v, cx| v.set_colour_theme_focus(cx, None));
                        false
                    }
                }
            }
        }
    }

    /// The status-line legend for Settings' current focus: the index rail's keys, or the open
    /// page's. The four list pages are handled before this (they own dialogs too), so it covers
    /// the index and the form and plain pages.
    pub(super) fn settings_hints(&self, cx: &App) -> Vec<(&'static str, String)> {
        if self.settings_view.read(cx).focus() == SettingsFocus::Index {
            return settings_index_hints();
        }
        match self.settings_view.read(cx).selected_section() {
            SettingsSection::Display
                if self.settings_view.read(cx).colour_theme_focus().is_some() =>
            {
                settings_colour_grid_hints()
            }
            SettingsSection::Display => settings_display_hints(),
            SettingsSection::Tracing if self.settings_dialog().is_some() => confirm_dialog_hints(),
            SettingsSection::Tracing => settings_tracing_hints(),
            _ => settings_plain_page_hints(),
        }
    }

    /// The `?` cheat-sheet's Settings group as `(action, keys)`: the index keys, then the open
    /// page's. Empty off the Settings noun, so the group is left out.
    pub(super) fn settings_cheat_sheet(&self, cx: &App) -> Vec<(String, &'static str)> {
        if self.nav.noun() != Noun::Settings {
            return Vec::new();
        }
        let page = match self.settings_view.read(cx).selected_section() {
            SettingsSection::Accounts => accounts_hints(),
            SettingsSection::Categories => settings_categories_hints(),
            SettingsSection::Tags => settings_tags_hints(),
            SettingsSection::Payees => settings_payees_hints(),
            SettingsSection::Documents => settings_documents_hints(),
            SettingsSection::Inventory => {
                settings_inventory_hints(self.settings_inventory_selected_row(cx))
            }
            SettingsSection::Display => {
                let mut keys = settings_display_hints();
                keys.extend(settings_colour_grid_hints().into_iter().take(2));
                keys
            }
            SettingsSection::Tracing => settings_tracing_hints(),
            _ => settings_plain_page_hints(),
        };
        settings_index_hints()
            .into_iter()
            .chain(page)
            .map(|(keys, action)| (action, keys))
            .collect()
    }

    /// Steps from the Colour Theme grid back up to the Display page's last control.
    pub(super) fn leave_colour_theme_grid(&mut self, cx: &mut App) {
        self.settings_view
            .update(cx, |v, cx| v.set_colour_theme_focus(cx, None));
        self.settings_view.update(cx, |v, cx| {
            v.set_display_field(cx, Some(DISPLAY_FIELD_COUNT - 1))
        });
    }

    /// Swaps the Settings page on show. Each page starts at its top.
    /// Opens a Settings page with focus in it -- what `:settings <page>`, the old `:accounts` /
    /// `:categories` / `:payees` / `:tags` aliases and every hand-off to a moved noun land on
    /// (the keyboard model's "commands and hand-offs land in the page"). Leaves the view's scroll
    /// alone when the page is already showing.
    pub(super) fn open_settings_page(&mut self, section: SettingsSection, cx: &mut App) {
        let noun_before = self.nav.noun();
        self.nav.set_noun(Noun::Settings);
        if noun_before != Noun::Settings {
            self.reset_view_scroll(cx);
        }
        self.select_settings_page(section, cx);
        self.focus_settings_page(cx);
    }

    pub(super) fn select_settings_page(&mut self, section: SettingsSection, cx: &mut App) {
        if self.settings_view.read(cx).selected_section() != section {
            self.view_scroll_handle.set_offset(gpui::Point::default());
        }
        self.settings_view
            .update(cx, |v, cx| v.set_selected_section(cx, section));
        self.settings_view
            .update(cx, |v, cx| v.set_colour_theme_focus(cx, None));
        self.settings_view
            .update(cx, |v, cx| v.set_display_field(cx, None));
    }

    /// Moves focus from the index rail into the open page (`l`/`enter`, `:settings <page>`). A
    /// page with nothing to focus keeps focus on the index, as a quiet no-op.
    pub(super) fn focus_settings_page(&mut self, cx: &mut App) {
        if !self
            .settings_view
            .read(cx)
            .selected_section()
            .has_controls()
        {
            return;
        }
        self.settings_view
            .update(cx, |v, cx| v.set_focus(cx, SettingsFocus::Page));
        self.settings_view.update(cx, |v, cx| {
            let on_display = v.selected_section() == SettingsSection::Display;
            v.set_display_field(cx, on_display.then_some(0));
        });
    }

    pub(super) fn focus_settings_index(&mut self, cx: &mut App) {
        self.settings_view
            .update(cx, |v, cx| v.set_focus(cx, SettingsFocus::Index));
        self.settings_view
            .update(cx, |v, cx| v.set_colour_theme_focus(cx, None));
        self.settings_view
            .update(cx, |v, cx| v.set_display_field(cx, None));
    }

    /// The Display page's form keys, ahead of Settings' focus keys: `j`/`k` walk the controls and
    /// on past the last into the Colour Theme grid, `h`/`l` change a segmented or radio control
    /// (or clear/tick the checkbox) in place, `enter`/`space` toggles the checkbox. `esc` is left
    /// to the focus keys, which step back to the index. `false` for any key it does not take.
    pub(super) fn handle_settings_form_key(
        &mut self,
        keystroke: &Keystroke,
        chosen: usize,
        cx: &mut App,
    ) -> bool {
        let pending_g_active = self
            .pending_g
            .is_some_and(|since| since.elapsed() <= PENDING_G_TIMEOUT);
        let Some(field) = self.settings_view.read(cx).display_field() else {
            return false;
        };
        if self.nav.mode() != InputMode::Normal
            || self.nav.noun() != Noun::Settings
            || self.nav.focus() != FocusZone::View
            || self.settings_view.read(cx).focus() != SettingsFocus::Page
            || self.settings_view.read(cx).selected_section() != SettingsSection::Display
            || keystroke.modifiers.control
            || keystroke.modifiers.shift
            || pending_g_active
        {
            return false;
        }
        let delta = match keystroke.key.as_str() {
            "h" | "left" => -1,
            "l" | "right" => 1,
            _ => 0,
        };
        match keystroke.key.as_str() {
            "j" | "down" => {
                self.chrome.status_message = None;
                if field + 1 >= DISPLAY_FIELD_COUNT {
                    self.settings_view
                        .update(cx, |v, cx| v.set_display_field(cx, None));
                    self.settings_view
                        .update(cx, |v, cx| v.set_colour_theme_focus(cx, Some(chosen)));
                } else {
                    self.settings_view
                        .update(cx, |v, cx| v.set_display_field(cx, Some(field + 1)));
                }
                true
            }
            "k" | "up" => {
                self.settings_view.update(cx, |v, cx| {
                    v.set_display_field(cx, Some(field.saturating_sub(1)))
                });
                true
            }
            "h" | "left" | "l" | "right" => {
                self.chrome.status_message = None;
                match field {
                    0 => {
                        self.settings_date_style =
                            step_choice(&DATE_STYLE_CHOICES, self.settings_date_style, delta);
                    }
                    1 => {
                        self.settings_row_density =
                            step_choice(&RowDensity::ALL, self.settings_row_density, delta);
                    }
                    2 => {
                        self.settings_status_glyphs =
                            step_choice(&StatusGlyphs::ALL, self.settings_status_glyphs, delta);
                    }
                    DISPLAY_FIELD_SIDEBAR => self.settings_start_sidebar_minimised = delta > 0,
                    _ => self.set_toasts_on(delta < 0),
                }
                true
            }
            "enter" | "space" => {
                if field == DISPLAY_FIELD_SIDEBAR {
                    self.settings_start_sidebar_minimised = !self.settings_start_sidebar_minimised;
                }
                true
            }
            _ => false,
        }
    }

    /// The Tracing page's keys while it has focus, ahead of Settings' focus keys so `h` steps the
    /// level rather than leaving (Display's radio grammar; `esc` leaves): `h`/`l` the level,
    /// `j`/`k` a line, `J`/`K` a page, `G` the oldest entry, `c` Clear logs. Focus never enters
    /// the box itself. `false` for any key it does not take.
    pub(super) fn handle_settings_tracing_key(
        &mut self,
        keystroke: &Keystroke,
        cx: &mut App,
    ) -> bool {
        let pending_g_active = self
            .pending_g
            .is_some_and(|since| since.elapsed() <= PENDING_G_TIMEOUT);
        let modifiers = &keystroke.modifiers;
        if self.nav.mode() != InputMode::Normal
            || self.nav.noun() != Noun::Settings
            || self.nav.focus() != FocusZone::View
            || self.settings_view.read(cx).focus() != SettingsFocus::Page
            || self.settings_view.read(cx).selected_section() != SettingsSection::Tracing
            || modifiers.control
            || modifiers.alt
            || modifiers.platform
            || pending_g_active
        {
            return false;
        }
        let list = self.settings_view.read(cx).log_list().clone();
        let page = list.viewport_bounds().size.height.max(LOG_LINE_STEP);
        match (modifiers.shift, keystroke.key.as_str()) {
            (false, "h" | "left") => self.step_tracing_level(-1, cx),
            (false, "l" | "right") => self.step_tracing_level(1, cx),
            (false, "j" | "down") => list.scroll_by(LOG_LINE_STEP),
            (false, "k" | "up") => list.scroll_by(-LOG_LINE_STEP),
            (true, "j") => list.scroll_by(page),
            (true, "k") => list.scroll_by(-page),
            (true, "g") => {
                if let Some(last) = list.item_count().checked_sub(1) {
                    list.scroll_to_reveal_item(last);
                }
            }
            (false, "c") => self.open_clear_logs_dialog(),
            _ => return false,
        }
        true
    }

    pub(super) fn step_tracing_level(&mut self, delta: isize, cx: &mut App) {
        self.chrome.status_message = None;
        let level = step_choice(
            &TracingLevel::ALL,
            self.settings_view.read(cx).log().level(),
            delta,
        );
        self.set_tracing_level(level, cx);
    }

    /// Settings' own focus keys, ahead of the Colour Theme grid and the global keymap: `l`/`right`
    /// /`enter` on the index step into the page, `h`/`left`/`esc` on the page step back out.
    /// `false` for any key it does not take.
    pub(super) fn handle_settings_focus_key(
        &mut self,
        keystroke: &Keystroke,
        cx: &mut App,
    ) -> bool {
        let pending_g_active = self
            .pending_g
            .is_some_and(|since| since.elapsed() <= PENDING_G_TIMEOUT);
        if self.nav.mode() != InputMode::Normal
            || self.nav.noun() != Noun::Settings
            || self.nav.focus() != FocusZone::View
            || keystroke.modifiers.control
            || keystroke.modifiers.shift
            || pending_g_active
        {
            return false;
        }
        match (self.settings_view.read(cx).focus(), keystroke.key.as_str()) {
            (SettingsFocus::Index, "l" | "right" | "enter") => {
                self.chrome.status_message = None;
                self.focus_settings_page(cx);
                true
            }
            // `left` on the Categories tree collapses or climbs first; only a top-level row with
            // nothing to fold hands it back to the index. `h` always leaves.
            (SettingsFocus::Page, "left")
                if self.settings_categories_page_has_focus(cx)
                    && self.settings_categories_left_is_local(cx) =>
            {
                false
            }
            (SettingsFocus::Page, "left")
                if self.settings_inventory_page_has_focus(cx)
                    && self.settings_inventory_left_is_local(cx) =>
            {
                false
            }
            (SettingsFocus::Page, "h" | "left")
                if self.settings_view.read(cx).colour_theme_focus().is_none() =>
            {
                self.chrome.status_message = None;
                self.focus_settings_index(cx);
                true
            }
            (SettingsFocus::Page, "escape")
                if self.settings_view.read(cx).colour_theme_focus().is_none() =>
            {
                self.focus_settings_index(cx);
                true
            }
            _ => false,
        }
    }

    /// `j`/`k`/`g g`/`G` on the Settings index step the highlight through the pages
    /// and swap the page live, like an index click.
    pub(super) fn apply_settings_section_movement(&mut self, movement: Movement, cx: &mut App) {
        let visible: Vec<SettingsSection> = SettingsSection::ALL.into_iter().collect();
        let Some(last) = visible.len().checked_sub(1) else {
            return;
        };
        let current = visible
            .iter()
            .position(|section| *section == self.settings_view.read(cx).selected_section())
            .unwrap_or(0);
        let next = match movement {
            Movement::Next => (current + 1).min(last),
            Movement::Prev => current.saturating_sub(1),
            Movement::First => 0,
            _ => last,
        };
        self.select_settings_page(visible[next], cx);
    }

    /// Whether Settings' Documents table owns the keyboard: the page, not the index, has focus.
    pub(super) fn settings_documents_page_has_focus(&self, cx: &App) -> bool {
        self.nav.noun() == Noun::Settings
            && self.nav.focus() == FocusZone::View
            && self.settings_view.read(cx).focus() == SettingsFocus::Page
            && self.settings_view.read(cx).selected_section() == SettingsSection::Documents
    }

    /// The Documents page's selected type: the stored id while it still exists, else the first
    /// row, so the cursor is never lost.
    pub(super) fn settings_documents_selected_id(&self, cx: &App) -> Option<u32> {
        self.settings_view
            .read(cx)
            .documents_selected()
            .filter(|id| documents::types::position(self.document_types(cx), *id).is_some())
            .or_else(|| self.document_types(cx).first().map(|row| row.id))
    }

    /// `j`/`k`/`g`/`G`/`Ctrl-d`/`Ctrl-u` walk the types in the user's order.
    pub(super) fn apply_settings_documents_movement(&mut self, movement: Movement, cx: &mut App) {
        let len = self.document_types(cx).len();
        let current = self
            .settings_documents_selected_id(cx)
            .and_then(|id| documents::types::position(self.document_types(cx), id))
            .unwrap_or(0);
        let next = match movement {
            Movement::Next => accounts::step_selection(current, len, 1),
            Movement::Prev => accounts::step_selection(current, len, -1),
            Movement::First => 0,
            Movement::Last => len.saturating_sub(1),
            Movement::HalfPageDown => {
                accounts::step_selection(current, len, SETTINGS_DOCUMENTS_HALF_PAGE)
            }
            Movement::HalfPageUp => {
                accounts::step_selection(current, len, -SETTINGS_DOCUMENTS_HALF_PAGE)
            }
            Movement::Enter => return,
        };
        self.settings_view.update(cx, |v, cx| {
            v.set_documents_selected(cx, self.document_types(cx).get(next).map(|row| row.id))
        });
    }

    /// `J`/`K` on the Documents page move the selected type a place, which is also its place in
    /// the Documents Type filter.
    pub(super) fn handle_settings_documents_reorder_key(
        &mut self,
        keystroke: &Keystroke,
        cx: &mut App,
    ) -> bool {
        if !self.settings_documents_page_has_focus(cx) {
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
        if !self.settings_documents_page_has_focus(cx) {
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

    /// Whether Settings' Inventory table owns the keyboard: the page, not the index, has focus.
    pub(super) fn settings_inventory_page_has_focus(&self, cx: &App) -> bool {
        self.nav.noun() == Noun::Settings
            && self.nav.focus() == FocusZone::View
            && self.settings_view.read(cx).focus() == SettingsFocus::Page
            && self.settings_view.read(cx).selected_section() == SettingsSection::Inventory
    }

    /// The Inventory page's selected row: the stored one while it is still on screen, else the
    /// first row, so the cursor is never lost. `None` only with no Properties.
    pub(super) fn settings_inventory_selected_row(&self, cx: &App) -> Option<InventoryRow> {
        let rows = inventory_view::visible_rows(
            self.inventory(cx),
            self.settings_view.read(cx).inventory_expanded(),
        );
        self.settings_view
            .read(cx)
            .inventory_selected()
            .filter(|row| rows.contains(row))
            .or_else(|| rows.first().copied())
    }

    /// Whether `left` has something to do on the Inventory page: close an open Property, or
    /// climb from a Room to its Property. Anything else hands it back to the index.
    pub(super) fn settings_inventory_left_is_local(&self, cx: &App) -> bool {
        match self.settings_inventory_selected_row(cx) {
            Some(InventoryRow::Room(_)) => true,
            Some(InventoryRow::Property(id)) => self
                .settings_view
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
            self.settings_view.read(cx).inventory_expanded(),
        );
        let Some(last) = rows.len().checked_sub(1) else {
            return;
        };
        let current = self
            .settings_inventory_selected_row(cx)
            .and_then(|row| rows.iter().position(|r| *r == row))
            .unwrap_or(0);
        let next = match movement {
            Movement::Next => (current + 1).min(last),
            Movement::Prev => current.saturating_sub(1),
            Movement::First => 0,
            Movement::Last => last,
            Movement::HalfPageDown => (current + SETTINGS_INVENTORY_HALF_PAGE).min(last),
            Movement::HalfPageUp => current.saturating_sub(SETTINGS_INVENTORY_HALF_PAGE),
            Movement::Enter => return,
        };
        self.settings_view.update(cx, |v, cx| {
            v.set_inventory_selected(cx, rows.get(next).copied())
        });
    }

    /// `J`/`K` on a Room move it a place within its Property; inert on a Property row.
    pub(super) fn handle_settings_inventory_reorder_key(
        &mut self,
        keystroke: &Keystroke,
        cx: &mut App,
    ) -> bool {
        if !self.settings_inventory_page_has_focus(cx) {
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
            self.inventory
                .update(cx, |store, cx| store.move_room(cx, id, delta));
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
        if !self.settings_inventory_page_has_focus(cx) {
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
        self.settings_view
            .update(cx, |v, cx| v.expand(cx, property));
        self.open_add_room_dialog(property, cx);
    }

    /// `right`: open a closed Property, or step from an open one to its first Room.
    pub(super) fn step_settings_inventory_in(&mut self, cx: &mut App) {
        let Some(InventoryRow::Property(id)) = self.settings_inventory_selected_row(cx) else {
            return;
        };
        if self.settings_view.update(cx, |v, cx| v.expand(cx, id)) {
            return;
        }
        let first_room = self
            .inventory(cx)
            .property(id)
            .and_then(|property| property.rooms.first().map(|room| room.id));
        if let Some(room) = first_room {
            self.settings_view.update(cx, |v, cx| {
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
                    self.settings_view.update(cx, |v, cx| {
                        v.set_inventory_selected(cx, Some(InventoryRow::Property(property)))
                    });
                }
            }
            Some(InventoryRow::Property(id)) => {
                self.settings_view.update(cx, |v, cx| v.collapse(cx, id));
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
        self.settings_view
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
        self.settings_view
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
        self.settings_view
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
        self.settings_view
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
        if self.settings_view.update(cx, |v, cx| v.collapse(cx, id)) {
            if let Some(InventoryRow::Room(room)) = self.settings_inventory_selected_row(cx)
                && self
                    .inventory(cx)
                    .room(room)
                    .is_some_and(|(property, _)| property.id == id)
            {
                self.settings_view.update(cx, |v, cx| {
                    v.set_inventory_selected(cx, Some(InventoryRow::Property(id)))
                });
            }
        } else {
            self.settings_view.update(cx, |v, cx| v.expand(cx, id));
        }
        cx.notify();
    }

    /// **edit** on an Inventory row: selects it and asks for the Edit dialog.
    pub(super) fn handle_settings_inventory_edit_click(
        &mut self,
        row: InventoryRow,
        cx: &mut Context<'_, Self>,
    ) {
        self.settings_view
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
        self.settings_view
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

    /// The Units section's own "+ Add unit" button (issue #184, replacing the stub #177 left
    /// behind): opens the Add unit dialog rather than flashing a status message.
    pub(super) fn handle_add_unit_click(&mut self, cx: &mut Context<'_, Self>) {
        self.open_dialog(OpenDialog::Settings(SettingsDialog::AddUnit(
            UnitForm::default(),
        )));
        cx.notify();
    }

    /// The Units table's own row "edit" button (issue #185, replacing the stub #177 left
    /// behind): opens the Edit unit dialog pre-filled from the clicked row
    /// (`UnitForm::from_row`) rather than flashing a status message. A no-op if `index` is
    /// somehow out of bounds (defensive only -- every caller is a row's own click handler, so
    /// this should never actually happen).
    pub(super) fn handle_unit_edit_click(&mut self, index: usize, cx: &mut Context<'_, Self>) {
        let Some(row) = self.units_store.read(cx).unit(index) else {
            return;
        };
        let form = UnitForm::from_row(row);
        self.open_dialog(OpenDialog::Settings(SettingsDialog::EditUnit(index, form)));
        cx.notify();
    }

    /// Shared by the Add/Edit unit dialogs' own field-focus clicks (issues #184/#185) -- which
    /// field a click targets doesn't depend on which dialog variant is open.
    pub(super) fn handle_unit_dialog_field_click(
        &mut self,
        field: AddUnitField,
        cx: &mut Context<'_, Self>,
    ) {
        if let Some(dialog) = self.settings_dialog_mut() {
            match dialog {
                SettingsDialog::AddUnit(form) | SettingsDialog::EditUnit(_, form) => {
                    form.focused_field = field;
                    cx.notify();
                }
                // Neither has more than one text field, always implicitly focused -- nothing to
                // click into.
                SettingsDialog::DeleteUnit(..)
                | SettingsDialog::AddInstitution(_)
                | SettingsDialog::ClearLogs => {}
            }
        }
    }

    /// Shared by the Add/Edit unit dialogs' own Type segmented control (issues #184/#185).
    pub(super) fn handle_unit_dialog_kind_click(
        &mut self,
        kind: UnitKind,
        cx: &mut Context<'_, Self>,
    ) {
        if let Some(dialog) = self.settings_dialog_mut() {
            match dialog {
                SettingsDialog::AddUnit(form) | SettingsDialog::EditUnit(_, form) => {
                    form.kind = kind;
                    cx.notify();
                }
                // Neither has a Type selector at all.
                SettingsDialog::DeleteUnit(..)
                | SettingsDialog::AddInstitution(_)
                | SettingsDialog::ClearLogs => {}
            }
        }
    }

    /// Shared by the Add/Edit unit dialogs' own Cancel button -- discards whatever was typed,
    /// same as `Esc` (`Self::handle_key_down`'s `ClosePopupsAndExitMode` arm).
    pub(super) fn handle_settings_dialog_cancel(&mut self, cx: &mut Context<'_, Self>) {
        self.close_dialog();
        cx.notify();
    }

    /// Applies a confirmed Settings dialog (reached through [`Self::confirm_open_dialog`], from
    /// the Add/Save/Delete button or `Enter`): the README's own "Dialog lifecycle" rows -- Add
    /// "validate -> append to Units table -> close", Edit "Save -> update the in-memory row ->
    /// close" (a changed code just relabels the row here; rewriting real references is out of
    /// scope per the map's own Destination), Delete "confirm -> remove + close". The form has
    /// already validated.
    pub(super) fn apply_settings_dialog(&mut self, dialog: SettingsDialog, cx: &mut App) {
        match dialog {
            SettingsDialog::AddUnit(form) => {
                // Neither Add unit field carries a source or a base or default flag: the store
                // defaults a dialog-created unit to "Manual entry", not base and not default.
                self.units_store.update(cx, |store, cx| {
                    store.add_unit(
                        cx,
                        form.code.into_text(),
                        form.name.into_text(),
                        form.kind.label().to_string(),
                    );
                });
            }
            SettingsDialog::EditUnit(index, form) => {
                // Source and the base and default flags aren't Edit unit fields either, so the
                // store keeps them from the row being edited.
                self.units_store.update(cx, |store, cx| {
                    store.edit_unit(
                        cx,
                        index,
                        form.code.into_text(),
                        form.name.into_text(),
                        form.kind.label().to_string(),
                    );
                });
            }
            SettingsDialog::DeleteUnit(index, form) => {
                // Defensive only: the dialog is modal, so the row it opened on is still there.
                let removed = self
                    .units_store
                    .update(cx, |store, cx| store.remove_unit(cx, index, &form.code));
                if let Some(unit) = removed {
                    self.raise_toast(
                        ToastKind::Success,
                        lib_locale::msg::toast_unit_deleted(&unit.code),
                    );
                }
            }
            SettingsDialog::AddInstitution(form) => {
                let account_type = form
                    .account_types
                    .iter()
                    .map(|account_type| account_type.label())
                    .collect::<Vec<_>>()
                    .join(" \u{b7} ");
                self.institutions_store.update(cx, |store, cx| {
                    store.add(cx, form.name.into_text(), account_type);
                });
            }
            SettingsDialog::ClearLogs => {
                self.settings_view
                    .update(cx, |view, cx| view.update_log(cx, |log| log.clear()));
                self.raise_toast(
                    ToastKind::Info,
                    crate::msg::desktop_settings_tracing_toast_cleared(),
                );
            }
        }
    }

    pub(super) fn handle_settings_dialog_confirm(&mut self, cx: &mut Context<'_, Self>) {
        self.confirm_open_dialog(cx);
        cx.notify();
    }

    /// The settings index rail's own row click (`chrome::rail::settings_index::OnEntryClick`):
    /// swaps the settings body to the clicked page and takes the active dark treatment
    /// (`docs/ux/desktop-mockups/16-settings/README.md`'s "Navigation" bullet).
    pub(super) fn handle_settings_index_click(
        &mut self,
        section: SettingsSection,
        cx: &mut Context<'_, Self>,
    ) {
        self.select_settings_page(section, cx);
        self.settings_view
            .update(cx, |v, cx| v.set_focus(cx, SettingsFocus::Index));
        cx.notify();
    }

    /// The Price Sources table's own row "test"/"edit"/"delete" buttons and its own "+ Add price
    /// source" button (issue #189): no dialog exists for any of these anywhere on this map (same
    /// reasoning as Institutions' own row edit/delete, `Self::handle_institution_edit_click`'s
    /// own doc), so each flashes a plain "not yet built" status message naming no issue.
    pub(super) fn handle_price_source_test_click(
        &mut self,
        index: usize,
        cx: &mut Context<'_, Self>,
    ) {
        let _ = index;
        self.chrome.status_message =
            Some(crate::msg::desktop_status_test_price_source_not_yet_built());
        cx.notify();
    }

    pub(super) fn handle_price_source_edit_click(
        &mut self,
        index: usize,
        cx: &mut Context<'_, Self>,
    ) {
        let _ = index;
        self.chrome.status_message =
            Some(crate::msg::desktop_status_edit_price_source_not_yet_built());
        cx.notify();
    }

    pub(super) fn handle_price_source_delete_click(
        &mut self,
        index: usize,
        cx: &mut Context<'_, Self>,
    ) {
        let _ = index;
        self.chrome.status_message =
            Some(crate::msg::desktop_status_delete_price_source_not_yet_built());
        cx.notify();
    }

    pub(super) fn handle_add_price_source_click(&mut self, cx: &mut Context<'_, Self>) {
        self.chrome.status_message =
            Some(crate::msg::desktop_status_add_price_source_not_yet_built());
        cx.notify();
    }

    /// The Units table's own row "delete" button (issue #186, replacing the stub #177 left
    /// behind): opens the destructive Delete unit confirm dialog rather than flashing a status
    /// message. A no-op if `index` is somehow out of bounds (defensive only, same reasoning as
    /// [`Self::handle_unit_edit_click`]).
    pub(super) fn handle_unit_delete_click(&mut self, index: usize, cx: &mut Context<'_, Self>) {
        let Some(row) = self.units_store.read(cx).unit(index) else {
            return;
        };
        let form = DeleteUnitForm::new(row.code.as_str());
        self.open_dialog(OpenDialog::Settings(SettingsDialog::DeleteUnit(
            index, form,
        )));
        cx.notify();
    }

    /// The Institutions table's own row "edit"/"delete" buttons (issue #178). Unlike
    /// [`Self::handle_unit_edit_click`]/[`Self::handle_unit_delete_click`], neither stub names an
    /// issue: no `EditInstitution`/`DeleteInstitution` dialog is specified anywhere on this map
    /// (the README's own "Dialog lifecycle" table and `State` block only ever mention
    /// `AddInstitution`), so there is no ticket to point at.
    pub(super) fn handle_institution_edit_click(
        &mut self,
        index: usize,
        cx: &mut Context<'_, Self>,
    ) {
        let _ = index; // no row-scoped state until a future ticket specifies this dialog
        self.chrome.status_message =
            Some(crate::msg::desktop_status_edit_institution_not_yet_built());
        cx.notify();
    }

    pub(super) fn handle_institution_delete_click(
        &mut self,
        index: usize,
        cx: &mut Context<'_, Self>,
    ) {
        let _ = index; // no row-scoped state until a future ticket specifies this dialog
        self.chrome.status_message =
            Some(crate::msg::desktop_status_delete_institution_not_yet_built());
        cx.notify();
    }

    /// The Institutions table's own "+ Add institution" button (issue #187, replacing the stub
    /// #178 left behind): opens the real Add institution dialog rather than flashing a status
    /// message. `AddInstitutionForm::new` seeds Default unit from `self.units_store.read(cx).units()`' own
    /// first entry, so this dialog reads Units' live state even though the two sections are
    /// otherwise independent.
    pub(super) fn handle_add_institution_click(&mut self, cx: &mut Context<'_, Self>) {
        let form = AddInstitutionForm::new(self.units_store.read(cx).units());
        self.open_dialog(OpenDialog::Settings(SettingsDialog::AddInstitution(form)));
        cx.notify();
    }

    pub(super) fn handle_add_institution_account_type_click(
        &mut self,
        account_type: AccountType,
        cx: &mut Context<'_, Self>,
    ) {
        if let Some(SettingsDialog::AddInstitution(form)) = self.settings_dialog_mut() {
            form.toggle_account_type(account_type);
            cx.notify();
        }
    }

    pub(super) fn handle_add_institution_unit_click(
        &mut self,
        code: String,
        cx: &mut Context<'_, Self>,
    ) {
        if let Some(SettingsDialog::AddInstitution(form)) = self.settings_dialog_mut() {
            form.default_unit_code = Some(code);
            cx.notify();
        }
    }

    /// The Sync server section's own "Sync now" button (issue #180): unlike the "+ Add"
    /// buttons above, this has no future ticket that will give it real behaviour -- the map's
    /// own Out-of-scope names it a permanent stand-in -- so the stub message names no issue.
    pub(super) fn handle_sync_now_click(&mut self, cx: &mut Context<'_, Self>) {
        self.chrome.status_message = Some(crate::msg::desktop_status_sync_now_not_implemented());
        cx.notify();
    }

    /// The Data & backup section's own "Backup now"/"Export ledger (CSV)" buttons (issue #181)
    /// -- same permanently-out-of-scope reasoning as [`Self::handle_sync_now_click`].
    pub(super) fn handle_backup_now_click(&mut self, cx: &mut Context<'_, Self>) {
        self.chrome.status_message = Some(crate::msg::desktop_status_backup_now_not_implemented());
        cx.notify();
    }

    pub(super) fn handle_export_ledger_click(&mut self, cx: &mut Context<'_, Self>) {
        self.chrome.status_message =
            Some(crate::msg::desktop_status_export_ledger_not_implemented());
        cx.notify();
    }

    /// The Tracing page's level radios: filters the log box at once, including what is already
    /// there, and starts it back at the newest entry.
    pub(super) fn handle_tracing_level_click(
        &mut self,
        level: TracingLevel,
        cx: &mut Context<'_, Self>,
    ) {
        self.set_tracing_level(level, cx);
        cx.notify();
    }

    pub(super) fn set_tracing_level(&mut self, level: TracingLevel, cx: &mut App) {
        self.settings_view.update(cx, |view, cx| {
            view.update_log(cx, |log| log.set_level(level))
        });
    }

    /// The Display section's own "Date format" segmented control (issue #179) -- a stored
    /// preference that also re-renders the PREVIEW table's own DATE column.
    pub(super) fn handle_date_style_click(
        &mut self,
        style: Option<DateStyle>,
        cx: &mut Context<'_, Self>,
    ) {
        self.settings_date_style = style;
        cx.notify();
    }

    /// The same section's "Row density" segmented control -- also re-renders the PREVIEW table's
    /// own row padding (`view::settings::display`'s own doc: the one field this map gives a real
    /// visual effect to, not just a stored preference).
    pub(super) fn handle_row_density_click(
        &mut self,
        density: RowDensity,
        cx: &mut Context<'_, Self>,
    ) {
        self.settings_row_density = density;
        // The Transactions table's scroll offset is in pixels, so a new row height would leave it
        // pointing at a different row: re-anchor on the selected one for its next paint.
        self.transactions_state(cx)
            .scroll
            .scroll_to_item_strict(self.transactions_state(cx).selected, ScrollStrategy::Center);
        cx.notify();
    }

    /// The same section's "Status glyphs" radio group -- also re-renders the PREVIEW table's own
    /// leftmost glyph column.
    pub(super) fn handle_status_glyphs_click(
        &mut self,
        glyphs: StatusGlyphs,
        cx: &mut Context<'_, Self>,
    ) {
        self.settings_status_glyphs = glyphs;
        cx.notify();
    }

    /// The same section's "Start Sidebar minimised" toggle -- takes effect at the next launch, so
    /// the current rail state is left alone.
    pub(super) fn handle_start_sidebar_minimised_click(&mut self, cx: &mut Context<'_, Self>) {
        self.settings_start_sidebar_minimised = !self.settings_start_sidebar_minimised;
        cx.notify();
    }

    /// The Tracing page's **Clear logs** button (and `c`): asks first, since the capture is
    /// emptied for good (issue #502).
    pub(super) fn handle_clear_logs_click(&mut self, cx: &mut Context<'_, Self>) {
        self.open_clear_logs_dialog();
        cx.notify();
    }

    pub(super) fn open_clear_logs_dialog(&mut self) {
        self.chrome.status_message = None;
        self.open_dialog(OpenDialog::Settings(SettingsDialog::ClearLogs));
    }
}
