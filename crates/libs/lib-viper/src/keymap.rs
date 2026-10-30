//! A module's keys: each command name and its default chord, mapped to the event it sends.
//! `[keybindings]` overrides a key by command name ([`KeyBindingConfig`]). A name other modules
//! share stays flat (`new`, `delete`, `move_down`, the constants here), so one override applies
//! everywhere; a command only one module has is namespaced by it (`units.set_base`) and defined in
//! its crate.
//!
//! A chord is text as `[keybindings]` writes it: modifiers joined to the key with `+`
//! (`ctrl+d`), and a sequence of keys separated by a space (`g g`). Each Client turns its toolkit's
//! key into that text before looking it up, so both share one keymap.
//!
//! The keymap is also the command palette's table: each binding says whether the palette lists its
//! Command ([`Palette`]), so a module can't add a key and forget its palette entry.

use lib_config::KeyBindingConfig;

/// One command: its name in `[keybindings]`, its chord when nothing overrides it, its event, and
/// whether the command palette lists it. An empty chord leaves the command unbound until
/// `[keybindings]` sets one.
#[derive(Debug, Clone)]
pub struct Binding<E> {
    pub command: &'static str,
    pub chord: &'static str,
    pub event: E,
    pub palette: Palette,
}

impl<E> Binding<E> {
    /// A binding the palette doesn't list: a movement, or a key whose meaning hangs on the row.
    pub const fn hidden(command: &'static str, chord: &'static str, event: E) -> Self {
        Self {
            command,
            chord,
            event,
            palette: Palette::Hidden,
        }
    }

    /// A binding the palette lists as `name`, described by the `description` Message.
    pub const fn listed(
        command: &'static str,
        chord: &'static str,
        event: E,
        name: &'static str,
        description: fn() -> String,
    ) -> Self {
        Self {
            command,
            chord,
            event,
            palette: Palette::Listed { name, description },
        }
    }
}

/// Whether the command palette lists a binding's Command.
#[derive(Debug, Clone, Copy)]
pub enum Palette {
    /// Listed under `name`, as typed: `<domain> <verb>` in kebab case (`units set-base`), a stable
    /// English id, never a Message. `description` is the module's Message for it.
    Listed {
        name: &'static str,
        description: fn() -> String,
    },
    /// Not listed: list movement, `select`, and keys that mean something only on the row they
    /// act on. A Dialog's bindings are all hidden.
    Hidden,
}

/// One Command the palette lists, as a keymap holds it: the chord is the live one, empty while
/// unbound.
#[derive(Debug, Clone, Copy)]
pub struct Listed<'a> {
    pub command: &'static str,
    pub name: &'static str,
    pub description: fn() -> String,
    pub chord: &'a str,
}

/// A module's bindings with any overrides applied, looked up by chord.
#[derive(Debug, Clone)]
pub struct Keymap<E> {
    bindings: Vec<Bound<E>>,
}

#[derive(Debug, Clone)]
struct Bound<E> {
    chord: String,
    command: &'static str,
    event: E,
    palette: Palette,
}

impl<E: Clone> Keymap<E> {
    /// `defaults`, each chord replaced by the one `config` binds its command to.
    pub fn new(defaults: &[Binding<E>], config: &KeyBindingConfig) -> Self {
        let bindings = defaults
            .iter()
            .map(|binding| {
                let chord = config.key_for(binding.command).unwrap_or(binding.chord);
                Bound {
                    chord: chord.to_string(),
                    command: binding.command,
                    event: binding.event.clone(),
                    palette: binding.palette,
                }
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
            .find(|bound| bound.chord == chord)
            .map(|bound| bound.event.clone())
    }

    /// The event `command` sends, whatever its chord: the palette runs a Command by name, so it
    /// reaches the same event its key does, bound or not.
    pub fn event_for(&self, command: &str) -> Option<E> {
        self.bindings
            .iter()
            .find(|bound| bound.command == command)
            .map(|bound| bound.event.clone())
    }

    /// Whether `chord` begins a bound sequence (the `g` of `g g`), so the Client waits for the
    /// next key before looking the whole sequence up.
    pub fn is_prefix(&self, chord: &str) -> bool {
        self.bindings.iter().any(|bound| {
            bound
                .chord
                .strip_prefix(chord)
                .is_some_and(|rest| rest.starts_with(' '))
        })
    }

    /// The chord `command` is bound to, for the legend.
    pub fn chord(&self, command: &str) -> Option<&str> {
        self.bindings
            .iter()
            .find(|bound| bound.command == command)
            .map(|bound| bound.chord.as_str())
    }

    /// The Commands the palette lists, in binding order, each with its live chord.
    pub fn listed(&self) -> impl Iterator<Item = Listed<'_>> {
        self.bindings
            .iter()
            .filter_map(|bound| match bound.palette {
                Palette::Listed { name, description } => Some(Listed {
                    command: bound.command,
                    name,
                    description,
                    chord: &bound.chord,
                }),
                Palette::Hidden => None,
            })
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

    fn delete_description() -> String {
        "delete the selected row".to_string()
    }

    const BINDINGS: &[Binding<Event>] = &[
        Binding::hidden(NEW, "n", Event::New),
        Binding::listed(
            DELETE,
            "d",
            Event::Delete,
            "rows delete",
            delete_description,
        ),
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
        let bindings = [Binding::hidden(EDIT, "g n", Event::New)];
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

    #[test]
    fn a_command_reaches_its_event_by_name_under_any_chord() {
        let keymap = Keymap::new(BINDINGS, &KeyBindingConfig::default());
        assert_eq!(keymap.event_for(DELETE), Some(Event::Delete));
        assert_eq!(keymap.event_for("unbound"), None);

        let mut config = KeyBindingConfig::default();
        config
            .bindings
            .insert(DELETE.to_string(), "ctrl+x".to_string());
        assert_eq!(
            Keymap::new(BINDINGS, &config).event_for(DELETE),
            Some(Event::Delete),
            "an override moves the key, not the name"
        );
    }

    #[test]
    fn only_listed_commands_are_listed_each_with_its_live_chord() {
        let mut config = KeyBindingConfig::default();
        config
            .bindings
            .insert(DELETE.to_string(), "ctrl+x".to_string());
        let keymap = Keymap::new(BINDINGS, &config);

        let listed: Vec<_> = keymap
            .listed()
            .map(|listed| (listed.command, listed.name, listed.chord))
            .collect();
        assert_eq!(listed, [(DELETE, "rows delete", "ctrl+x")]);
    }
}
