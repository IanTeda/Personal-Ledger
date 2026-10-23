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
/// `[keybindings]` sets it.
pub const GLOBAL_BINDINGS: &[Binding<ShellCommand>] = &[
    Binding {
        command: HELP,
        chord: "?",
        event: ShellCommand::Help,
    },
    Binding {
        command: OPEN_COMMAND_POPUP,
        chord: ":",
        event: ShellCommand::Command,
    },
    Binding {
        command: SEARCH,
        chord: "/",
        event: ShellCommand::Search,
    },
    Binding {
        command: TOGGLE_RAIL,
        chord: "b",
        event: ShellCommand::ToggleRail,
    },
    Binding {
        command: FOCUS_PREVIOUS,
        chord: "shift+tab",
        event: ShellCommand::FocusPrevious,
    },
    Binding {
        command: BACK,
        chord: "esc",
        event: ShellCommand::Back,
    },
    // Not `esc`: a sticky Error Toast must not vanish on the key that backs out of a Dialog.
    Binding {
        command: DISMISS_TOASTS,
        chord: "ctrl+l",
        event: ShellCommand::DismissToasts,
    },
    Binding {
        command: TOAST_HISTORY,
        chord: "",
        event: ShellCommand::ToastHistory,
    },
    Binding {
        command: JUMP_DASHBOARD,
        chord: "g d",
        event: ShellCommand::Jump(Noun::Dashboard),
    },
    Binding {
        command: JUMP_TRANSACTIONS,
        chord: "g l",
        event: ShellCommand::Jump(Noun::Transactions),
    },
    Binding {
        command: JUMP_DOCUMENTS,
        chord: "g f",
        event: ShellCommand::Jump(Noun::Documents),
    },
    Binding {
        command: JUMP_NOTIFICATIONS,
        chord: "g a",
        event: ShellCommand::Jump(Noun::Notifications),
    },
    Binding {
        command: JUMP_CASH,
        chord: "g c",
        event: ShellCommand::Jump(Noun::Cash),
    },
    Binding {
        command: JUMP_INVENTORY,
        chord: "g o",
        event: ShellCommand::Jump(Noun::Inventory),
    },
    Binding {
        command: JUMP_LOANS,
        chord: "g n",
        event: ShellCommand::Jump(Noun::Loans),
    },
    Binding {
        command: JUMP_CREDIT_CARDS,
        chord: "g k",
        event: ShellCommand::Jump(Noun::CreditCards),
    },
    Binding {
        command: JUMP_INVESTMENTS,
        chord: "g i",
        event: ShellCommand::Jump(Noun::Investments),
    },
    Binding {
        command: JUMP_BILLS,
        chord: "g w",
        event: ShellCommand::Jump(Noun::Bills),
    },
    Binding {
        command: JUMP_BUDGETS,
        chord: "g b",
        event: ShellCommand::Jump(Noun::Budgets),
    },
    Binding {
        command: JUMP_REPORTS,
        chord: "g r",
        event: ShellCommand::Jump(Noun::Reports),
    },
    Binding {
        command: JUMP_SETTINGS,
        chord: "g s",
        event: ShellCommand::Jump(Noun::Settings),
    },
    Binding {
        command: INSERT,
        chord: "a",
        event: ShellCommand::Insert,
    },
    Binding {
        command: FOCUS_NEXT,
        chord: "tab",
        event: ShellCommand::FocusNext,
    },
    Binding {
        command: MOVE_DOWN,
        chord: "j",
        event: ShellCommand::Move(Step::Next),
    },
    Binding {
        command: ARROW_DOWN,
        chord: "down",
        event: ShellCommand::Move(Step::Next),
    },
    Binding {
        command: MOVE_UP,
        chord: "k",
        event: ShellCommand::Move(Step::Prev),
    },
    Binding {
        command: ARROW_UP,
        chord: "up",
        event: ShellCommand::Move(Step::Prev),
    },
    Binding {
        command: MOVE_FIRST,
        chord: "g g",
        event: ShellCommand::Move(Step::First),
    },
    Binding {
        command: HOME,
        chord: "home",
        event: ShellCommand::Move(Step::First),
    },
    Binding {
        command: MOVE_LAST,
        chord: "G",
        event: ShellCommand::Move(Step::Last),
    },
    Binding {
        command: END,
        chord: "end",
        event: ShellCommand::Move(Step::Last),
    },
    Binding {
        command: HALF_PAGE_DOWN,
        chord: "ctrl+d",
        event: ShellCommand::Move(Step::HalfPageDown),
    },
    Binding {
        command: PAGE_DOWN,
        chord: "pagedown",
        event: ShellCommand::Move(Step::HalfPageDown),
    },
    Binding {
        command: HALF_PAGE_UP,
        chord: "ctrl+u",
        event: ShellCommand::Move(Step::HalfPageUp),
    },
    Binding {
        command: PAGE_UP,
        chord: "pageup",
        event: ShellCommand::Move(Step::HalfPageUp),
    },
    Binding {
        command: SELECT,
        chord: "enter",
        event: ShellCommand::Select,
    },
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
