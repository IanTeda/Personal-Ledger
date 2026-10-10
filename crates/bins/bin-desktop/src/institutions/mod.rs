//! Institutions: the banks, brokers and funds that hold a Ledger's Accounts (`GLOSSARY.md`'s
//! Institution). `gpui`-free; Settings' Institutions page is only where they're edited.

pub(crate) mod form;

pub use lib_institutions::{InstitutionRow, default_institutions};

/// The Add institution dialog's own Account types multi-select chips
/// (`docs/ux/desktop-mockups/16-settings/README.md`'s "2e — Add institution": savings / credit card /
/// offset / loan / investment).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccountType {
    Savings,
    CreditCard,
    Offset,
    Loan,
    Investment,
}

impl AccountType {
    pub const ALL: [AccountType; 5] = [
        Self::Savings,
        Self::CreditCard,
        Self::Offset,
        Self::Loan,
        Self::Investment,
    ];

    pub fn label(self) -> String {
        match self {
            Self::Savings => crate::msg::desktop_settings_account_type_savings(),
            Self::CreditCard => lib_locale::msg::account_kind_credit_card(),
            Self::Offset => crate::msg::desktop_settings_account_type_offset(),
            Self::Loan => lib_locale::msg::account_kind_loan(),
            Self::Investment => lib_locale::msg::account_kind_investment(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_institutions_matches_the_mockups_own_six_seeded_rows() {
        let names: Vec<_> = default_institutions()
            .into_iter()
            .map(|institution| institution.name)
            .collect();
        assert_eq!(
            names,
            vec![
                "ANZ Banking Group",
                "American Express",
                "Vanguard Investments",
                "Westpac Banking",
                "Cryptocurrency Exchange",
                "Superannuation Fund",
            ]
        );
    }

    #[test]
    fn account_type_labels_come_from_the_catalogue() {
        crate::locale::init_for_tests();
        assert_eq!(AccountType::CreditCard.label(), "Credit card");
    }
}
