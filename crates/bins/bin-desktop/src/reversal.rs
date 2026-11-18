//! What the Desktop's History entries reverse (#1238, #1244, #1247): the changes an input made,
//! recorded below the Presenters as they are written and closed into one entry when the input's
//! `send` drains (`Root::drain_notices`). `DeviceConfigFile` records `[General]`'s and
//! `[Display]`'s keys here, and the Open Ledger's stores their row images in `lib-database`'s
//! journal, which the drain takes as the entry's Ledger part. Only a Units change makes a Ledger
//! entry yet: the Institutions record beside it, for the Unit delete that reassigns them, and the
//! other domains join with #1248 and #1249.
//!
//! The recording is per thread rather than held by `Root`, as the Notice queue is
//! (`crate::notices`), since the device writes from inside a page's update with no way to reach
//! `Root`.

use std::cell::RefCell;

use lib_core::configuration::{ConfigKey, Section};
use lib_database::{Domain, LedgerReversal};
use lib_shell::Entry;

/// One key's change to the user's configuration file: its raw value before and after, `None`
/// where the file didn't hold it.
pub(crate) type ConfigChange = (ConfigKey, Option<String>, Option<String>);

/// What reverses one History entry: the Configuration it changed, in the order written, and the
/// Open Ledger's rows it changed (#1240).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Reversal {
    pub configuration: Vec<ConfigChange>,
    pub ledger: Option<LedgerReversal>,
}

impl Reversal {
    /// Drops the Ledger's part as its Ledger closes, saying whether a Configuration part is left
    /// to step (`lib_shell::ShellPresenter::ledger_closed`).
    pub(crate) fn drop_ledger(&mut self) -> bool {
        self.ledger = None;
        !self.configuration.is_empty()
    }
}

thread_local! {
    static RECORDED: RefCell<Vec<ConfigChange>> = const { RefCell::new(Vec::new()) };
}

/// Notes that the running input changed `key` in the file from `before` to `after`. A write that
/// left the file's value as it was is no change, and only `[General]` and `[Display]` are recorded.
pub(crate) fn record(key: ConfigKey, before: Option<String>, after: Option<String>) {
    if before == after || !matches!(key.section(), Section::General | Section::Display) {
        return;
    }
    RECORDED.with_borrow_mut(|recorded| recorded.push((key, before, after)));
}

/// What the input recorded, as one History entry labelled by `labels`, the `Notice::Changed`s it
/// raised, with `ledger`, the Open Ledger's row images the input wrote: `None` when it changed
/// nothing. With no label, the keys' names and the Ledger's domains stand in for one.
pub(crate) fn close(
    labels: Vec<String>,
    ledger: Option<LedgerReversal>,
) -> Option<Entry<Reversal>> {
    let configuration = RECORDED.with_borrow_mut(std::mem::take);
    // The halfway state (#1247): only a change that wrote the Units is undone yet.
    let ledger = ledger.filter(|ledger| ledger.domains().contains(&Domain::Units));
    if configuration.is_empty() && ledger.is_none() {
        return None;
    }
    let label = if labels.is_empty() {
        let mut names: Vec<String> = configuration
            .iter()
            .map(|(key, _, _)| key.name().to_string())
            .collect();
        if let Some(ledger) = &ledger {
            let domains: Vec<String> = ledger.domains().into_iter().map(domain_name).collect();
            names.push(crate::msg::desktop_history_ledger_changed(
                &domains.join(", "),
            ));
        }
        names.join(", ")
    } else {
        labels.join(", ")
    };
    Some(Entry {
        label,
        reversal: Reversal {
            configuration,
            ledger,
        },
    })
}

/// A Ledger domain as a History label names it.
fn domain_name(domain: Domain) -> String {
    match domain {
        Domain::Units => crate::msg::desktop_history_domain_units(),
        Domain::Institutions => crate::msg::desktop_history_domain_institutions(),
        Domain::Accounts => crate::msg::desktop_history_domain_accounts(),
        Domain::Categories => crate::msg::desktop_history_domain_categories(),
        Domain::Tags => crate::msg::desktop_history_domain_tags(),
        Domain::Payees => crate::msg::desktop_history_domain_payees(),
        Domain::Transactions => crate::msg::desktop_history_domain_transactions(),
        Domain::Bills => crate::msg::desktop_history_domain_bills(),
        Domain::Budgets => crate::msg::desktop_history_domain_budgets(),
        Domain::Inventory => crate::msg::desktop_history_domain_inventory(),
        Domain::Documents => crate::msg::desktop_history_domain_documents(),
        Domain::DocumentTypes => crate::msg::desktop_history_domain_document_types(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn some(value: &str) -> Option<String> {
        Some(value.to_string())
    }

    #[test]
    fn an_input_that_changed_nothing_closes_no_entry() {
        close(Vec::new(), None);
        record(ConfigKey::Toasts, some("true"), some("true"));

        assert_eq!(close(vec!["Toasts: On".to_string()], None), None);
    }

    #[test]
    fn a_change_closes_into_one_entry_labelled_by_the_presenter() {
        close(Vec::new(), None);
        record(ConfigKey::StartView, some("cash"), some("dashboard"));

        assert_eq!(
            close(vec!["Start view: Dashboard".to_string()], None),
            Some(Entry {
                label: "Start view: Dashboard".to_string(),
                reversal: Reversal {
                    configuration: vec![(ConfigKey::StartView, some("cash"), some("dashboard"))],
                    ledger: None,
                },
            })
        );
        assert_eq!(close(Vec::new(), None), None, "drained");
    }

    #[test]
    fn a_display_change_is_recorded() {
        close(Vec::new(), None);
        record(ConfigKey::ColourTheme, None, some("nord"));

        assert_eq!(
            close(Vec::new(), None).map(|entry| entry.reversal.configuration),
            Some(vec![(ConfigKey::ColourTheme, None, some("nord"))])
        );
    }

    #[test]
    fn a_change_with_no_label_is_named_by_its_key() {
        close(Vec::new(), None);
        record(ConfigKey::Locale, None, some("en-GB"));

        assert_eq!(
            close(Vec::new(), None).map(|entry| entry.label),
            Some("locale".to_string())
        );
    }

    #[test]
    fn only_generals_and_displays_keys_are_recorded_yet() {
        close(Vec::new(), None);
        record(ConfigKey::Log, None, some("debug"));

        assert_eq!(close(Vec::new(), None), None);
    }

    fn units_change() -> LedgerReversal {
        LedgerReversal::from(vec![lib_database::RowImage::BaseUnit(
            lib_core::UnitId::SEED_AUD,
            lib_core::UnitId::SEED_BTC,
        )])
    }

    #[test]
    fn a_ledger_change_closes_into_an_entry_named_by_its_domains_without_a_label() {
        close(Vec::new(), None);

        let entry = close(Vec::new(), Some(units_change())).expect("one change recorded");

        assert_eq!(entry.label, "Change units");
        assert_eq!(entry.reversal.ledger, Some(units_change()));
    }

    #[test]
    fn a_ledger_change_that_wrote_no_units_makes_no_entry_yet() {
        close(Vec::new(), None);
        let institutions = LedgerReversal::from(vec![lib_database::RowImage::Institution(
            None,
            Some(lib_institutions::personal_institution("Personal")),
        )]);

        assert_eq!(close(Vec::new(), Some(institutions)), None);
    }

    #[test]
    fn dropping_the_ledger_part_keeps_an_entry_only_with_configuration_left() {
        let mut ledger_only = Reversal {
            configuration: Vec::new(),
            ledger: Some(units_change()),
        };
        let mut mixed = Reversal {
            configuration: vec![(ConfigKey::Toasts, None, some("false"))],
            ledger: Some(units_change()),
        };

        assert!(!ledger_only.drop_ledger());
        assert!(mixed.drop_ledger());
        assert_eq!(mixed.ledger, None);
    }
}
