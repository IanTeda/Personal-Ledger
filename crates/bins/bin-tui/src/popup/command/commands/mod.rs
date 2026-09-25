//! The command list the command popup renders — grounded in what `crate::screen` actually
//! implements today (Units, Categories, Accounts, Transactions, Payees, Balance Checks,
//! Budgets, Reports, CSV import), grouped one file per domain, rather than
//! `docs/ux/tui/README.md`'s aspirational grammar where nothing yet backs it (`sync`,
//! `price`).
//!
//! This is data only: no action registry, no `:help`/footer/keymap generation, and (bar
//! `Shell` special-casing `Enter` on the 6 commands with real content behind them — `unit`,
//! `unit new <code> <type>`, `unit edit <code>`, `unit delete <code>`, `dashboard`,
//! `settings` — every other command showing a "not yet built" message instead, per
//! `CommandPopup`'s `not_yet_built` state) no key dispatch. `Chord` models each command's
//! eventual binding as typed data so a later dispatch ticket is additive (it consumes the
//! same value already sitting here) rather than needing its own binding representation. `Arg`
//! does the same for a command's argument-preview row: fixed fake-data content now, a
//! resolver against real `lib-database` data later.

mod accounts;
mod balance_checks;
mod budgets;
mod categories;
mod dashboard;
mod help;
mod payees;
mod quit;
mod reports;
mod settings;
mod tags;
mod transactions;
mod units;

use std::fmt;

use crossterm::event::KeyCode;

/// A key chord as display data only. `Chord::NONE` renders as `—` for a command with no
/// binding yet (e.g. a specific report, reached only via its screen's own picker).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Chord(pub &'static [KeyCode]);

impl Chord {
    pub const NONE: Chord = Chord(&[]);
}

impl fmt::Display for Chord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.0.is_empty() {
            return write!(f, "—");
        }
        for (idx, code) in self.0.iter().enumerate() {
            if idx > 0 {
                write!(f, " ")?;
            }
            match code {
                KeyCode::Char(c) => write!(f, "{c}")?,
                KeyCode::Enter => write!(f, "enter")?,
                KeyCode::Esc => write!(f, "esc")?,
                other => write!(f, "{other:?}")?,
            }
        }
        Ok(())
    }
}

/// A command's stable id: what the shell dispatches on. Separate from the display text
/// (`description` and each `Arg`'s own `preview`), which are Messages, so changing that text
/// cannot change what a command does. The typed `name` and the `<...>`/`[...]` tokens inside it
/// stay stable English the user types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CommandId {
    // accounts
    Account,
    AccountNew,
    AccountEdit,
    AccountDelete,
    AccountOff,
    AccountOn,
    AccountCheck,
    // balance_checks
    CheckList,
    CheckNew,
    CheckEdit,
    CheckDelete,
    CheckImport,
    // budgets
    BudgetList,
    BudgetNew,
    BudgetEdit,
    BudgetDelete,
    // categories
    Category,
    CategoryNew,
    CategoryEdit,
    CategoryMove,
    CategoryRename,
    CategoryMerge,
    CategoryArchive,
    CategoryTree,
    // dashboard
    Dashboard,
    // help
    Help,
    // payees
    Payee,
    PayeeNew,
    PayeeEdit,
    PayeeRename,
    PayeeMatch,
    PayeeMatchAdd,
    PayeeDefault,
    PayeeOff,
    PayeeOn,
    PayeeDelete,
    // quit
    Quit,
    // reports
    ReportList,
    ReportAccountBalance,
    ReportCategoryTotal,
    ReportPayeeTotal,
    ReportBudgetVariance,
    ReportBalanceCheckVariance,
    // settings
    Settings,
    // tags
    Tag,
    TagNew,
    TagEdit,
    TagOff,
    TagOn,
    TagDelete,
    // transactions
    TxnRecent,
    TxnNew,
    TxnEdit,
    TxnDelete,
    // units
    Unit,
    UnitNew,
    UnitEdit,
    UnitDelete,
}

/// One command: its stable id, its `:name` (excluding the leading `:`, added when rendering), its eventual
/// binding, its description, and the arguments its name's `<...>`/`[...]` placeholders name —
/// empty for a command with none.
pub struct Command {
    pub id: CommandId,
    pub name: &'static str,
    pub chord: Chord,
    /// The description Message, resolved in the Locale in effect when called — a function rather
    /// than a string so this table stays a `const`, mirroring `bin-desktop`'s own palette.
    pub description: fn() -> String,
    pub args: &'static [Arg],
}

/// One argument a command takes, feeding the command popup's single combined preview row
/// (`CommandPopup::arg_preview`) for whichever command is currently highlighted. Still fixed
/// content, now behind a Message rather than a literal — no resolver against real data yet;
/// `preview` may be as rich as the command needs (e.g. a budget's current actual-vs-limit
/// alongside its category), not structurally split between "the argument's value" and "context
/// about it".
pub struct Arg {
    /// The argument's own token as the command's `name` spells it (`<acct>`, `[period]`). Command
    /// syntax, not a Message: it must read exactly as the usage line the user types against, and
    /// that line is a stable English id.
    pub placeholder: &'static str,

    /// The example Message shown beside the token, resolved in the Locale in effect when called.
    pub preview: fn() -> String,
}

/// One domain's commands, grouped for the command popup's resting-state list. The header text is a
/// Message; `DOMAINS`' own order is what fixes the grouping, so nothing navigates on this name.
pub struct Domain {
    pub name: fn() -> String,
    pub commands: &'static [Command],
}

/// The chord that holds inactive rows visible on a list, named by the `off`/`on` commands' own
/// descriptions and previews. Display text for the hint, not a binding this table dispatches on.
const SHOW_INACTIVE_CHORD: &str = "za";

/// The description every `<noun> off` command shares, word for word.
fn off_description() -> String {
    crate::msg::tui_command_off_description(SHOW_INACTIVE_CHORD)
}

/// The preview every argument that resolves against the list's own selection shares.
fn list_selection_preview() -> String {
    crate::msg::tui_command_preview_list_selection()
}

/// The same, for an `on` command: its effect is only visible with the inactive rows showing.
fn list_selection_inactive_preview() -> String {
    crate::msg::tui_command_preview_list_selection_inactive(SHOW_INACTIVE_CHORD)
}

/// Every domain, Dashboard first then alphabetical — the command popup's resting-state order.
pub const DOMAINS: &[Domain] = &[
    Domain {
        name: lib_locale::msg::nav_dashboard,
        commands: dashboard::COMMANDS,
    },
    Domain {
        name: lib_locale::msg::nav_accounts,
        commands: accounts::COMMANDS,
    },
    Domain {
        name: crate::msg::tui_view_balance_checks_title,
        commands: balance_checks::COMMANDS,
    },
    Domain {
        name: lib_locale::msg::nav_budgets,
        commands: budgets::COMMANDS,
    },
    Domain {
        name: lib_locale::msg::nav_categories,
        commands: categories::COMMANDS,
    },
    Domain {
        name: lib_locale::msg::nav_help,
        commands: help::COMMANDS,
    },
    Domain {
        name: lib_locale::msg::nav_payees,
        commands: payees::COMMANDS,
    },
    Domain {
        name: crate::msg::tui_command_domain_quit,
        commands: quit::COMMANDS,
    },
    Domain {
        name: lib_locale::msg::nav_reports,
        commands: reports::COMMANDS,
    },
    Domain {
        name: lib_locale::msg::nav_settings,
        commands: settings::COMMANDS,
    },
    Domain {
        name: lib_locale::msg::nav_tags,
        commands: tags::COMMANDS,
    },
    Domain {
        name: lib_locale::msg::nav_transactions,
        commands: transactions::COMMANDS,
    },
    Domain {
        name: crate::msg::tui_view_units_title,
        commands: units::COMMANDS,
    },
];

/// Total command count across every domain, for the command popup's match-count row.
pub fn total_commands() -> usize {
    DOMAINS.iter().map(|domain| domain.commands.len()).sum()
}

/// Every command with its owning domain's header text in the Locale in effect, in `DOMAINS`
/// order — the flat shape a filtered (non-resting) command popup view renders from.
pub fn all() -> impl Iterator<Item = (String, &'static Command)> {
    DOMAINS.iter().flat_map(|domain| {
        let name = (domain.name)();
        domain
            .commands
            .iter()
            .map(move |command| (name.clone(), command))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_domain_has_at_least_one_command() {
        crate::locale::init_for_tests();
        for domain in DOMAINS {
            assert!(
                !domain.commands.is_empty(),
                "{} has no commands",
                (domain.name)()
            );
        }
    }

    #[test]
    fn every_command_has_its_own_id() {
        crate::locale::init_for_tests();
        let mut seen = std::collections::HashSet::new();
        for (domain, command) in all() {
            assert!(
                seen.insert(command.id),
                "{domain}: `{}` reuses the id {:?}",
                command.name,
                command.id
            );
        }
        assert_eq!(seen.len(), total_commands());
    }

    #[test]
    fn total_commands_matches_the_flat_count() {
        crate::locale::init_for_tests();
        assert_eq!(total_commands(), all().count());
    }

    /// Checked against the source Locale's own headers: `DOMAINS`' order is fixed here, and a
    /// translated header never reorders the list.
    #[test]
    fn dashboard_is_first_and_the_rest_are_alphabetical() {
        crate::locale::init_for_tests();
        assert_eq!((DOMAINS[0].name)(), "Dashboard");
        let rest: Vec<String> = DOMAINS[1..].iter().map(|domain| (domain.name)()).collect();
        let mut sorted = rest.clone();
        sorted.sort_unstable();
        assert_eq!(rest, sorted);
    }

    /// Every description and preview resolves to a real Message, in every supported Locale — a
    /// missing id would otherwise only show as the id itself, on screen.
    #[test]
    fn every_description_and_preview_resolves_in_every_locale() {
        for locale in [
            lib_locale::Locale::EnUs,
            lib_locale::Locale::EnGb,
            lib_locale::Locale::EnAu,
            lib_locale::Locale::EnXa,
        ] {
            crate::locale::init_for_tests();
            lib_locale::with_locale(locale, || {
                for (_, command) in all() {
                    let description = (command.description)();
                    assert!(
                        !description.is_empty() && !description.contains("tui-command"),
                        "{locale:?}: `{}` has no description Message",
                        command.name
                    );
                    for arg in command.args {
                        let preview = (arg.preview)();
                        assert!(
                            !preview.is_empty() && !preview.contains("tui-command"),
                            "{locale:?}: `{}`'s `{}` has no preview Message",
                            command.name,
                            arg.placeholder
                        );
                    }
                }
            });
        }
    }

    /// Each argument's token is the one its command's own usage line spells, so the preview row and
    /// the usage line can never drift apart.
    #[test]
    fn every_argument_token_appears_in_its_commands_usage_line() {
        for (_, command) in all() {
            for arg in command.args {
                assert!(
                    command.name.contains(arg.placeholder),
                    "`{}` does not spell `{}`",
                    command.name,
                    arg.placeholder
                );
            }
        }
    }

    #[test]
    fn chord_none_displays_as_an_em_dash() {
        assert_eq!(Chord::NONE.to_string(), "—");
    }

    #[test]
    fn a_two_key_chord_displays_space_separated() {
        let chord = Chord(&[KeyCode::Char('g'), KeyCode::Char('u')]);
        assert_eq!(chord.to_string(), "g u");
    }
}
