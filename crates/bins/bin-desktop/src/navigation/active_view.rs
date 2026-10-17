//! `ActiveView` -- the one thing `Shell` dispatches on to decide which View owns the keyboard,
//! the page status and the main pane. Pure and `gpui`-free: it is derived from the `Noun`, the
//! selected Settings section and whether a CSV import is open, so a new View fails to compile
//! at every exhaustive `match` until it is handled (ADR-0016).

use super::nav::Noun;
use crate::settings::SettingsPage;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveView {
    Dashboard,
    Transactions,
    /// The CSV import step, which takes over the Transactions pane while it is open.
    Import,
    Documents,
    Bills,
    Budgets,
    /// Settings' index-versus-page focus stays in Settings domain code, not in this enum.
    Settings(SettingsPage),
    /// A Noun whose surface is a later map's work (#420).
    Placeholder(Noun),
}

impl ActiveView {
    /// The Noun this View belongs to: the CSV import is part of Transactions, and every Settings
    /// section is part of Settings.
    pub fn noun(self) -> Noun {
        match self {
            Self::Dashboard => Noun::Dashboard,
            Self::Transactions | Self::Import => Noun::Transactions,
            Self::Documents => Noun::Documents,
            Self::Bills => Noun::Bills,
            Self::Budgets => Noun::Budgets,
            Self::Settings(_) => Noun::Settings,
            Self::Placeholder(noun) => noun,
        }
    }

    /// The status line's mode label in `Normal` mode, where the View on show names its own: the
    /// Import step's `IMPORT`.
    pub fn mode_label(self) -> Option<String> {
        match self {
            Self::Import => Some(crate::msg::desktop_mode_import()),
            Self::Dashboard
            | Self::Transactions
            | Self::Documents
            | Self::Bills
            | Self::Budgets
            | Self::Settings(_)
            | Self::Placeholder(_) => None,
        }
    }

    pub fn derive(noun: Noun, settings_section: SettingsPage, import_open: bool) -> Self {
        match noun {
            Noun::Dashboard => Self::Dashboard,
            Noun::Transactions if import_open => Self::Import,
            Noun::Transactions => Self::Transactions,
            Noun::Documents => Self::Documents,
            Noun::Bills => Self::Bills,
            Noun::Budgets => Self::Budgets,
            Noun::Settings => Self::Settings(settings_section),
            Noun::Notifications
            | Noun::Cash
            | Noun::Inventory
            | Noun::Loans
            | Noun::CreditCards
            | Noun::Investments
            | Noun::Reports => Self::Placeholder(noun),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const GENERAL: SettingsPage = SettingsPage::General;

    #[test]
    fn import_only_takes_over_transactions() {
        assert_eq!(
            ActiveView::derive(Noun::Transactions, GENERAL, true),
            ActiveView::Import
        );
        assert_eq!(
            ActiveView::derive(Noun::Transactions, GENERAL, false),
            ActiveView::Transactions
        );
        // A stale import never leaks into another Noun.
        assert_eq!(
            ActiveView::derive(Noun::Bills, GENERAL, true),
            ActiveView::Bills
        );
    }

    #[test]
    fn settings_carries_each_section() {
        for section in [
            SettingsPage::General,
            SettingsPage::Display,
            SettingsPage::Units,
            SettingsPage::Institutions,
            SettingsPage::Accounts,
            SettingsPage::Categories,
            SettingsPage::Tags,
            SettingsPage::Payees,
            SettingsPage::Documents,
            SettingsPage::Inventory,
            SettingsPage::SyncServer,
            SettingsPage::DataBackup,
            SettingsPage::Tracing,
            SettingsPage::About,
        ] {
            assert_eq!(
                ActiveView::derive(Noun::Settings, section, false),
                ActiveView::Settings(section)
            );
        }
    }

    #[test]
    fn domain_nouns_map_to_their_views() {
        assert_eq!(
            ActiveView::derive(Noun::Dashboard, GENERAL, false),
            ActiveView::Dashboard
        );
        assert_eq!(
            ActiveView::derive(Noun::Documents, GENERAL, false),
            ActiveView::Documents
        );
        assert_eq!(
            ActiveView::derive(Noun::Budgets, GENERAL, false),
            ActiveView::Budgets
        );
    }

    #[test]
    fn unbuilt_nouns_are_placeholders() {
        for noun in [
            Noun::Notifications,
            Noun::Cash,
            Noun::Inventory,
            Noun::Loans,
            Noun::CreditCards,
            Noun::Investments,
            Noun::Reports,
        ] {
            assert_eq!(
                ActiveView::derive(noun, GENERAL, false),
                ActiveView::Placeholder(noun)
            );
        }
    }
}
