//! `ActiveView` -- the one thing `Shell` dispatches on to decide which View owns the keyboard,
//! the page status and the main pane. Pure and `gpui`-free: it is derived from the `Noun`, the
//! selected Settings section and whether a CSV import is open, so a new View fails to compile
//! at every exhaustive `match` until it is handled (ADR-0016).

use super::nav::Noun;
use crate::settings::SettingsSection;

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
    Settings(SettingsSection),
    /// A Noun whose surface is a later map's work (#420).
    Placeholder(Noun),
}

impl ActiveView {
    pub fn derive(noun: Noun, settings_section: SettingsSection, import_open: bool) -> Self {
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

    const GENERAL: SettingsSection = SettingsSection::General;

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
            SettingsSection::General,
            SettingsSection::Display,
            SettingsSection::Units,
            SettingsSection::Institutions,
            SettingsSection::Accounts,
            SettingsSection::Categories,
            SettingsSection::Tags,
            SettingsSection::Payees,
            SettingsSection::Documents,
            SettingsSection::Inventory,
            SettingsSection::SyncServer,
            SettingsSection::DataBackup,
            SettingsSection::Tracing,
            SettingsSection::About,
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
