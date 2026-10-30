//! The Payees page's default keys, built on `lib_viper`'s [`Keymap`]. Every command is one every
//! module shares, under its flat name from `lib_viper::keymap`; Payees has none of its own.

use lib_config::KeyBindingConfig;
use lib_list_cursor::Step;
use lib_viper::keymap::{
    ARROW_DOWN, ARROW_UP, Binding, DELETE, EDIT, END, HALF_PAGE_DOWN, HALF_PAGE_UP, HOME, Keymap,
    MOVE_DOWN, MOVE_FIRST, MOVE_LAST, MOVE_UP, NEW, PAGE_DOWN, PAGE_UP, SELECT,
};

use crate::presenter::PayeesPageEvent;

/// The Payees page's commands, with today's keys. The arrow, Home/End and Page keys move as
/// `j`/`k`, `g g`/`G` and `ctrl+d`/`ctrl+u` do, each under its own name so an override can move it.
pub const PAYEES_PAGE_BINDINGS: &[Binding<PayeesPageEvent>] = &[
    Binding {
        command: MOVE_DOWN,
        chord: "j",
        event: PayeesPageEvent::Move(Step::Next),
    },
    Binding {
        command: MOVE_UP,
        chord: "k",
        event: PayeesPageEvent::Move(Step::Prev),
    },
    Binding {
        command: MOVE_FIRST,
        chord: "g g",
        event: PayeesPageEvent::Move(Step::First),
    },
    Binding {
        command: MOVE_LAST,
        chord: "G",
        event: PayeesPageEvent::Move(Step::Last),
    },
    Binding {
        command: HALF_PAGE_DOWN,
        chord: "ctrl+d",
        event: PayeesPageEvent::Move(Step::HalfPageDown),
    },
    Binding {
        command: HALF_PAGE_UP,
        chord: "ctrl+u",
        event: PayeesPageEvent::Move(Step::HalfPageUp),
    },
    Binding {
        command: ARROW_DOWN,
        chord: "down",
        event: PayeesPageEvent::Move(Step::Next),
    },
    Binding {
        command: ARROW_UP,
        chord: "up",
        event: PayeesPageEvent::Move(Step::Prev),
    },
    Binding {
        command: HOME,
        chord: "home",
        event: PayeesPageEvent::Move(Step::First),
    },
    Binding {
        command: END,
        chord: "end",
        event: PayeesPageEvent::Move(Step::Last),
    },
    Binding {
        command: PAGE_DOWN,
        chord: "pagedown",
        event: PayeesPageEvent::Move(Step::HalfPageDown),
    },
    Binding {
        command: PAGE_UP,
        chord: "pageup",
        event: PayeesPageEvent::Move(Step::HalfPageUp),
    },
    Binding {
        command: SELECT,
        chord: "enter",
        event: PayeesPageEvent::OpenTransactions,
    },
    Binding {
        command: NEW,
        chord: "n",
        event: PayeesPageEvent::New,
    },
    Binding {
        command: EDIT,
        chord: "e",
        event: PayeesPageEvent::Edit,
    },
    Binding {
        command: DELETE,
        chord: "d",
        event: PayeesPageEvent::Delete,
    },
];

/// The Payees page's keymap under `config`'s overrides.
pub fn payees_page_keymap(config: &KeyBindingConfig) -> Keymap<PayeesPageEvent> {
    Keymap::new(PAYEES_PAGE_BINDINGS, config)
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
    fn a_default_chord_reaches_its_event() {
        let keymap = payees_page_keymap(&KeyBindingConfig::default());

        assert_eq!(keymap.event("n"), Some(PayeesPageEvent::New));
        assert_eq!(keymap.event("e"), Some(PayeesPageEvent::Edit));
        assert_eq!(keymap.event("d"), Some(PayeesPageEvent::Delete));
        assert_eq!(keymap.event("j"), Some(PayeesPageEvent::Move(Step::Next)));
        assert_eq!(
            keymap.event("g g"),
            Some(PayeesPageEvent::Move(Step::First))
        );
        assert_eq!(keymap.event("end"), Some(PayeesPageEvent::Move(Step::Last)));
        assert_eq!(
            keymap.event("r"),
            None,
            "reactivating is the Delete Dialog's"
        );
    }

    #[test]
    fn a_keybindings_override_moves_the_event_to_the_new_chord() {
        let keymap = payees_page_keymap(&overriding("new", "a"));

        assert_eq!(keymap.event("a"), Some(PayeesPageEvent::New));
        assert_eq!(keymap.event("n"), None, "the default no longer binds");
        assert_eq!(keymap.chord(NEW), Some("a"));
        // `payees.new` names no Payees command: they all keep their flat names.
        let namespaced = payees_page_keymap(&overriding("payees.new", "x"));
        assert_eq!(namespaced.event("n"), Some(PayeesPageEvent::New));
    }
}
