//! Reading the statement and matching each line against the Payee Aliases.

use lib_core::AccountId;

use crate::entity::{ImportRow, match_line};
use crate::protocols::{ImportPayees, StatementSource};

/// A statement with each of its lines matched against the Payees.
#[derive(Debug, Clone, PartialEq)]
pub struct Matched {
    pub account_id: AccountId,
    pub file_name: String,
    pub rows: Vec<ImportRow>,
}

/// Reads the statement and matches each line against the Payee Aliases (`match_line`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MatchStatement;

impl MatchStatement {
    pub fn run(source: &dyn StatementSource, payees: &dyn ImportPayees) -> Matched {
        let statement = source.statement();
        let payees = payees.import_payees();
        let rows: Vec<ImportRow> = statement
            .lines
            .iter()
            .map(|line| match_line(&payees, line))
            .collect();
        Matched {
            account_id: statement.account_id,
            file_name: statement.file_name,
            rows,
        }
    }
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;

    use super::*;
    use crate::entity::RowStatus;
    use crate::fake::stub_ports;
    use crate::statement::SeededStatement;

    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 9, 28).expect("a September day")
    }

    #[test]
    fn matching_reads_the_statement_and_tags_each_line() {
        let (ports, _) = stub_ports(today());

        let matched = MatchStatement::run(&*ports.statement, &*ports.payees.borrow());

        assert_eq!(matched.account_id, SeededStatement::STUB_ACCOUNT);
        assert_eq!(matched.rows.len(), 18);
        let rule = matched
            .rows
            .iter()
            .filter(|row| row.status() == RowStatus::RuleMatch)
            .count();
        assert_eq!(rule, 14);
    }
}
