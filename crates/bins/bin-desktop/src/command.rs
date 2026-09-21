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
    /// An `accounts <verb> [<account name>]` command: `Shell::run_command` jumps to the Accounts
    /// page and opens the matching dialog, resolving the typed name (see [`split_input`]).
    Accounts(AccountsVerb),
    /// No real behaviour behind this command yet (`docs/ux/tui/README.md`'s commitment: "a
    /// command that has no real behaviour yet says so explicitly when run") --
    /// `Shell::run_command` turns this into the status-line flash.
    NotYetBuilt,
}

/// The Accounts verbs the palette understands -- the same three the page's `n`/`e`/`d` keys and
/// buttons reach, through the same dialog-opening handlers. The TUI's `account off`/`on`/`check`
/// have no desktop counterpart: the desktop design has no active flag and no balance checks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccountsVerb {
    /// `accounts new [<account name>]`: the Add dialog, with Name pre-filled when given.
    New,
    /// `accounts edit [<account name>]`: the Edit dialog for the named (or selected) account.
    Edit,
    /// `accounts delete [<account name>]`: the Delete dialog for the named (or selected) account.
    Delete,
}

/// The palette's resting-state group a command sits under: one per noun, plus `Ledger` for the
/// file-level `open`/`new`/`close` trio, which has no noun of its own. The header text is a
/// Message; the variants are the stable ids.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Domain {
    Dashboard,
    Accounts,
    Bills,
    Budgets,
    Categories,
    Ledger,
    Payees,
    Reports,
    Settings,
    Tags,
    Transactions,
}

impl Domain {
    /// The header text in the Locale in effect, in sentence case.
    pub fn label(self) -> String {
        match self {
            Domain::Dashboard => Noun::Dashboard.label(),
            Domain::Accounts => Noun::Accounts.label(),
            Domain::Bills => Noun::Bills.label(),
            Domain::Budgets => Noun::Budgets.label(),
            Domain::Categories => Noun::Categories.label(),
            Domain::Ledger => crate::msg::desktop_command_domain_ledger(),
            Domain::Payees => Noun::Payees.label(),
            Domain::Reports => Noun::Reports.label(),
            Domain::Settings => Noun::Settings.label(),
            Domain::Tags => Noun::Tags.label(),
            Domain::Transactions => Noun::Transactions.label(),
        }
    }
}

/// One command: the palette's own unit of data. `binding` is a plain display string (unlike the
/// TUI's `crossterm`-typed `Chord`, since `gpui`'s key model has no equivalent to render) --
/// `None` renders as an em dash in the palette, matching the TUI's `Chord::NONE`.
pub struct Command {
    /// The command as typed: a stable English id, never translated.
    pub name: &'static str,
    /// The palette's resting-state group (mirroring `bin-tui`'s own per-domain grouping).
    pub domain: Domain,
    /// The description Message, resolved in the Locale in effect when called.
    pub description: fn() -> String,
    pub binding: Option<&'static str>,
    pub effect: CommandEffect,
}

impl Command {
    /// Whether text typed after the command name is an argument (`accounts delete Home Loan`)
    /// rather than more of the query -- see [`split_input`].
    pub fn takes_argument(&self) -> bool {
        matches!(self.effect, CommandEffect::Accounts(_))
    }
}

/// Splits palette input into the command query and its argument text: when the input starts with
/// the full name of a command that takes an argument (case-insensitively, followed by a space or
/// the end of the input), that name is the query and the rest, trimmed, is the argument; any
/// other input is all query. The longest such name wins. Free text after the name may contain
/// spaces (`accounts new Rainy Day`).
pub fn split_input(input: &str) -> (&str, &str) {
    let best = COMMANDS
        .iter()
        .filter(|command| command.takes_argument())
        .filter(|command| {
            input
                .get(..command.name.len())
                .is_some_and(|head| head.eq_ignore_ascii_case(command.name))
                && input[command.name.len()..]
                    .chars()
                    .next()
                    .is_none_or(|next| next == ' ')
        })
        .max_by_key(|command| command.name.len());
    match best {
        Some(command) => (
            &input[..command.name.len()],
            input[command.name.len()..].trim(),
        ),
        None => (input, ""),
    }
}

/// Every command the palette can rank and run today, grouped by [`Command::domain`] --
/// `Dashboard` first, then every other domain alphabetically, mirroring the TUI's own
/// `commands::DOMAINS` order exactly (`bin-tui/src/popup/command/commands/mod.rs`: "Dashboard
/// first then alphabetical"). One [`CommandEffect::Navigate`] command per rail item (`Noun::ALL`'s
/// own order, each its own single-command domain), plus the one real footer affordance
/// (`crate::rail::context::footer`'s "+ new account · :accounts new", grouped under "Accounts"
/// alongside its own noun's command) and the file-level `open`/`new`/`close` trio under
/// `Ledger`, which has no noun of its own. The three `accounts <verb>` commands take a typed
/// account name (see [`split_input`]).
pub const COMMANDS: &[Command] = &[
    Command {
        name: "dashboard",
        domain: Domain::Dashboard,
        description: crate::msg::desktop_command_dashboard_description,
        binding: Some("g d"),
        effect: CommandEffect::Navigate(Noun::Dashboard),
    },
    Command {
        name: "accounts",
        domain: Domain::Accounts,
        description: crate::msg::desktop_command_accounts_description,
        binding: Some("g a"),
        effect: CommandEffect::Navigate(Noun::Accounts),
    },
    Command {
        name: "accounts new",
        domain: Domain::Accounts,
        description: accounts_new_description,
        binding: Some("n"),
        effect: CommandEffect::Accounts(AccountsVerb::New),
    },
    Command {
        name: "accounts edit",
        domain: Domain::Accounts,
        description: crate::msg::desktop_command_accounts_edit_description,
        binding: Some("e"),
        effect: CommandEffect::Accounts(AccountsVerb::Edit),
    },
    Command {
        name: "accounts delete",
        domain: Domain::Accounts,
        description: crate::msg::desktop_command_accounts_delete_description,
        binding: Some("d"),
        effect: CommandEffect::Accounts(AccountsVerb::Delete),
    },
    Command {
        name: "bills",
        domain: Domain::Bills,
        description: crate::msg::desktop_command_bills_description,
        binding: Some("g w"),
        effect: CommandEffect::Navigate(Noun::Bills),
    },
    Command {
        name: "budgets",
        domain: Domain::Budgets,
        description: crate::msg::desktop_command_budgets_description,
        binding: Some("g b"),
        effect: CommandEffect::Navigate(Noun::Budgets),
    },
    Command {
        name: "categories",
        domain: Domain::Categories,
        description: crate::msg::desktop_command_categories_description,
        binding: Some("g c"),
        effect: CommandEffect::Navigate(Noun::Categories),
    },
    Command {
        name: "open",
        domain: Domain::Ledger,
        description: crate::msg::desktop_command_open_description,
        binding: None,
        effect: CommandEffect::OpenDialog(ExplorerMode::Open),
    },
    Command {
        name: "new",
        domain: Domain::Ledger,
        description: crate::msg::desktop_command_new_description,
        binding: None,
        effect: CommandEffect::OpenDialog(ExplorerMode::New),
    },
    Command {
        name: "close",
        domain: Domain::Ledger,
        description: crate::msg::desktop_command_close_description,
        binding: None,
        effect: CommandEffect::CloseLedger,
    },
    Command {
        name: "payees",
        domain: Domain::Payees,
        description: crate::msg::desktop_command_payees_description,
        binding: Some("g p"),
        effect: CommandEffect::Navigate(Noun::Payees),
    },
    Command {
        name: "reports",
        domain: Domain::Reports,
        description: crate::msg::desktop_command_reports_description,
        binding: Some("g r"),
        effect: CommandEffect::Navigate(Noun::Reports),
    },
    Command {
        name: "settings",
        domain: Domain::Settings,
        description: crate::msg::desktop_command_settings_description,
        binding: Some("g s"),
        effect: CommandEffect::Navigate(Noun::Settings),
    },
    Command {
        name: "tags",
        domain: Domain::Tags,
        description: crate::msg::desktop_command_tags_description,
        binding: Some("g t"),
        effect: CommandEffect::Navigate(Noun::Tags),
    },
    Command {
        name: "transactions",
        domain: Domain::Transactions,
        description: crate::msg::desktop_command_transactions_description,
        binding: Some("g l"),
        effect: CommandEffect::Navigate(Noun::Transactions),
    },
];

fn accounts_new_description() -> String {
    crate::msg::desktop_command_accounts_new_description("accounts new")
}

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
        crate::locale::init_for_tests();
        for command in COMMANDS {
            assert!(!command.name.is_empty());
            assert!(!(command.description)().is_empty());
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
        let mut order: Vec<Domain> = Vec::new();
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
        assert_eq!(order[0], Domain::Dashboard);
        // The domains' stable ids (their variant names) sort alphabetically, whatever the Locale
        // calls them.
        let rest = &order[1..];
        let mut sorted_rest = rest.to_vec();
        sorted_rest.sort_unstable_by_key(|domain| format!("{domain:?}"));
        assert_eq!(rest, sorted_rest.as_slice());
    }

    #[test]
    fn the_accounts_verbs_carry_their_effects_and_take_an_argument() {
        for (name, verb) in [
            ("accounts new", AccountsVerb::New),
            ("accounts edit", AccountsVerb::Edit),
            ("accounts delete", AccountsVerb::Delete),
        ] {
            let command = COMMANDS.iter().find(|c| c.name == name).unwrap();
            assert_eq!(command.effect, CommandEffect::Accounts(verb));
            assert!(command.takes_argument());
        }
        let accounts = COMMANDS.iter().find(|c| c.name == "accounts").unwrap();
        assert!(!accounts.takes_argument(), "the bare noun jump takes none");
    }

    #[test]
    fn split_input_separates_the_argument_after_a_full_command_name() {
        assert_eq!(
            split_input("accounts new Rainy Day"),
            ("accounts new", "Rainy Day")
        );
        assert_eq!(
            split_input("accounts delete   Home Loan  "),
            ("accounts delete", "Home Loan")
        );
        assert_eq!(split_input("accounts edit"), ("accounts edit", ""));
        assert_eq!(split_input("accounts edit "), ("accounts edit", ""));
    }

    #[test]
    fn split_input_matches_the_command_name_case_insensitively_keeping_the_arguments_case() {
        assert_eq!(
            split_input("Accounts New ANZ Offset"),
            ("Accounts New", "ANZ Offset")
        );
    }

    #[test]
    fn split_input_leaves_everything_else_as_the_query() {
        assert_eq!(split_input(""), ("", ""));
        assert_eq!(split_input("accounts"), ("accounts", ""));
        assert_eq!(split_input("accounts del"), ("accounts del", ""));
        assert_eq!(split_input("accounts deleted"), ("accounts deleted", ""));
        assert_eq!(split_input("open something"), ("open something", ""));
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
