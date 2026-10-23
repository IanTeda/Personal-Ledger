//! A module's keys: each command name and its default chord, mapped to the event it sends.
//! `[keybindings]` overrides a key by command name ([`KeyBindingConfig`]). A name other modules
//! share stays flat (`new`, `delete`, `move_down`, the constants here), so one override applies
//! everywhere; a command only one module has is namespaced by it (`units.set_base`) and defined in
//! its crate.
//!
//! A chord is text as `[keybindings]` writes it: modifiers joined to the key with `+`
//! (`ctrl+d`), and a sequence of keys separated by a space (`g g`). Each Client turns its toolkit's
//! key into that text before looking it up, so both share one keymap.

use lib_config::KeyBindingConfig;

/// One command: its name in `[keybindings]`, its chord when nothing overrides it, and its event. An
/// empty chord leaves the command unbound until `[keybindings]` sets one.
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
        if chord.is_empty() {
            return None;
        }
        self.bindings
            .iter()
            .find(|(bound, ..)| bound == chord)
            .map(|(.., event)| event.clone())
    }

    /// Whether `chord` begins a bound sequence (the `g` of `g g`), so the Client waits for the
    /// next key before looking the whole sequence up.
    pub fn is_prefix(&self, chord: &str) -> bool {
        self.bindings.iter().any(|(bound, ..)| {
            bound
                .strip_prefix(chord)
                .is_some_and(|rest| rest.starts_with(' '))
        })
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
/// The arrow, Home/End and Page keys' commands: each moves as the `move_*` command beside it
/// does, under a name of its own, since `[keybindings]` gives a command one chord and its defaults
/// already bind `move_down` and `move_up` to `j` and `k`.
pub const ARROW_DOWN: &str = "arrow_down";
pub const ARROW_UP: &str = "arrow_up";
pub const HOME: &str = "home";
pub const END: &str = "end";
pub const PAGE_DOWN: &str = "page_down";
pub const PAGE_UP: &str = "page_up";
pub const NEW: &str = "new";
pub const EDIT: &str = "edit";
pub const DELETE: &str = "delete";
pub const REACTIVATE: &str = "reactivate";

/// The Dialog keymap's commands (`lib_shell::dialog`), flat so one override applies in every
/// Dialog. `select` and `back` are the names Views already back out and choose by. Not `confirm`
/// and `cancel`: `[keybindings]` binds those to the TUI's `y` and `x`, so one override would change
/// both meanings.
pub const SELECT: &str = "select";
pub const BACK: &str = "back";
pub const NEXT_FIELD: &str = "next_field";
pub const PREVIOUS_FIELD: &str = "previous_field";
pub const SELECT_NEXT: &str = "select_next";
pub const SELECT_PREVIOUS: &str = "select_previous";
pub const SUBMIT: &str = "submit";

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum Event {
        New,
        Delete,
    }

    const BINDINGS: &[Binding<Event>] = &[
        Binding {
            command: NEW,
            chord: "n",
            event: Event::New,
        },
        Binding {
            command: DELETE,
            chord: "d",
            event: Event::Delete,
        },
    ];

    #[test]
    fn a_default_chord_reaches_its_event_and_names_its_command() {
        let keymap = Keymap::new(BINDINGS, &KeyBindingConfig::default());

        assert_eq!(keymap.event("n"), Some(Event::New));
        assert_eq!(keymap.event("q"), None);
        assert_eq!(keymap.chord(DELETE), Some("d"));
        assert_eq!(keymap.chord("unbound"), None);
    }

    #[test]
    fn a_sequences_first_key_is_a_prefix_and_a_whole_chord_is_not() {
        // `edit`, since the `[keybindings]` defaults bind `new` to `n`.
        let bindings = [Binding {
            command: EDIT,
            chord: "g n",
            event: Event::New,
        }];
        let keymap = Keymap::new(&bindings, &KeyBindingConfig::default());

        assert!(keymap.is_prefix("g"));
        assert!(!keymap.is_prefix("g n"), "a whole sequence is no prefix");
        assert!(!keymap.is_prefix("n"));
        assert!(
            !Keymap::new(BINDINGS, &KeyBindingConfig::default()).is_prefix("d"),
            "a single-key chord is no prefix"
        );
    }

    #[test]
    fn an_override_moves_only_its_command_to_the_new_chord() {
        let mut config = KeyBindingConfig::default();
        config
            .bindings
            .insert(DELETE.to_string(), "ctrl+x".to_string());
        let keymap = Keymap::new(BINDINGS, &config);

        assert_eq!(keymap.event("ctrl+x"), Some(Event::Delete));
        assert_eq!(keymap.event("d"), None, "the default no longer binds");
        assert_eq!(keymap.event("n"), Some(Event::New));
    }
}
