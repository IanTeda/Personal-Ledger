//! What the Desktop's History entries reverse (#1238, #1244, #1247): the changes an input made,
//! recorded below the Presenters as they are written and closed into one entry by the input's
//! `send` right after its commit (`crate::input`, #1277). `DeviceConfigFile` records `[General]`'s
//! and `[Display]`'s keys in its [`ConfigJournal`], and the Open Ledger's stores their row images in
//! `lib-database`'s journal, which `send` takes as the entry's Ledger part. Every Ledger domain
//! records (#1247, #1248, #1249).
//!
//! The device keeps its own journal, as the Ledger's stores do, and `Root` holds a clone of it from
//! when the Shell's root Router is built, since the device writes from inside a page's update with
//! no way to reach `Root`.

use std::cell::RefCell;
use std::rc::Rc;

use lib_core::configuration::{ConfigKey, Section};
use lib_database::{Domain, LedgerReversal};
use lib_shell::{Entry, Step};

/// One key's change to the user's configuration file: its raw value before and after, `None`
/// where the file didn't hold it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ConfigChange {
    pub(crate) key: ConfigKey,
    pub(crate) before: Option<String>,
    pub(crate) after: Option<String>,
}

/// The Configuration part of one History entry: the keys an input changed, in the order written,
/// as [`LedgerReversal`] is the Ledger's part. It holds the order a step puts them back in.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ConfigReversal(Vec<ConfigChange>);

impl ConfigReversal {
    pub(crate) fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// The keys changed, in the order written.
    pub(crate) fn keys(&self) -> impl Iterator<Item = ConfigKey> + '_ {
        self.0.iter().map(|change| change.key)
    }

    /// What `step` writes to the file, each key with its raw value on that side of the change:
    /// for Undo each `before`, the last written first, so a key changed twice ends as it first
    /// was; for Redo each `after`, in the order written.
    pub(crate) fn writes(&self, step: Step) -> Vec<(ConfigKey, Option<String>)> {
        match step {
            Step::Undo => self
                .0
                .iter()
                .rev()
                .map(|change| (change.key, change.before.clone()))
                .collect(),
            Step::Redo => self
                .0
                .iter()
                .map(|change| (change.key, change.after.clone()))
                .collect(),
        }
    }
}

impl From<Vec<ConfigChange>> for ConfigReversal {
    fn from(changes: Vec<ConfigChange>) -> Self {
        Self(changes)
    }
}

/// What reverses one History entry: the Configuration it changed and the Open Ledger's rows it
/// changed (#1240).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Reversal {
    pub configuration: ConfigReversal,
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

/// The Configuration changes the running input made, shared by the device that writes them and
/// the `Root` whose `send` takes them; a clone records into the same list.
#[derive(Debug, Clone, Default)]
pub(crate) struct ConfigJournal(Rc<RefCell<Vec<ConfigChange>>>);

impl ConfigJournal {
    /// Notes that the running input changed `key` in the file from `before` to `after`. A write
    /// that left the file's value as it was is no change, and only `[General]` and `[Display]` are
    /// recorded.
    pub(crate) fn record(&self, key: ConfigKey, before: Option<String>, after: Option<String>) {
        if before == after || !matches!(key.section(), Section::General | Section::Display) {
            return;
        }
        self.0
            .borrow_mut()
            .push(ConfigChange { key, before, after });
    }

    /// Every change recorded since the last take, in the order written.
    pub(crate) fn take(&self) -> ConfigReversal {
        ConfigReversal(std::mem::take(&mut *self.0.borrow_mut()))
    }
}

/// What the input recorded, as one History entry labelled by `labels`, the `Notice::Changed`s it
/// raised, with `configuration`, the keys it changed, and `ledger`, the Open Ledger's row images
/// it wrote: `None` when it changed nothing. With no label, the keys' names and the Ledger's
/// domains stand in for one.
pub(crate) fn close(
    labels: Vec<String>,
    configuration: ConfigReversal,
    ledger: Option<LedgerReversal>,
) -> Option<Entry<Reversal>> {
    if configuration.is_empty() && ledger.is_none() {
        return None;
    }
    let label = if labels.is_empty() {
        let mut names: Vec<String> = configuration
            .keys()
            .map(|key| key.name().to_string())
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

    fn change(key: ConfigKey, before: Option<String>, after: Option<String>) -> ConfigReversal {
        ConfigReversal::from(vec![ConfigChange { key, before, after }])
    }

    /// Start view changed, then Toasts, then Start view again, as one input.
    fn two_keys() -> ConfigReversal {
        ConfigReversal::from(vec![
            ConfigChange {
                key: ConfigKey::StartView,
                before: some("cash"),
                after: some("dashboard"),
            },
            ConfigChange {
                key: ConfigKey::Toasts,
                before: None,
                after: some("false"),
            },
            ConfigChange {
                key: ConfigKey::StartView,
                before: some("dashboard"),
                after: some("last"),
            },
        ])
    }

    #[test]
    fn undo_writes_each_before_the_last_written_first() {
        assert_eq!(
            two_keys().writes(Step::Undo),
            [
                (ConfigKey::StartView, some("dashboard")),
                (ConfigKey::Toasts, None),
                (ConfigKey::StartView, some("cash")),
            ]
        );
    }

    #[test]
    fn redo_writes_each_after_in_the_order_written() {
        assert_eq!(
            two_keys().writes(Step::Redo),
            [
                (ConfigKey::StartView, some("dashboard")),
                (ConfigKey::Toasts, some("false")),
                (ConfigKey::StartView, some("last")),
            ]
        );
    }

    #[test]
    fn an_input_that_changed_nothing_closes_no_entry() {
        let journal = ConfigJournal::default();
        journal.record(ConfigKey::Toasts, some("true"), some("true"));

        assert_eq!(
            close(vec!["Toasts: On".to_string()], journal.take(), None),
            None
        );
    }

    #[test]
    fn a_change_closes_into_one_entry_labelled_by_the_presenter() {
        let journal = ConfigJournal::default();
        journal
            .clone()
            .record(ConfigKey::StartView, some("cash"), some("dashboard"));

        assert_eq!(
            close(
                vec!["Start view: Dashboard".to_string()],
                journal.take(),
                None
            ),
            Some(Entry {
                label: "Start view: Dashboard".to_string(),
                reversal: Reversal {
                    configuration: change(ConfigKey::StartView, some("cash"), some("dashboard")),
                    ledger: None,
                },
            })
        );
        assert_eq!(journal.take(), ConfigReversal::default(), "taken");
    }

    #[test]
    fn a_display_change_is_recorded() {
        let journal = ConfigJournal::default();
        journal.record(ConfigKey::ColourTheme, None, some("nord"));

        assert_eq!(
            close(Vec::new(), journal.take(), None).map(|entry| entry.reversal.configuration),
            Some(change(ConfigKey::ColourTheme, None, some("nord")))
        );
    }

    #[test]
    fn a_change_with_no_label_is_named_by_its_key() {
        let journal = ConfigJournal::default();
        journal.record(ConfigKey::Locale, None, some("en-GB"));

        assert_eq!(
            close(Vec::new(), journal.take(), None).map(|entry| entry.label),
            Some("locale".to_string())
        );
    }

    #[test]
    fn only_generals_and_displays_keys_are_recorded_yet() {
        let journal = ConfigJournal::default();
        journal.record(ConfigKey::Log, None, some("debug"));

        assert_eq!(journal.take(), ConfigReversal::default());
    }

    fn units_change() -> LedgerReversal {
        LedgerReversal::from(vec![lib_database::RowImage::BaseUnit(
            lib_core::UnitId::SEED_AUD,
            lib_core::UnitId::SEED_BTC,
        )])
    }

    #[test]
    fn a_ledger_change_closes_into_an_entry_named_by_its_domains_without_a_label() {
        let entry = close(Vec::new(), ConfigReversal::default(), Some(units_change()))
            .expect("one change recorded");

        assert_eq!(entry.label, "Change units");
        assert_eq!(entry.reversal.ledger, Some(units_change()));
    }

    #[test]
    fn a_ledger_change_to_any_recording_domain_makes_an_entry_named_by_it() {
        let institutions = LedgerReversal::from(vec![lib_database::RowImage::Institution(
            lib_database::Image {
                before: None,
                after: Some(lib_institutions::personal_institution("Personal")),
            },
        )]);

        let entry = close(Vec::new(), ConfigReversal::default(), Some(institutions))
            .expect("one change recorded");

        assert_eq!(entry.label, "Change institutions");
    }

    #[test]
    fn dropping_the_ledger_part_keeps_an_entry_only_with_configuration_left() {
        let mut ledger_only = Reversal {
            configuration: ConfigReversal::default(),
            ledger: Some(units_change()),
        };
        let mut mixed = Reversal {
            configuration: change(ConfigKey::Toasts, None, some("false")),
            ledger: Some(units_change()),
        };

        assert!(!ledger_only.drop_ledger());
        assert!(mixed.drop_ledger());
        assert_eq!(mixed.ledger, None);
    }
}
