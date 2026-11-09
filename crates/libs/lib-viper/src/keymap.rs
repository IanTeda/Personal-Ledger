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

/// Re-exported so a module builds its keymap from `[keybindings]` without depending on
/// `lib-config` itself, which `lib-settings` may not.
pub use lib_config::KeyBindingConfig;

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

    /// Every event `chord` sends, in binding order: a module whose tabs each bind one chord to an
    /// event of their own (Budgets' `x`) takes the one for the tab shown.
    pub fn events(&self, chord: &str) -> impl Iterator<Item = E> + '_ {
        let chord = chord.to_string();
        self.bindings
            .iter()
            .filter(move |bound| !chord.is_empty() && bound.chord == chord)
            .map(|bound| bound.event.clone())
    }

    /// Every event `command` sends, in binding order, for a command several tabs bind, each to its
    /// own event.
    pub fn events_for<'a>(&'a self, command: &'a str) -> impl Iterator<Item = E> + 'a {
        self.bindings
            .iter()
            .filter(move |bound| bound.command == command)
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

    /// This keymap for one state of its page (a tab, an open select): a chord reaches only the
    /// events `own` keeps, while the palette still finds every command by name, the state's own
    /// binding of it first. The other bindings keep their command but lose their chord, so a key
    /// another state binds falls through rather than acting where it does nothing.
    pub fn scoped(&self, own: impl Fn(&E) -> bool) -> Self {
        let (kept, others): (Vec<_>, Vec<_>) = self
            .bindings
            .iter()
            .cloned()
            .partition(|bound| own(&bound.event));
        Self {
            bindings: kept
                .into_iter()
                .chain(others.into_iter().map(|bound| Bound {
                    chord: String::new(),
                    ..bound
                }))
                .collect(),
        }
    }

    /// This keymap with each event wrapped by `wrap`, chords and palette entries kept: a page
    /// Presenter that holds a view of its own (Units' prices view) takes that view's keys as page
    /// events.
    pub fn map<F>(&self, wrap: impl Fn(E) -> F) -> Keymap<F> {
        Keymap {
            bindings: self
                .bindings
                .iter()
                .cloned()
                .map(|bound| Bound {
                    chord: bound.chord,
                    command: bound.command,
                    event: wrap(bound.event),
                    palette: bound.palette,
                })
                .collect(),
        }
    }

    /// This keymap's bindings, then `other`'s: a chord or a command bound in both reaches this
    /// keymap's event.
    #[must_use]
    pub fn chain(mut self, other: Self) -> Self {
        self.bindings.extend(other.bindings);
        self
    }
}

pub const MOVE_DOWN: &str = "move_down";
pub const MOVE_UP: &str = "move_up";
pub const MOVE_FIRST: &str = "move_first";
pub const MOVE_LAST: &str = "move_last";
/// The Shell's focus `tab`, which it falls back on: a module binds its own `tab` under this name
/// only where the key goes on to the Shell after it (the Colour Theme grid's), so one override moves
/// both.
pub const FOCUS_NEXT: &str = "focus_next";
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
/// A page's Search: `/` sends the event its page binds to this name, which opens the page's own
/// Search mode (`PageMode::Search`).
pub const SEARCH: &str = "search";

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
/// A list Dialog's own row step (the Pay Dialog's Match list, the Budgets lists), on letters by
/// default: only a Dialog stepping a list right then reads them, so in any other they type.
/// Their own names, not `move_down` and `move_up`, so rebinding a page's rows leaves a Dialog's
/// alone.
pub const LIST_NEXT: &str = "list_next";
pub const LIST_PREVIOUS: &str = "list_previous";

/// The list step's two chords as a hint shows them, `j/k` by default, read from a Dialog `keymap`.
pub fn list_step_chords<E: Clone>(keymap: &Keymap<E>) -> String {
    format!(
        "{}/{}",
        keymap.chord(LIST_NEXT).unwrap_or_default(),
        keymap.chord(LIST_PREVIOUS).unwrap_or_default()
    )
}

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
    fn a_chord_or_command_bound_twice_reaches_each_event_in_binding_order() {
        let bindings = [
            Binding::hidden("tabs.act", "x", Event::New),
            Binding::hidden("tabs.act", "x", Event::Delete),
            Binding::hidden(NEW, "n", Event::New),
        ];
        let keymap = Keymap::new(&bindings, &KeyBindingConfig::default());

        assert_eq!(
            keymap.events("x").collect::<Vec<_>>(),
            [Event::New, Event::Delete]
        );
        assert_eq!(
            keymap.events_for("tabs.act").collect::<Vec<_>>(),
            [Event::New, Event::Delete]
        );
        assert_eq!(keymap.events("").count(), 0);
        let mut config = KeyBindingConfig::default();
        config
            .bindings
            .insert("tabs.act".to_string(), "ctrl+x".to_string());
        let moved = Keymap::new(&bindings, &config);
        assert_eq!(moved.events("ctrl+x").count(), 2, "one override moves both");
        assert_eq!(moved.events("x").count(), 0);
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

    #[test]
    fn a_scoped_keymap_reaches_only_its_own_events_by_chord_but_every_command_by_name() {
        let keymap = Keymap::new(BINDINGS, &KeyBindingConfig::default());
        let scoped = keymap.scoped(|event| *event == Event::New);

        assert_eq!(scoped.event("n"), Some(Event::New));
        assert_eq!(
            scoped.event("d"),
            None,
            "another state's chord falls through"
        );
        assert_eq!(
            scoped.event_for(DELETE),
            Some(Event::Delete),
            "the palette still runs it by name"
        );
        assert_eq!(scoped.event(""), None);
        assert_eq!(keymap.scoped(|_| false).event("n"), None);
    }

    #[test]
    fn a_mapped_keymap_wraps_each_event_and_a_chained_one_looks_up_the_first_keymap_first() {
        let keymap = Keymap::new(BINDINGS, &KeyBindingConfig::default());
        let wrapped = keymap.map(Some);

        assert_eq!(wrapped.event("d"), Some(Some(Event::Delete)));
        assert_eq!(wrapped.listed().count(), 1, "the palette entry is kept");

        // `toy.new`, a name `[keybindings]`' defaults leave alone, beside the first keymap's `new`.
        let others = Keymap::new(
            &[
                Binding::hidden(NEW, "", None),
                Binding::hidden("toy.new", "x", None),
            ],
            &KeyBindingConfig::default(),
        );
        let chained = wrapped.chain(others);
        assert_eq!(chained.event_for(NEW), Some(Some(Event::New)));
        assert_eq!(
            chained.event("x"),
            Some(None),
            "the second keymap's chords still bind"
        );
    }
}
