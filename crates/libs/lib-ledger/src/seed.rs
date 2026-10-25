//! The stub Transactions joined to the other domains' stubs: `lib_transactions::fake`'s generator,
//! given the stub Accounts and the Category, Payee and Tag names it seeds by. It lives here, in the
//! one crate that sees every domain, since the Categories, Payees and Tags crates depend on
//! `lib-transactions`.

use chrono::NaiveDate;
use lib_accounts::Account;
use lib_categories::{self as categories, Category};
use lib_core::{CategoryId, PayeeId, TagId};
use lib_payees::{self as payees, Payee};
use lib_tags::{self as tags, Tag};
use lib_transactions::Transaction;
use lib_transactions::fake::{SeedAccount, SeedNames, stub_transactions};

pub use lib_transactions::fake::{DEFAULT_SEED, HISTORY_DAYS};

/// The names the generator seeds by, looked up in the stub collections.
struct StubNames<'a> {
    categories: &'a [Category],
    payees: &'a [Payee],
    tags: &'a [Tag],
}

impl SeedNames for StubNames<'_> {
    fn category(&self, name: &str) -> Option<CategoryId> {
        categories::find_by_name(self.categories, name)
    }

    fn payee(&self, name: &str) -> Option<PayeeId> {
        payees::find_by_name(self.payees, name)
    }

    fn tag(&self, name: &str) -> Option<TagId> {
        tags::find_by_name(self.tags, name)
    }
}

/// The stub Transactions over these stubs: newest first, ids assigned oldest-first (a higher id is
/// newer). Per-account counts equal each account's `transaction_count`; accounts that take no
/// Transactions, and any account the seed has no profile for, get none.
pub fn default_transactions(
    accounts: &[Account],
    categories: &[Category],
    payees: &[Payee],
    tags: &[Tag],
    today: NaiveDate,
) -> Vec<Transaction> {
    let accounts: Vec<SeedAccount> = accounts
        .iter()
        .map(|account| SeedAccount {
            id: account.id,
            name: account.name.clone(),
            account_type: account.account_type.clone(),
            transaction_count: account.transaction_count,
        })
        .collect();
    let names = StubNames {
        categories,
        payees,
        tags,
    };
    stub_transactions(&accounts, &names, today)
}

#[cfg(test)]
mod tests {
    use bigdecimal::BigDecimal;
    use chrono::Duration;
    use lib_core::{AccountId, Money, TransactionId, TransactionStatus};
    use lib_transactions::Split;

    use super::*;
    use lib_accounts::default_accounts;
    use lib_categories::default_categories;
    use lib_payees::default_payees;
    use lib_tags::default_tags;

    fn cents_money(cents: i64) -> Money {
        Money(BigDecimal::new(cents.into(), 2))
    }

    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 9, 19).unwrap()
    }

    /// `(accounts, categories, payees, tags, transactions)`, seeded as the app does.
    type Seed = (
        Vec<Account>,
        Vec<Category>,
        Vec<Payee>,
        Vec<Tag>,
        Vec<Transaction>,
    );

    fn seed() -> Seed {
        let accounts = default_accounts();
        let categories = default_categories();
        let payees = default_payees();
        let tags = default_tags();
        let transactions = default_transactions(&accounts, &categories, &payees, &tags, today());
        (accounts, categories, payees, tags, transactions)
    }

    fn count_for(transactions: &[Transaction], accounts: &[Account], name: &str) -> usize {
        let id = accounts.iter().find(|a| a.name == name).unwrap().id;
        transactions.iter().filter(|t| t.account_id == id).count()
    }

    #[test]
    fn every_payees_default_category_is_a_leaf() {
        let categories = default_categories();
        for payee in default_payees() {
            if let Some(category) = payee.default_category {
                assert!(
                    lib_categories::is_leaf(&categories, category),
                    "{}",
                    payee.name
                );
            }
        }
    }

    #[test]
    fn every_stub_account_names_a_stub_institution_or_the_placeholder() {
        let institutions = lib_institutions::default_institutions();
        for account in default_accounts() {
            assert!(
                account.institution == lib_accounts::NO_INSTITUTION
                    || lib_institutions::get(&institutions, account.institution).is_some(),
                "{} names an unknown Institution",
                account.name
            );
        }
    }

    #[test]
    fn per_account_counts_match_the_accounts_stub() {
        let (accounts, _, _, _, transactions) = seed();
        assert_eq!(count_for(&transactions, &accounts, "Wallet"), 96);
        assert_eq!(count_for(&transactions, &accounts, "ANZ Everyday"), 312);
        assert_eq!(count_for(&transactions, &accounts, "ANZ Offset"), 88);
        assert_eq!(count_for(&transactions, &accounts, "Amex Platinum"), 204);
        assert_eq!(transactions.len(), 700);
    }

    #[test]
    fn loan_and_investment_accounts_hold_no_transactions() {
        let (accounts, _, _, _, transactions) = seed();
        for name in ["Home Loan", "Vanguard VAS", "Bitcoin"] {
            assert_eq!(count_for(&transactions, &accounts, name), 0, "{name}");
        }
    }

    #[test]
    fn the_seed_is_deterministic() {
        let (_, _, _, _, first) = seed();
        let (_, _, _, _, second) = seed();
        assert_eq!(first, second);
    }

    #[test]
    fn a_different_today_shifts_the_dates_but_not_the_shape() {
        let (accounts, categories, payees, tags, first) = seed();
        let later = today() + Duration::days(10);
        let second = default_transactions(&accounts, &categories, &payees, &tags, later);
        assert_eq!(first.len(), second.len());
        assert_eq!(first[0].date + Duration::days(10), second[0].date);
    }

    #[test]
    fn transactions_are_newest_first_with_unique_ids_that_rise_with_the_date() {
        let (_, _, _, _, transactions) = seed();
        let mut ids: Vec<_> = transactions.iter().map(|t| t.id).collect();
        for pair in transactions.windows(2) {
            assert!(pair[0].date >= pair[1].date, "newest first");
            assert!(pair[0].id > pair[1].id, "ids fall down the list");
        }
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), transactions.len());
        assert_eq!(ids[0], TransactionId::seed(1));
    }

    #[test]
    fn every_date_falls_within_the_history_window() {
        let (_, _, _, _, transactions) = seed();
        let oldest = today() - Duration::days(HISTORY_DAYS);
        for transaction in &transactions {
            assert!(transaction.date <= today());
            assert!(transaction.date >= oldest);
        }
    }

    #[test]
    fn splits_are_leaf_categories_with_real_payees_and_tags_and_the_right_sign() {
        let (_, categories, payees, tags, transactions) = seed();
        for transaction in &transactions {
            assert!(!transaction.splits.is_empty());
            for split in &transaction.splits {
                assert!(categories::is_leaf(&categories, split.category_id));
                let category = categories
                    .iter()
                    .find(|c| c.id == split.category_id)
                    .unwrap();
                let negative = split.amount.0.to_string().starts_with('-');
                let income = category.category_type == lib_core::CategoryTypes::Income;
                assert_eq!(negative, !income, "{} has the wrong sign", category.name);
                if let Some(payee) = split.payee_id {
                    assert!(payees.iter().any(|p| p.id == payee));
                }
                for tag in &split.tag_ids {
                    assert!(tags.iter().any(|t| t.id == *tag));
                }
            }
        }
    }

    #[test]
    fn a_transactions_total_is_the_sum_of_its_splits() {
        let receipt = Transaction {
            id: TransactionId::seed(1),
            date: today(),
            account_id: AccountId::seed(1),
            status: TransactionStatus::Open,
            is_flagged: false,
            description: None,
            splits: vec![
                Split {
                    amount: cents_money(-18_000),
                    category_id: CategoryId::seed(7),
                    payee_id: None,
                    tag_ids: vec![],
                },
                Split {
                    amount: cents_money(-3_430),
                    category_id: CategoryId::seed(10),
                    payee_id: None,
                    tag_ids: vec![],
                },
            ],
        };
        assert_eq!(receipt.total(), cents_money(-21_430));
        assert!(receipt.is_split());
    }

    #[test]
    fn about_eight_percent_are_multi_split_with_two_or_three_splits() {
        let (_, _, _, _, transactions) = seed();
        let multi: Vec<_> = transactions.iter().filter(|t| t.is_split()).collect();
        let share = multi.len() * 100 / transactions.len();
        assert!((5..=12).contains(&share), "{share}% multi-split");
        for transaction in multi {
            assert!((2..=3).contains(&transaction.splits.len()));
        }
    }

    fn special<'a>(
        transactions: &'a [Transaction],
        accounts: &[Account],
        account: &str,
        description: &str,
    ) -> &'a Transaction {
        let id = accounts.iter().find(|a| a.name == account).unwrap().id;
        transactions
            .iter()
            .find(|t| t.account_id == id && t.description.as_deref() == Some(description))
            .unwrap_or_else(|| panic!("no special {description:?} on {account}"))
    }

    #[test]
    fn the_groceries_and_household_receipt_shares_one_payee() {
        let (accounts, categories, payees, _, transactions) = seed();
        let receipt = special(
            &transactions,
            &accounts,
            "ANZ Everyday",
            "Weekly shop and cleaning supplies",
        );
        assert_eq!(receipt.splits.len(), 2);
        let woolworths = payees::find_by_name(&payees, "Woolworths");
        assert!(receipt.splits.iter().all(|s| s.payee_id == woolworths));
        let groceries = categories::find_by_name(&categories, "Groceries").unwrap();
        let household = categories::find_by_name(&categories, "Household").unwrap();
        assert_eq!(receipt.splits[0].category_id, groceries);
        assert_eq!(receipt.splits[1].category_id, household);
        assert_eq!(receipt.total(), cents_money(-21_430));
    }

    #[test]
    fn one_receipt_has_splits_with_different_payees() {
        let (accounts, _, _, _, transactions) = seed();
        let receipt = special(&transactions, &accounts, "Amex Platinum", "Airport");
        assert_ne!(receipt.splits[0].payee_id, receipt.splits[1].payee_id);
    }

    #[test]
    fn one_receipt_has_a_tag_on_only_one_of_its_splits() {
        let (accounts, _, _, tags, transactions) = seed();
        let receipt = special(
            &transactions,
            &accounts,
            "ANZ Everyday",
            "Tokyo dinner and souvenirs",
        );
        let japan = tags::find_by_name(&tags, "Japan Trip 2026").unwrap();
        assert_eq!(receipt.splits[0].tag_ids, vec![japan]);
        assert!(receipt.splits[1].tag_ids.is_empty());
    }

    #[test]
    fn the_salary_is_positive_and_reconciled() {
        let (accounts, categories, _, _, transactions) = seed();
        let id = accounts
            .iter()
            .find(|a| a.name == "ANZ Everyday")
            .unwrap()
            .id;
        let salary = categories::find_by_name(&categories, "Salary").unwrap();
        let pay = transactions
            .iter()
            .find(|t| {
                t.account_id == id
                    && t.date == today() - Duration::days(9)
                    && t.splits[0].category_id == salary
            })
            .expect("the hand-authored salary");
        assert_eq!(pay.total(), cents_money(421_000));
        assert_eq!(pay.status, TransactionStatus::Reconciled);
    }

    #[test]
    fn every_status_and_flagged_combination_appears() {
        let (_, _, _, _, transactions) = seed();
        for status in [
            TransactionStatus::Open,
            TransactionStatus::Cleared,
            TransactionStatus::Reconciled,
        ] {
            for flagged in [false, true] {
                assert!(
                    transactions
                        .iter()
                        .any(|t| t.status == status && t.is_flagged == flagged),
                    "no {status:?} transaction with flagged = {flagged}"
                );
            }
        }
    }

    #[test]
    fn wallet_amounts_are_scaled_down() {
        let (accounts, _, _, _, transactions) = seed();
        let id = accounts.iter().find(|a| a.name == "Wallet").unwrap().id;
        for transaction in transactions.iter().filter(|t| t.account_id == id) {
            let text = transaction.total().0.to_string();
            let digits: String = text.chars().filter(char::is_ascii_digit).collect();
            let cents: i64 = digits.parse().unwrap();
            assert!(cents <= 10_000, "{text} is not a small cash amount");
        }
    }

    #[test]
    fn a_smaller_transaction_count_keeps_the_specials_first_and_the_total_exact() {
        let mut accounts = default_accounts();
        for account in &mut accounts {
            if account.name == "ANZ Everyday" {
                account.transaction_count = 3;
            }
        }
        let categories = default_categories();
        let payees = default_payees();
        let tags = default_tags();
        let transactions = default_transactions(&accounts, &categories, &payees, &tags, today());
        assert_eq!(count_for(&transactions, &accounts, "ANZ Everyday"), 3);
    }

    #[test]
    fn an_account_with_no_transaction_count_gets_none() {
        let mut accounts = default_accounts();
        for account in &mut accounts {
            account.transaction_count = 0;
        }
        let transactions = default_transactions(
            &accounts,
            &default_categories(),
            &default_payees(),
            &default_tags(),
            today(),
        );
        assert!(transactions.is_empty());
    }
}
