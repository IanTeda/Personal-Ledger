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
//! `Command::effect` (issue #144's own architecture review, "deepen the command's interface")
//! replaces an earlier `handler: Option<fn(&mut NavState)>` plus a `CommandKind` taxonomy field
//! that was almost entirely decorative (nothing rendered it, one test read it). That shape
//! couldn't express "open a `Shell`-owned dialog" -- `open`/`new` had `handler: None` and
//! `Shell::run_command` matched on `command.name` string literals to special-case them, a
//! typo-prone escape hatch mirrored in the type system by nothing at all. `CommandEffect` is a
//! closed enum instead: every command's effect is real, exhaustively matched, compiler-checked
//! -- the same shape `bin-tui`'s own `Action` enum already uses (ADR-0013), just per-command
//! rather than per-keypress.

use crate::{explorer::ExplorerMode, nav::Noun};

/// What running a command does -- the palette's, and `Shell::run_command`'s, one source of
/// truth. Every variant is a real effect: there's no `None`/`Option` case standing in for "not
/// wired up yet" the way the old `handler` field had, because [`Self::NotYetBuilt`] says so
/// directly instead.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandEffect {
    /// Jump the primary rail to this noun -- `Shell::run_command` calls `NavState::set_noun`
    /// directly; there's no per-noun `fn(&mut NavState)` to indirect through any more.
    Navigate(Noun),
    /// Open the real "1e" file explorer dialog in this mode (`:open`/`:new`, issues #165/#167).
    OpenDialog(ExplorerMode),
    /// `:close`: returns to the "1a" empty state, same as cold start.
    CloseLedger,
    /// No real behaviour behind this command yet (`docs/ux/tui/README.md`'s commitment: "a
    /// command that has no real behaviour yet says so explicitly when run") --
    /// `Shell::run_command` turns this into the status-line flash.
    NotYetBuilt,
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
    pub binding: Option<&'static str>,
    pub effect: CommandEffect,
}

/// Every command the palette can rank and run today, grouped by [`Command::domain`] --
/// `"Dashboard"` first, then every other domain alphabetically, mirroring the TUI's own
/// `commands::DOMAINS` order exactly (`bin-tui/src/popup/command/commands/mod.rs`: "Dashboard
/// first then alphabetical"). One [`CommandEffect::Navigate`] command per rail item (`Noun::ALL`'s
/// own order, each its own single-command domain), plus the one real footer affordance
/// (`crate::rail::context::footer`'s "+ new account · :account new", grouped under "Accounts"
/// alongside its own noun's command) and the file-level `open`/`new`/`close` trio under
/// "Ledger", which has no noun of its own. `account new`'s effect is [`CommandEffect::NotYetBuilt`]
/// -- there is no new-account popup yet (out of scope for this map, issue #144).
pub const COMMANDS: &[Command] = &[
    Command {
        name: "dashboard",
        domain: "Dashboard",
        description: "net worth and budget health",
        binding: Some("g d"),
        effect: CommandEffect::Navigate(Noun::Dashboard),
    },
    Command {
        name: "accounts",
        domain: "Accounts",
        description: "accounts grouped by type",
        binding: Some("g a"),
        effect: CommandEffect::Navigate(Noun::Accounts),
    },
    Command {
        name: "account new",
        domain: "Accounts",
        description: "add an account",
        binding: None,
        effect: CommandEffect::NotYetBuilt,
    },
    Command {
        name: "bills",
        domain: "Bills",
        description: "recurring and upcoming bills",
        binding: Some("g w"),
        effect: CommandEffect::Navigate(Noun::Bills),
    },
    Command {
        name: "budgets",
        domain: "Budgets",
        description: "category limits and actuals",
        binding: Some("g b"),
        effect: CommandEffect::Navigate(Noun::Budgets),
    },
    Command {
        name: "categories",
        domain: "Categories",
        description: "the category tree",
        binding: Some("g c"),
        effect: CommandEffect::Navigate(Noun::Categories),
    },
    Command {
        name: "open",
        domain: "Ledger",
        description: "load a ledger file",
        binding: None,
        effect: CommandEffect::OpenDialog(ExplorerMode::Open),
    },
    Command {
        name: "new",
        domain: "Ledger",
        description: "start a new ledger",
        binding: None,
        effect: CommandEffect::OpenDialog(ExplorerMode::New),
    },
    Command {
        name: "close",
        domain: "Ledger",
        description: "close the open ledger",
        binding: None,
        effect: CommandEffect::CloseLedger,
    },
    Command {
        name: "payees",
        domain: "Payees",
        description: "payees and default categories",
        binding: Some("g p"),
        effect: CommandEffect::Navigate(Noun::Payees),
    },
    Command {
        name: "reports",
        domain: "Reports",
        description: "net worth and variance reports",
        binding: Some("g r"),
        effect: CommandEffect::Navigate(Noun::Reports),
    },
    Command {
        name: "settings",
        domain: "Settings",
        description: "ledger preferences",
        binding: Some("g s"),
        effect: CommandEffect::Navigate(Noun::Settings),
    },
    Command {
        name: "tags",
        domain: "Tags",
        description: "the tags every transaction can carry any number of",
        binding: Some("g t"),
        effect: CommandEffect::Navigate(Noun::Tags),
    },
    Command {
        name: "transactions",
        domain: "Transactions",
        description: "the transaction ledger",
        binding: Some("g l"),
        effect: CommandEffect::Navigate(Noun::Transactions),
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
            assert!(
                COMMANDS
                    .iter()
                    .any(|c| c.effect == CommandEffect::Navigate(noun)),
                "no command carries CommandEffect::Navigate({noun:?})"
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
    fn account_new_has_no_real_behaviour_yet() {
        let account_new = COMMANDS.iter().find(|c| c.name == "account new").unwrap();
        assert_eq!(account_new.effect, CommandEffect::NotYetBuilt);
    }

    #[test]
    fn open_and_new_carry_the_matching_explorer_mode() {
        let open = COMMANDS.iter().find(|c| c.name == "open").unwrap();
        assert_eq!(open.effect, CommandEffect::OpenDialog(ExplorerMode::Open));

        let new = COMMANDS.iter().find(|c| c.name == "new").unwrap();
        assert_eq!(new.effect, CommandEffect::OpenDialog(ExplorerMode::New));
    }

    #[test]
    fn close_carries_the_close_ledger_effect() {
        let close = COMMANDS.iter().find(|c| c.name == "close").unwrap();
        assert_eq!(close.effect, CommandEffect::CloseLedger);
    }
}
