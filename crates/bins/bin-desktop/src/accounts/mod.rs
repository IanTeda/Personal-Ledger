//! Pure Accounts-surface domain types (`docs/ux/desktop-mockups/17-accounts/README.md`, section 3) --
//! `gpui`-free, the same "pure state, chrome renders it" split `settings.rs` uses. `view::accounts`
//! and its dialogs are the chrome; this module only knows what an account row holds, the fixed
//! order the page groups them in, and the stub rows the page is seeded with.
//!
//! All data is stubbed and in-memory (the Desktop Accounts Surface map's Destination): nothing
//! here reads `lib_database`.

pub(crate) mod form;

use bigdecimal::BigDecimal;
use chrono::NaiveDate;
use lib_core::{AccountType, Money};
/// The Institution a Cash account links to. The glossary gives every Account a mandatory
/// Institution, and Cash has no real one, so it points at a system-seeded placeholder. It is
/// deliberately absent from `institutions::default_institutions()`: it is not a user-managed row.
///
/// This is a stable key, never display text: the row syncs between Clients, so storing a rendered
/// name would show one Client's Locale on another. Render it with [`institution_label`].
pub const NO_INSTITUTION: &str = "system:institution-none";

/// What to show for an account's `institution`: the placeholder's Message in the Locale in
/// effect, or a user-managed Institution's name unchanged (Ledger data, not a Message).
pub fn institution_label(institution: &str) -> String {
    if institution == NO_INSTITUTION {
        lib_locale::msg::institution_none()
    } else {
        institution.to_string()
    }
}

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

/// One row of the Accounts page. `institution` and `unit` are the Institution's name and the
/// Unit's code -- the same value keys the select control stores (issue: select control for the
/// Add/Edit dialogs), so a row survives the Settings lists changing underneath it.
#[derive(Debug, Clone, PartialEq)]
pub struct Account {
    pub id: u32,
    pub name: String,
    pub institution: String,
    pub account_type: AccountType,
    /// Unit code (e.g. `"aud"`). Fixed at creation.
    pub unit: String,
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

/// The next unused account id.
pub fn next_account_id(accounts: &[Account]) -> u32 {
    accounts
        .iter()
        .map(|account| account.id)
        .max()
        .map_or(1, |max| max + 1)
}

/// The page header's figures under the no-cross-Unit rule: one net figure in the base Unit
/// only, with every other Unit held named rather than summed or silently dropped.
#[derive(Debug, Clone, PartialEq)]
pub struct NetWorth {
    pub account_count: usize,
    /// `None` when Settings has no base Unit, in which case there is no net figure at all.
    pub base_unit: Option<String>,
    /// Signed sum of the base-Unit accounts' balances (loans and cards negative).
    pub base_net: Money,
    /// Other Unit codes held, in first-seen order, each once.
    pub held_separately: Vec<String>,
}

/// Computes the header's [`NetWorth`] for `accounts` against the Settings base Unit.
pub fn net_worth(accounts: &[Account], base_unit: Option<&str>) -> NetWorth {
    let mut base_net = Money(BigDecimal::from(0));
    let mut held_separately: Vec<String> = Vec::new();
    for account in accounts {
        if base_unit == Some(account.unit.as_str()) {
            base_net = Money(base_net.0 + account.balance.0.clone());
        } else if !held_separately.contains(&account.unit) {
            held_separately.push(account.unit.clone());
        }
    }
    NetWorth {
        account_count: accounts.len(),
        base_unit: base_unit.map(str::to_string),
        base_net,
        held_separately,
    }
}

impl NetWorth {
    /// `"7 accounts"`, or `"1 account"`.
    pub fn count_text(&self) -> String {
        crate::msg::desktop_accounts_count(i64::try_from(self.account_count).unwrap_or(i64::MAX))
    }

    /// `"vas, btc held separately"`, or `None` when every account is in the base Unit.
    pub fn held_separately_text(&self) -> Option<String> {
        (!self.held_separately.is_empty())
            .then(|| crate::msg::desktop_accounts_held_separately(&self.held_separately.join(", ")))
    }
}

/// Moves a selection (a position in [`display_order`]) by `delta` rows, clamped to `0..len`. An
/// empty list always selects `0`.
pub fn step_selection(selected: usize, len: usize, delta: isize) -> usize {
    if len == 0 {
        return 0;
    }
    selected.saturating_add_signed(delta).min(len - 1)
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
            id: 1,
            name: "Wallet".to_string(),
            institution: NO_INSTITUTION.to_string(),
            account_type: AccountType::Cash,
            unit: "aud".to_string(),
            balance: money("240.00"),
            account_number: None,
            opened_at: date(2021, 2),
            transaction_count: 96,
            budget_count: 0,
        },
        Account {
            id: 2,
            name: "ANZ Everyday".to_string(),
            institution: "ANZ Banking Group".to_string(),
            account_type: AccountType::Bank,
            unit: "aud".to_string(),
            balance: money("4182.55"),
            account_number: Some("1234 5678".to_string()),
            opened_at: date(2019, 3),
            transaction_count: 312,
            budget_count: 3,
        },
        Account {
            id: 3,
            name: "ANZ Offset".to_string(),
            institution: "ANZ Banking Group".to_string(),
            account_type: AccountType::Bank,
            unit: "aud".to_string(),
            balance: money("463203.10"),
            account_number: None,
            opened_at: date(2019, 3),
            transaction_count: 88,
            budget_count: 0,
        },
        Account {
            id: 4,
            name: "Amex Platinum".to_string(),
            institution: "American Express".to_string(),
            account_type: AccountType::CreditCard,
            unit: "aud".to_string(),
            balance: money("-2318.44"),
            account_number: None,
            opened_at: date(2020, 8),
            transaction_count: 204,
            budget_count: 2,
        },
        Account {
            id: 5,
            name: "Home Loan".to_string(),
            institution: "Westpac Banking".to_string(),
            account_type: AccountType::Loan,
            unit: "aud".to_string(),
            balance: money("-381311.34"),
            account_number: None,
            opened_at: date(2019, 6),
            transaction_count: 36,
            budget_count: 0,
        },
        Account {
            id: 6,
            name: "Vanguard VAS".to_string(),
            institution: "Vanguard Investments".to_string(),
            account_type: AccountType::Investment,
            unit: "vas".to_string(),
            balance: money("1240"),
            account_number: None,
            opened_at: date(2022, 1),
            transaction_count: 27,
            budget_count: 0,
        },
        Account {
            id: 7,
            name: "Bitcoin".to_string(),
            institution: "Cryptocurrency Exchange".to_string(),
            account_type: AccountType::Investment,
            unit: "btc".to_string(),
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
    use lib_locale::Label;

    use super::*;

    fn account(id: u32, name: &str, account_type: AccountType) -> Account {
        Account {
            id,
            name: name.to_string(),
            institution: NO_INSTITUTION.to_string(),
            account_type,
            unit: "aud".to_string(),
            balance: money("0"),
            account_number: None,
            opened_at: date(2024, 1),
            transaction_count: 0,
            budget_count: 0,
        }
    }

    #[test]
    fn group_order_puts_loan_before_investment() {
        let labels: Vec<_> = GROUP_ORDER.iter().map(Label::label).collect();
        assert_eq!(
            labels,
            vec!["Cash", "Bank", "Credit card", "Loan", "Investment"]
        );
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
            account(1, "Fund", AccountType::Investment),
            account(2, "Mortgage", AccountType::Loan),
            account(3, "Card", AccountType::CreditCard),
            account(4, "Savings", AccountType::Bank),
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
        let accounts = vec![account(1, "Savings", AccountType::Bank)];
        let groups = group_accounts(&accounts);
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].account_type, AccountType::Bank);
        assert!(group_accounts(&[]).is_empty());
    }

    #[test]
    fn within_a_group_accounts_keep_insertion_order() {
        let accounts = vec![
            account(1, "Zebra", AccountType::Bank),
            account(2, "Apple", AccountType::Bank),
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
    fn next_account_id_is_one_past_the_max_and_starts_at_one() {
        assert_eq!(next_account_id(&[]), 1);
        assert_eq!(next_account_id(&default_accounts()), 8);
    }

    #[test]
    fn seed_rows_parse_and_link() {
        let accounts = default_accounts();
        assert_eq!(accounts.len(), 7);

        let mut ids: Vec<_> = accounts.iter().map(|a| a.id).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), accounts.len(), "ids must be unique");

        let units = crate::units::default_units();
        let institutions = crate::institutions::default_institutions();
        for account in &accounts {
            assert!(
                units.iter().any(|unit| unit.code == account.unit),
                "{} names an unknown unit {}",
                account.name,
                account.unit
            );
            assert!(
                account.institution == NO_INSTITUTION
                    || institutions.iter().any(|i| i.name == account.institution),
                "{} names an unknown institution {}",
                account.name,
                account.institution
            );
        }
    }

    #[test]
    fn seed_covers_all_five_groups() {
        assert_eq!(group_accounts(&default_accounts()).len(), 5);
    }

    #[test]
    fn seeded_base_unit_rows_sum_to_the_mockups_net_worth() {
        let base = crate::units::default_units()
            .into_iter()
            .find(|unit| unit.is_base)
            .map(|unit| unit.code)
            .unwrap_or_default();
        let net = default_accounts()
            .into_iter()
            .filter(|account| account.unit == base)
            .fold(money("0"), |sum, account| Money(sum.0 + account.balance.0));
        assert_eq!(net, money("83995.87"));
    }

    #[test]
    fn net_worth_sums_base_unit_only_and_names_the_rest() {
        crate::locale::init_for_tests();
        let net = net_worth(&default_accounts(), Some("aud"));
        assert_eq!(net.account_count, 7);
        assert_eq!(net.base_net, money("83995.87"));
        assert_eq!(
            net.held_separately,
            vec!["vas".to_string(), "btc".to_string()]
        );
        assert_eq!(net.count_text(), "7 accounts");
        assert_eq!(
            net.held_separately_text().as_deref(),
            Some("vas, btc held separately")
        );
    }

    #[test]
    fn net_worth_with_no_base_unit_accounts_is_zero_and_lists_every_unit() {
        let accounts = vec![Account {
            unit: "btc".to_string(),
            ..account(1, "Coins", AccountType::Investment)
        }];
        let net = net_worth(&accounts, Some("aud"));
        assert_eq!(net.base_net, money("0"));
        assert_eq!(net.held_separately, vec!["btc".to_string()]);
    }

    #[test]
    fn net_worth_with_no_base_unit_sums_nothing() {
        let net = net_worth(&default_accounts(), None);
        assert_eq!(net.base_net, money("0"));
        assert_eq!(net.base_unit, None);
        assert_eq!(net.held_separately.len(), 3);
    }

    #[test]
    fn net_worth_count_text_is_singular_for_one_account() {
        crate::locale::init_for_tests();
        let net = net_worth(&[account(1, "Only", AccountType::Cash)], Some("aud"));
        assert_eq!(net.count_text(), "1 account");
        assert_eq!(net.held_separately_text(), None);
    }

    #[test]
    fn step_selection_clamps_at_both_ends_and_tolerates_an_empty_list() {
        assert_eq!(step_selection(0, 7, -1), 0);
        assert_eq!(step_selection(3, 7, 1), 4);
        assert_eq!(step_selection(6, 7, 1), 6);
        assert_eq!(step_selection(2, 7, -5), 0);
        assert_eq!(step_selection(2, 7, 100), 6);
        assert_eq!(step_selection(0, 0, 1), 0);
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
    fn institution_label_renders_the_placeholder_and_passes_names_through() {
        lib_locale::with_locale(lib_locale::Locale::EnUs, || {
            assert_eq!(institution_label(NO_INSTITUTION), "No institution");
            assert_eq!(institution_label("ANZ Banking Group"), "ANZ Banking Group");
        });
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
            ..account(20, "ANZ", AccountType::Bank)
        });
        assert_eq!(find_by_name(&accounts, "anz"), NameLookup::Found(7));
    }

    #[test]
    fn duplicate_exact_names_are_ambiguous() {
        let accounts = vec![
            account(1, "Savings", AccountType::Bank),
            account(2, "Savings", AccountType::Bank),
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
}
