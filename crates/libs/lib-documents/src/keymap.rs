//! The Documents module's default keys, built on `lib_viper`'s [`Keymap`]: the Documents page's
//! and the Document Types page's. Movement, `select`, `new` and `edit` are the flat names every
//! module shares (`lib_viper::keymap`); the rest are the module's own (`documents.accept`), each on
//! today's key, checked against both Clients' global keys by their clash tests.
//!
//! The Document Types page removes on `x`, since the flat `delete` defaults to `d`, and reorders,
//! which only a user-ordered list does. The Documents page's keys mean what the focus zone and the
//! mode make them: `l` enters the list from the index and opens the link picker in it, and the
//! Inbox's verbs do nothing in the Library ([`DocumentsPagePresenter::takes`]). `+ Add` is on `a`,
//! not the flat `new`'s `n`, as the page has always had it.
//!
//! [`DocumentsPagePresenter::takes`]: crate::presenter::DocumentsPagePresenter::takes

use lib_config::KeyBindingConfig;
use lib_list_cursor::Step;
use lib_viper::keymap::{
    ARROW_DOWN, ARROW_UP, Binding, EDIT, END, HALF_PAGE_DOWN, HALF_PAGE_UP, HOME, Keymap,
    MOVE_DOWN, MOVE_FIRST, MOVE_LAST, MOVE_UP, NEW, PAGE_DOWN, PAGE_UP, SELECT,
};

use crate::msg;
use crate::presenter::{DocumentTypesPageEvent, DocumentsPageEvent};

/// Show the Inbox.
pub const SHOW_INBOX: &str = "documents.show_inbox";
/// Show the Library, on its last scope.
pub const SHOW_LIBRARY: &str = "documents.show_library";
/// Switch between the Inbox and the Library.
pub const TOGGLE_MODE: &str = "documents.toggle_mode";
/// `+ Add`: a Filed Document from a path.
pub const ADD: &str = "documents.add";
/// `Import…`: paths into the Inbox.
pub const IMPORT: &str = "documents.import";
/// The Library's sort, Newest or Expiring.
pub const SORT: &str = "documents.sort";
/// From the index into the list; in the list, the link picker.
pub const LINK: &str = "documents.link";
/// Follow the selected Document's Link, or the Inbox row's Suggested Link.
pub const FOLLOW: &str = "documents.follow";
/// From the index into the list, on the right arrow.
pub const ARROW_RIGHT: &str = "documents.arrow_right";
/// From the list back to the index.
pub const FOCUS_INDEX: &str = "documents.focus_index";
/// From the list back to the index, on the left arrow.
pub const ARROW_LEFT: &str = "documents.arrow_left";
/// Show the selected file in its folder.
pub const SHOW_IN_FOLDER: &str = "documents.show_in_folder";
/// File the focused Inbox row under its Suggested Link.
pub const ACCEPT: &str = "documents.accept";
/// Accept all strong matches, after a count-first confirm.
pub const ACCEPT_ALL: &str = "documents.accept_all";
/// Set the focused Inbox row aside for the session.
pub const SKIP: &str = "documents.skip";
/// Reverse the last filing.
pub const UNDO: &str = "documents.undo";

/// The Documents page's commands, with today's keys. The moves and `select` act on the focus zone;
/// the arrow, Home/End and Page keys move as `j`/`k`, `g g`/`G` and `ctrl+d`/`ctrl+u` do, each
/// under its own name so an override can move it. Showing the Inbox or the Library has no key
/// until `[keybindings]` gives it one: `i` switches between them.
pub const DOCUMENTS_PAGE_BINDINGS: &[Binding<DocumentsPageEvent>] = &[
    Binding::hidden(MOVE_DOWN, "j", DocumentsPageEvent::Move(Step::Next)),
    Binding::hidden(MOVE_UP, "k", DocumentsPageEvent::Move(Step::Prev)),
    Binding::hidden(MOVE_FIRST, "g g", DocumentsPageEvent::Move(Step::First)),
    Binding::hidden(MOVE_LAST, "G", DocumentsPageEvent::Move(Step::Last)),
    Binding::hidden(
        HALF_PAGE_DOWN,
        "ctrl+d",
        DocumentsPageEvent::Move(Step::HalfPageDown),
    ),
    Binding::hidden(
        HALF_PAGE_UP,
        "ctrl+u",
        DocumentsPageEvent::Move(Step::HalfPageUp),
    ),
    Binding::hidden(ARROW_DOWN, "down", DocumentsPageEvent::Move(Step::Next)),
    Binding::hidden(ARROW_UP, "up", DocumentsPageEvent::Move(Step::Prev)),
    Binding::hidden(HOME, "home", DocumentsPageEvent::Move(Step::First)),
    Binding::hidden(END, "end", DocumentsPageEvent::Move(Step::Last)),
    Binding::hidden(
        PAGE_DOWN,
        "pagedown",
        DocumentsPageEvent::Move(Step::HalfPageDown),
    ),
    Binding::hidden(
        PAGE_UP,
        "pageup",
        DocumentsPageEvent::Move(Step::HalfPageUp),
    ),
    Binding::hidden(SELECT, "enter", DocumentsPageEvent::Select),
    Binding::hidden(LINK, "l", DocumentsPageEvent::Link),
    Binding::hidden(ARROW_RIGHT, "right", DocumentsPageEvent::FocusList),
    Binding::hidden(FOCUS_INDEX, "h", DocumentsPageEvent::FocusIndex),
    Binding::hidden(ARROW_LEFT, "left", DocumentsPageEvent::FocusIndex),
    Binding::hidden(FOLLOW, "L", DocumentsPageEvent::Follow),
    Binding::hidden(TOGGLE_MODE, "i", DocumentsPageEvent::ToggleMode),
    Binding::listed(
        SHOW_INBOX,
        "",
        DocumentsPageEvent::ShowInbox,
        "documents inbox",
        msg::documents_command_inbox,
    ),
    Binding::listed(
        SHOW_LIBRARY,
        "",
        DocumentsPageEvent::ShowLibrary,
        "documents library",
        msg::documents_command_library,
    ),
    Binding::listed(
        ADD,
        "a",
        DocumentsPageEvent::Add,
        "documents add",
        msg::documents_command_add,
    ),
    Binding::listed(
        IMPORT,
        "I",
        DocumentsPageEvent::Import,
        "documents import",
        msg::documents_command_import,
    ),
    Binding::hidden(SORT, "s", DocumentsPageEvent::Sort),
    Binding::hidden(EDIT, "e", DocumentsPageEvent::Edit),
    Binding::hidden(SHOW_IN_FOLDER, "o", DocumentsPageEvent::ShowInFolder),
    Binding::hidden(ACCEPT, "y", DocumentsPageEvent::Accept),
    Binding::listed(
        ACCEPT_ALL,
        "Y",
        DocumentsPageEvent::AcceptAll,
        "documents accept-all",
        msg::documents_command_accept_all,
    ),
    Binding::hidden(SKIP, "x", DocumentsPageEvent::Skip),
    Binding::hidden(UNDO, "u", DocumentsPageEvent::Undo),
];

/// The Documents page's keymap under `config`'s overrides.
pub fn documents_page_keymap(config: &KeyBindingConfig) -> Keymap<DocumentsPageEvent> {
    Keymap::new(DOCUMENTS_PAGE_BINDINGS, config)
}

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
    fn a_documents_default_chord_reaches_its_event() {
        let keymap = documents_page_keymap(&KeyBindingConfig::default());

        assert_eq!(keymap.event("y"), Some(DocumentsPageEvent::Accept));
        assert_eq!(keymap.event("Y"), Some(DocumentsPageEvent::AcceptAll));
        assert_eq!(keymap.event("I"), Some(DocumentsPageEvent::Import));
        assert_eq!(keymap.event("l"), Some(DocumentsPageEvent::Link));
        assert_eq!(keymap.event("L"), Some(DocumentsPageEvent::Follow));
        assert_eq!(keymap.event("enter"), Some(DocumentsPageEvent::Select));
        assert_eq!(
            keymap.event("g g"),
            Some(DocumentsPageEvent::Move(Step::First))
        );
        assert_eq!(keymap.event("n"), None, "add is on a, not the flat new");
        assert_eq!(keymap.event(""), None, "an unbound command has no chord");
    }

    #[test]
    fn a_documents_override_moves_the_event_and_frees_the_default() {
        let keymap = documents_page_keymap(&overriding(ACCEPT, "ctrl+y"));

        assert_eq!(keymap.event("ctrl+y"), Some(DocumentsPageEvent::Accept));
        assert_eq!(keymap.event("y"), None);
        let bound = documents_page_keymap(&overriding(SHOW_INBOX, "g x"));
        assert_eq!(bound.event("g x"), Some(DocumentsPageEvent::ShowInbox));
        assert!(bound.is_prefix("g"));
    }

    #[test]
    fn no_two_documents_commands_share_a_default_chord() {
        let mut chords: Vec<&str> = DOCUMENTS_PAGE_BINDINGS
            .iter()
            .map(|binding| binding.chord)
            .filter(|chord| !chord.is_empty())
            .collect();
        chords.sort_unstable();
        let total = chords.len();
        chords.dedup();
        assert_eq!(chords.len(), total);
    }

    #[test]
    fn the_palette_lists_the_documents_verbs() {
        let keymap = documents_page_keymap(&KeyBindingConfig::default());
        let names: Vec<&str> = keymap.listed().map(|listed| listed.name).collect();

        assert_eq!(
            names,
            [
                "documents inbox",
                "documents library",
                "documents add",
                "documents import",
                "documents accept-all"
            ]
        );
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
