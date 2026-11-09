//! The Dashboard's default keys, built on `lib_viper`'s [`Keymap`]. The page has no list, so each
//! movement is a scroll request the Client applies to the page's own scroll. They keep the flat
//! names every module shares, so one `[keybindings]` override moves the Dashboard's scroll with the
//! other pages' rows.

use lib_list_cursor::Step;
use lib_viper::keymap::{
    ARROW_DOWN, ARROW_UP, Binding, END, HALF_PAGE_DOWN, HALF_PAGE_UP, HOME, KeyBindingConfig,
    Keymap, MOVE_DOWN, MOVE_FIRST, MOVE_LAST, MOVE_UP, PAGE_DOWN, PAGE_UP, SEARCH,
};

use crate::presenter::DashboardEvent;

const fn scrolls(
    command: &'static str,
    chord: &'static str,
    step: Step,
) -> Binding<DashboardEvent> {
    Binding::hidden(command, chord, DashboardEvent::Scroll(step))
}

/// The Dashboard's commands, with today's keys: the moves the Shell would otherwise fall back on,
/// and `search`, which the Shell's `/` sends by name, so its chord is left empty here.
pub const DASHBOARD_BINDINGS: &[Binding<DashboardEvent>] = &[
    scrolls(MOVE_DOWN, "j", Step::Next),
    scrolls(MOVE_UP, "k", Step::Prev),
    scrolls(MOVE_FIRST, "g g", Step::First),
    scrolls(MOVE_LAST, "G", Step::Last),
    scrolls(HALF_PAGE_DOWN, "ctrl+d", Step::HalfPageDown),
    scrolls(HALF_PAGE_UP, "ctrl+u", Step::HalfPageUp),
    scrolls(ARROW_DOWN, "down", Step::Next),
    scrolls(ARROW_UP, "up", Step::Prev),
    scrolls(HOME, "home", Step::First),
    scrolls(END, "end", Step::Last),
    scrolls(PAGE_DOWN, "pagedown", Step::HalfPageDown),
    scrolls(PAGE_UP, "pageup", Step::HalfPageUp),
    Binding::hidden(SEARCH, "", DashboardEvent::Search),
];

/// The Dashboard's keymap under `config`'s overrides.
pub fn dashboard_keymap(config: &KeyBindingConfig) -> Keymap<DashboardEvent> {
    Keymap::new(DASHBOARD_BINDINGS, config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_movement_is_a_scroll_and_search_is_reached_by_name() {
        let keymap = dashboard_keymap(&KeyBindingConfig::default());

        assert_eq!(keymap.event("j"), Some(DashboardEvent::Scroll(Step::Next)));
        assert_eq!(
            keymap.event("g g"),
            Some(DashboardEvent::Scroll(Step::First))
        );
        assert_eq!(
            keymap.event("pageup"),
            Some(DashboardEvent::Scroll(Step::HalfPageUp))
        );
        assert_eq!(keymap.event_for(SEARCH), Some(DashboardEvent::Search));
        assert_eq!(keymap.event("/"), None, "the Shell keeps `/`");
    }
}
