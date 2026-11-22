//! What every new Ledger starts with (ADR-0054, `GLOSSARY.md`'s Ledger file), and nothing else:
//! no Accounts, Categories, Payees or Tags, so the person grows their own rather than trimming a
//! template. Uncategorised stays lazy, made the first time a Split needs it.

use lib_budgets::Budget;
use lib_core::{FinancialYearStart, LedgerName, Owner};
use lib_documents::types::DocumentTypeRow;
use lib_institutions::Institution;
use lib_units::UnitRow;

/// Every row a new Ledger is written with, in one write.
#[derive(Debug, Clone, PartialEq)]
pub struct NewLedger {
    /// The Personal Institution a Cash Account names (ADR-0049). It has no Usual unit, so a new
    /// Cash Account starts on whatever the base Unit is at the time.
    pub personal: Institution,
    /// The base Unit, the Ledger's only Unit.
    pub base_unit: UnitRow,
    /// Personal spending, the default Budget, in the base Unit on no Accounts: an Account joins a
    /// Budget only once it is added to it, and none exist yet.
    pub budget: Budget,
    /// The default Document Types, so there is always a Default type (ADR-0031).
    pub document_types: Vec<DocumentTypeRow>,
    /// The Ledger name, Owner and Financial year start, on the one-row `ledger` table beside the
    /// base Unit (ADR-0061). A Client starts them from the file name and the Locale, asking
    /// nothing; the person changes them later on Settings › Ledger.
    pub name: LedgerName,
    pub owner: Option<Owner>,
    pub financial_year_start: FinancialYearStart,
}

impl NewLedger {
    /// A new Ledger on `base_unit`, its Personal Institution named `personal`, the Client's
    /// "Personal" Message in the Locale the Ledger is made in, named `name`, owned by `owner` and
    /// with its Financial Year starting in `financial_year_start`.
    pub fn new(
        base_unit: UnitRow,
        personal: &str,
        name: LedgerName,
        owner: Option<Owner>,
        financial_year_start: FinancialYearStart,
    ) -> Self {
        Self {
            personal: lib_institutions::personal_institution(personal),
            budget: lib_budgets::personal_spending(base_unit.id, &[]),
            document_types: lib_documents::types::default_types(),
            base_unit,
            name,
            owner,
            financial_year_start,
        }
    }
}
