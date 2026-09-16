//! The command registry driving `crate::palette::Palette`
//! (`docs/ux/desktop/Shell & Navigation/README.md`'s "Command registry" section): "every rail
//! item, every context-rail footer affordance, and every view action registers a command with:
//! command string, description, kind ... optional binding, and a handler." Mirrors the shape of
//! `bin-tui`'s own per-domain registry (`crates/bins/bin-tui/src/popup/command/commands/`,
//! `docs/ux/tui/navigation.md`'s "Commands" section) -- plain, hand-authored data, one flat list
//! rather than domain modules, since the desktop registry is deliberately never grouped by
//! domain (README's "1d" spec: "Results are ranked across kinds, not grouped").
//!
//! Unlike the TUI's registry, `Command` carries a real `handler` here, matching this document's
//! own spec rather than deferring dispatch to a special case in `Shell`. `handler: None` is the
//! "not yet built" case (`docs/ux/tui/README.md`'s commitment: "a command that has no real
//! behaviour yet says so explicitly when run") -- `Shell::run_command` is what turns that into
//! the status-line flash, this module only records which commands qualify.

use crate::nav::{NavState, Noun};

/// The registry's own taxonomy (README's "Command registry" section) -- not yet rendered
/// anywhere (the "1d" result row shows name/description/binding only), kept because it's part
/// of the literal spec for what a `Command` carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandKind {
    Navigate,
    Create,
    Run,
    #[allow(dead_code)]
    Setting,
}

/// One command: the palette's own unit of data. `binding` is a plain display string (unlike the
/// TUI's `crossterm`-typed `Chord`, since `gpui`'s key model has no equivalent to render) --
/// `None` renders as an em dash in the palette, matching the TUI's `Chord::NONE`.
pub struct Command {
    pub name: &'static str,
    pub description: &'static str,
    #[allow(dead_code)]
    pub kind: CommandKind,
    pub binding: Option<&'static str>,
    pub handler: Option<fn(&mut NavState)>,
}

/// A `fn(&mut NavState)` per noun -- `Command::handler` is a bare function pointer (matching
/// the spec literally), which can't capture a `noun` value the way a closure could, so each
/// noun gets its own one-line function instead of one generic one.
fn goto_dashboard(nav: &mut NavState) {
    nav.set_noun(Noun::Dashboard);
}
fn goto_transactions(nav: &mut NavState) {
    nav.set_noun(Noun::Transactions);
}
fn goto_accounts(nav: &mut NavState) {
    nav.set_noun(Noun::Accounts);
}
fn goto_categories(nav: &mut NavState) {
    nav.set_noun(Noun::Categories);
}
fn goto_payees(nav: &mut NavState) {
    nav.set_noun(Noun::Payees);
}
fn goto_tags(nav: &mut NavState) {
    nav.set_noun(Noun::Tags);
}
fn goto_bills(nav: &mut NavState) {
    nav.set_noun(Noun::Bills);
}
fn goto_budgets(nav: &mut NavState) {
    nav.set_noun(Noun::Budgets);
}
fn goto_reports(nav: &mut NavState) {
    nav.set_noun(Noun::Reports);
}
fn goto_settings(nav: &mut NavState) {
    nav.set_noun(Noun::Settings);
}

/// `:open`/`:new`'s stand-in handler (`docs/ux/desktop/Shell & Navigation/README.md`'s "1e"
/// file explorer and "Empty state" section) -- real file I/O is out of scope for this map
/// (issue #144's "Out of scope"), so both commands just flip `NavState::ledger_open` on,
/// enough to demonstrate the "1a" empty state giving way to the populated `Dashboard`.
fn open_ledger(nav: &mut NavState) {
    nav.open_ledger();
}

/// `:close`'s handler: returns to the "1a" empty state, same as cold start.
fn close_ledger(nav: &mut NavState) {
    nav.close_ledger();
}

/// Every command the palette can rank and run today: one `Navigate` command per rail item
/// (`Noun::ALL`'s own order), plus the one real footer affordance
/// (`crate::rail::context::footer`'s "+ new account · :account new"). `account new`'s handler
/// is `None` -- there is no new-account popup yet (out of scope for this map, issue #144), so
/// running it shows the "not yet built" message rather than silently doing nothing.
pub const COMMANDS: &[Command] = &[
    Command {
        name: "dashboard",
        description: "net worth and budget health",
        kind: CommandKind::Navigate,
        binding: Some("g d"),
        handler: Some(goto_dashboard),
    },
    Command {
        name: "transactions",
        description: "the transaction ledger",
        kind: CommandKind::Navigate,
        binding: Some("g l"),
        handler: Some(goto_transactions),
    },
    Command {
        name: "accounts",
        description: "accounts grouped by type",
        kind: CommandKind::Navigate,
        binding: Some("g a"),
        handler: Some(goto_accounts),
    },
    Command {
        name: "categories",
        description: "the category tree",
        kind: CommandKind::Navigate,
        binding: Some("g c"),
        handler: Some(goto_categories),
    },
    Command {
        name: "payees",
        description: "payees and default categories",
        kind: CommandKind::Navigate,
        binding: Some("g p"),
        handler: Some(goto_payees),
    },
    Command {
        name: "tags",
        description: "the tags every transaction can carry any number of",
        kind: CommandKind::Navigate,
        binding: Some("g t"),
        handler: Some(goto_tags),
    },
    Command {
        name: "bills",
        description: "recurring and upcoming bills",
        kind: CommandKind::Navigate,
        binding: Some("g w"),
        handler: Some(goto_bills),
    },
    Command {
        name: "budgets",
        description: "category limits and actuals",
        kind: CommandKind::Navigate,
        binding: Some("g b"),
        handler: Some(goto_budgets),
    },
    Command {
        name: "reports",
        description: "net worth and variance reports",
        kind: CommandKind::Navigate,
        binding: Some("g r"),
        handler: Some(goto_reports),
    },
    Command {
        name: "settings",
        description: "ledger preferences",
        kind: CommandKind::Navigate,
        binding: Some("g s"),
        handler: Some(goto_settings),
    },
    Command {
        name: "account new",
        description: "add an account",
        kind: CommandKind::Create,
        binding: None,
        handler: None,
    },
    Command {
        name: "open",
        description: "load a ledger file",
        kind: CommandKind::Run,
        binding: None,
        handler: Some(open_ledger),
    },
    Command {
        name: "new",
        description: "start a new ledger",
        kind: CommandKind::Create,
        binding: None,
        handler: Some(open_ledger),
    },
    Command {
        name: "close",
        description: "close the open ledger",
        kind: CommandKind::Run,
        binding: None,
        handler: Some(close_ledger),
    },
];

/// Every registered command, in registration order -- the palette's resting-state (empty
/// query) order, and the order ties fall back to once ranked.
pub fn all() -> impl Iterator<Item = &'static Command> {
    COMMANDS.iter()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_command_has_a_non_empty_name_and_description() {
        for command in COMMANDS {
            assert!(!command.name.is_empty());
            assert!(!command.description.is_empty());
        }
    }

    #[test]
    fn every_noun_has_a_navigate_command() {
        for noun in Noun::ALL {
            let label = format!("{noun:?}").to_lowercase();
            assert!(
                COMMANDS
                    .iter()
                    .any(|c| c.kind == CommandKind::Navigate && c.name == label),
                "no navigate command named {label:?}"
            );
        }
    }

    #[test]
    fn account_new_has_no_handler_yet() {
        let account_new = COMMANDS.iter().find(|c| c.name == "account new").unwrap();
        assert!(account_new.handler.is_none());
    }

    #[test]
    fn open_and_new_open_the_ledger() {
        for name in ["open", "new"] {
            let mut nav = NavState::new();
            assert!(!nav.ledger_open());

            let command = COMMANDS.iter().find(|c| c.name == name).unwrap();
            (command.handler.expect("open/new have a handler"))(&mut nav);

            assert!(nav.ledger_open(), "{name:?} should open the ledger");
        }
    }

    #[test]
    fn close_closes_the_ledger() {
        let mut nav = NavState::new();
        nav.open_ledger();

        let close = COMMANDS.iter().find(|c| c.name == "close").unwrap();
        (close.handler.expect("close has a handler"))(&mut nav);

        assert!(!nav.ledger_open());
    }

    #[test]
    fn navigate_handlers_actually_move_to_their_own_noun() {
        let mut nav = NavState::new();
        for noun in Noun::ALL {
            let label = format!("{noun:?}").to_lowercase();
            let command = COMMANDS
                .iter()
                .find(|c| c.name == label)
                .expect("every noun has a command");
            (command.handler.expect("navigate commands have a handler"))(&mut nav);
            assert_eq!(nav.noun(), noun);
        }
    }
}
