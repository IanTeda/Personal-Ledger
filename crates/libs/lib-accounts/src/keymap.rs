//! The Accounts page's default keys, built on `lib_viper`'s [`Keymap`]. Every command is one every
//! module shares, under its flat name from `lib_viper::keymap`; Accounts has none of its own.
//! `enter` is `select`, as it is on the Bills page, and opens the selected Account's ledger.

use lib_config::KeyBindingConfig;
use lib_list_cursor::Step;
use lib_viper::keymap::{
    ARROW_DOWN, ARROW_UP, Binding, DELETE, EDIT, END, HALF_PAGE_DOWN, HALF_PAGE_UP, HOME, Keymap,
    MOVE_DOWN, MOVE_FIRST, MOVE_LAST, MOVE_UP, NEW, PAGE_DOWN, PAGE_UP, SELECT,
};

use crate::presenter::AccountsPageEvent;

/// The Accounts page's commands, with today's keys. The arrow, Home/End and Page keys move as
/// `j`/`k`, `g g`/`G` and `ctrl+d`/`ctrl+u` do, each under its own name so an override can move it.
pub const ACCOUNTS_PAGE_BINDINGS: &[Binding<AccountsPageEvent>] = &[
    Binding::hidden(MOVE_DOWN, "j", AccountsPageEvent::Move(Step::Next)),
    Binding::hidden(MOVE_UP, "k", AccountsPageEvent::Move(Step::Prev)),
    Binding::hidden(MOVE_FIRST, "g g", AccountsPageEvent::Move(Step::First)),
    Binding::hidden(MOVE_LAST, "G", AccountsPageEvent::Move(Step::Last)),
    Binding::hidden(
        HALF_PAGE_DOWN,
        "ctrl+d",
        AccountsPageEvent::Move(Step::HalfPageDown),
    ),
    Binding::hidden(
        HALF_PAGE_UP,
        "ctrl+u",
        AccountsPageEvent::Move(Step::HalfPageUp),
    ),
    Binding::hidden(ARROW_DOWN, "down", AccountsPageEvent::Move(Step::Next)),
    Binding::hidden(ARROW_UP, "up", AccountsPageEvent::Move(Step::Prev)),
    Binding::hidden(HOME, "home", AccountsPageEvent::Move(Step::First)),
    Binding::hidden(END, "end", AccountsPageEvent::Move(Step::Last)),
    Binding::hidden(
        PAGE_DOWN,
        "pagedown",
        AccountsPageEvent::Move(Step::HalfPageDown),
    ),
    Binding::hidden(PAGE_UP, "pageup", AccountsPageEvent::Move(Step::HalfPageUp)),
    Binding::hidden(SELECT, "enter", AccountsPageEvent::OpenLedger),
    Binding::hidden(NEW, "n", AccountsPageEvent::New),
    Binding::hidden(EDIT, "e", AccountsPageEvent::Edit),
    Binding::hidden(DELETE, "d", AccountsPageEvent::Delete),
];

/// The Accounts page's keymap under `config`'s overrides.
pub fn accounts_page_keymap(config: &KeyBindingConfig) -> Keymap<AccountsPageEvent> {
    Keymap::new(ACCOUNTS_PAGE_BINDINGS, config)
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
        let keymap = accounts_page_keymap(&KeyBindingConfig::default());

        assert_eq!(keymap.event("n"), Some(AccountsPageEvent::New));
        assert_eq!(keymap.event("e"), Some(AccountsPageEvent::Edit));
        assert_eq!(keymap.event("d"), Some(AccountsPageEvent::Delete));
        assert_eq!(keymap.event("j"), Some(AccountsPageEvent::Move(Step::Next)));
        assert_eq!(
            keymap.event("g g"),
            Some(AccountsPageEvent::Move(Step::First))
        );
        assert_eq!(
            keymap.event("end"),
            Some(AccountsPageEvent::Move(Step::Last))
        );
        assert_eq!(keymap.event("enter"), Some(AccountsPageEvent::OpenLedger));
        assert_eq!(keymap.event("r"), None, "an Account is never reactivated");
    }

    #[test]
    fn a_keybindings_override_moves_the_event_to_the_new_chord() {
        let keymap = accounts_page_keymap(&overriding("new", "a"));

        assert_eq!(keymap.event("a"), Some(AccountsPageEvent::New));
        assert_eq!(keymap.event("n"), None, "the default no longer binds");
        assert_eq!(keymap.chord(NEW), Some("a"));
        // `accounts.new` names no Accounts command: they all keep their flat names.
        let namespaced = accounts_page_keymap(&overriding("accounts.new", "x"));
        assert_eq!(namespaced.event("n"), Some(AccountsPageEvent::New));
    }
}
