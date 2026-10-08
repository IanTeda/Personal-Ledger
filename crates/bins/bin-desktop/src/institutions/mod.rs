//! Institutions: the banks, brokers and funds that hold a Ledger's Accounts (`CONTEXT.md`'s
//! Institution). `gpui`-free; Settings' Institutions page is only where they're edited.

pub(crate) mod form;

/// One row of the **Institutions** section's table (`docs/ux/desktop/16-settings/README.md`'s "2a
/// resting state" markup: INSTITUTION / ACCOUNT TYPE columns). Owned `String` fields, not
/// `&'static str` -- issue #187's own Add institution dialog produces real typed text, same
/// reasoning as [`UnitRow`]'s own migration for issue #184.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstitutionRow {
    pub name: String,
    /// The ACCOUNT TYPE column, e.g. `"savings \u{b7} offset"` -- the mockup joins multiple
    /// types with the same `\u{b7}` separator the scope notes use, as one free-form label (not a
    /// `Vec` of chips -- that multi-select shape belongs to the Add institution dialog's own
    /// input, not this read-only table cell). `AddInstitutionForm::account_types` joins the same
    /// way when a dialog-created row is appended (see `Shell::confirm_settings_dialog`).
    pub account_type: String,
}

/// The mockup's own six seeded rows, in its own order (the mockup's static scope note claims "7
/// institutions", but only six rows are actually drawn -- treated as the same kind of
/// mockup-authoring slip [`default_units`]'s own doc calls out elsewhere, not a seventh row to
/// invent; the scope note is dynamic and derived from `Shell::settings_institutions.len()`
/// regardless, so it self-corrects to whatever this slice actually holds). A function rather
/// than a `const` slice now that [`InstitutionRow`] owns its strings.
pub fn default_institutions() -> Vec<InstitutionRow> {
    vec![
        InstitutionRow {
            name: "ANZ Banking Group".to_string(),
            account_type: "savings \u{b7} offset".to_string(),
        },
        InstitutionRow {
            name: "American Express".to_string(),
            account_type: "credit card".to_string(),
        },
        InstitutionRow {
            name: "Vanguard Investments".to_string(),
            account_type: "investment".to_string(),
        },
        InstitutionRow {
            name: "Westpac Banking".to_string(),
            account_type: "savings".to_string(),
        },
        InstitutionRow {
            name: "Cryptocurrency Exchange".to_string(),
            account_type: "crypto".to_string(),
        },
        InstitutionRow {
            name: "Superannuation Fund".to_string(),
            account_type: "retirement".to_string(),
        },
    ]
}

/// The Add institution dialog's own Account types multi-select chips
/// (`docs/ux/desktop/16-settings/README.md`'s "2e — Add institution": savings / credit card /
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
