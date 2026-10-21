//! The Units page's default keys: each command name and its default chord, mapped to the event it
//! sends. `[keybindings]` overrides a key by command name ([`KeyBindingConfig`]). A name other
//! modules share stays flat (`new`, `delete`, `move_down`), so one override applies everywhere; a
//! command only Units has is namespaced (`units.set_base`).
//!
//! A chord is text as `[keybindings]` writes it: modifiers joined to the key with `+`
//! (`ctrl+d`), and a sequence of keys separated by a space (`g g`). Each Client turns its toolkit's
//! key into that text before looking it up, so both share one keymap.

use lib_config::KeyBindingConfig;
use lib_list_cursor::Step;

use crate::presenter::UnitsPageEvent;

/// One command: its name in `[keybindings]`, its chord when nothing overrides it, and its event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Binding<E> {
    pub command: &'static str,
    pub chord: &'static str,
    pub event: E,
}

/// A module's bindings with any overrides applied, looked up by chord.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Keymap<E> {
    bindings: Vec<(String, &'static str, E)>,
}

impl<E: Clone> Keymap<E> {
    /// `defaults`, each chord replaced by the one `config` binds its command to.
    pub fn new(defaults: &[Binding<E>], config: &KeyBindingConfig) -> Self {
        let bindings = defaults
            .iter()
            .map(|binding| {
                let chord = config.key_for(binding.command).unwrap_or(binding.chord);
                (chord.to_string(), binding.command, binding.event.clone())
            })
            .collect();
        Self { bindings }
    }

    /// The event `chord` sends, if it is bound.
    pub fn event(&self, chord: &str) -> Option<E> {
        self.bindings
            .iter()
            .find(|(bound, ..)| bound == chord)
            .map(|(.., event)| event.clone())
    }

    /// The chord `command` is bound to, for the legend.
    pub fn chord(&self, command: &str) -> Option<&str> {
        self.bindings
            .iter()
            .find(|(_, name, _)| *name == command)
            .map(|(chord, ..)| chord.as_str())
    }
}

pub const MOVE_DOWN: &str = "move_down";
pub const MOVE_UP: &str = "move_up";
pub const MOVE_FIRST: &str = "move_first";
pub const MOVE_LAST: &str = "move_last";
pub const HALF_PAGE_DOWN: &str = "half_page_down";
pub const HALF_PAGE_UP: &str = "half_page_up";
pub const NEW: &str = "new";
pub const EDIT: &str = "edit";
pub const DELETE: &str = "delete";
pub const REACTIVATE: &str = "reactivate";
pub const TEST_SOURCE: &str = "units.test_source";
pub const NEXT_TABLE: &str = "units.next_table";
pub const SET_BASE: &str = "units.set_base";

/// The Units page's commands, with today's keys. `units.set_base` is new: setting the base Unit
/// was only the General page's select.
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
        chord: "b",
        event: UnitsPageEvent::SetBase,
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
        assert_eq!(keymap.event("b"), Some(UnitsPageEvent::SetBase));
        assert_eq!(keymap.event("g g"), Some(UnitsPageEvent::Move(Step::First)));
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
        let keymap = units_page_keymap(&overriding("units.set_base", "B"));

        assert_eq!(keymap.event("B"), Some(UnitsPageEvent::SetBase));
        assert_eq!(keymap.event("b"), None);
        // `set_base` alone names no Units command, so it can't move this one.
        let flat = units_page_keymap(&overriding("set_base", "x"));
        assert_eq!(flat.event("b"), Some(UnitsPageEvent::SetBase));
    }
}
