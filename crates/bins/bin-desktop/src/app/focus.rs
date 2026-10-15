//! Focus-zone movement and the view-scroll reset of `Shell`. An `impl Shell` block for the
//! concern, not the focus model itself (`navigation::nav`).

use super::Shell;
use crate::{
    chrome::rail as chrome_rail,
    navigation::key_router::Movement,
    navigation::nav::FocusZone,
    settings::SettingsPage,
    view::active::{ActiveViewEntity, ShellView},
};

impl Shell {
    pub(super) fn apply_movement(&mut self, movement: Movement, cx: &mut gpui::Context<'_, Self>) {
        let noun_before = self.nav.noun();
        match self.nav.focus() {
            FocusZone::PrimaryRail => self.apply_primary_rail_movement(movement),
            FocusZone::ContextRail => self.apply_context_rail_movement(movement),
            FocusZone::View => self.apply_view_movement(movement, cx),
        }
        if self.nav.noun() != noun_before {
            self.reset_view_scroll(cx);
        }
    }

    /// A new noun's view is a different (usually much shorter) length -- carrying over the
    /// old scroll offset could leave it scrolled past all its content, rendering blank. Every
    /// fresh noun starts scrolled to the top, and the View that is now the destination is told
    /// (`on_enter`), so it can reset its own scroll and focus.
    pub(super) fn reset_view_scroll(&mut self, cx: &mut gpui::App) {
        self.view_scroll_handle.set_offset(gpui::Point::default());
        self.active_view_entity(cx).on_enter(cx);
    }

    fn apply_primary_rail_movement(&mut self, movement: Movement) {
        match movement {
            Movement::Next => self.nav.move_primary_highlight_next(),
            Movement::Prev => self.nav.move_primary_highlight_prev(),
            Movement::First => self.nav.move_primary_highlight_first(),
            Movement::Last => self.nav.move_primary_highlight_last(),
            // The rail's fixed 10 rows are not a real page; the List Cursor's half page is a
            // reasonable stand-in for `Ctrl-d`/`Ctrl-u` there.
            Movement::HalfPageDown => {
                for _ in 0..lib_list_cursor::HALF_PAGE {
                    self.nav.move_primary_highlight_next();
                }
            }
            Movement::HalfPageUp => {
                for _ in 0..lib_list_cursor::HALF_PAGE {
                    self.nav.move_primary_highlight_prev();
                }
            }
            Movement::Enter => self.nav.commit_primary_highlight(),
        }
    }

    fn apply_context_rail_movement(&mut self, movement: Movement) {
        let count = chrome_rail::context::entity_count(self.nav.noun());
        // Every noun besides Dashboard has a placeholder context rail with nothing in it yet
        // (see `chrome::rail::context::entity_count`'s own doc) -- movement is a no-op there, not an
        // out-of-bounds index.
        if count == 0 {
            return;
        }
        let half_page = (count / 2).max(1);
        let current = self.nav.context().unwrap_or(0);
        let next = match movement.step() {
            Some(step) => lib_list_cursor::step_by(current, count, step, half_page),
            // The handoff's own "Movement" bullet: `Enter` "selects the entity and leaves
            // focus where it is" -- but movement here already updates `context` directly
            // (rule 2 guarantees that's side-effect-free), so there's nothing left for
            // `Enter` to additionally commit until a real per-entity detail view exists
            // (out of scope for this map, per issue #144).
            None => current,
        };
        self.nav.set_context(Some(next));
    }

    fn apply_view_movement(&mut self, movement: Movement, cx: &mut gpui::Context<'_, Self>) {
        let view = match self.active_view_entity(cx) {
            ActiveViewEntity::Shell(view) => view,
            view => {
                if !self.apply_settings_data_page_movement(movement, cx) {
                    view.apply_movement(movement, cx);
                }
                return;
            }
        };
        match view {
            ShellView::Dashboard | ShellView::Placeholder(_) => self.scroll_view(movement),
        }
    }

    /// Movement on a Settings page with Ledger data, which `Shell` still drives until the page
    /// moves into the Settings View. `false` when no such page has focus.
    fn apply_settings_data_page_movement(
        &mut self,
        movement: Movement,
        cx: &mut gpui::Context<'_, Self>,
    ) -> bool {
        let focused = |section| self.settings_data_page_has_focus(section, cx);
        let section = [
            SettingsPage::Categories,
            SettingsPage::Tags,
            SettingsPage::Documents,
            SettingsPage::Inventory,
        ]
        .into_iter()
        .find(|section| focused(*section));
        match section {
            Some(SettingsPage::Categories) => {
                self.apply_settings_categories_movement(movement, cx);
            }
            Some(SettingsPage::Tags) => self.apply_settings_tags_movement(movement, cx),
            Some(SettingsPage::Documents) => {
                self.apply_settings_documents_movement(movement, cx);
            }
            Some(SettingsPage::Inventory) => {
                self.apply_settings_inventory_movement(movement, cx);
            }
            _ => return false,
        }
        true
    }
}
