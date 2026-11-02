//! The Document Types page's default keys, built on `lib_viper`'s [`Keymap`]. Movement, `new` and
//! `edit` are the flat names every module shares (`lib_viper::keymap`). Remove is the module's own:
//! the flat `delete` defaults to `d` in `[keybindings]`, and the page has always removed on `x`.
//! So is reordering, which only a user-ordered list has.

use lib_config::KeyBindingConfig;
use lib_list_cursor::Step;
use lib_viper::keymap::{
    ARROW_DOWN, ARROW_UP, Binding, EDIT, END, HALF_PAGE_DOWN, HALF_PAGE_UP, HOME, Keymap,
    MOVE_DOWN, MOVE_FIRST, MOVE_LAST, MOVE_UP, NEW, PAGE_DOWN, PAGE_UP,
};

use crate::msg;
use crate::presenter::DocumentTypesPageEvent;

/// Remove the selected Document Type, opening the Remove Dialog on it (the notice on Other).
pub const REMOVE_TYPE: &str = "documents.remove_type";
/// Move the selected Document Type a place down the type filter's order.
pub const MOVE_TYPE_DOWN: &str = "documents.move_type_down";
/// Move the selected Document Type a place up the type filter's order.
pub const MOVE_TYPE_UP: &str = "documents.move_type_up";

/// The Document Types page's commands, with today's keys. The arrow, Home/End and Page keys move as
/// `j`/`k`, `g g`/`G` and `ctrl+d`/`ctrl+u` do, each under its own name so an override can move it.
pub const DOCUMENT_TYPES_PAGE_BINDINGS: &[Binding<DocumentTypesPageEvent>] = &[
    Binding::hidden(MOVE_DOWN, "j", DocumentTypesPageEvent::Move(Step::Next)),
    Binding::hidden(MOVE_UP, "k", DocumentTypesPageEvent::Move(Step::Prev)),
    Binding::hidden(MOVE_FIRST, "g g", DocumentTypesPageEvent::Move(Step::First)),
    Binding::hidden(MOVE_LAST, "G", DocumentTypesPageEvent::Move(Step::Last)),
    Binding::hidden(
        HALF_PAGE_DOWN,
        "ctrl+d",
        DocumentTypesPageEvent::Move(Step::HalfPageDown),
    ),
    Binding::hidden(
        HALF_PAGE_UP,
        "ctrl+u",
        DocumentTypesPageEvent::Move(Step::HalfPageUp),
    ),
    Binding::hidden(ARROW_DOWN, "down", DocumentTypesPageEvent::Move(Step::Next)),
    Binding::hidden(ARROW_UP, "up", DocumentTypesPageEvent::Move(Step::Prev)),
    Binding::hidden(HOME, "home", DocumentTypesPageEvent::Move(Step::First)),
    Binding::hidden(END, "end", DocumentTypesPageEvent::Move(Step::Last)),
    Binding::hidden(
        PAGE_DOWN,
        "pagedown",
        DocumentTypesPageEvent::Move(Step::HalfPageDown),
    ),
    Binding::hidden(
        PAGE_UP,
        "pageup",
        DocumentTypesPageEvent::Move(Step::HalfPageUp),
    ),
    Binding::listed(
        NEW,
        "n",
        DocumentTypesPageEvent::New,
        "document-types new",
        msg::documents_types_command_new,
    ),
    Binding::listed(
        EDIT,
        "e",
        DocumentTypesPageEvent::Edit,
        "document-types edit",
        msg::documents_types_command_edit,
    ),
    Binding::listed(
        REMOVE_TYPE,
        "x",
        DocumentTypesPageEvent::Remove,
        "document-types remove",
        msg::documents_types_command_remove,
    ),
    Binding::hidden(MOVE_TYPE_DOWN, "J", DocumentTypesPageEvent::Reorder(1)),
    Binding::hidden(MOVE_TYPE_UP, "K", DocumentTypesPageEvent::Reorder(-1)),
];

/// The Document Types page's keymap under `config`'s overrides.
pub fn document_types_page_keymap(config: &KeyBindingConfig) -> Keymap<DocumentTypesPageEvent> {
    Keymap::new(DOCUMENT_TYPES_PAGE_BINDINGS, config)
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
        let keymap = document_types_page_keymap(&KeyBindingConfig::default());

        assert_eq!(keymap.event("n"), Some(DocumentTypesPageEvent::New));
        assert_eq!(keymap.event("e"), Some(DocumentTypesPageEvent::Edit));
        assert_eq!(keymap.event("x"), Some(DocumentTypesPageEvent::Remove));
        assert_eq!(keymap.event("J"), Some(DocumentTypesPageEvent::Reorder(1)));
        assert_eq!(keymap.event("K"), Some(DocumentTypesPageEvent::Reorder(-1)));
        assert_eq!(
            keymap.event("j"),
            Some(DocumentTypesPageEvent::Move(Step::Next))
        );
        assert_eq!(
            keymap.event("g g"),
            Some(DocumentTypesPageEvent::Move(Step::First))
        );
        assert_eq!(
            keymap.event("end"),
            Some(DocumentTypesPageEvent::Move(Step::Last))
        );
        assert_eq!(
            keymap.event("d"),
            None,
            "remove is on x, not the flat delete"
        );
    }

    #[test]
    fn a_keybindings_override_moves_the_event_to_the_new_chord() {
        let keymap = document_types_page_keymap(&overriding("new", "a"));

        assert_eq!(keymap.event("a"), Some(DocumentTypesPageEvent::New));
        assert_eq!(keymap.event("n"), None, "the default no longer binds");
        assert_eq!(keymap.chord(NEW), Some("a"));
        let moved = document_types_page_keymap(&overriding(MOVE_TYPE_DOWN, "ctrl+j"));
        assert_eq!(
            moved.event("ctrl+j"),
            Some(DocumentTypesPageEvent::Reorder(1))
        );
        assert_eq!(moved.event("J"), None);
    }

    #[test]
    fn no_two_commands_share_a_default_chord() {
        let mut chords: Vec<&str> = DOCUMENT_TYPES_PAGE_BINDINGS
            .iter()
            .map(|binding| binding.chord)
            .collect();
        chords.sort_unstable();
        let total = chords.len();
        chords.dedup();
        assert_eq!(chords.len(), total);
    }

    #[test]
    fn the_palette_lists_new_edit_and_remove() {
        let keymap = document_types_page_keymap(&KeyBindingConfig::default());
        let names: Vec<&str> = keymap.listed().map(|listed| listed.name).collect();

        assert_eq!(
            names,
            [
                "document-types new",
                "document-types edit",
                "document-types remove"
            ]
        );
    }
}
