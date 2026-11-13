//! The Transactions page's default keys, built on `lib_viper`'s [`Keymap`]. The commands every
//! module shares keep their flat names from `lib_viper::keymap`; the one only Transactions has is
//! namespaced here (`transactions.filter`). Each is today's key, checked against both Clients'
//! global keys by their clash tests. `/` is the Shell's, which sends the page's `search` by name.
//!
//! While the filter popover is open the page reads its keys through the popover's keymap instead:
//! the Dialog commands under their flat names, so one `[keybindings]` override moves them in every
//! Dialog and here alike, and the popover's own `transactions.reset_filters`.

use lib_config::KeyBindingConfig;
use lib_list_cursor::Step;
use lib_viper::keymap::{
    ARROW_DOWN, ARROW_UP, BACK, Binding, EDIT, END, HALF_PAGE_DOWN, HALF_PAGE_UP, HOME, Keymap,
    MOVE_DOWN, MOVE_FIRST, MOVE_LAST, MOVE_UP, NEW, NEXT_FIELD, PAGE_DOWN, PAGE_UP, PREVIOUS_FIELD,
    SEARCH, SELECT, SELECT_NEXT, SELECT_PREVIOUS, SUBMIT,
};

use crate::msg;

use crate::presenter::{FilterEvent, TransactionsPageEvent};

pub const FILTER: &str = "transactions.filter";
pub const RESET_FILTERS: &str = "transactions.reset_filters";
/// `Left` and `Right` on the popover's Status control, under names of their own so an override can
/// move them.
pub const STATUS_PREVIOUS: &str = "transactions.status_previous";
pub const STATUS_NEXT: &str = "transactions.status_next";

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
    // The Shell's `/` sends it by name, so it binds the Shell's chord, which the legend reads.
    Binding::hidden(SEARCH, "/", TransactionsPageEvent::StartSearch),
];

/// The Transactions page's keymap under `config`'s overrides.
pub fn transactions_page_keymap(config: &KeyBindingConfig) -> Keymap<TransactionsPageEvent> {
    Keymap::new(TRANSACTIONS_PAGE_BINDINGS, config)
}

const fn filter(
    command: &'static str,
    chord: &'static str,
    event: FilterEvent,
) -> Binding<TransactionsPageEvent> {
    Binding::hidden(command, chord, TransactionsPageEvent::Filter(event))
}

/// The filter popover's own commands: `ctrl+r` puts the draft back to the defaults, and `Left` and
/// `Right` step the Status control. Hidden from the palette, since they mean something only with
/// the popover open; the popover's legend names reset.
pub const TRANSACTIONS_FILTER_BINDINGS: &[Binding<TransactionsPageEvent>] = &[
    filter(RESET_FILTERS, "ctrl+r", FilterEvent::Reset),
    filter(STATUS_PREVIOUS, "left", FilterEvent::Left),
    filter(STATUS_NEXT, "right", FilterEvent::Right),
];

/// The Dialog commands the popover reads, with `lib-shell`'s Dialog keymap's defaults: `select`
/// opens or commits a select's list and otherwise applies, `submit` applies from anywhere,
/// `next_field` and `previous_field` move between fields, and `select_next` and `select_previous`
/// step a closed select or move an open list's highlight. The list step's letters aren't among
/// them, so they type. The popover is a mode the Shell offers its keys to first, so these sit on
/// keys the Shell keeps (`shift+tab`) without clashing.
pub const TRANSACTIONS_POPOVER_BINDINGS: &[Binding<TransactionsPageEvent>] = &[
    filter(SELECT, "enter", FilterEvent::Activate),
    filter(BACK, "esc", FilterEvent::Back),
    filter(NEXT_FIELD, "tab", FilterEvent::NextField),
    filter(PREVIOUS_FIELD, "shift+tab", FilterEvent::PreviousField),
    filter(SELECT_NEXT, "down", FilterEvent::SelectNext),
    filter(SELECT_PREVIOUS, "up", FilterEvent::SelectPrevious),
    filter(SUBMIT, "ctrl+enter", FilterEvent::Apply),
    filter(SELECT_NEXT, "ctrl+n", FilterEvent::SelectNext),
    filter(SELECT_PREVIOUS, "ctrl+p", FilterEvent::SelectPrevious),
];

/// The filter popover's keymap under `config`'s overrides: the Dialog commands and its own.
pub fn transactions_filter_keymap(config: &KeyBindingConfig) -> Keymap<TransactionsPageEvent> {
    let bindings: Vec<_> = TRANSACTIONS_POPOVER_BINDINGS
        .iter()
        .chain(TRANSACTIONS_FILTER_BINDINGS)
        .cloned()
        .collect();
    Keymap::new(&bindings, config)
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert_eq!(
            keymap.event_for(SEARCH),
            Some(TransactionsPageEvent::StartSearch)
        );
    }

    #[test]
    fn reset_is_ctrl_r_until_overridden() {
        let default = transactions_filter_keymap(&KeyBindingConfig::default());
        let reset = Some(TransactionsPageEvent::Filter(FilterEvent::Reset));
        assert_eq!(default.event("ctrl+r"), reset);

        let moved =
            transactions_filter_keymap(&KeyBindingConfig::overriding(RESET_FILTERS, "ctrl+x"));
        assert_eq!(moved.event("ctrl+x"), reset);
        assert_eq!(moved.event("ctrl+r"), None);
    }

    #[test]
    fn the_popover_reads_the_dialog_commands_under_their_flat_names() {
        let filter = |event| Some(TransactionsPageEvent::Filter(event));
        let default = transactions_filter_keymap(&KeyBindingConfig::default());
        assert_eq!(default.event("enter"), filter(FilterEvent::Activate));
        assert_eq!(
            default.event("shift+tab"),
            filter(FilterEvent::PreviousField)
        );
        assert_eq!(default.event("ctrl+p"), filter(FilterEvent::SelectPrevious));
        assert_eq!(default.event("ctrl+enter"), filter(FilterEvent::Apply));
        assert_eq!(default.event("j"), None, "the list step's letters type");

        let moved = transactions_filter_keymap(&KeyBindingConfig::overriding(NEXT_FIELD, "ctrl+f"));
        assert_eq!(moved.event("ctrl+f"), filter(FilterEvent::NextField));
        assert_eq!(moved.event("tab"), None);
    }

    #[test]
    fn a_keybindings_override_moves_only_that_command() {
        let keymap =
            transactions_page_keymap(&KeyBindingConfig::overriding("transactions.filter", "F"));

        assert_eq!(
            keymap.event("F"),
            Some(TransactionsPageEvent::OpenFilter(None))
        );
        assert_eq!(keymap.event("f"), None);
        // `filter` alone names no Transactions command, so it can't move this one.
        let flat = transactions_page_keymap(&KeyBindingConfig::overriding("filter", "x"));
        assert_eq!(
            flat.event("f"),
            Some(TransactionsPageEvent::OpenFilter(None))
        );
    }
}
