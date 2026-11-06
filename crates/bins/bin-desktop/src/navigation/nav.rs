//! Where the person is, as the Desktop names and draws it. The state machine itself, `NavState`
//! with its focus zones and input modes, is `lib-shell`'s, held by the Shell's Presenter (#826);
//! the Desktop adds each Noun's display name and the rail's drawn state.

use crate::settings::SettingsPage;

pub use lib_shell::{ContextSelection, FocusZone, InputMode, Noun};

/// The rail state machine over the Desktop's Settings pages.
pub type NavState = lib_shell::NavState<SettingsPage>;

/// Whether showing `noun` needs a Ledger open (#977): every noun but the Dashboard, whose no Ledger
/// open content offers `:open` and `:new`, and Settings, for its This device pages. The rail draws
/// such a noun muted with none open, and every way onto it is refused.
pub fn needs_ledger(noun: Noun) -> bool {
    !matches!(noun, Noun::Dashboard | Noun::Settings)
}

/// A [`Noun`]'s name as the Desktop shows it, a Message: the Noun itself is `lib-shell`'s.
pub trait NounLabel {
    /// The noun's display name in the Locale in effect, in sentence case.
    fn label(self) -> String;
}

impl NounLabel for Noun {
    fn label(self) -> String {
        match self {
            Noun::Dashboard => lib_locale::msg::nav_dashboard(),
            Noun::Transactions => lib_locale::msg::nav_transactions(),
            Noun::Documents => crate::msg::desktop_nav_documents(),
            Noun::Notifications => crate::msg::desktop_nav_notifications(),
            Noun::Cash => crate::msg::desktop_nav_cash(),
            Noun::Inventory => crate::msg::desktop_nav_inventory(),
            Noun::Loans => crate::msg::desktop_nav_loans(),
            Noun::CreditCards => crate::msg::desktop_nav_credit_cards(),
            Noun::Investments => crate::msg::desktop_nav_investments(),
            Noun::Bills => lib_locale::msg::nav_bills(),
            Noun::Budgets => lib_locale::msg::nav_budgets(),
            Noun::Reports => lib_locale::msg::nav_reports(),
            Noun::Settings => lib_locale::msg::nav_settings(),
        }
    }
}

/// The primary rail's drawn states, which the Shell's Presenter picks (ADR-0042).
pub use lib_shell::RailMode;
