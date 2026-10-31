//! The seeded statement, the one [`StatementSource`] while the Import is stubbed (#284): no real
//! CSV upload or parsing, the handoff's mock lines dated back from today. A real importer is a new
//! source, not a change to the rules.

use chrono::{Duration, NaiveDate};
use lib_core::AccountId;

use crate::cents;
use crate::entity::{Statement, StatementLine};
use crate::protocols::StatementSource;

/// The seeded statement's file name, which the step names.
pub const STATEMENT_FILE: &str = "statement.csv";

/// The handoff's seven visible lines plus eleven more, 18 in all -- against the stub Payees, 14
/// an alias matches and 4 that need a Payee (`(raw description, amount in cents, days ago)`).
const STATEMENT: [(&str, i64, i64); 18] = [
    ("WOOLWORTHS 2137 SYDNEY AU", -8_640, 1),
    ("COLES 0332 BRISBANE AU", -4_190, 1),
    ("NETFLIX.COM 866-579-7172", -2_299, 2),
    ("BP SERVICE STN 4471", -6_820, 2),
    ("SP AUSSIE CANDLE CO", -3_400, 3),
    ("TFR TO J SMITH REF PAYMENT", -64_000, 3),
    ("ORIGIN ENERGY DIRECT DEBIT", -11_020, 4),
    ("WW SUPERMARKET 1044 PARRAMATTA", -5_215, 5),
    ("EMPLOYER PTY LTD PAY 0925", 325_000, 6),
    ("UBER *TRIP HELP.UBER.COM", -1_860, 6),
    ("BUNNINGS 7731 ALEXANDRIA", -6_495, 7),
    ("KMART 1123 BONDI JUNCTION", -2_900, 8),
    ("OFFICEWORKS 0561 SYDNEY", -4_580, 9),
    ("SYDNEY WATER CORP BPAY", -9_635, 10),
    ("SQ *BLUE BOTTLE CAFE SYDNEY", -1_150, 11),
    ("COLES ONLINE 8842", -12_870, 12),
    ("ANZ INTEREST CREDIT", 412, 13),
    ("PAYPAL *HANDMADE SOAPS 4029357733", -2_700, 14),
];

/// The seeded statement for `account_id`, dated back from `today`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SeededStatement {
    pub account_id: AccountId,
    pub today: NaiveDate,
}

impl SeededStatement {
    /// The stub Accounts' "ANZ Everyday", which the seeded statement has always belonged to.
    pub const STUB_ACCOUNT: AccountId = AccountId::seed(2);

    /// The seeded statement for the stub Ledger's Account, dated back from `today`.
    pub fn stub(today: NaiveDate) -> Self {
        Self {
            account_id: Self::STUB_ACCOUNT,
            today,
        }
    }
}

impl StatementSource for SeededStatement {
    fn statement(&self) -> Statement {
        Statement {
            account_id: self.account_id,
            file_name: STATEMENT_FILE.to_string(),
            lines: STATEMENT
                .iter()
                .map(|&(raw, amount, days)| StatementLine {
                    raw: raw.to_string(),
                    amount: cents(amount),
                    date: self.today - Duration::days(days),
                })
                .collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_seeded_statement_is_dated_back_from_today_for_its_account() {
        let today = NaiveDate::from_ymd_opt(2026, 9, 28).expect("a September day");
        let account_id = AccountId::seed(5);
        let statement = SeededStatement { account_id, today }.statement();

        assert_eq!(statement.account_id, account_id);
        assert_eq!(statement.file_name, STATEMENT_FILE);
        assert_eq!(statement.lines.len(), 18);
        assert_eq!(statement.lines[0].raw, "WOOLWORTHS 2137 SYDNEY AU");
        assert_eq!(statement.lines[0].amount, cents(-8_640));
        assert_eq!(statement.lines[0].date, today - Duration::days(1));
        assert_eq!(statement.lines[17].date, today - Duration::days(14));
    }
}
