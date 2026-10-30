//! The Tags page's default keys, built on `lib_viper`'s [`Keymap`]. Movement, `new` and `edit` are
//! the flat names every module shares (`lib_viper::keymap`). Remove and merge are the module's own:
//! the flat `delete` defaults to `d` in `[keybindings]`, and the page has always removed on `x`.

use lib_config::KeyBindingConfig;
use lib_list_cursor::Step;
use lib_viper::keymap::{
    ARROW_DOWN, ARROW_UP, Binding, EDIT, END, HALF_PAGE_DOWN, HALF_PAGE_UP, HOME, Keymap,
    MOVE_DOWN, MOVE_FIRST, MOVE_LAST, MOVE_UP, NEW, PAGE_DOWN, PAGE_UP, SELECT,
};

use crate::msg;

use crate::presenter::TagsPageEvent;

/// Remove the selected Tag, opening the Remove Dialog on it.
pub const REMOVE: &str = "tags.remove";
/// Merge the selected Tag into another, opening the Merge Dialog on it.
pub const MERGE: &str = "tags.merge";

/// The Tags page's commands, with today's keys. The arrow, Home/End and Page keys move as
/// `j`/`k`, `g g`/`G` and `ctrl+d`/`ctrl+u` do, each under its own name so an override can move it.
pub const TAGS_PAGE_BINDINGS: &[Binding<TagsPageEvent>] = &[
    Binding::hidden(MOVE_DOWN, "j", TagsPageEvent::Move(Step::Next)),
    Binding::hidden(MOVE_UP, "k", TagsPageEvent::Move(Step::Prev)),
    Binding::hidden(MOVE_FIRST, "g g", TagsPageEvent::Move(Step::First)),
    Binding::hidden(MOVE_LAST, "G", TagsPageEvent::Move(Step::Last)),
    Binding::hidden(
        HALF_PAGE_DOWN,
        "ctrl+d",
        TagsPageEvent::Move(Step::HalfPageDown),
    ),
    Binding::hidden(
        HALF_PAGE_UP,
        "ctrl+u",
        TagsPageEvent::Move(Step::HalfPageUp),
    ),
    Binding::hidden(ARROW_DOWN, "down", TagsPageEvent::Move(Step::Next)),
    Binding::hidden(ARROW_UP, "up", TagsPageEvent::Move(Step::Prev)),
    Binding::hidden(HOME, "home", TagsPageEvent::Move(Step::First)),
    Binding::hidden(END, "end", TagsPageEvent::Move(Step::Last)),
    Binding::hidden(
        PAGE_DOWN,
        "pagedown",
        TagsPageEvent::Move(Step::HalfPageDown),
    ),
    Binding::hidden(PAGE_UP, "pageup", TagsPageEvent::Move(Step::HalfPageUp)),
    Binding::hidden(SELECT, "enter", TagsPageEvent::OpenTransactions),
    Binding::listed(
        NEW,
        "n",
        TagsPageEvent::New,
        "tags new",
        msg::tags_command_new,
    ),
    Binding::listed(
        EDIT,
        "e",
        TagsPageEvent::Edit,
        "tags edit",
        msg::tags_command_edit,
    ),
    Binding::listed(
        REMOVE,
        "x",
        TagsPageEvent::Remove,
        "tags remove",
        msg::tags_command_remove,
    ),
    Binding::listed(
        MERGE,
        "m",
        TagsPageEvent::Merge,
        "tags merge",
        msg::tags_command_merge,
    ),
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
