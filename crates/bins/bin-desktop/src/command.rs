//! The command registry driving `crate::palette::Palette`
//! (`docs/ux/desktop/Shell & Navigation/README.md`'s "Command registry" section): "every rail
//! item, every context-rail footer affordance, and every view action registers a command with:
//! command string, description, kind ... optional binding, and a handler." Mirrors the shape of
//! `bin-tui`'s own per-domain registry (`crates/bins/bin-tui/src/popup/command/commands/`,
//! `docs/ux/tui/navigation.md`'s "Commands" section) -- plain, hand-authored data, one flat
//! array rather than domain modules (the desktop registry is small enough not to need
//! splitting the way the TUI's much larger one does), but each `Command` now carries a
//! `domain` field so the palette's own resting-state list can group by it the same way.
//!
//! **This reverses the "1d" spec's original "Results are ranked across kinds, not grouped"**
//! (recorded when the palette was first built, issue #151) -- the user's own later call, for
//! consistency with the TUI's own domain-headed resting-state list
//! (`bin-tui/src/popup/command/mod.rs`'s `Row::Header`), take precedence over that earlier
//! spec text.
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
    /// The palette's resting-state group header (mirroring `bin-tui`'s own per-domain
    /// grouping) -- one domain per noun, plus "Ledger" for the file-level `open`/`new`/`close`
    /// trio, which has no noun of its own.
    pub domain: &'static str,
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

/// `:close`'s handler: returns to the "1a" empty state, same as cold start. `:open`/`:new`
/// have no equivalent handler here -- both open a real `Shell`-owned dialog first (issues
/// #165/#167), state a bare `fn(&mut NavState)` can't reach, so both are `Command::handler:
/// None` below and `Shell::run_command` special-cases them by name instead.
fn close_ledger(nav: &mut NavState) {
    nav.close_ledger();
}

/// Every command the palette can rank and run today, grouped by [`Command::domain`] --
/// `"Dashboard"` first, then every other domain alphabetically, mirroring the TUI's own
/// `commands::DOMAINS` order exactly (`bin-tui/src/popup/command/commands/mod.rs`: "Dashboard
/// first then alphabetical"). One `Navigate` command per rail item (`Noun::ALL`'s own order,
/// each its own single-command domain), plus the one real footer affordance
/// (`crate::rail::context::footer`'s "+ new account · :account new", grouped under "Accounts"
/// alongside its own noun's command) and the file-level `open`/`new`/`close` trio under
/// "Ledger", which has no noun of its own. `account new`'s handler is `None` -- there is no
/// new-account popup yet (out of scope for this map, issue #144), so running it shows the "not
/// yet built" message rather than silently doing nothing.
pub const COMMANDS: &[Command] = &[
    Command {
        name: "dashboard",
        domain: "Dashboard",
        description: "net worth and budget health",
        kind: CommandKind::Navigate,
        binding: Some("g d"),
        handler: Some(goto_dashboard),
    },
    Command {
        name: "accounts",
        domain: "Accounts",
        description: "accounts grouped by type",
        kind: CommandKind::Navigate,
        binding: Some("g a"),
        handler: Some(goto_accounts),
    },
    Command {
        name: "account new",
        domain: "Accounts",
        description: "add an account",
        kind: CommandKind::Create,
        binding: None,
        handler: None,
    },
    Command {
        name: "bills",
        domain: "Bills",
        description: "recurring and upcoming bills",
        kind: CommandKind::Navigate,
        binding: Some("g w"),
        handler: Some(goto_bills),
    },
    Command {
        name: "budgets",
        domain: "Budgets",
        description: "category limits and actuals",
        kind: CommandKind::Navigate,
        binding: Some("g b"),
        handler: Some(goto_budgets),
    },
    Command {
        name: "categories",
        domain: "Categories",
        description: "the category tree",
        kind: CommandKind::Navigate,
        binding: Some("g c"),
        handler: Some(goto_categories),
    },
    Command {
        name: "open",
        domain: "Ledger",
        description: "load a ledger file",
        kind: CommandKind::Run,
        binding: None,
        // `Shell::run_command` intercepts this command by name before ever consulting
        // `handler` -- see `crate::explorer` and the `close_ledger` doc comment above.
        handler: None,
    },
    Command {
        name: "new",
        domain: "Ledger",
        description: "start a new ledger",
        kind: CommandKind::Create,
        binding: None,
        // Same interception as "open" above (issue #167) -- a literal copy of its dialog for
        // now, relabelled.
        handler: None,
    },
    Command {
        name: "close",
        domain: "Ledger",
        description: "close the open ledger",
        kind: CommandKind::Run,
        binding: None,
        handler: Some(close_ledger),
    },
    Command {
        name: "payees",
        domain: "Payees",
        description: "payees and default categories",
        kind: CommandKind::Navigate,
        binding: Some("g p"),
        handler: Some(goto_payees),
    },
    Command {
        name: "reports",
        domain: "Reports",
        description: "net worth and variance reports",
        kind: CommandKind::Navigate,
        binding: Some("g r"),
        handler: Some(goto_reports),
    },
    Command {
        name: "settings",
        domain: "Settings",
        description: "ledger preferences",
        kind: CommandKind::Navigate,
        binding: Some("g s"),
        handler: Some(goto_settings),
    },
    Command {
        name: "tags",
        domain: "Tags",
        description: "the tags every transaction can carry any number of",
        kind: CommandKind::Navigate,
        binding: Some("g t"),
        handler: Some(goto_tags),
    },
    Command {
        name: "transactions",
        domain: "Transactions",
        description: "the transaction ledger",
        kind: CommandKind::Navigate,
        binding: Some("g l"),
        handler: Some(goto_transactions),
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
    fn domains_are_contiguous_dashboard_first_then_alphabetical() {
        // Mirrors bin-tui's own `dashboard_is_first_and_the_rest_are_alphabetical` invariant --
        // the palette's resting-state grouping (`Palette::rows`) assumes each domain's commands
        // sit together, never split across two separate runs.
        let mut order: Vec<&str> = Vec::new();
        for command in COMMANDS {
            if order.last() != Some(&command.domain) {
                assert!(
                    !order.contains(&command.domain),
                    "domain {:?} is split across non-adjacent commands",
                    command.domain
                );
                order.push(command.domain);
            }
        }
        assert_eq!(order[0], "Dashboard");
        let rest = &order[1..];
        let mut sorted_rest = rest.to_vec();
        sorted_rest.sort_unstable();
        assert_eq!(rest, sorted_rest.as_slice());
    }

    #[test]
    fn account_new_has_no_handler_yet() {
        let account_new = COMMANDS.iter().find(|c| c.name == "account new").unwrap();
        assert!(account_new.handler.is_none());
    }

    #[test]
    fn open_and_new_have_no_navstate_handler_shell_owns_their_behaviour() {
        // `Shell::run_command` special-cases both by name to launch the real file explorer
        // dialog (issues #165/#167) -- a `Command::handler` can't reach `Shell`-level state,
        // only `NavState`.
        for name in ["open", "new"] {
            let command = COMMANDS.iter().find(|c| c.name == name).unwrap();
            assert!(command.handler.is_none(), "{name:?} should have no handler");
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
