//! The Institutions page's default keys, built on `lib_viper`'s [`Keymap`]. Every command is one
//! every module shares, under its flat name from `lib_viper::keymap`; Institutions has none of its
//! own.

use lib_config::KeyBindingConfig;
use lib_list_cursor::Step;
use lib_viper::keymap::{
    ARROW_DOWN, ARROW_UP, Binding, DELETE, EDIT, END, HALF_PAGE_DOWN, HALF_PAGE_UP, HOME, Keymap,
    MOVE_DOWN, MOVE_FIRST, MOVE_LAST, MOVE_UP, NEW, PAGE_DOWN, PAGE_UP,
};

use crate::presenter::InstitutionsPageEvent;

/// The Institutions page's commands, with today's keys for the shared ones. The arrow, Home/End and
/// Page keys move as `j`/`k`, `g g`/`G` and `ctrl+d`/`ctrl+u` do, each under its own name so an
/// override can move it.
pub const INSTITUTIONS_PAGE_BINDINGS: &[Binding<InstitutionsPageEvent>] = &[
    Binding {
        command: MOVE_DOWN,
        chord: "j",
        event: InstitutionsPageEvent::Move(Step::Next),
    },
    Binding {
        command: MOVE_UP,
        chord: "k",
        event: InstitutionsPageEvent::Move(Step::Prev),
    },
    Binding {
        command: MOVE_FIRST,
        chord: "g g",
        event: InstitutionsPageEvent::Move(Step::First),
    },
    Binding {
        command: MOVE_LAST,
        chord: "G",
        event: InstitutionsPageEvent::Move(Step::Last),
    },
    Binding {
        command: HALF_PAGE_DOWN,
        chord: "ctrl+d",
        event: InstitutionsPageEvent::Move(Step::HalfPageDown),
    },
    Binding {
        command: HALF_PAGE_UP,
        chord: "ctrl+u",
        event: InstitutionsPageEvent::Move(Step::HalfPageUp),
    },
    Binding {
        command: ARROW_DOWN,
        chord: "down",
        event: InstitutionsPageEvent::Move(Step::Next),
    },
    Binding {
        command: ARROW_UP,
        chord: "up",
        event: InstitutionsPageEvent::Move(Step::Prev),
    },
    Binding {
        command: HOME,
        chord: "home",
        event: InstitutionsPageEvent::Move(Step::First),
    },
    Binding {
        command: END,
        chord: "end",
        event: InstitutionsPageEvent::Move(Step::Last),
    },
    Binding {
        command: PAGE_DOWN,
        chord: "pagedown",
        event: InstitutionsPageEvent::Move(Step::HalfPageDown),
    },
    Binding {
        command: PAGE_UP,
        chord: "pageup",
        event: InstitutionsPageEvent::Move(Step::HalfPageUp),
    },
    Binding {
        command: NEW,
        chord: "n",
        event: InstitutionsPageEvent::New,
    },
    Binding {
        command: EDIT,
        chord: "e",
        event: InstitutionsPageEvent::Edit,
    },
    Binding {
        command: DELETE,
        chord: "d",
        event: InstitutionsPageEvent::Delete,
    },
];

/// The Institutions page's keymap under `config`'s overrides.
pub fn institutions_page_keymap(config: &KeyBindingConfig) -> Keymap<InstitutionsPageEvent> {
    Keymap::new(INSTITUTIONS_PAGE_BINDINGS, config)
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
        let keymap = institutions_page_keymap(&KeyBindingConfig::default());

        assert_eq!(keymap.event("n"), Some(InstitutionsPageEvent::New));
        assert_eq!(keymap.event("e"), Some(InstitutionsPageEvent::Edit));
        assert_eq!(keymap.event("d"), Some(InstitutionsPageEvent::Delete));
        assert_eq!(
            keymap.event("j"),
            Some(InstitutionsPageEvent::Move(Step::Next))
        );
        assert_eq!(
            keymap.event("g g"),
            Some(InstitutionsPageEvent::Move(Step::First))
        );
        assert_eq!(
            keymap.event("end"),
            Some(InstitutionsPageEvent::Move(Step::Last))
        );
        assert_eq!(keymap.event("r"), None);
    }

    #[test]
    fn a_keybindings_override_moves_the_event_to_the_new_chord() {
        let keymap = institutions_page_keymap(&overriding("new", "a"));

        assert_eq!(keymap.event("a"), Some(InstitutionsPageEvent::New));
        assert_eq!(keymap.event("n"), None, "the default no longer binds");
        assert_eq!(keymap.chord(NEW), Some("a"));
        // `institutions.new` names no Institutions command: they all keep their flat names.
        let namespaced = institutions_page_keymap(&overriding("institutions.new", "x"));
        assert_eq!(namespaced.event("n"), Some(InstitutionsPageEvent::New));
    }
}
