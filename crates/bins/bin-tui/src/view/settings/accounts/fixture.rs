//! wireframe: Settings › Accounts' rows, TUI-041's seven Accounts, until #805's Accounts chain
//! moves the TUI onto `lib-accounts`. The pre-VIPER `crate::account` fixture has no Institutions,
//! and wiring it would be work that chain throws away, so the page reads these instead.

use bigdecimal::BigDecimal;
use lib_core::{AccountType, Money};

/// The base Unit every balance converts to for net worth.
pub const BASE_UNIT: &str = "aud";

/// One Account as the page lists it.
#[derive(Debug, Clone, PartialEq)]
pub struct FixtureAccount {
    pub name: &'static str,
    pub institution: &'static str,
    pub kind: AccountType,
    pub unit: &'static str,
    pub balance: Money,
    /// The Unit's decimal places.
    pub places: i64,
    /// One of the Unit in the base Unit at the latest weekly close.
    pub base_price: Money,
}

/// `minor` at `scale` decimal places.
fn amount(minor: i64, scale: i64) -> Money {
    Money(BigDecimal::new(minor.into(), scale))
}

fn account(
    name: &'static str,
    institution: &'static str,
    kind: AccountType,
    balance: Money,
) -> FixtureAccount {
    FixtureAccount {
        name,
        institution,
        kind,
        unit: BASE_UNIT,
        balance,
        places: 2,
        base_price: amount(1, 0),
    }
}

/// The Accounts, in the Ledger's order.
pub fn accounts() -> Vec<FixtureAccount> {
    vec![
        account("ANZ Everyday", "ANZ", AccountType::Bank, amount(418_255, 2)),
        account("ANZ Offset", "ANZ", AccountType::Bank, amount(6_120_410, 2)),
        account(
            "Westpac Savings",
            "Westpac",
            AccountType::Bank,
            amount(1_850_000, 2),
        ),
        account(
            "Amex Platinum",
            "American Express",
            AccountType::CreditCard,
            amount(-231_844, 2),
        ),
        account(
            "Home Loan",
            "ANZ",
            AccountType::Loan,
            amount(-38_131_134, 2),
        ),
        FixtureAccount {
            name: "Vanguard VAS",
            institution: "Vanguard",
            kind: AccountType::Investment,
            unit: "vas",
            balance: amount(1_240, 0),
            places: 0,
            base_price: amount(100, 0),
        },
        FixtureAccount {
            name: "Bitcoin",
            institution: "Crypto.com",
            kind: AccountType::Investment,
            unit: "btc",
            balance: amount(4_120, 4),
            places: 4,
            base_price: amount(150_000, 0),
        },
    ]
}
