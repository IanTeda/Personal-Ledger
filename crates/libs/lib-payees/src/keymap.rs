//! The Payees page's default keys, built on `lib_viper`'s [`Keymap`]. Every command is one every
//! module shares, under its flat name from `lib_viper::keymap`; Payees has none of its own.

use lib_config::KeyBindingConfig;
use lib_list_cursor::Step;
use lib_viper::keymap::{
    ARROW_DOWN, ARROW_UP, Binding, DELETE, EDIT, END, HALF_PAGE_DOWN, HALF_PAGE_UP, HOME, Keymap,
    MOVE_DOWN, MOVE_FIRST, MOVE_LAST, MOVE_UP, NEW, PAGE_DOWN, PAGE_UP, SELECT,
};

use crate::msg;

use crate::presenter::PayeesPageEvent;

/// The Payees page's commands, with today's keys. The arrow, Home/End and Page keys move as
/// `j`/`k`, `g g`/`G` and `ctrl+d`/`ctrl+u` do, each under its own name so an override can move it.
pub const PAYEES_PAGE_BINDINGS: &[Binding<PayeesPageEvent>] = &[
    Binding::hidden(MOVE_DOWN, "j", PayeesPageEvent::Move(Step::Next)),
    Binding::hidden(MOVE_UP, "k", PayeesPageEvent::Move(Step::Prev)),
    Binding::hidden(MOVE_FIRST, "g g", PayeesPageEvent::Move(Step::First)),
    Binding::hidden(MOVE_LAST, "G", PayeesPageEvent::Move(Step::Last)),
    Binding::hidden(
        HALF_PAGE_DOWN,
        "ctrl+d",
        PayeesPageEvent::Move(Step::HalfPageDown),
    ),
    Binding::hidden(
        HALF_PAGE_UP,
        "ctrl+u",
        PayeesPageEvent::Move(Step::HalfPageUp),
    ),
    Binding::hidden(ARROW_DOWN, "down", PayeesPageEvent::Move(Step::Next)),
    Binding::hidden(ARROW_UP, "up", PayeesPageEvent::Move(Step::Prev)),
    Binding::hidden(HOME, "home", PayeesPageEvent::Move(Step::First)),
    Binding::hidden(END, "end", PayeesPageEvent::Move(Step::Last)),
    Binding::hidden(
        PAGE_DOWN,
        "pagedown",
        PayeesPageEvent::Move(Step::HalfPageDown),
    ),
    Binding::hidden(PAGE_UP, "pageup", PayeesPageEvent::Move(Step::HalfPageUp)),
    Binding::hidden(SELECT, "enter", PayeesPageEvent::OpenTransactions),
    Binding::listed(
        NEW,
        "n",
        PayeesPageEvent::New,
        "payees new",
        msg::payees_command_new,
    ),
    Binding::listed(
        EDIT,
        "e",
        PayeesPageEvent::Edit,
        "payees edit",
        msg::payees_command_edit,
    ),
    Binding::listed(
        DELETE,
        "d",
        PayeesPageEvent::Delete,
        "payees delete",
        msg::payees_command_delete,
    ),
];

/// The Payees page's keymap under `config`'s overrides.
pub fn payees_page_keymap(config: &KeyBindingConfig) -> Keymap<PayeesPageEvent> {
    Keymap::new(PAYEES_PAGE_BINDINGS, config)
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
        let keymap = payees_page_keymap(&KeyBindingConfig::default());

        assert_eq!(keymap.event("n"), Some(PayeesPageEvent::New));
        assert_eq!(keymap.event("e"), Some(PayeesPageEvent::Edit));
        assert_eq!(keymap.event("d"), Some(PayeesPageEvent::Delete));
        assert_eq!(keymap.event("j"), Some(PayeesPageEvent::Move(Step::Next)));
        assert_eq!(
            keymap.event("g g"),
            Some(PayeesPageEvent::Move(Step::First))
        );
        assert_eq!(keymap.event("end"), Some(PayeesPageEvent::Move(Step::Last)));
        assert_eq!(
            keymap.event("r"),
            None,
            "reactivating is the Delete Dialog's"
        );
    }

    #[test]
    fn a_keybindings_override_moves_the_event_to_the_new_chord() {
        let keymap = payees_page_keymap(&overriding("new", "a"));

        assert_eq!(keymap.event("a"), Some(PayeesPageEvent::New));
        assert_eq!(keymap.event("n"), None, "the default no longer binds");
        assert_eq!(keymap.chord(NEW), Some("a"));
        // `payees.new` names no Payees command: they all keep their flat names.
        let namespaced = payees_page_keymap(&overriding("payees.new", "x"));
        assert_eq!(namespaced.event("n"), Some(PayeesPageEvent::New));
    }
}
