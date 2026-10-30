//! The Tags page's default keys, built on `lib_viper`'s [`Keymap`]. Movement, `new` and `edit` are
//! the flat names every module shares (`lib_viper::keymap`). Remove and merge are the module's own:
//! the flat `delete` defaults to `d` in `[keybindings]`, and the page has always removed on `x`.

use lib_config::KeyBindingConfig;
use lib_list_cursor::Step;
use lib_viper::keymap::{
    ARROW_DOWN, ARROW_UP, Binding, EDIT, END, HALF_PAGE_DOWN, HALF_PAGE_UP, HOME, Keymap,
    MOVE_DOWN, MOVE_FIRST, MOVE_LAST, MOVE_UP, NEW, PAGE_DOWN, PAGE_UP, SELECT,
};

use crate::presenter::TagsPageEvent;

/// Remove the selected Tag, opening the Remove Dialog on it.
pub const REMOVE: &str = "tags.remove";
/// Merge the selected Tag into another, opening the Merge Dialog on it.
pub const MERGE: &str = "tags.merge";

/// The Tags page's commands, with today's keys. The arrow, Home/End and Page keys move as
/// `j`/`k`, `g g`/`G` and `ctrl+d`/`ctrl+u` do, each under its own name so an override can move it.
pub const TAGS_PAGE_BINDINGS: &[Binding<TagsPageEvent>] = &[
    Binding {
        command: MOVE_DOWN,
        chord: "j",
        event: TagsPageEvent::Move(Step::Next),
    },
    Binding {
        command: MOVE_UP,
        chord: "k",
        event: TagsPageEvent::Move(Step::Prev),
    },
    Binding {
        command: MOVE_FIRST,
        chord: "g g",
        event: TagsPageEvent::Move(Step::First),
    },
    Binding {
        command: MOVE_LAST,
        chord: "G",
        event: TagsPageEvent::Move(Step::Last),
    },
    Binding {
        command: HALF_PAGE_DOWN,
        chord: "ctrl+d",
        event: TagsPageEvent::Move(Step::HalfPageDown),
    },
    Binding {
        command: HALF_PAGE_UP,
        chord: "ctrl+u",
        event: TagsPageEvent::Move(Step::HalfPageUp),
    },
    Binding {
        command: ARROW_DOWN,
        chord: "down",
        event: TagsPageEvent::Move(Step::Next),
    },
    Binding {
        command: ARROW_UP,
        chord: "up",
        event: TagsPageEvent::Move(Step::Prev),
    },
    Binding {
        command: HOME,
        chord: "home",
        event: TagsPageEvent::Move(Step::First),
    },
    Binding {
        command: END,
        chord: "end",
        event: TagsPageEvent::Move(Step::Last),
    },
    Binding {
        command: PAGE_DOWN,
        chord: "pagedown",
        event: TagsPageEvent::Move(Step::HalfPageDown),
    },
    Binding {
        command: PAGE_UP,
        chord: "pageup",
        event: TagsPageEvent::Move(Step::HalfPageUp),
    },
    Binding {
        command: SELECT,
        chord: "enter",
        event: TagsPageEvent::OpenTransactions,
    },
    Binding {
        command: NEW,
        chord: "n",
        event: TagsPageEvent::New,
    },
    Binding {
        command: EDIT,
        chord: "e",
        event: TagsPageEvent::Edit,
    },
    Binding {
        command: REMOVE,
        chord: "x",
        event: TagsPageEvent::Remove,
    },
    Binding {
        command: MERGE,
        chord: "m",
        event: TagsPageEvent::Merge,
    },
];

/// The Tags page's keymap under `config`'s overrides.
pub fn tags_page_keymap(config: &KeyBindingConfig) -> Keymap<TagsPageEvent> {
    Keymap::new(TAGS_PAGE_BINDINGS, config)
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
        let keymap = tags_page_keymap(&KeyBindingConfig::default());

        assert_eq!(keymap.event("n"), Some(TagsPageEvent::New));
        assert_eq!(keymap.event("e"), Some(TagsPageEvent::Edit));
        assert_eq!(keymap.event("x"), Some(TagsPageEvent::Remove));
        assert_eq!(keymap.event("m"), Some(TagsPageEvent::Merge));
        assert_eq!(keymap.event("j"), Some(TagsPageEvent::Move(Step::Next)));
        assert_eq!(keymap.event("g g"), Some(TagsPageEvent::Move(Step::First)));
    }

    #[test]
    fn a_keybindings_override_moves_the_event_to_the_new_chord() {
        let keymap = tags_page_keymap(&overriding("tags.merge", "u"));

        assert_eq!(keymap.event("u"), Some(TagsPageEvent::Merge));
        assert_eq!(keymap.event("m"), None, "the default no longer binds");
        // `new` is the flat name every module shares, so one override moves it here too.
        let flat = tags_page_keymap(&overriding("new", "a"));
        assert_eq!(flat.event("a"), Some(TagsPageEvent::New));
        assert_eq!(flat.chord(NEW), Some("a"));
    }
}
