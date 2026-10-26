//! `lib-tags`' [`TagUsage`] counted over rows (#861): a Tag's Transactions, total and last-used
//! date from the Transactions themselves. The Tags adapter answers the port from the file (#862);
//! this is what an in-memory Client counts with ([`Ledger::tag_uses`]) until the Transactions View
//! reads its Splits through its own domain.

use bigdecimal::BigDecimal;
use chrono::NaiveDate;
use lib_accounts::Account;
use lib_core::{AccountId, Money, TagId, Total, UnitId};
use lib_tags::{TagUsage, TagUse};
use lib_transactions::Transaction;

/// Whether any Split on `transaction` carries Tag `id`.
fn carries(transaction: &Transaction, id: TagId) -> bool {
    transaction
        .splits
        .iter()
        .any(|split| split.tag_ids.contains(&id))
}

/// Distinct Transactions carrying Tag `id`: the TRANSACTIONS column, the figure the Remove and
/// Merge dialogs quote, and the sort key.
pub fn transaction_count(transactions: &[Transaction], id: TagId) -> usize {
    transactions.iter().filter(|t| carries(t, id)).count()
}

/// The rows a Tag's usage is counted from. `unit_code` turns an Account's Unit id into the code
/// the [`Total`] names, since the Accounts hold only the id.
pub struct TagRows<'a> {
    pub transactions: &'a [Transaction],
    pub accounts: &'a [Account],
    pub unit_code: &'a dyn Fn(UnitId) -> String,
}

impl TagUsage for TagRows<'_> {
    fn tag_usage(&self, id: TagId) -> TagUse {
        let unit_of = |account_id: AccountId| {
            self.accounts
                .iter()
                .find(|a| a.id == account_id)
                .map(|a| a.unit)
        };
        let mut count = 0;
        let mut last_used: Option<NaiveDate> = None;
        let mut unit: Option<UnitId> = None;
        let mut mixed = false;
        let mut sum = BigDecimal::new(0.into(), 2);
        for transaction in self.transactions.iter().filter(|t| carries(t, id)) {
            count += 1;
            last_used = last_used.max(Some(transaction.date));
            let this_unit = unit_of(transaction.account_id);
            match unit {
                None => unit = this_unit,
                Some(seen) if this_unit != Some(seen) => mixed = true,
                Some(_) => {}
            }
            for split in transaction
                .splits
                .iter()
                .filter(|s| s.tag_ids.contains(&id))
            {
                sum += split.amount.0.clone();
            }
        }
        let total = match unit {
            _ if count == 0 => Total::Empty,
            _ if mixed => Total::Mixed,
            Some(unit) => Total::Single {
                unit: (self.unit_code)(unit),
                amount: Money(sum),
            },
            // Carrying Transactions whose Account is gone: no Unit to name, so no sum either.
            None => Total::Mixed,
        };
        TagUse {
            transactions: count,
            total,
            last_used,
        }
    }
}

#[cfg(test)]
mod tests {
    use lib_accounts::default_accounts;
    use lib_core::{CategoryId, TransactionId, TransactionStatus};
    use lib_transactions::Split;

    use super::*;

    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 9, 19).expect("a fixed valid date")
    }

    fn split(cents: i64, tag_ids: &[TagId]) -> Split {
        Split {
            amount: Money(BigDecimal::new(cents.into(), 2)),
            category_id: CategoryId::seed(7),
            payee_id: None,
            tag_ids: tag_ids.to_vec(),
        }
    }

    fn transaction(
        id: u32,
        account_id: AccountId,
        date: NaiveDate,
        splits: Vec<Split>,
    ) -> Transaction {
        Transaction {
            id: TransactionId::seed(id),
            date,
            account_id,
            status: TransactionStatus::Open,
            is_flagged: false,
            description: None,
            splits,
        }
    }

    fn account_id(accounts: &[Account], name: &str) -> AccountId {
        accounts
            .iter()
            .find(|a| a.name == name)
            .expect("a stub Account")
            .id
    }

    fn usage_of(transactions: &[Transaction], accounts: &[Account], id: TagId) -> TagUse {
        TagRows {
            transactions,
            accounts,
            unit_code: &|_| "aud".to_string(),
        }
        .tag_usage(id)
    }

    #[test]
    fn transaction_count_counts_each_transaction_once() {
        let account = AccountId::seed(1);
        let transactions = vec![
            transaction(
                1,
                account,
                today(),
                vec![split(1, &[TagId::seed(7)]), split(1, &[TagId::seed(7)])],
            ),
            transaction(2, account, today(), vec![split(1, &[TagId::seed(8)])]),
        ];

        assert_eq!(transaction_count(&transactions, TagId::seed(7)), 1);
    }

    #[test]
    fn usage_counts_distinct_transactions_sums_the_tagged_splits_and_takes_the_newest_date() {
        let accounts = default_accounts();
        let everyday = account_id(&accounts, "ANZ Everyday");
        let day = today();
        let transactions = vec![
            transaction(
                1,
                everyday,
                day - chrono::Duration::days(3),
                vec![
                    split(-1_000, &[TagId::seed(5)]),
                    split(-250, &[TagId::seed(5)]),
                    split(-9_999, &[]),
                ],
            ),
            // Future-dated still counts as last used.
            transaction(
                2,
                everyday,
                day + chrono::Duration::days(2),
                vec![split(-500, &[TagId::seed(5)])],
            ),
            transaction(3, everyday, day, vec![split(-700, &[TagId::seed(6)])]),
        ];

        let used = usage_of(&transactions, &accounts, TagId::seed(5));

        assert_eq!(used.transactions, 2);
        assert_eq!(
            used.total,
            Total::Single {
                unit: "aud".to_string(),
                amount: Money(BigDecimal::new((-1_750).into(), 2)),
            }
        );
        assert_eq!(used.last_used, Some(day + chrono::Duration::days(2)));
    }

    #[test]
    fn usage_across_units_is_mixed_and_an_unused_tag_is_empty() {
        let accounts = default_accounts();
        let day = today();
        let transactions = vec![
            transaction(
                1,
                account_id(&accounts, "ANZ Everyday"),
                day,
                vec![split(-1_000, &[TagId::seed(5)])],
            ),
            transaction(
                2,
                account_id(&accounts, "Bitcoin"),
                day,
                vec![split(-500, &[TagId::seed(5)])],
            ),
        ];

        assert_eq!(
            usage_of(&transactions, &accounts, TagId::seed(5)).total,
            Total::Mixed
        );
        assert_eq!(
            usage_of(&transactions, &accounts, TagId::seed(6)),
            TagUse::default()
        );
    }

    #[test]
    fn transactions_on_a_gone_account_have_no_unit_to_name() {
        let day = today();
        let transactions = vec![transaction(
            1,
            AccountId::seed(999),
            day,
            vec![split(-1, &[TagId::seed(5)])],
        )];

        let used = usage_of(&transactions, &default_accounts(), TagId::seed(5));

        assert_eq!((used.transactions, used.total), (1, Total::Mixed));
    }
}
