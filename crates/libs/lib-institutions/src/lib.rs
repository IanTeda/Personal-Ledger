//! Institutions: the banks, brokers and funds that hold a Ledger's Accounts (`CONTEXT.md`'s
//! Institution). Pure and I/O-free: the desktop's Settings › Institutions page reads them through
//! `InstitutionsStore`, and the Account Type labels stay in the desktop, which owns the Message
//! catalogue.

/// One row of the **Institutions** section's table (`docs/ux/desktop-mockups/16-settings/README.md`'s "2a
/// resting state" markup: INSTITUTION / ACCOUNT TYPE columns). Owned `String` fields, not
/// `&'static str` -- issue #187's own Add institution dialog produces real typed text, same
/// reasoning as `UnitRow`'s own migration for issue #184.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstitutionRow {
    pub name: String,
    /// The ACCOUNT TYPE column, e.g. `"savings \u{b7} offset"` -- the mockup joins multiple
    /// types with the same `\u{b7}` separator the scope notes use, as one free-form label (not a
    /// `Vec` of chips -- that multi-select shape belongs to the Add institution dialog's own
    /// input, not this read-only table cell). `AddInstitutionForm::account_types` joins the same
    /// way when a dialog-created row is appended (see the desktop's settings dialog wiring).
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

/// The Institutions table's rows, read and appended through this service.
pub struct InstitutionService {
    institutions: Vec<InstitutionRow>,
}

impl InstitutionService {
    pub fn new(institutions: Vec<InstitutionRow>) -> Self {
        Self { institutions }
    }

    pub fn institutions(&self) -> &[InstitutionRow] {
        &self.institutions
    }

    /// Appends an institution from the Add institution dialog. `account_type` is the joined
    /// label the dialog built, so this service stores it as given.
    pub fn add(&mut self, name: String, account_type: String) {
        self.institutions.push(InstitutionRow { name, account_type });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_added_institution_is_appended_with_its_joined_account_types() {
        let mut service = InstitutionService::new(default_institutions());
        let before = service.institutions().len();
        service.add("Bank X".into(), "savings \u{b7} offset".into());
        assert_eq!(service.institutions().len(), before + 1);
        let added = service.institutions().last().expect("appended");
        assert_eq!(added.name, "Bank X");
        assert_eq!(added.account_type, "savings \u{b7} offset");
    }
}
