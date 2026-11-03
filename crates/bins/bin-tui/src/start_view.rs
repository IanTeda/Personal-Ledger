//! Where the TUI lands as it starts: Start view (ADR-0053), the `start_view` key the Desktop reads
//! too. `lib_core::StartView` resolves `last` to the View remembered at the last quit; this names
//! the TUI's nouns and turns the result into the [`Action`] that opens it.
//!
//! The nouns are the Views a `g` jump opens. The placeholder Views (Transactions, Budgets, Balance
//! checks and Reports) are left out, as their jumps are, so a noun the TUI doesn't have, the
//! Desktop's among them, lands on the Dashboard.

use lib_core::StartView;

use crate::msg;
use crate::view::{Action, ViewId};

/// The nouns Start view can name, by the word the file and `:set start-view` spell, with their
/// Views. Each needs a Ledger.
const NOUNS: [(&str, ViewId); 5] = [
    ("units", ViewId::Units),
    ("accounts", ViewId::Accounts),
    ("categories", ViewId::Categories),
    ("payees", ViewId::Payees),
    ("tags", ViewId::Tags),
];

/// Start view's choices, in the order Settings › General shows them, by the value the file and
/// `:set start-view` take.
pub const CHOICES: [&str; 8] = [
    StartView::DASHBOARD,
    StartView::LAST,
    StartView::SETTINGS,
    "units",
    "accounts",
    "categories",
    "payees",
    "tags",
];

/// A Start view choice's label, by its value. A noun the TUI doesn't have, read from the file,
/// shows as the file spells it.
pub fn label(key: &str) -> String {
    match key {
        StartView::DASHBOARD => msg::tui_settings_start_view_dashboard(),
        StartView::LAST => msg::tui_settings_start_view_last(),
        StartView::SETTINGS => msg::tui_settings_start_view_settings(),
        "units" => msg::tui_settings_start_view_units(),
        "accounts" => msg::tui_settings_start_view_accounts(),
        "categories" => msg::tui_settings_start_view_categories(),
        "payees" => msg::tui_settings_start_view_payees(),
        "tags" => msg::tui_settings_start_view_tags(),
        other => other.to_string(),
    }
}

/// The word `view` is remembered by for `last`, or `None` for one Start view can't name (Help and
/// the placeholders), which `last` then reads as the Dashboard.
pub fn noun(view: ViewId) -> Option<&'static str> {
    match view {
        ViewId::Dashboard => Some(StartView::DASHBOARD),
        ViewId::Settings => Some(StartView::SETTINGS),
        view => NOUNS
            .iter()
            .find(|(_, noun_view)| *noun_view == view)
            .map(|(name, _)| *name),
    }
}

/// What opens the View `start_view` lands on, `last` read as `remembered`. A noun needs a Ledger,
/// so with none open it lands on the Dashboard, whose place shows the no Ledger open note;
/// Settings opens either way, on the index with General selected.
pub fn landing(start_view: &StartView, remembered: Option<&str>, ledger_open: bool) -> Action {
    match start_view.resolve(remembered) {
        StartView::Settings => Action::OpenSettings,
        StartView::Noun(noun) if ledger_open => {
            match NOUNS.iter().find(|(name, _)| *name == noun) {
                Some((_, view)) => open(*view),
                None => {
                    tracing::warn!(%noun, "Start view names a noun the TUI doesn't have");
                    Action::OpenDashboard
                }
            }
        }
        StartView::Dashboard | StartView::Last | StartView::Noun(_) => Action::OpenDashboard,
    }
}

/// The Action that opens a noun's View.
fn open(view: ViewId) -> Action {
    match view {
        ViewId::Units => Action::OpenUnits,
        ViewId::Accounts => Action::OpenAccounts,
        ViewId::Categories => Action::OpenCategories,
        ViewId::Payees => Action::OpenPayees,
        ViewId::Tags => Action::OpenTags,
        _ => Action::OpenDashboard,
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
        for (view, action) in [
            (StartView::Dashboard, Action::OpenDashboard),
            (StartView::Settings, Action::OpenSettings),
            (noun("units"), Action::OpenUnits),
            (noun("accounts"), Action::OpenAccounts),
            (noun("categories"), Action::OpenCategories),
            (noun("payees"), Action::OpenPayees),
            (noun("tags"), Action::OpenTags),
        ] {
            assert_eq!(landing(&view, None, true), action, "{view:?}");
        }
    }

    #[test]
    fn with_no_ledger_open_a_noun_lands_on_the_dashboard_and_settings_still_opens() {
        assert_eq!(landing(&noun("units"), None, false), Action::OpenDashboard);
        assert_eq!(
            landing(&StartView::Last, Some("tags"), false),
            Action::OpenDashboard
        );
        assert_eq!(
            landing(&StartView::Settings, None, false),
            Action::OpenSettings
        );
    }

    #[test]
    fn last_lands_on_the_remembered_view_or_the_dashboard() {
        assert_eq!(
            landing(&StartView::Last, Some("payees"), true),
            Action::OpenPayees
        );
        assert_eq!(
            landing(&StartView::Last, Some("settings"), false),
            Action::OpenSettings
        );
        assert_eq!(landing(&StartView::Last, None, true), Action::OpenDashboard);
    }

    #[test]
    fn a_noun_the_tui_doesnt_have_lands_on_the_dashboard() {
        assert_eq!(landing(&noun("bills"), None, true), Action::OpenDashboard);
    }

    #[test]
    fn every_view_start_view_names_is_remembered_by_a_choice_it_takes() {
        for view in [
            ViewId::Dashboard,
            ViewId::Settings,
            ViewId::Units,
            ViewId::Accounts,
            ViewId::Categories,
            ViewId::Payees,
            ViewId::Tags,
        ] {
            let name = super::noun(view).expect("a View Start view names");
            assert!(CHOICES.contains(&name), "{name}");
        }
        assert_eq!(super::noun(ViewId::Help), None);
        assert_eq!(super::noun(ViewId::Budgets), None);
    }
}
