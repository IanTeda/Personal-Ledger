//! The stub Transactions seed's tests that read the other domains' stubs: the generated history
//! over the stub Accounts and the Category, Payee and Tag names it books by, the rows the Bills and
//! Documents stubs name, and each Account's counted Transactions. Here, beside the joined stub,
//! since `lib-transactions` depends on no other domain (moved from `lib-ledger`, #1056).

use bigdecimal::BigDecimal;
use chrono::Duration;
use lib_core::{Money, TransactionStatus};
use lib_transactions::Split;

use lib_accounts::{Account, default_accounts};
use lib_categories::{self as categories, Category, default_categories};
use lib_payees::{self as payees, Payee, default_payees};
use lib_tags::{self as tags, Tag, default_tags};
use lib_transactions::fake::{FIRST_BOOKED, HISTORY_DAYS, stub_transaction_count};

use super::*;

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
fn every_stub_account_names_a_stub_institution() {
    let institutions = lib_institutions::default_institutions();
    for account in default_accounts() {
        assert!(
            lib_institutions::get(&institutions, account.institution).is_some(),
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
    let accounts = default_accounts();
    let categories = default_categories();
    let payees = default_payees();
    let tags = default_tags();
    let transactions = counted_transactions(
        &accounts,
        |id| {
            if id == AccountId::seed(2) {
                3
            } else {
                stub_transaction_count(id)
            }
        },
        &categories,
        &payees,
        &tags,
        today(),
    );
    assert_eq!(count_for(&transactions, &accounts, "ANZ Everyday"), 3);
}

/// `lib-transactions`' stub, as `lib-database` seeds it, holds every Transaction the other
/// domains' seeds name: each Paid entry's, which carries its Bill Plan's Account, Category,
/// Payee and name, and each Document Link's.
#[test]
fn every_paid_entry_and_document_link_names_a_stub_transaction() {
    let accounts = default_accounts();
    let categories = default_categories();
    let payees = default_payees();
    let transactions = stub_transactions(today());
    let bills = stub_bills(today());
    assert_eq!(
        FIRST_BOOKED,
        TransactionId::seed(
            default_transactions(&accounts, &categories, &payees, &default_tags(), today()).len()
                as u32
                + 1
        )
    );
    let named = |id| transactions.iter().find(|t| t.id == id);

    let mut paid = 0;
    for entry in &bills.entries {
        let lib_bills::Resolution::Paid(split) = entry.resolution else {
            continue;
        };
        paid += 1;
        let plan = lib_bills::get(&bills.plans, entry.plan_id).unwrap();
        let transaction = named(split.transaction_id)
            .unwrap_or_else(|| panic!("{} on {} names no Transaction", plan.name, entry.due));
        let paid_by = &transaction.splits[split.split_index];
        assert_eq!(transaction.account_id, plan.account_id, "{}", plan.name);
        assert_eq!(paid_by.category_id, plan.category_id, "{}", plan.name);
        assert_eq!(paid_by.payee_id, plan.payee_id, "{}", plan.name);
        assert_eq!(transaction.description.as_ref(), Some(&plan.name));
    }
    assert_eq!(paid, 67, "the stub Bills' Paid entries");

    let links: Vec<_> = stub_documents(today(), &stub_institutions())
        .iter()
        .flat_map(|document| &document.links)
        .filter_map(|link| match link {
            lib_documents::DocumentLink::Transaction(id) => Some(*id),
            _ => None,
        })
        .collect();
    assert!(!links.is_empty());
    for id in links {
        assert!(
            named(id).is_some(),
            "a Document links {id}, not in the stub"
        );
    }
}

#[test]
fn an_account_with_no_transaction_count_gets_none() {
    let transactions = counted_transactions(
        &default_accounts(),
        |_| 0,
        &default_categories(),
        &default_payees(),
        &default_tags(),
        today(),
    );
    assert!(transactions.is_empty());
}

/// Every id a Transaction names must resolve in the stub, or a View would render a dangling row.
#[test]
fn every_transaction_reference_resolves_in_the_stub() {
    let (accounts, categories, payees, tags) = (
        stub_accounts(),
        stub_categories(),
        stub_payees(),
        stub_tags(),
    );
    for transaction in stub_transactions(today()) {
        assert!(accounts.iter().any(|a| a.id == transaction.account_id));
        for split in &transaction.splits {
            assert!(categories.iter().any(|c| c.id == split.category_id));
            if let Some(payee_id) = split.payee_id {
                assert!(payees.iter().any(|p| p.id == payee_id));
            }
            for tag_id in &split.tag_ids {
                assert!(tags.iter().any(|t| t.id == *tag_id));
            }
        }
    }
}

#[test]
fn the_stubs_accounts_count_their_transactions() {
    let transactions = stub_transactions(today());
    for account in stub_accounts() {
        let booked = transactions
            .iter()
            .filter(|t| t.account_id == account.id)
            .count();
        assert_eq!(
            account_transactions(&transactions, account.id),
            booked,
            "{}",
            account.name
        );
    }
    assert!(account_transactions(&transactions, AccountId::seed(2)) > 0);
    assert_eq!(account_transactions(&transactions, AccountId::new()), 0);
}

#[test]
fn a_new_account_has_no_transactions_until_one_is_booked_to_it() {
    use lib_accounts::{AccountDraft, NewAccount};
    use lib_core::{AccountType, InstitutionId, UnitId};

    let mut accounts = stub_accounts();
    let mut transactions = stub_transactions(today());
    let id = lib_accounts::insert_account(
        &mut accounts,
        NewAccount {
            draft: AccountDraft {
                name: "Fresh".into(),
                institution: InstitutionId::SEED_ANZ,
                account_type: AccountType::Bank,
                account_number: None,
            },
            unit: UnitId::SEED_AUD,
            balance: "0".parse().expect("a decimal"),
            opened_at: NaiveDate::from_ymd_opt(2026, 1, 1).expect("a date"),
        },
    )
    .id;

    assert_eq!(account_transactions(&transactions, id), 0);
    let mut booked = transactions[0].clone();
    booked.id = TransactionId::seed(9_999);
    booked.account_id = id;
    transactions.push(booked);
    assert_eq!(account_transactions(&transactions, id), 1);
}
