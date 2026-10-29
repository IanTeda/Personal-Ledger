//! The Units page's default keys, built on `lib_viper`'s [`Keymap`]. The commands every module
//! shares keep their flat names from `lib_viper::keymap`; those only Units has are namespaced here
//! (`units.set_base`).

use lib_config::KeyBindingConfig;
use lib_list_cursor::Step;
use lib_viper::keymap::{
    ARROW_DOWN, ARROW_UP, Binding, DELETE, EDIT, END, HALF_PAGE_DOWN, HALF_PAGE_UP, HOME, Keymap,
    MOVE_DOWN, MOVE_FIRST, MOVE_LAST, MOVE_UP, NEW, PAGE_DOWN, PAGE_UP, REACTIVATE,
};

use crate::presenter::UnitsPageEvent;

pub const TEST_SOURCE: &str = "units.test_source";
pub const NEXT_TABLE: &str = "units.next_table";
pub const SET_BASE: &str = "units.set_base";
pub const SET_PRICE: &str = "units.set_price";

/// The Units page's commands, with today's keys. The arrow, Home/End and Page keys move as `j`/`k`,
/// `g g`/`G` and `ctrl+d`/`ctrl+u` do, each under its own name so an override can move it. `units.set_base` is new: setting the base Unit
/// was only the General page's select. Its `B` is capital because `b` is the Desktop's rail toggle,
/// and a module's default must not be a Client's global key (each bin's clash test checks).
pub const UNITS_PAGE_BINDINGS: &[Binding<UnitsPageEvent>] = &[
    Binding {
        command: MOVE_DOWN,
        chord: "j",
        event: UnitsPageEvent::Move(Step::Next),
    },
    Binding {
        command: MOVE_UP,
        chord: "k",
        event: UnitsPageEvent::Move(Step::Prev),
    },
    Binding {
        command: MOVE_FIRST,
        chord: "g g",
        event: UnitsPageEvent::Move(Step::First),
    },
    Binding {
        command: MOVE_LAST,
        chord: "G",
        event: UnitsPageEvent::Move(Step::Last),
    },
    Binding {
        command: HALF_PAGE_DOWN,
        chord: "ctrl+d",
        event: UnitsPageEvent::Move(Step::HalfPageDown),
    },
    Binding {
        command: HALF_PAGE_UP,
        chord: "ctrl+u",
        event: UnitsPageEvent::Move(Step::HalfPageUp),
    },
    Binding {
        command: ARROW_DOWN,
        chord: "down",
        event: UnitsPageEvent::Move(Step::Next),
    },
    Binding {
        command: ARROW_UP,
        chord: "up",
        event: UnitsPageEvent::Move(Step::Prev),
    },
    Binding {
        command: HOME,
        chord: "home",
        event: UnitsPageEvent::Move(Step::First),
    },
    Binding {
        command: END,
        chord: "end",
        event: UnitsPageEvent::Move(Step::Last),
    },
    Binding {
        command: PAGE_DOWN,
        chord: "pagedown",
        event: UnitsPageEvent::Move(Step::HalfPageDown),
    },
    Binding {
        command: PAGE_UP,
        chord: "pageup",
        event: UnitsPageEvent::Move(Step::HalfPageUp),
    },
    Binding {
        command: NEW,
        chord: "n",
        event: UnitsPageEvent::New,
    },
    Binding {
        command: EDIT,
        chord: "e",
        event: UnitsPageEvent::Edit,
    },
    Binding {
        command: DELETE,
        chord: "d",
        event: UnitsPageEvent::Delete,
    },
    Binding {
        command: REACTIVATE,
        chord: "r",
        event: UnitsPageEvent::Reactivate,
    },
    Binding {
        command: TEST_SOURCE,
        chord: "t",
        event: UnitsPageEvent::Test,
    },
    Binding {
        command: NEXT_TABLE,
        chord: "tab",
        event: UnitsPageEvent::NextTable,
    },
    Binding {
        command: SET_BASE,
        chord: "B",
        event: UnitsPageEvent::SetBase,
    },
    Binding {
        command: SET_PRICE,
        chord: "p",
        event: UnitsPageEvent::SetPrice,
    },
];

/// The Units page's keymap under `config`'s overrides.
pub fn units_page_keymap(config: &KeyBindingConfig) -> Keymap<UnitsPageEvent> {
    Keymap::new(UNITS_PAGE_BINDINGS, config)
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
        assert_eq!(keymap.event("q"), None);
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
