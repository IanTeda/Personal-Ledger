//! The global keymap: the keys the Shell answers on every View, recast from the Desktop's old
//! `route_key` as `lib-viper` bindings under flat command names, so `[keybindings]` overrides
//! them as it does any module's (#820, story 9).
//!
//! Its commands come in two kinds. Most the Shell keeps: it takes their keys before any View or
//! module sees them (`?`, `:`, `/`, the rail toggle, `shift+tab`, `esc`, the Toast keys and every
//! `g` jump), and a module's default never sits on one (each bin's clash test checks). The rest it
//! falls back on ([`ShellCommand::falls_back`]): focus `tab`, `a`, list movement and `enter`, which
//! the focused module's keymap or View takes first, so a list's own `tab` or `a` wins. The
//! movement commands keep the flat names every module's keymap binds, so one override moves a list
//! and the rails alike.
//!
//! A sequence (`g d`) is bound as one chord. Its first key is a prefix: the Shell waits for the next
//! key, and the Client's timer ends the wait with [`crate::ShellInput::PrefixExpired`].

use lib_config::KeyBindingConfig;
use lib_list_cursor::Step;
use lib_viper::keymap::{
    ARROW_DOWN, ARROW_UP, BACK, Binding, END, HALF_PAGE_DOWN, HALF_PAGE_UP, HOME, Keymap,
    MOVE_DOWN, MOVE_FIRST, MOVE_LAST, MOVE_UP, PAGE_DOWN, PAGE_UP, SELECT,
};

use lib_locale::msg;

use crate::navigation::Noun;

pub const HELP: &str = "help";
/// The palette. Named as `[keybindings]` already names it for the TUI.
pub const OPEN_COMMAND_POPUP: &str = "open_command_popup";
pub const SEARCH: &str = "search";
pub const TOGGLE_RAIL: &str = "toggle_rail";
pub const FOCUS_NEXT: &str = "focus_next";
pub const FOCUS_PREVIOUS: &str = "focus_previous";
pub const INSERT: &str = "insert";
pub const DISMISS_TOASTS: &str = "dismiss_toasts";
pub const TOAST_HISTORY: &str = "toast_history";
pub const JUMP_DASHBOARD: &str = "jump_dashboard";
pub const JUMP_TRANSACTIONS: &str = "jump_transactions";
pub const JUMP_DOCUMENTS: &str = "jump_documents";
pub const JUMP_NOTIFICATIONS: &str = "jump_notifications";
pub const JUMP_CASH: &str = "jump_cash";
pub const JUMP_INVENTORY: &str = "jump_inventory";
pub const JUMP_LOANS: &str = "jump_loans";
pub const JUMP_CREDIT_CARDS: &str = "jump_credit_cards";
pub const JUMP_INVESTMENTS: &str = "jump_investments";
pub const JUMP_BILLS: &str = "jump_bills";
pub const JUMP_BUDGETS: &str = "jump_budgets";
pub const JUMP_REPORTS: &str = "jump_reports";
pub const JUMP_SETTINGS: &str = "jump_settings";

/// A global key's command, as the Shell's Presenter hands it to the root Router.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShellCommand {
    /// Opens the Help Dialog.
    Help,
    /// Opens the command palette.
    Command,
    /// Opens the active View's Search.
    Search,
    ToggleRail,
    FocusPrevious,
    /// Backs out of whatever the View has open.
    Back,
    DismissToasts,
    ToastHistory,
    Jump(Noun),
    /// Enters Insert mode.
    Insert,
    FocusNext,
    Move(Step),
    /// Activates the focused row.
    Select,
}

impl ShellCommand {
    /// Whether the Shell takes this command's key only once the focused module or View has
    /// declined it, so a list may bind it for itself.
    pub fn falls_back(self) -> bool {
        matches!(
            self,
            Self::Insert | Self::FocusNext | Self::Move(_) | Self::Select
        )
    }
}

/// The global keymap's commands with their default keys. `toast_history` is unbound unless
/// `[keybindings]` sets it. The palette lists every command the Shell keeps but `back` and the
/// palette's own `:`; the ones it falls back on are a list's movement and `enter`, and `insert`.
pub const GLOBAL_BINDINGS: &[Binding<ShellCommand>] = &[
    Binding::listed(HELP, "?", ShellCommand::Help, "help", msg::command_help),
    Binding::hidden(OPEN_COMMAND_POPUP, ":", ShellCommand::Command),
    Binding::listed(
        SEARCH,
        "/",
        ShellCommand::Search,
        "search",
        msg::command_search,
    ),
    Binding::listed(
        TOGGLE_RAIL,
        "b",
        ShellCommand::ToggleRail,
        "toggle-rail",
        msg::command_toggle_rail,
    ),
    Binding::listed(
        FOCUS_PREVIOUS,
        "shift+tab",
        ShellCommand::FocusPrevious,
        "focus-previous",
        msg::command_focus_previous,
    ),
    Binding::hidden(BACK, "esc", ShellCommand::Back),
    // Not `esc`: a sticky Error Toast must not vanish on the key that backs out of a Dialog.
    Binding::listed(
        DISMISS_TOASTS,
        "ctrl+l",
        ShellCommand::DismissToasts,
        "dismiss all",
        msg::command_dismiss_toasts,
    ),
    Binding::listed(
        TOAST_HISTORY,
        "",
        ShellCommand::ToastHistory,
        "toasts",
        msg::command_toast_history,
    ),
    Binding::listed(
        JUMP_DASHBOARD,
        "g d",
        ShellCommand::Jump(Noun::Dashboard),
        "dashboard",
        msg::command_jump_dashboard,
    ),
    Binding::listed(
        JUMP_TRANSACTIONS,
        "g l",
        ShellCommand::Jump(Noun::Transactions),
        "transactions",
        msg::command_jump_transactions,
    ),
    Binding::listed(
        JUMP_DOCUMENTS,
        "g f",
        ShellCommand::Jump(Noun::Documents),
        "documents",
        msg::command_jump_documents,
    ),
    Binding::listed(
        JUMP_NOTIFICATIONS,
        "g a",
        ShellCommand::Jump(Noun::Notifications),
        "notifications",
        msg::command_jump_notifications,
    ),
    Binding::listed(
        JUMP_CASH,
        "g c",
        ShellCommand::Jump(Noun::Cash),
        "cash",
        msg::command_jump_cash,
    ),
    Binding::listed(
        JUMP_INVENTORY,
        "g o",
        ShellCommand::Jump(Noun::Inventory),
        "inventory",
        msg::command_jump_inventory,
    ),
    Binding::listed(
        JUMP_LOANS,
        "g n",
        ShellCommand::Jump(Noun::Loans),
        "loans",
        msg::command_jump_loans,
    ),
    Binding::listed(
        JUMP_CREDIT_CARDS,
        "g k",
        ShellCommand::Jump(Noun::CreditCards),
        "credit-cards",
        msg::command_jump_credit_cards,
    ),
    Binding::listed(
        JUMP_INVESTMENTS,
        "g i",
        ShellCommand::Jump(Noun::Investments),
        "investments",
        msg::command_jump_investments,
    ),
    Binding::listed(
        JUMP_BILLS,
        "g w",
        ShellCommand::Jump(Noun::Bills),
        "bills",
        msg::command_jump_bills,
    ),
    Binding::listed(
        JUMP_BUDGETS,
        "g b",
        ShellCommand::Jump(Noun::Budgets),
        "budgets",
        msg::command_jump_budgets,
    ),
    Binding::listed(
        JUMP_REPORTS,
        "g r",
        ShellCommand::Jump(Noun::Reports),
        "reports",
        msg::command_jump_reports,
    ),
    Binding::listed(
        JUMP_SETTINGS,
        "g s",
        ShellCommand::Jump(Noun::Settings),
        "settings",
        msg::command_jump_settings,
    ),
    Binding::hidden(INSERT, "a", ShellCommand::Insert),
    Binding::listed(
        FOCUS_NEXT,
        "tab",
        ShellCommand::FocusNext,
        "focus-next",
        msg::command_focus_next,
    ),
    Binding::hidden(MOVE_DOWN, "j", ShellCommand::Move(Step::Next)),
    Binding::hidden(ARROW_DOWN, "down", ShellCommand::Move(Step::Next)),
    Binding::hidden(MOVE_UP, "k", ShellCommand::Move(Step::Prev)),
    Binding::hidden(ARROW_UP, "up", ShellCommand::Move(Step::Prev)),
    Binding::hidden(MOVE_FIRST, "g g", ShellCommand::Move(Step::First)),
    Binding::hidden(HOME, "home", ShellCommand::Move(Step::First)),
    Binding::hidden(MOVE_LAST, "G", ShellCommand::Move(Step::Last)),
    Binding::hidden(END, "end", ShellCommand::Move(Step::Last)),
    Binding::hidden(
        HALF_PAGE_DOWN,
        "ctrl+d",
        ShellCommand::Move(Step::HalfPageDown),
    ),
    Binding::hidden(
        PAGE_DOWN,
        "pagedown",
        ShellCommand::Move(Step::HalfPageDown),
    ),
    Binding::hidden(HALF_PAGE_UP, "ctrl+u", ShellCommand::Move(Step::HalfPageUp)),
    Binding::hidden(PAGE_UP, "pageup", ShellCommand::Move(Step::HalfPageUp)),
    Binding::hidden(SELECT, "enter", ShellCommand::Select),
];

/// The global keymap under `config`'s overrides.
pub fn global_keymap(config: &KeyBindingConfig) -> Keymap<ShellCommand> {
    Keymap::new(GLOBAL_BINDINGS, config)
}

/// Whether the Shell takes some key of `chord` before any View sees it, under `keymap`: a kept
/// command's chord, or a sequence whose first key is a prefix but which the keymap doesn't bind to
/// a command it falls back on. A module default must not be one.
pub fn keeps(keymap: &Keymap<ShellCommand>, chord: &str) -> bool {
    if let Some(command) = keymap.event(chord) {
        return !command.falls_back();
    }
    let first = chord.split(' ').next().unwrap_or(chord);
    keymap.is_prefix(first) || (first != chord && keeps(keymap, first))
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
    fn the_keybindings_defaults_agree_with_every_global_default() {
        let keymap = global_keymap(&KeyBindingConfig::default());

        for binding in GLOBAL_BINDINGS {
            assert_eq!(
                keymap.chord(binding.command),
                Some(binding.chord),
                "{}",
                binding.command
            );
        }
    }

    #[test]
    fn a_kept_key_is_kept_and_a_fallback_key_is_a_lists_to_take() {
        let keymap = global_keymap(&KeyBindingConfig::default());

        for chord in [
            "b",
            "?",
            ":",
            "/",
            "shift+tab",
            "esc",
            "ctrl+l",
            "g",
            "g x",
            "g d",
            "b x",
        ] {
            assert!(keeps(&keymap, chord), "{chord}");
        }
        for chord in [
            "a", "tab", "j", "down", "g g", "G", "home", "pagedown", "enter", "B", "n",
        ] {
            assert!(!keeps(&keymap, chord), "{chord}");
        }
    }

    #[test]
    fn an_override_moves_a_kept_key() {
        let keymap = global_keymap(&overriding(TOGGLE_RAIL, "ctrl+b"));

        assert!(keeps(&keymap, "ctrl+b"));
        assert!(!keeps(&keymap, "b"), "the default no longer binds");
    }
}
