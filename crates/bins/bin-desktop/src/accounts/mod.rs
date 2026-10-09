//! Pure Accounts-surface domain types (`docs/ux/desktop-mockups/17-accounts/README.md`, section 3) --
//! `gpui`-free, the same "pure state, chrome renders it" split `settings.rs` uses. `view::accounts`
//! and its dialogs are the chrome; this module only knows what an account row holds, the fixed
//! order the page groups them in, and the stub rows the page is seeded with.
//!
//! All data is stubbed and in-memory (the Desktop Accounts Surface map's Destination): nothing
//! here reads `lib_database`.

pub(crate) mod form;

pub use lib_accounts::*;

use lib_locale::Label;

/// What to show for an account's `institution`: the placeholder's Message in the Locale in
/// effect, or a user-managed Institution's name unchanged (Ledger data, not a Message).
pub fn institution_label(institution: &str) -> String {
    if institution == NO_INSTITUTION {
        lib_locale::msg::institution_none()
    } else {
        institution.to_string()
    }
}

/// `"7 accounts"`, or `"1 account"`.
pub fn count_text(net: &NetWorth) -> String {
    crate::msg::desktop_accounts_count(i64::try_from(net.account_count).unwrap_or(i64::MAX))
}

/// `"vas, btc held separately"`, or `None` when every account is in the base Unit.
pub fn held_separately_text(net: &NetWorth) -> Option<String> {
    (!net.held_separately.is_empty())
        .then(|| crate::msg::desktop_accounts_held_separately(&net.held_separately.join(", ")))
}

/// The Accounts group labels as Messages, in [`GROUP_ORDER`]'s order.
pub fn group_labels() -> Vec<String> {
    GROUP_ORDER.iter().map(Label::label).collect()
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;
    use lib_core::{AccountType, Money};

    use super::*;

    fn date(year: i32, month: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(year, month, 1).expect("test dates are valid")
    }

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

    fn money(text: &str) -> Money {
        text.parse().expect("test amounts are valid decimals")
    }

    #[test]
    fn group_order_puts_loan_before_investment() {
        let labels = group_labels();
        assert_eq!(
            labels,
            vec!["Cash", "Bank", "Credit card", "Loan", "Investment"]
        );
    }

    #[test]
    fn seeded_net_worth_text_names_the_held_separately_units() {
        crate::locale::init_for_tests();
        let net = net_worth(&default_accounts(), Some("aud"));
        assert_eq!(count_text(&net), "7 accounts");
        assert_eq!(
            held_separately_text(&net).as_deref(),
            Some("vas, btc held separately")
        );
    }

    #[test]
    fn account_type_is_still_in_scope_for_the_group_labels() {
        assert_eq!(GROUP_ORDER[0], AccountType::Cash);
    }

    #[test]
    fn institution_label_renders_the_placeholder_and_passes_names_through() {
        lib_locale::with_locale(lib_locale::Locale::EnUs, || {
            assert_eq!(institution_label(NO_INSTITUTION), "No institution");
            assert_eq!(institution_label("ANZ Banking Group"), "ANZ Banking Group");
        });
    }

    #[test]
    fn net_worth_count_text_is_singular_for_one_account() {
        crate::locale::init_for_tests();
        let net = net_worth(&[account(1, "Only", AccountType::Cash)], Some("aud"));
        assert_eq!(count_text(&net), "1 account");
        assert_eq!(held_separately_text(&net), None);
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
}
