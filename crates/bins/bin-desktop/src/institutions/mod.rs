//! Institutions: the banks, brokers and funds that hold a Ledger's Accounts (`GLOSSARY.md`'s
//! Institution). `gpui`-free; Settings' Institutions page is only where they're edited.

pub(crate) mod form;
pub(crate) mod store;

pub use lib_institutions::InstitutionRow;
#[cfg(test)]
pub use lib_institutions::default_institutions;
pub use store::InstitutionsStore;

/// The Add institution dialog's own product multi-select chips (not `lib_core::AccountType`, which classes an Account)
/// (`docs/ux/desktop-mockups/16-settings/README.md`'s "2e — Add institution": savings / credit card /
/// offset / loan / investment).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstitutionProduct {
    Savings,
    CreditCard,
    Offset,
    Loan,
    Investment,
}

impl InstitutionProduct {
    pub const ALL: [InstitutionProduct; 5] = [
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
        assert_eq!(InstitutionProduct::CreditCard.label(), "Credit card");
    }
}
