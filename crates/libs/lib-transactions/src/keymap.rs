//! The Transactions page's default keys, built on `lib_viper`'s [`Keymap`]. The commands every
//! module shares keep their flat names from `lib_viper::keymap`; the one only Transactions has is
//! namespaced here (`transactions.filter`). Each is today's key, checked against both Clients'
//! global keys by their clash tests. `/` is the Shell's, which hands the page its Search.

use lib_config::KeyBindingConfig;
use lib_list_cursor::Step;
use lib_viper::keymap::{
    ARROW_DOWN, ARROW_UP, Binding, EDIT, END, HALF_PAGE_DOWN, HALF_PAGE_UP, HOME, Keymap,
    MOVE_DOWN, MOVE_FIRST, MOVE_LAST, MOVE_UP, NEW, PAGE_DOWN, PAGE_UP, SELECT,
};

use crate::msg;

use crate::presenter::TransactionsPageEvent;

pub const FILTER: &str = "transactions.filter";

const fn moves(
    command: &'static str,
    chord: &'static str,
    step: Step,
) -> Binding<TransactionsPageEvent> {
    Binding::hidden(command, chord, TransactionsPageEvent::Move(step))
}

/// The Transactions page's commands, with today's keys. The arrow, Home/End and Page keys move as
/// `j`/`k`, `g g`/`G` and `ctrl+d`/`ctrl+u` do, each under its own name so an override can move
/// it; `select` opens the row.
pub const TRANSACTIONS_PAGE_BINDINGS: &[Binding<TransactionsPageEvent>] = &[
    moves(MOVE_DOWN, "j", Step::Next),
    moves(MOVE_UP, "k", Step::Prev),
    moves(MOVE_FIRST, "g g", Step::First),
    moves(MOVE_LAST, "G", Step::Last),
    moves(HALF_PAGE_DOWN, "ctrl+d", Step::HalfPageDown),
    moves(HALF_PAGE_UP, "ctrl+u", Step::HalfPageUp),
    moves(ARROW_DOWN, "down", Step::Next),
    moves(ARROW_UP, "up", Step::Prev),
    moves(HOME, "home", Step::First),
    moves(END, "end", Step::Last),
    moves(PAGE_DOWN, "pagedown", Step::HalfPageDown),
    moves(PAGE_UP, "pageup", Step::HalfPageUp),
    Binding::hidden(SELECT, "enter", TransactionsPageEvent::Open),
    Binding::listed(
        NEW,
        "n",
        TransactionsPageEvent::New,
        "transactions new",
        msg::transactions_command_new,
    ),
    Binding::listed(
        EDIT,
        "e",
        TransactionsPageEvent::Edit,
        "transactions edit",
        msg::transactions_command_edit,
    ),
    Binding::listed(
        FILTER,
        "f",
        TransactionsPageEvent::OpenFilter(None),
        "transactions filter",
        msg::transactions_command_filter,
    ),
];

/// The Transactions page's keymap under `config`'s overrides.
pub fn transactions_page_keymap(config: &KeyBindingConfig) -> Keymap<TransactionsPageEvent> {
    Keymap::new(TRANSACTIONS_PAGE_BINDINGS, config)
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
    fn the_default_chords_reach_their_events() {
        let keymap = transactions_page_keymap(&KeyBindingConfig::default());

        assert_eq!(
            keymap.event("j"),
            Some(TransactionsPageEvent::Move(Step::Next))
        );
        assert_eq!(
            keymap.event("g g"),
            Some(TransactionsPageEvent::Move(Step::First))
        );
        assert_eq!(
            keymap.event("ctrl+d"),
            Some(TransactionsPageEvent::Move(Step::HalfPageDown))
        );
        assert_eq!(keymap.event("enter"), Some(TransactionsPageEvent::Open));
        assert_eq!(keymap.event("n"), Some(TransactionsPageEvent::New));
        assert_eq!(keymap.event("e"), Some(TransactionsPageEvent::Edit));
        assert_eq!(
            keymap.event("f"),
            Some(TransactionsPageEvent::OpenFilter(None))
        );
        assert_eq!(keymap.event("/"), None, "the Shell keeps `/`");
    }

    #[test]
    fn a_keybindings_override_moves_only_that_command() {
        let keymap = transactions_page_keymap(&overriding("transactions.filter", "F"));

        assert_eq!(
            keymap.event("F"),
            Some(TransactionsPageEvent::OpenFilter(None))
        );
        assert_eq!(keymap.event("f"), None);
        // `filter` alone names no Transactions command, so it can't move this one.
        let flat = transactions_page_keymap(&overriding("filter", "x"));
        assert_eq!(
            flat.event("f"),
            Some(TransactionsPageEvent::OpenFilter(None))
        );
    }
}
