//! The Dialog keymap: the keys every Dialog shares, so `Enter`, `Esc`, `Tab` and select stepping
//! behave the same in each and follow `[keybindings]` (#820, Q1, Q14, Q15). The Client resolves a
//! key through it while a Dialog is open and hands the open Dialog the [`DialogCommand`]; a key it
//! doesn't bind is the Dialog's own, typing into a field.
//!
//! What a command does is the Dialog's call, not the Client's: [`DialogCommand::Select`] opens a
//! closed select, picks the highlighted Choice on an open one, and otherwise confirms only a valid
//! form; [`DialogCommand::Submit`] confirms a valid form from anywhere, a multi-line box or a
//! select included. Only [`DialogCommand::Back`] the host decides itself: it closes an open select
//! first, otherwise cancels the Dialog and closes it.
//!
//! [`DialogCommand::ListNext`] and [`DialogCommand::ListPrevious`] are bound to letters, so they
//! are a step only in a Dialog stepping a list right then (the Pay Dialog's Match list, the
//! Budgets lists); every other Dialog and field takes the key as typing
//! ([`DialogCommand::is_list_step`]).

//!
//! The keymap itself lives in `lib_viper::dialog` (#1167), so a page Presenter outside the bins
//! builds its Dialog keys from it; it is re-exported here so the Shell's callers don't change.

pub use lib_viper::dialog::{DIALOG_BINDINGS, DialogCommand, dialog_keymap};
