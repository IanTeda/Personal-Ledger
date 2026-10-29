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
    /// The Account this one offsets, as an offset account does a loan's interest.
    pub offsets: Option<&'static str>,
    pub transactions: i64,
    /// The latest transactions, newest first, which the detail pane lists (DESKTOP-142).
    pub recent: Vec<Recent>,
}

/// A transaction as the detail pane lists it: its day, what it was, and its amount at the
/// Account's Unit's places.
#[derive(Debug, Clone, PartialEq)]
pub struct Recent {
    pub on: (i32, u32, u32),
    pub what: &'static str,
    pub minor: i64,
}

const fn recent(on: (i32, u32, u32), what: &'static str, minor: i64) -> Recent {
    Recent { on, what, minor }
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
    transactions: i64,
    recent: Vec<Recent>,
) -> FixtureAccount {
    FixtureAccount {
        name,
        institution,
        kind,
        unit: BASE_UNIT,
        balance,
        places: 2,
        base_price: amount(1, 0),
        offsets: None,
        transactions,
        recent,
    }
}

/// The Accounts, in the Ledger's order.
pub fn accounts() -> Vec<FixtureAccount> {
    vec![
        account(
            "ANZ Everyday",
            "ANZ",
            AccountType::Bank,
            amount(418_255, 2),
            1_406,
            vec![
                recent((2026, 9, 12), "Woolworths Metro", -8_640),
                recent((2026, 9, 10), "Transfer to Offset", -200_000),
                recent((2026, 9, 9), "Dept of Education", 412_850),
            ],
        ),
        FixtureAccount {
            offsets: Some("Home Loan"),
            // TUI-056's selected row.
            ..account(
                "ANZ Offset",
                "ANZ",
                AccountType::Bank,
                amount(6_120_410, 2),
                212,
                vec![
                    recent((2026, 9, 12), "Interest", 8_412),
                    recent((2026, 9, 10), "Transfer from Everyday", 200_000),
                    recent((2026, 9, 1), "Home Loan repayment", -341_200),
                    recent((2026, 8, 28), "Transfer from Everyday", 200_000),
                ],
            )
        },
        account(
            "Westpac Savings",
            "Westpac",
            AccountType::Bank,
            amount(1_850_000, 2),
            38,
            vec![recent((2026, 8, 31), "Interest", 4_625)],
        ),
        account(
            "Amex Platinum",
            "American Express",
            AccountType::CreditCard,
            amount(-231_844, 2),
            64,
            vec![
                recent((2026, 9, 11), "Netflix", -2_299),
                recent((2026, 9, 8), "Camera House", -438_000),
            ],
        ),
        account(
            "Home Loan",
            "ANZ",
            AccountType::Loan,
            amount(-38_131_134, 2),
            96,
            vec![recent((2026, 9, 1), "Repayment from Offset", 341_200)],
        ),
        FixtureAccount {
            name: "Vanguard VAS",
            institution: "Vanguard",
            kind: AccountType::Investment,
            unit: "vas",
            balance: amount(1_240, 0),
            places: 0,
            base_price: amount(100, 0),
            offsets: None,
            transactions: 12,
            recent: vec![recent((2026, 8, 15), "Bought VAS", 120)],
        },
        FixtureAccount {
            name: "Bitcoin",
            institution: "Crypto.com",
            kind: AccountType::Investment,
            unit: "btc",
            balance: amount(4_120, 4),
            places: 4,
            base_price: amount(150_000, 0),
            offsets: None,
            transactions: 18,
            recent: vec![
                recent((2026, 9, 5), "Bought BTC", 500),
                recent((2026, 7, 2), "Bought BTC", 1_200),
            ],
        },
    ]
}
