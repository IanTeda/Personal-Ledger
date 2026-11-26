//! The Accounts page's default keys, built on `lib_viper`'s [`Keymap`]. Every command is one every
//! module shares, under its flat name from `lib_viper::keymap`; Accounts has none of its own.
//! `enter` is `select`, as it is on the Bills page, and opens the selected Account's ledger.
//! `new`, `edit` and `delete` are listed with an Argument (`accounts delete Home Loan`): the palette
//! sends the typed Command, the key the plain verb.

use lib_core::configuration::KeyBindingConfig;
use lib_list_cursor::Step;
use lib_viper::keymap::{
    ARROW_DOWN, ARROW_UP, Binding, DELETE, EDIT, END, HALF_PAGE_DOWN, HALF_PAGE_UP, HOME, Keymap,
    MOVE_DOWN, MOVE_FIRST, MOVE_LAST, MOVE_UP, NEW, PAGE_DOWN, PAGE_UP, SELECT,
};

use crate::msg;
use crate::presenter::{AccountsCommand, AccountsPageEvent};

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
    Binding::listed_with_argument(
        NEW,
        "n",
        AccountsPageEvent::New,
        typed_new,
        "accounts new",
        command_new_description,
    ),
    Binding::listed_with_argument(
        EDIT,
        "e",
        AccountsPageEvent::Edit,
        typed_edit,
        "accounts edit",
        msg::accounts_command_edit,
    ),
    Binding::listed_with_argument(
        DELETE,
        "d",
        AccountsPageEvent::Delete,
        typed_delete,
        "accounts delete",
        msg::accounts_command_delete,
    ),
];

fn command_new_description() -> String {
    msg::accounts_command_new("accounts new")
}

fn typed_new(command: &'static str, argument: String) -> AccountsPageEvent {
    typed(command, AccountsCommand::New, argument)
}

fn typed_edit(command: &'static str, argument: String) -> AccountsPageEvent {
    typed(command, AccountsCommand::Edit, argument)
}

fn typed_delete(command: &'static str, argument: String) -> AccountsPageEvent {
    typed(command, AccountsCommand::Delete, argument)
}

/// The palette's event for a verb: the typed Command, so a name that fits no Account reaches the
/// status line, where the key's plain event acts on the selected row or stays silent.
fn typed(command: &'static str, verb: AccountsCommand, argument: String) -> AccountsPageEvent {
    AccountsPageEvent::Command {
        command: command.to_string(),
        verb,
        argument,
    }
}

/// The Accounts page's keymap under `config`'s overrides.
pub fn accounts_page_keymap(config: &KeyBindingConfig) -> Keymap<AccountsPageEvent> {
    Keymap::new(ACCOUNTS_PAGE_BINDINGS, config)
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let keymap = accounts_page_keymap(&KeyBindingConfig::overriding("new", "a"));

        assert_eq!(keymap.event("a"), Some(AccountsPageEvent::New));
        assert_eq!(keymap.event("n"), None, "the default no longer binds");
        assert_eq!(keymap.chord(NEW), Some("a"));
        // `accounts.new` names no Accounts command: they all keep their flat names.
        let namespaced = accounts_page_keymap(&KeyBindingConfig::overriding("accounts.new", "x"));
        assert_eq!(namespaced.event("n"), Some(AccountsPageEvent::New));
    }

    #[test]
    fn the_verbs_are_listed_with_an_argument_and_the_palette_sends_the_typed_command() {
        let keymap = accounts_page_keymap(&KeyBindingConfig::default());

        let listed: Vec<_> = keymap
            .bindings()
            .filter_map(|bound| match bound.palette {
                lib_viper::keymap::Palette::Listed {
                    name,
                    takes_argument,
                    ..
                } => Some((bound.command, name, takes_argument)),
                lib_viper::keymap::Palette::Hidden => None,
            })
            .collect();
        assert_eq!(
            listed,
            [
                (NEW, "accounts new", true),
                (EDIT, "accounts edit", true),
                (DELETE, "accounts delete", true),
            ]
        );
        assert_eq!(
            keymap.palette_event(DELETE, " Home Loan "),
            Some(AccountsPageEvent::Command {
                command: "accounts delete".to_string(),
                verb: AccountsCommand::Delete,
                argument: "Home Loan".to_string(),
            })
        );
        assert_eq!(
            keymap.palette_event(EDIT, ""),
            Some(AccountsPageEvent::Command {
                command: "accounts edit".to_string(),
                verb: AccountsCommand::Edit,
                argument: String::new(),
            }),
            "with no Argument the palette still sends the Command, for its refusals"
        );
    }

    #[test]
    fn the_verbs_are_described_by_the_accounts_messages() {
        crate::presenter::test_support::locale();
        let keymap = accounts_page_keymap(&KeyBindingConfig::default());
        let described: Vec<_> = keymap
            .listed()
            .map(|listed| (listed.description)())
            .collect();

        assert_eq!(
            described,
            [
                "add an account: accounts new <account name>",
                "edit an account by name, or the selected row",
                "delete an account by name, or the selected row",
            ]
        );
    }
}
