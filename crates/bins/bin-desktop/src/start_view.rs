//! Where the Desktop lands as it starts: Start view (ADR-0053), the `start_view` key the TUI reads
//! too. `lib_core::StartView` resolves `last` to the View remembered at the last quit; this names
//! the Desktop's nouns and turns the result into the [`Noun`] the Shell starts on.
//!
//! Every noun but the Dashboard and Settings needs a Ledger ([`needs_ledger`]), so with none open
//! the Desktop lands on the Dashboard, whose place shows the no Ledger open content. A noun the
//! Desktop doesn't have, the TUI's `units` among them, lands on the Dashboard too.

use lib_core::StartView;

use crate::navigation::nav::{Noun, needs_ledger};

/// Every noun Start view can name, by the word the file spells, in the primary rail's order.
const NOUNS: [(&str, Noun); 13] = [
    (StartView::DASHBOARD, Noun::Dashboard),
    ("transactions", Noun::Transactions),
    ("documents", Noun::Documents),
    ("notifications", Noun::Notifications),
    ("cash", Noun::Cash),
    ("inventory", Noun::Inventory),
    ("loans", Noun::Loans),
    ("credit-cards", Noun::CreditCards),
    ("investments", Noun::Investments),
    ("bills", Noun::Bills),
    ("budgets", Noun::Budgets),
    ("reports", Noun::Reports),
    (StartView::SETTINGS, Noun::Settings),
];

/// Start view's choices, in the order Settings › General shows them, by the value the file takes.
/// The placeholder Views are left out, as the TUI leaves out its own, though a file naming one
/// still lands on it.
pub const CHOICES: [&str; 7] = [
    StartView::DASHBOARD,
    StartView::LAST,
    StartView::SETTINGS,
    "transactions",
    "documents",
    "bills",
    "budgets",
];

/// A Start view choice's label, by its value. A noun not among [`CHOICES`], read from the file,
/// shows as the file spells it.
pub fn label(key: &str) -> String {
    match key {
        StartView::DASHBOARD => lib_locale::msg::nav_dashboard(),
        StartView::LAST => crate::msg::desktop_settings_general_start_view_last(),
        StartView::SETTINGS => lib_locale::msg::nav_settings(),
        "transactions" => lib_locale::msg::nav_transactions(),
        "documents" => crate::msg::desktop_nav_documents(),
        "bills" => lib_locale::msg::nav_bills(),
        "budgets" => lib_locale::msg::nav_budgets(),
        other => other.to_string(),
    }
}

/// The word `noun` is remembered by for `last`.
pub fn key(noun: Noun) -> &'static str {
    NOUNS
        .iter()
        .find(|(_, named)| *named == noun)
        .map_or(StartView::DASHBOARD, |(key, _)| key)
}

/// The noun `start_view` lands on, `last` read as `remembered`, the noun showing at the last quit.
/// A noun that needs a Ledger lands on the Dashboard with none open; Settings opens either way, on
/// the page the state file remembers.
pub fn landing(start_view: &StartView, remembered: Noun, ledger_open: bool) -> Noun {
    let resolved = start_view.resolve(Some(key(remembered)));
    let noun = NOUNS
        .iter()
        .find(|(key, _)| *key == resolved.key())
        .map(|(_, noun)| *noun);
    match noun {
        Some(noun) if ledger_open || !needs_ledger(noun) => noun,
        Some(_) => Noun::Dashboard,
        None => {
            tracing::warn!(
                start_view = resolved.key(),
                "Start view names a noun the Desktop doesn't have"
            );
            Noun::Dashboard
        }
    }
}

/// The noun the state file remembers at quit, which `last` lands on next time: the one showing,
/// unless no Ledger is open, when the Shell can only show the Dashboard or Settings and a noun
/// remembered from a run with one open stays, unless Settings is showing.
pub fn noun_at_quit(ledger_open: bool, showing: Noun, remembered: Noun) -> Noun {
    if ledger_open || showing == Noun::Settings {
        showing
    } else {
        remembered
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn noun(name: &str) -> StartView {
        StartView::Noun(name.into())
    }

    #[test]
    fn each_value_lands_where_it_says_with_a_ledger_open() {
        for (view, landed) in [
            (StartView::Dashboard, Noun::Dashboard),
            (StartView::Settings, Noun::Settings),
            (noun("transactions"), Noun::Transactions),
            (noun("documents"), Noun::Documents),
            (noun("bills"), Noun::Bills),
            (noun("budgets"), Noun::Budgets),
            (noun("credit-cards"), Noun::CreditCards),
        ] {
            assert_eq!(landing(&view, Noun::Reports, true), landed, "{view:?}");
        }
    }

    #[test]
    fn with_no_ledger_open_a_noun_lands_on_the_dashboard_and_settings_still_opens() {
        assert_eq!(
            landing(&noun("bills"), Noun::Dashboard, false),
            Noun::Dashboard
        );
        assert_eq!(
            landing(&StartView::Last, Noun::Budgets, false),
            Noun::Dashboard
        );
        assert_eq!(
            landing(&StartView::Settings, Noun::Bills, false),
            Noun::Settings
        );
        assert_eq!(
            landing(&StartView::Last, Noun::Settings, false),
            Noun::Settings
        );
    }

    #[test]
    fn last_lands_on_the_remembered_noun() {
        for remembered in Noun::ALL {
            assert_eq!(landing(&StartView::Last, remembered, true), remembered);
        }
    }

    #[test]
    fn a_noun_the_desktop_doesnt_have_lands_on_the_dashboard() {
        assert_eq!(landing(&noun("units"), Noun::Bills, true), Noun::Dashboard);
    }

    #[test]
    fn every_choice_is_a_start_view_the_desktop_lands_on() {
        for choice in CHOICES {
            let view = StartView::from_key(choice);
            assert_eq!(view.key(), choice);
            if view != StartView::Last {
                assert_ne!(
                    landing(&view, Noun::Reports, true),
                    Noun::Reports,
                    "{choice}"
                );
            }
        }
    }

    #[test]
    fn the_noun_at_quit_keeps_the_remembered_one_while_no_ledger_is_open() {
        assert_eq!(
            noun_at_quit(true, Noun::Dashboard, Noun::Bills),
            Noun::Dashboard
        );
        assert_eq!(
            noun_at_quit(false, Noun::Dashboard, Noun::Bills),
            Noun::Bills
        );
        assert_eq!(
            noun_at_quit(false, Noun::Settings, Noun::Bills),
            Noun::Settings
        );
    }
}
