//! Labels for enumerated domain values.
//!
//! Each variant of `AccountType`, `TransactionStatus`, `CategoryTypes`, `UnitKind` and
//! `BudgetPeriod`, plus Flagged, has one Message with one canonical translation per Locale,
//! shared by both bins. A call site reads `account_type.label()`. Every implementation is an
//! exhaustive `match`, so a new variant fails to compile until its Message exists. The stored
//! token (`as_str()`) stays stable and is never displayed.

use lib_core::{AccountType, BudgetPeriod, CategoryTypes, TransactionStatus, UnitKind};

use crate::msg;

/// The Locale's display label for a domain value, in sentence case. Upper-casing for a heading
/// is the renderer's job, through [`crate::format::upper`].
pub trait Label {
    /// The label in the Locale in effect.
    fn label(&self) -> String;
}

impl Label for AccountType {
    fn label(&self) -> String {
        match self {
            AccountType::Cash => msg::account_kind_cash(),
            AccountType::Bank => msg::account_kind_bank(),
            AccountType::CreditCard => msg::account_kind_credit_card(),
            AccountType::Investment => msg::account_kind_investment(),
            AccountType::Loan => msg::account_kind_loan(),
        }
    }
}

impl Label for TransactionStatus {
    fn label(&self) -> String {
        match self {
            TransactionStatus::Open => msg::transaction_status_open(),
            TransactionStatus::Cleared => msg::transaction_status_cleared(),
            TransactionStatus::Reconciled => msg::transaction_status_reconciled(),
        }
    }
}

impl Label for CategoryTypes {
    fn label(&self) -> String {
        match self {
            CategoryTypes::Asset => msg::category_type_asset(),
            CategoryTypes::Equity => msg::category_type_equity(),
            CategoryTypes::Expense => msg::category_type_expense(),
            CategoryTypes::Income => msg::category_type_income(),
            CategoryTypes::Liability => msg::category_type_liability(),
        }
    }
}

impl Label for UnitKind {
    fn label(&self) -> String {
        match self {
            UnitKind::Fiat => msg::unit_kind_fiat(),
            UnitKind::Crypto => msg::unit_kind_crypto(),
            UnitKind::Stock => msg::unit_kind_stock(),
            UnitKind::PreciousMetal => msg::unit_kind_precious_metal(),
            UnitKind::Other => msg::unit_kind_other(),
        }
    }
}

impl Label for BudgetPeriod {
    fn label(&self) -> String {
        match self {
            BudgetPeriod::Weekly => msg::budget_period_weekly(),
            BudgetPeriod::Monthly => msg::budget_period_monthly(),
            BudgetPeriod::Quarterly => msg::budget_period_quarterly(),
            BudgetPeriod::Yearly => msg::budget_period_yearly(),
        }
    }
}

/// The label for the Flagged marker, which is independent of a Transaction's status and so has
/// no enum of its own.
pub fn flagged_label() -> String {
    msg::transaction_flagged()
}
