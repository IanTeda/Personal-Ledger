//! Every domain value has a Message, and its label is never the stored token.

use lib_core::{AccountType, BudgetPeriod, CategoryTypes, TransactionStatus, UnitKind};
use lib_locale::{Label, Locale, flagged_label, with_locale};

/// `(stored token, label)` for every variant of every enum, in the Locale in effect.
fn every_label() -> Vec<(String, String)> {
    let mut labels = Vec::new();
    labels.extend(
        AccountType::all()
            .iter()
            .map(|v| (v.as_str().to_string(), v.label())),
    );
    labels.extend(
        TransactionStatus::all()
            .iter()
            .map(|v| (v.as_str().to_string(), v.label())),
    );
    labels.extend(
        CategoryTypes::all()
            .iter()
            .map(|v| (v.as_str().to_string(), v.label())),
    );
    labels.extend(
        UnitKind::all()
            .iter()
            .map(|v| (v.as_str().to_string(), v.label())),
    );
    labels.extend(
        BudgetPeriod::all()
            .iter()
            .map(|v| (v.as_str().to_string(), v.label())),
    );
    labels.push(("flagged".to_string(), flagged_label()));
    labels
}

#[test]
fn there_are_twenty_labels() {
    with_locale(Locale::EnUs, || assert_eq!(every_label().len(), 20));
}

#[test]
fn every_variant_has_a_message_in_every_locale() {
    for locale in Locale::SUPPORTED {
        with_locale(locale, || {
            for (token, label) in every_label() {
                assert!(!label.contains('⟦'), "{locale}: `{token}` has no Message");
                assert!(!label.is_empty(), "{locale}: `{token}` is empty");
            }
        });
    }
}

#[test]
fn a_label_is_never_the_stored_token() {
    for locale in Locale::SUPPORTED {
        with_locale(locale, || {
            for (token, label) in every_label() {
                assert_ne!(label, token, "{locale}");
                assert!(
                    !label.contains('_'),
                    "{locale}: `{label}` looks like a token"
                );
            }
        });
    }
}

#[test]
fn en_us_labels_use_sentence_case() {
    with_locale(Locale::EnUs, || {
        assert_eq!(AccountType::CreditCard.label(), "Credit card");
        assert_eq!(TransactionStatus::Reconciled.label(), "Reconciled");
        assert_eq!(UnitKind::PreciousMetal.label(), "Precious metal");
        assert_eq!(flagged_label(), "Flagged");
    });
}

#[test]
fn en_xa_wraps_labels() {
    with_locale(Locale::EnXa, || {
        let label = AccountType::Bank.label();
        assert!(label.starts_with('[') && label.ends_with(']'), "{label}");
    });
}

#[test]
fn the_seeded_placeholder_institution_has_a_message_in_every_locale() {
    for locale in Locale::SUPPORTED {
        with_locale(locale, || {
            let text = lib_locale::msg::institution_none();
            assert!(!text.contains('⟦'), "{locale}: {text}");
        });
    }
    with_locale(Locale::EnUs, || {
        assert_eq!(lib_locale::msg::institution_none(), "No institution")
    });
}
