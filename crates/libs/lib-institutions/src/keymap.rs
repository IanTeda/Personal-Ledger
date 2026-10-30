//! The Institutions page's default keys, built on `lib_viper`'s [`Keymap`]. Every command is one
//! every module shares, under its flat name from `lib_viper::keymap`; Institutions has none of its
//! own.

use lib_config::KeyBindingConfig;
use lib_list_cursor::Step;
use lib_viper::keymap::{
    ARROW_DOWN, ARROW_UP, Binding, DELETE, EDIT, END, HALF_PAGE_DOWN, HALF_PAGE_UP, HOME, Keymap,
    MOVE_DOWN, MOVE_FIRST, MOVE_LAST, MOVE_UP, NEW, PAGE_DOWN, PAGE_UP,
};

use crate::msg;

use crate::presenter::InstitutionsPageEvent;

/// The Institutions page's commands, with today's keys for the shared ones. The arrow, Home/End and
/// Page keys move as `j`/`k`, `g g`/`G` and `ctrl+d`/`ctrl+u` do, each under its own name so an
/// override can move it.
pub const INSTITUTIONS_PAGE_BINDINGS: &[Binding<InstitutionsPageEvent>] = &[
    Binding::hidden(MOVE_DOWN, "j", InstitutionsPageEvent::Move(Step::Next)),
    Binding::hidden(MOVE_UP, "k", InstitutionsPageEvent::Move(Step::Prev)),
    Binding::hidden(MOVE_FIRST, "g g", InstitutionsPageEvent::Move(Step::First)),
    Binding::hidden(MOVE_LAST, "G", InstitutionsPageEvent::Move(Step::Last)),
    Binding::hidden(
        HALF_PAGE_DOWN,
        "ctrl+d",
        InstitutionsPageEvent::Move(Step::HalfPageDown),
    ),
    Binding::hidden(
        HALF_PAGE_UP,
        "ctrl+u",
        InstitutionsPageEvent::Move(Step::HalfPageUp),
    ),
    Binding::hidden(ARROW_DOWN, "down", InstitutionsPageEvent::Move(Step::Next)),
    Binding::hidden(ARROW_UP, "up", InstitutionsPageEvent::Move(Step::Prev)),
    Binding::hidden(HOME, "home", InstitutionsPageEvent::Move(Step::First)),
    Binding::hidden(END, "end", InstitutionsPageEvent::Move(Step::Last)),
    Binding::hidden(
        PAGE_DOWN,
        "pagedown",
        InstitutionsPageEvent::Move(Step::HalfPageDown),
    ),
    Binding::hidden(
        PAGE_UP,
        "pageup",
        InstitutionsPageEvent::Move(Step::HalfPageUp),
    ),
    Binding::listed(
        NEW,
        "n",
        InstitutionsPageEvent::New,
        "institutions new",
        msg::institutions_command_new,
    ),
    Binding::listed(
        EDIT,
        "e",
        InstitutionsPageEvent::Edit,
        "institutions edit",
        msg::institutions_command_edit,
    ),
    Binding::listed(
        DELETE,
        "d",
        InstitutionsPageEvent::Delete,
        "institutions delete",
        msg::institutions_command_delete,
    ),
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
