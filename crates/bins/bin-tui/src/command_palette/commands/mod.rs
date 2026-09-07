//! The command list the palette renders — grounded in what `crate::screen` actually
//! implements today (Units, Categories, Accounts, Transactions, Payees, Balance Checks,
//! Budgets, Reports, CSV import), grouped one file per domain, rather than
//! `docs/ux/tui/README.md`'s aspirational grammar where nothing yet backs it (`sync`,
//! `price`).
//!
//! This is data only: no action registry, no `:help`/footer/keymap generation, and no key
//! dispatch. `Chord` models each command's eventual binding as typed data so a later dispatch
//! ticket is additive (it consumes the same value already sitting here) rather than needing
//! its own binding representation — but nothing here matches a live `KeyEvent` against it yet.

mod accounts;
mod balance_checks;
mod budgets;
mod categories;
mod dashboard;
mod help;
mod payees;
mod reports;
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

/// One command: its `:name` (excluding the leading `:`, added when rendering), its eventual
/// binding, and its description.
pub struct Command {
    pub name: &'static str,
    pub chord: Chord,
    pub description: &'static str,
}

/// One domain's commands, grouped for the palette's resting-state list.
pub struct Domain {
    pub name: &'static str,
    pub commands: &'static [Command],
}

/// Every domain, Dashboard first then alphabetical — the palette's resting-state order.
pub const DOMAINS: &[Domain] = &[
    Domain {
        name: "Dashboard",
        commands: dashboard::COMMANDS,
    },
    Domain {
        name: "Accounts",
        commands: accounts::COMMANDS,
    },
    Domain {
        name: "Balance Checks",
        commands: balance_checks::COMMANDS,
    },
    Domain {
        name: "Budgets",
        commands: budgets::COMMANDS,
    },
    Domain {
        name: "Categories",
        commands: categories::COMMANDS,
    },
    Domain {
        name: "Help",
        commands: help::COMMANDS,
    },
    Domain {
        name: "Payees",
        commands: payees::COMMANDS,
    },
    Domain {
        name: "Reports",
        commands: reports::COMMANDS,
    },
    Domain {
        name: "Transactions",
        commands: transactions::COMMANDS,
    },
    Domain {
        name: "Units",
        commands: units::COMMANDS,
    },
];

/// Total command count across every domain, for the palette's match-count row.
pub fn total_commands() -> usize {
    DOMAINS.iter().map(|domain| domain.commands.len()).sum()
}

/// Every command with its owning domain's name, in `DOMAINS` order — the flat shape a
/// filtered (non-resting) palette view renders from.
pub fn all() -> impl Iterator<Item = (&'static str, &'static Command)> {
    DOMAINS.iter().flat_map(|domain| {
        domain
            .commands
            .iter()
            .map(move |command| (domain.name, command))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_domain_has_at_least_one_command() {
        for domain in DOMAINS {
            assert!(
                !domain.commands.is_empty(),
                "{} has no commands",
                domain.name
            );
        }
    }

    #[test]
    fn total_commands_matches_the_flat_count() {
        assert_eq!(total_commands(), all().count());
    }

    #[test]
    fn dashboard_is_first_and_the_rest_are_alphabetical() {
        assert_eq!(DOMAINS[0].name, "Dashboard");
        let rest: Vec<&str> = DOMAINS[1..].iter().map(|domain| domain.name).collect();
        let mut sorted = rest.clone();
        sorted.sort_unstable();
        assert_eq!(rest, sorted);
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
