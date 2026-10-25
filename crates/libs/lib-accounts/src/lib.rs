//! Pure Accounts-surface domain types (`docs/ux/desktop-mockups/17-accounts/README.md`, section 3) --
//! `gpui`-free, the same "pure state, chrome renders it" split `settings.rs` uses. `view::accounts`
//! and its dialogs are the chrome; this module only knows what an account row holds, the fixed
//! order the page groups them in, and the stub rows the page is seeded with.
//!
//! All data is stubbed and in-memory (the Desktop Accounts Surface map's Destination): nothing
//! here reads `lib_database`.

use bigdecimal::BigDecimal;
use chrono::NaiveDate;
use lib_core::{AccountId, AccountType, InstitutionId, Money, UnitId};
/// The Institution a Cash account links to. The glossary gives every Account a mandatory
/// Institution, and Cash has no real one, so it points at a system-seeded placeholder. It is
/// deliberately absent from `lib_institutions::default_institutions()`: it is not a user-managed
/// row. Its id is the Institutions seed's place 0, which no seeded Institution takes.
///
/// This is a stable id, never display text: the row syncs between Clients, so storing a rendered
/// name would show one Client's Locale on another. Render it through the Desktop's
/// `institution_label`, which owns the Message.
pub const NO_INSTITUTION: InstitutionId = InstitutionId::seed(0);

/// The order the Accounts page groups accounts in -- fixed, never alphabetised or sorted by
/// balance (the README's implementation note 3). Not `AccountType::all()`, whose order puts
/// Investment before Loan; the desktop design lists Loan first.
pub const GROUP_ORDER: [AccountType; 5] = [
    AccountType::Cash,
    AccountType::Bank,
    AccountType::CreditCard,
    AccountType::Loan,
    AccountType::Investment,
];

/// One row of the Accounts page. `institution` and `unit` are ids, so the Account stays on its
/// Institution and Unit whatever they're called; their names are looked up when shown.
#[derive(Debug, Clone, PartialEq)]
pub struct Account {
    pub id: AccountId,
    pub name: String,
    pub institution: InstitutionId,
    pub account_type: AccountType,
    /// The Unit this Account counts in. Fixed at creation.
    pub unit: UnitId,
    /// In `unit`, at whatever precision the Unit carries (`0.4120` for btc). Its opening value is
    /// fixed at creation; nothing in this map moves it afterwards.
    pub balance: Money,
    /// UI-only: the design has the field, the domain model does not, so it is never persisted.
    pub account_number: Option<String>,
    pub opened_at: NaiveDate,
    /// Stub count shown by the Edit and Delete dialogs. Loan and Investment accounts take
    /// Repayments and Trades rather than Transactions, but this map shows one uniform figure.
    pub transaction_count: u32,
    /// Stub count of Budgets referencing the account, shown by the Delete dialog.
    pub budget_count: u32,
}

/// What the Edit dialog's Save writes onto an Account: the fields the form can change. Unit,
/// opening balance and opened date are fixed at creation, so they are not here.
#[derive(Debug, Clone, PartialEq)]
pub struct AccountDraft {
    pub name: String,
    pub institution: InstitutionId,
    pub account_type: AccountType,
    pub account_number: Option<String>,
}

/// What the Add dialog's Save creates: the editable fields plus those fixed at creation.
#[derive(Debug, Clone, PartialEq)]
pub struct NewAccount {
    pub draft: AccountDraft,
    pub unit: UnitId,
    pub balance: Money,
    pub opened_at: NaiveDate,
}

/// Why an Account write was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum AccountError {
    #[error("account not found")]
    NotFound,
    /// A form was confirmed without everything an Account needs.
    #[error("the account details are incomplete")]
    Incomplete,
}

/// Adds the Account `new` describes, with no Transactions or Budgets yet, and returns its id.
pub fn insert_account(accounts: &mut Vec<Account>, new: NewAccount) -> AccountId {
    let id = AccountId::new();
    accounts.push(Account {
        id,
        name: new.draft.name,
        institution: new.draft.institution,
        account_type: new.draft.account_type,
        unit: new.unit,
        balance: new.balance,
        account_number: new.draft.account_number,
        opened_at: new.opened_at,
        transaction_count: 0,
        budget_count: 0,
    });
    id
}

/// Writes `draft` onto Account `id`, leaving Unit, balance, opened date and the stub counts alone.
pub fn edit_account(
    accounts: &mut [Account],
    id: AccountId,
    draft: &AccountDraft,
) -> Result<(), AccountError> {
    let account = accounts
        .iter_mut()
        .find(|account| account.id == id)
        .ok_or(AccountError::NotFound)?;
    account.name.clone_from(&draft.name);
    account.institution = draft.institution;
    account.account_type = draft.account_type.clone();
    account.account_number.clone_from(&draft.account_number);
    Ok(())
}

/// Removes every row with Account `id`, so a deleted id can never leave the list orphaned, and
/// returns the first one removed. The Transactions booked to it are the Ledger's to delete.
pub fn remove_account(accounts: &mut Vec<Account>, id: AccountId) -> Result<Account, AccountError> {
    let first = get(accounts, id).ok_or(AccountError::NotFound)?.clone();
    accounts.retain(|account| account.id != id);
    Ok(first)
}

/// Adds `imported` to Account `id`'s stub Transaction count, after an Import books that many
/// Transactions to it.
pub fn count_imported(
    accounts: &mut [Account],
    id: AccountId,
    imported: u32,
) -> Result<(), AccountError> {
    let account = accounts
        .iter_mut()
        .find(|account| account.id == id)
        .ok_or(AccountError::NotFound)?;
    account.transaction_count = account.transaction_count.saturating_add(imported);
    Ok(())
}

/// Account `id`, if it exists.
pub fn get(accounts: &[Account], id: AccountId) -> Option<&Account> {
    accounts.iter().find(|account| account.id == id)
}

/// One type's block on the page: the indices (into the `Vec<Account>` it was built from) of its
/// accounts, in the order they were added.
#[derive(Debug, Clone, PartialEq)]
pub struct AccountGroup {
    pub account_type: AccountType,
    pub indices: Vec<usize>,
}

/// Groups `accounts` by type in [`GROUP_ORDER`], omitting types with no accounts.
pub fn group_accounts(accounts: &[Account]) -> Vec<AccountGroup> {
    GROUP_ORDER
        .iter()
        .filter_map(|account_type| {
            let indices: Vec<usize> = accounts
                .iter()
                .enumerate()
                .filter(|(_, account)| &account.account_type == account_type)
                .map(|(index, _)| index)
                .collect();
            (!indices.is_empty()).then(|| AccountGroup {
                account_type: account_type.clone(),
                indices,
            })
        })
        .collect()
}

/// Indices into `accounts` in the order the page shows them top to bottom -- what `j`/`k` step
/// through.
pub fn display_order(accounts: &[Account]) -> Vec<usize> {
    group_accounts(accounts)
        .into_iter()
        .flat_map(|group| group.indices)
        .collect()
}

/// The page header's figures under the no-cross-Unit rule: one net figure in the base Unit
/// only, with every other Unit held named rather than summed or silently dropped.
#[derive(Debug, Clone, PartialEq)]
pub struct NetWorth {
    pub account_count: usize,
    /// `None` when Settings has no base Unit, in which case there is no net figure at all.
    pub base_unit: Option<UnitId>,
    /// Signed sum of the base-Unit accounts' balances (loans and cards negative).
    pub base_net: Money,
    /// Other Units held, in first-seen order, each once.
    pub held_separately: Vec<UnitId>,
}

/// Computes the header's [`NetWorth`] for `accounts` against the Settings base Unit.
pub fn net_worth(accounts: &[Account], base_unit: Option<UnitId>) -> NetWorth {
    let mut base_net = Money(BigDecimal::from(0));
    let mut held_separately: Vec<UnitId> = Vec::new();
    for account in accounts {
        if base_unit == Some(account.unit) {
            base_net = Money(base_net.0 + account.balance.0.clone());
        } else if !held_separately.contains(&account.unit) {
            held_separately.push(account.unit);
        }
    }
    NetWorth {
        account_count: accounts.len(),
        base_unit,
        base_net,
        held_separately,
    }
}

/// The zero-based group position of the account at `accounts[index]` -- which group block on the
/// page holds it, for scrolling that block into view.
pub fn group_position(accounts: &[Account], index: usize) -> Option<usize> {
    group_accounts(accounts)
        .iter()
        .position(|group| group.indices.contains(&index))
}

/// The outcome of resolving a typed account name (`accounts delete Home Loan`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NameLookup {
    /// The account's index in the slice searched.
    Found(usize),
    NotFound,
    /// More than one account fits; their names, in list order.
    Ambiguous(Vec<String>),
}

/// Finds the account `text` names, case-insensitively, trying an exact name first, then a name
/// that starts with `text`, then one that merely contains it. The first tier with any hits
/// decides: one hit is the account, several are [`NameLookup::Ambiguous`] rather than a guess.
/// Account names are not unique (the schema has no such rule), so even an exact name can be
/// ambiguous.
pub fn find_by_name(accounts: &[Account], text: &str) -> NameLookup {
    let needle = text.trim().to_lowercase();
    if needle.is_empty() {
        return NameLookup::NotFound;
    }
    let tiers: [fn(&str, &str) -> bool; 3] = [
        |name, needle| name == needle,
        |name, needle| name.starts_with(needle),
        |name, needle| name.contains(needle),
    ];
    for tier in tiers {
        let hits: Vec<usize> = accounts
            .iter()
            .enumerate()
            .filter(|(_, account)| tier(&account.name.to_lowercase(), &needle))
            .map(|(index, _)| index)
            .collect();
        match hits.as_slice() {
            [] => continue,
            [index] => return NameLookup::Found(*index),
            many => {
                return NameLookup::Ambiguous(
                    many.iter().map(|&i| accounts[i].name.clone()).collect(),
                );
            }
        }
    }
    NameLookup::NotFound
}

#[expect(
    clippy::expect_used,
    reason = "only called with hand-written seed literals, and seed_rows_parse_and_link builds every one, so an invalid literal fails the test suite"
)]
fn money(text: &str) -> Money {
    text.parse()
        .expect("seed amounts are valid decimals (see seed_rows_parse_and_link)")
}

#[expect(
    clippy::expect_used,
    reason = "only called with hand-written seed literals, and seed_rows_parse_and_link builds every one, so an invalid literal fails the test suite"
)]
const fn date(year: i32, month: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(year, month, 1).expect("seed dates are valid calendar dates")
}

/// The mockup's rows (Bank, Credit card, Loan, Investment) plus one Cash row, so all five groups
/// exist. The mockup says "7 accounts" but draws six; Wallet is the seventh.
///
/// Two balances differ from the mockup so the header's net figure is *computed* rather than
/// hard-coded: ANZ Offset is 463,203.10 (not 61,204.10), and Wallet is 240.00. The mockup's own
/// aud rows sum to a large negative (the loan dwarfs the deposits) and its printed 83,995.87
/// matches no sum of them; with these two changes the aud rows sum to exactly 83,995.87.
pub fn default_accounts() -> Vec<Account> {
    vec![
        Account {
            id: AccountId::seed(1),
            name: "Wallet".to_string(),
            institution: NO_INSTITUTION,
            account_type: AccountType::Cash,
            unit: UnitId::SEED_AUD,
            balance: money("240.00"),
            account_number: None,
            opened_at: date(2021, 2),
            transaction_count: 96,
            budget_count: 0,
        },
        Account {
            id: AccountId::seed(2),
            name: "ANZ Everyday".to_string(),
            institution: InstitutionId::SEED_ANZ,
            account_type: AccountType::Bank,
            unit: UnitId::SEED_AUD,
            balance: money("4182.55"),
            account_number: Some("1234 5678".to_string()),
            opened_at: date(2019, 3),
            transaction_count: 312,
            budget_count: 3,
        },
        Account {
            id: AccountId::seed(3),
            name: "ANZ Offset".to_string(),
            institution: InstitutionId::SEED_ANZ,
            account_type: AccountType::Bank,
            unit: UnitId::SEED_AUD,
            balance: money("463203.10"),
            account_number: None,
            opened_at: date(2019, 3),
            transaction_count: 88,
            budget_count: 0,
        },
        Account {
            id: AccountId::seed(4),
            name: "Amex Platinum".to_string(),
            institution: InstitutionId::SEED_AMEX,
            account_type: AccountType::CreditCard,
            unit: UnitId::SEED_AUD,
            balance: money("-2318.44"),
            account_number: None,
            opened_at: date(2020, 8),
            transaction_count: 204,
            budget_count: 2,
        },
        Account {
            id: AccountId::seed(5),
            name: "Home Loan".to_string(),
            institution: InstitutionId::SEED_WESTPAC,
            account_type: AccountType::Loan,
            unit: UnitId::SEED_AUD,
            balance: money("-381311.34"),
            account_number: None,
            opened_at: date(2019, 6),
            transaction_count: 36,
            budget_count: 0,
        },
        Account {
            id: AccountId::seed(6),
            name: "Vanguard VAS".to_string(),
            institution: InstitutionId::SEED_VANGUARD,
            account_type: AccountType::Investment,
            unit: UnitId::SEED_VAS,
            balance: money("1240"),
            account_number: None,
            opened_at: date(2022, 1),
            transaction_count: 27,
            budget_count: 0,
        },
        Account {
            id: AccountId::seed(7),
            name: "Bitcoin".to_string(),
            institution: InstitutionId::SEED_CRYPTO_EXCHANGE,
            account_type: AccountType::Investment,
            unit: UnitId::SEED_BTC,
            balance: money("0.4120"),
            account_number: None,
            opened_at: date(2021, 11),
            transaction_count: 9,
            budget_count: 0,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn account(id: AccountId, name: &str, account_type: AccountType) -> Account {
        Account {
            id,
            name: name.to_string(),
            institution: NO_INSTITUTION,
            account_type,
            unit: UnitId::SEED_AUD,
            balance: money("0"),
            account_number: None,
            opened_at: date(2024, 1),
            transaction_count: 0,
            budget_count: 0,
        }
    }

    #[test]
    fn group_order_covers_every_account_type_once() {
        for account_type in AccountType::all() {
            assert_eq!(
                GROUP_ORDER.iter().filter(|t| *t == account_type).count(),
                1,
                "{account_type} must appear exactly once"
            );
        }
    }

    #[test]
    fn groups_follow_fixed_order_not_insertion_order() {
        let accounts = vec![
            account(AccountId::seed(1), "Fund", AccountType::Investment),
            account(AccountId::seed(2), "Mortgage", AccountType::Loan),
            account(AccountId::seed(3), "Card", AccountType::CreditCard),
            account(AccountId::seed(4), "Savings", AccountType::Bank),
        ];
        let types: Vec<_> = group_accounts(&accounts)
            .into_iter()
            .map(|group| group.account_type)
            .collect();
        assert_eq!(
            types,
            vec![
                AccountType::Bank,
                AccountType::CreditCard,
                AccountType::Loan,
                AccountType::Investment,
            ]
        );
    }

    #[test]
    fn empty_types_are_omitted() {
        let accounts = vec![account(AccountId::seed(1), "Savings", AccountType::Bank)];
        let groups = group_accounts(&accounts);
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].account_type, AccountType::Bank);
        assert!(group_accounts(&[]).is_empty());
    }

    #[test]
    fn within_a_group_accounts_keep_insertion_order() {
        let accounts = vec![
            account(AccountId::seed(1), "Zebra", AccountType::Bank),
            account(AccountId::seed(2), "Apple", AccountType::Bank),
        ];
        assert_eq!(group_accounts(&accounts)[0].indices, vec![0, 1]);
    }

    #[test]
    fn display_order_visits_every_account_exactly_once() {
        let accounts = default_accounts();
        let mut order = display_order(&accounts);
        assert_eq!(order.len(), accounts.len());
        order.sort_unstable();
        order.dedup();
        assert_eq!(order.len(), accounts.len());
    }

    #[test]
    fn seed_covers_all_five_groups() {
        assert_eq!(group_accounts(&default_accounts()).len(), 5);
    }

    #[test]
    fn net_worth_sums_base_unit_only_and_names_the_rest() {
        let net = net_worth(&default_accounts(), Some(UnitId::SEED_AUD));
        assert_eq!(net.account_count, 7);
        assert_eq!(net.base_net, money("83995.87"));
        assert_eq!(
            net.held_separately,
            vec![UnitId::SEED_VAS, UnitId::SEED_BTC]
        );
    }

    #[test]
    fn net_worth_with_no_base_unit_accounts_is_zero_and_lists_every_unit() {
        let accounts = vec![Account {
            unit: UnitId::SEED_BTC,
            ..account(AccountId::seed(1), "Coins", AccountType::Investment)
        }];
        let net = net_worth(&accounts, Some(UnitId::SEED_AUD));
        assert_eq!(net.base_net, money("0"));
        assert_eq!(net.held_separately, vec![UnitId::SEED_BTC]);
    }

    #[test]
    fn net_worth_with_no_base_unit_sums_nothing() {
        let net = net_worth(&default_accounts(), None);
        assert_eq!(net.base_net, money("0"));
        assert_eq!(net.base_unit, None);
        assert_eq!(net.held_separately.len(), 3);
    }

    #[test]
    fn group_position_finds_the_block_holding_an_account() {
        let accounts = default_accounts();
        // Wallet (Cash) is the first block; Bitcoin (Investment) is the last of five.
        assert_eq!(group_position(&accounts, 0), Some(0));
        assert_eq!(group_position(&accounts, 6), Some(4));
        assert_eq!(group_position(&accounts, 99), None);
    }

    #[test]
    fn find_by_name_prefers_an_exact_name_case_insensitively() {
        let accounts = default_accounts();
        assert_eq!(find_by_name(&accounts, "home loan"), NameLookup::Found(4));
        assert_eq!(
            find_by_name(&accounts, "  HOME LOAN "),
            NameLookup::Found(4)
        );
    }

    #[test]
    fn find_by_name_falls_back_to_a_unique_prefix_then_a_unique_substring() {
        let accounts = default_accounts();
        assert_eq!(find_by_name(&accounts, "amex"), NameLookup::Found(3));
        assert_eq!(find_by_name(&accounts, "platinum"), NameLookup::Found(3));
        assert_eq!(find_by_name(&accounts, "bitc"), NameLookup::Found(6));
    }

    #[test]
    fn find_by_name_reports_ambiguity_instead_of_guessing() {
        let accounts = default_accounts();
        assert_eq!(
            find_by_name(&accounts, "anz"),
            NameLookup::Ambiguous(vec!["ANZ Everyday".to_string(), "ANZ Offset".to_string()])
        );
    }

    #[test]
    fn an_exact_name_wins_over_longer_names_that_contain_it() {
        let mut accounts = default_accounts();
        accounts.push(Account {
            name: "ANZ".to_string(),
            ..account(AccountId::seed(20), "ANZ", AccountType::Bank)
        });
        assert_eq!(find_by_name(&accounts, "anz"), NameLookup::Found(7));
    }

    #[test]
    fn duplicate_exact_names_are_ambiguous() {
        let accounts = vec![
            account(AccountId::seed(1), "Savings", AccountType::Bank),
            account(AccountId::seed(2), "Savings", AccountType::Bank),
        ];
        assert_eq!(
            find_by_name(&accounts, "savings"),
            NameLookup::Ambiguous(vec!["Savings".to_string(), "Savings".to_string()])
        );
    }

    #[test]
    fn find_by_name_finds_nothing_for_an_unknown_or_empty_name() {
        let accounts = default_accounts();
        assert_eq!(find_by_name(&accounts, "zzz"), NameLookup::NotFound);
        assert_eq!(find_by_name(&accounts, "   "), NameLookup::NotFound);
        assert_eq!(find_by_name(&[], "wallet"), NameLookup::NotFound);
    }

    fn draft(name: &str) -> AccountDraft {
        AccountDraft {
            name: name.to_string(),
            institution: InstitutionId::SEED_WESTPAC,
            account_type: AccountType::CreditCard,
            account_number: Some("42".to_string()),
        }
    }

    #[test]
    fn insert_gives_a_fresh_id_with_zero_counts() {
        let mut accounts = vec![account(AccountId::seed(4), "Savings", AccountType::Bank)];
        let id = insert_account(
            &mut accounts,
            NewAccount {
                draft: draft("Card"),
                unit: UnitId::SEED_AUD,
                balance: money("5"),
                opened_at: date(2026, 2),
            },
        );
        assert_ne!(id, AccountId::seed(4));
        let added = get(&accounts, id).expect("inserted");
        assert_eq!(added.name, "Card");
        assert_eq!((added.transaction_count, added.budget_count), (0, 0));
    }

    #[test]
    fn edit_writes_the_draft_and_leaves_the_fixed_fields() {
        let mut accounts = vec![account(AccountId::seed(7), "Old", AccountType::Bank)];
        edit_account(&mut accounts, AccountId::seed(7), &draft("New")).expect("account 7 exists");
        let edited = get(&accounts, AccountId::seed(7)).expect("still there");
        assert_eq!(edited.name, "New");
        assert_eq!(edited.account_type, AccountType::CreditCard);
        assert_eq!(edited.account_number.as_deref(), Some("42"));
        assert_eq!(edited.unit, UnitId::SEED_AUD);
        assert_eq!(
            edit_account(&mut accounts, AccountId::seed(8), &draft("x")),
            Err(AccountError::NotFound)
        );
    }

    #[test]
    fn remove_takes_every_row_with_the_id() {
        let mut accounts = vec![
            account(AccountId::seed(4), "First", AccountType::Bank),
            account(AccountId::seed(4), "Second", AccountType::Bank),
            account(AccountId::seed(5), "Other", AccountType::Bank),
        ];
        let removed = remove_account(&mut accounts, AccountId::seed(4)).expect("id 4 exists");
        assert_eq!(removed.name, "First");
        assert_eq!(accounts.len(), 1);
        assert_eq!(
            remove_account(&mut accounts, AccountId::seed(4)),
            Err(AccountError::NotFound)
        );
    }
}
