//! The Units page's default keys, built on `lib_viper`'s [`Keymap`]. The commands every module
//! shares keep their flat names from `lib_viper::keymap`; those only Units has are namespaced here
//! (`units.set_base`).

use lib_config::KeyBindingConfig;
use lib_list_cursor::Step;
use lib_viper::keymap::{
    ARROW_DOWN, ARROW_UP, Binding, DELETE, EDIT, END, HALF_PAGE_DOWN, HALF_PAGE_UP, HOME, Keymap,
    MOVE_DOWN, MOVE_FIRST, MOVE_LAST, MOVE_UP, NEW, PAGE_DOWN, PAGE_UP, REACTIVATE,
};

use crate::msg;

use crate::presenter::{UnitPricesEvent, UnitsPageEvent};

pub const TEST_SOURCE: &str = "units.test_source";
pub const NEXT_TABLE: &str = "units.next_table";
pub const SET_BASE: &str = "units.set_base";
pub const SET_PRICE: &str = "units.set_price";
pub const OPEN: &str = "units.open";
pub const EARLIER: &str = "units.earlier";
pub const LATER: &str = "units.later";

/// The Units page's commands, with today's keys. The arrow, Home/End and Page keys move as `j`/`k`,
/// `g g`/`G` and `ctrl+d`/`ctrl+u` do, each under its own name so an override can move it. `units.set_base` is new: setting the base Unit
/// was only the General page's select. Its `B` is capital because `b` is the Desktop's rail toggle,
/// and a module's default must not be a Client's global key (each bin's clash test checks).
pub const UNITS_PAGE_BINDINGS: &[Binding<UnitsPageEvent>] = &[
    Binding::hidden(MOVE_DOWN, "j", UnitsPageEvent::Move(Step::Next)),
    Binding::hidden(MOVE_UP, "k", UnitsPageEvent::Move(Step::Prev)),
    Binding::hidden(MOVE_FIRST, "g g", UnitsPageEvent::Move(Step::First)),
    Binding::hidden(MOVE_LAST, "G", UnitsPageEvent::Move(Step::Last)),
    Binding::hidden(
        HALF_PAGE_DOWN,
        "ctrl+d",
        UnitsPageEvent::Move(Step::HalfPageDown),
    ),
    Binding::hidden(
        HALF_PAGE_UP,
        "ctrl+u",
        UnitsPageEvent::Move(Step::HalfPageUp),
    ),
    Binding::hidden(ARROW_DOWN, "down", UnitsPageEvent::Move(Step::Next)),
    Binding::hidden(ARROW_UP, "up", UnitsPageEvent::Move(Step::Prev)),
    Binding::hidden(HOME, "home", UnitsPageEvent::Move(Step::First)),
    Binding::hidden(END, "end", UnitsPageEvent::Move(Step::Last)),
    Binding::hidden(
        PAGE_DOWN,
        "pagedown",
        UnitsPageEvent::Move(Step::HalfPageDown),
    ),
    Binding::hidden(PAGE_UP, "pageup", UnitsPageEvent::Move(Step::HalfPageUp)),
    Binding::listed(
        NEW,
        "n",
        UnitsPageEvent::New,
        "units new",
        msg::units_command_new,
    ),
    Binding::listed(
        EDIT,
        "e",
        UnitsPageEvent::Edit,
        "units edit",
        msg::units_command_edit,
    ),
    Binding::listed(
        DELETE,
        "d",
        UnitsPageEvent::Delete,
        "units delete",
        msg::units_command_delete,
    ),
    Binding::listed(
        REACTIVATE,
        "r",
        UnitsPageEvent::Reactivate,
        "units reactivate",
        msg::units_command_reactivate,
    ),
    Binding::listed(
        TEST_SOURCE,
        "t",
        UnitsPageEvent::Test,
        "units test-source",
        msg::units_command_test_source,
    ),
    Binding::listed(
        NEXT_TABLE,
        "tab",
        UnitsPageEvent::NextTable,
        "units next-table",
        msg::units_command_next_table,
    ),
    Binding::listed(
        SET_BASE,
        "B",
        UnitsPageEvent::SetBase,
        "units set-base",
        msg::units_command_set_base,
    ),
    Binding::listed(
        SET_PRICE,
        "p",
        UnitsPageEvent::SetPrice,
        "units set-price",
        msg::units_command_set_price,
    ),
    Binding::hidden(OPEN, "enter", UnitsPageEvent::Open),
];

/// The Units page's keymap under `config`'s overrides.
pub fn units_page_keymap(config: &KeyBindingConfig) -> Keymap<UnitsPageEvent> {
    Keymap::new(UNITS_PAGE_BINDINGS, config)
}

/// The Unit prices view's commands. The moves walk the weeks as the page's walk its rows, and
/// `units.set_price` is the page's own command, so one override moves both. `back` is not here:
/// `esc` is a key the Shell keeps, which each Client hands the view as [`UnitPricesEvent::Back`].
pub const UNIT_PRICES_BINDINGS: &[Binding<UnitPricesEvent>] = &[
    Binding::hidden(MOVE_DOWN, "j", UnitPricesEvent::Move(Step::Next)),
    Binding::hidden(MOVE_UP, "k", UnitPricesEvent::Move(Step::Prev)),
    Binding::hidden(MOVE_FIRST, "g g", UnitPricesEvent::Move(Step::First)),
    Binding::hidden(MOVE_LAST, "G", UnitPricesEvent::Move(Step::Last)),
    Binding::hidden(
        HALF_PAGE_DOWN,
        "ctrl+d",
        UnitPricesEvent::Move(Step::HalfPageDown),
    ),
    Binding::hidden(
        HALF_PAGE_UP,
        "ctrl+u",
        UnitPricesEvent::Move(Step::HalfPageUp),
    ),
    Binding::hidden(ARROW_DOWN, "down", UnitPricesEvent::Move(Step::Next)),
    Binding::hidden(ARROW_UP, "up", UnitPricesEvent::Move(Step::Prev)),
    Binding::hidden(HOME, "home", UnitPricesEvent::Move(Step::First)),
    Binding::hidden(END, "end", UnitPricesEvent::Move(Step::Last)),
    Binding::hidden(
        PAGE_DOWN,
        "pagedown",
        UnitPricesEvent::Move(Step::HalfPageDown),
    ),
    Binding::hidden(PAGE_UP, "pageup", UnitPricesEvent::Move(Step::HalfPageUp)),
    Binding::hidden(EARLIER, "[", UnitPricesEvent::Earlier),
    Binding::hidden(LATER, "]", UnitPricesEvent::Later),
    Binding::hidden(SET_PRICE, "p", UnitPricesEvent::SetPrice),
];

/// The Unit prices view's keymap under `config`'s overrides.
pub fn unit_prices_keymap(config: &KeyBindingConfig) -> Keymap<UnitPricesEvent> {
    Keymap::new(UNIT_PRICES_BINDINGS, config)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn overriding(command: &str, chord: &str) -> KeyBindingConfig {
        let mut config = KeyBindingConfig::default();
        config
            .bindings
            .insert(command.to_string(), chord.to_string());
        config
    }

    #[test]
    fn a_default_chord_reaches_its_event_for_a_shared_and_a_namespaced_command() {
        let keymap = units_page_keymap(&KeyBindingConfig::default());

        assert_eq!(keymap.event("d"), Some(UnitsPageEvent::Delete));
        assert_eq!(keymap.event("B"), Some(UnitsPageEvent::SetBase));
        assert_eq!(keymap.event("p"), Some(UnitsPageEvent::SetPrice));
        assert_eq!(keymap.event("g g"), Some(UnitsPageEvent::Move(Step::First)));
        assert_eq!(keymap.event("down"), Some(UnitsPageEvent::Move(Step::Next)));
        assert_eq!(keymap.event("end"), Some(UnitsPageEvent::Move(Step::Last)));
        assert_eq!(
            keymap.event("pageup"),
            Some(UnitsPageEvent::Move(Step::HalfPageUp))
        );
        assert_eq!(keymap.event("enter"), Some(UnitsPageEvent::Open));
        assert_eq!(keymap.event("q"), None);
    }

    #[test]
    fn the_unit_prices_keys_step_the_window_and_share_set_price_with_the_page() {
        let keymap = unit_prices_keymap(&KeyBindingConfig::default());
        assert_eq!(keymap.event("["), Some(UnitPricesEvent::Earlier));
        assert_eq!(keymap.event("]"), Some(UnitPricesEvent::Later));
        assert_eq!(keymap.event("j"), Some(UnitPricesEvent::Move(Step::Next)));

        let config = overriding("units.set_price", "P");
        assert_eq!(
            unit_prices_keymap(&config).event("P"),
            Some(UnitPricesEvent::SetPrice)
        );
        assert_eq!(
            units_page_keymap(&config).event("P"),
            Some(UnitsPageEvent::SetPrice)
        );
    }

    #[test]
    fn a_keybindings_override_of_a_shared_name_moves_its_event_to_the_new_chord() {
        let keymap = units_page_keymap(&overriding("delete", "ctrl+x"));

        assert_eq!(keymap.event("ctrl+x"), Some(UnitsPageEvent::Delete));
        assert_eq!(keymap.event("d"), None, "the default no longer binds");
        assert_eq!(keymap.chord(DELETE), Some("ctrl+x"));
    }

    #[test]
    fn a_keybindings_override_of_a_namespaced_name_moves_only_that_command() {
        let keymap = units_page_keymap(&overriding("units.set_base", "S"));

        assert_eq!(keymap.event("S"), Some(UnitsPageEvent::SetBase));
        assert_eq!(keymap.event("B"), None);
        // `set_base` alone names no Units command, so it can't move this one.
        let flat = units_page_keymap(&overriding("set_base", "x"));
        assert_eq!(flat.event("B"), Some(UnitsPageEvent::SetBase));
    }
}
