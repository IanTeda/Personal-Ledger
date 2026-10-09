//! Focus-zone movement and the view-scroll reset of `Shell`. An `impl Shell` block for the
//! concern, not the focus model itself (`navigation::nav`).

use super::Shell;
use crate::{
    chrome::rail as chrome_rail,
    navigation::active_view::ActiveView,
    navigation::key_router::Movement,
    navigation::nav::FocusZone,
    settings::{SettingsFocus, SettingsSection},
    view::documents::DocumentsFocus,
};

/// The primary rail has a fixed 10-row list, not a real "page" of variable-height content --
/// half of that is a reasonable stand-in for `Ctrl-d`/`Ctrl-u` there.
const PRIMARY_RAIL_HALF_PAGE: usize = 5;

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
    /// fresh noun starts scrolled to the top. Also puts
    /// Settings' focus back on the index (`g s` "lands on the index"); the page on show is the
    /// last-visited one and stays.
    pub(super) fn reset_view_scroll(&mut self, cx: &mut gpui::App) {
        self.view_scroll_handle.set_offset(gpui::Point::default());
        self.settings_focus = SettingsFocus::default();
        self.colour_theme_focus = None;
        // `g f` and the palette land on the Documents list, unlike Settings' index.
        self.edit_documents_state(cx, |state| state.focus = DocumentsFocus::List);
    }

    fn apply_primary_rail_movement(&mut self, movement: Movement) {
        match movement {
            Movement::Next => self.nav.move_primary_highlight_next(),
            Movement::Prev => self.nav.move_primary_highlight_prev(),
            Movement::First => self.nav.move_primary_highlight_first(),
            Movement::Last => self.nav.move_primary_highlight_last(),
            Movement::HalfPageDown => {
                for _ in 0..PRIMARY_RAIL_HALF_PAGE {
                    self.nav.move_primary_highlight_next();
                }
            }
            Movement::HalfPageUp => {
                for _ in 0..PRIMARY_RAIL_HALF_PAGE {
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
        let next = match movement {
            Movement::Next => (current + 1).min(count - 1),
            Movement::Prev => current.saturating_sub(1),
            Movement::First => 0,
            Movement::Last => count - 1,
            Movement::HalfPageDown => (current + half_page).min(count - 1),
            Movement::HalfPageUp => current.saturating_sub(half_page),
            // The handoff's own "Movement" bullet: `Enter` "selects the entity and leaves
            // focus where it is" -- but movement here already updates `context` directly
            // (rule 2 guarantees that's side-effect-free), so there's nothing left for
            // `Enter` to additionally commit until a real per-entity detail view exists
            // (out of scope for this map, per issue #144).
            Movement::Enter => current,
        };
        self.nav.set_context(Some(next));
    }

    fn apply_view_movement(&mut self, movement: Movement, cx: &mut gpui::Context<'_, Self>) {
        let page_focused =
            self.nav.focus() == FocusZone::View && self.settings_focus == SettingsFocus::Page;
        match self.active_view() {
            ActiveView::Settings(SettingsSection::Accounts) if page_focused => {
                self.apply_accounts_movement(movement, cx);
            }
            ActiveView::Settings(SettingsSection::Categories) if page_focused => {
                self.apply_settings_categories_movement(movement);
            }
            ActiveView::Settings(SettingsSection::Tags) if page_focused => {
                self.apply_settings_tags_movement(movement);
            }
            ActiveView::Settings(SettingsSection::Payees) if page_focused => {
                self.apply_settings_payees_movement(movement);
            }
            ActiveView::Settings(SettingsSection::Documents) if page_focused => {
                self.apply_settings_documents_movement(movement, cx);
            }
            ActiveView::Settings(SettingsSection::Inventory) if page_focused => {
                self.apply_settings_inventory_movement(movement);
            }
            ActiveView::Settings(_) => {
                if self.settings_focus == SettingsFocus::Index
                    && matches!(
                        movement,
                        Movement::Next | Movement::Prev | Movement::First | Movement::Last
                    )
                {
                    self.apply_settings_section_movement(movement);
                } else {
                    self.scroll_view(movement);
                }
            }
            // The import step keeps the Transactions movement, as it did before `ActiveView`.
            ActiveView::Transactions | ActiveView::Import => {
                self.apply_transactions_movement(movement, cx);
            }
            ActiveView::Documents => self.apply_documents_movement(movement, cx),
            ActiveView::Bills => self.apply_bills_movement(movement, cx),
            ActiveView::Budgets => self.apply_budgets_movement(movement, cx),
            ActiveView::Dashboard | ActiveView::Placeholder(_) => self.scroll_view(movement),
        }
    }
}
