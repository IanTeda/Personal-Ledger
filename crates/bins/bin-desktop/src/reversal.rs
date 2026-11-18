//! What the Desktop's History entries reverse (#1238, #1244): the changes an input made, recorded
//! below the Presenters as they are written and closed into one entry when the input's `send`
//! drains (`Root::drain_notices`). Only `DeviceConfigFile` records yet, and `[General]`'s and
//! `[Display]`'s keys: the Ledger joins in its own slice.
//!
//! The recording is per thread rather than held by `Root`, as the Notice queue is
//! (`crate::notices`), since the device writes from inside a page's update with no way to reach
//! `Root`.

use std::cell::RefCell;
use std::convert::Infallible;

use lib_core::configuration::{ConfigKey, Section};
use lib_shell::Entry;

/// One key's change to the user's configuration file: its raw value before and after, `None`
/// where the file didn't hold it.
pub(crate) type ConfigChange = (ConfigKey, Option<String>, Option<String>);

/// What reverses one History entry: the Configuration it changed, in the order written, and the
/// Ledger's part, which stays empty until Ledger changes are recorded (#1240).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Reversal {
    pub configuration: Vec<ConfigChange>,
    pub ledger: Option<Infallible>,
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
/// raised; `None` when it changed nothing. With no label, the keys' names stand in for one.
pub(crate) fn close(labels: Vec<String>) -> Option<Entry<Reversal>> {
    let configuration = RECORDED.with_borrow_mut(std::mem::take);
    if configuration.is_empty() {
        return None;
    }
    let label = if labels.is_empty() {
        configuration
            .iter()
            .map(|(key, _, _)| key.name())
            .collect::<Vec<_>>()
            .join(", ")
    } else {
        labels.join(", ")
    };
    Some(Entry {
        label,
        reversal: Reversal {
            configuration,
            ledger: None,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn some(value: &str) -> Option<String> {
        Some(value.to_string())
    }

    #[test]
    fn an_input_that_changed_nothing_closes_no_entry() {
        close(Vec::new());
        record(ConfigKey::Toasts, some("true"), some("true"));

        assert_eq!(close(vec!["Toasts: On".to_string()]), None);
    }

    #[test]
    fn a_change_closes_into_one_entry_labelled_by_the_presenter() {
        close(Vec::new());
        record(ConfigKey::StartView, some("cash"), some("dashboard"));

        assert_eq!(
            close(vec!["Start view: Dashboard".to_string()]),
            Some(Entry {
                label: "Start view: Dashboard".to_string(),
                reversal: Reversal {
                    configuration: vec![(ConfigKey::StartView, some("cash"), some("dashboard"))],
                    ledger: None,
                },
            })
        );
        assert_eq!(close(Vec::new()), None, "drained");
    }

    #[test]
    fn a_display_change_is_recorded() {
        close(Vec::new());
        record(ConfigKey::ColourTheme, None, some("nord"));

        assert_eq!(
            close(Vec::new()).map(|entry| entry.reversal.configuration),
            Some(vec![(ConfigKey::ColourTheme, None, some("nord"))])
        );
    }

    #[test]
    fn a_change_with_no_label_is_named_by_its_key() {
        close(Vec::new());
        record(ConfigKey::Locale, None, some("en-GB"));

        assert_eq!(
            close(Vec::new()).map(|entry| entry.label),
            Some("locale".to_string())
        );
    }

    #[test]
    fn only_generals_and_displays_keys_are_recorded_yet() {
        close(Vec::new());
        record(ConfigKey::Log, None, some("debug"));

        assert_eq!(close(Vec::new()), None);
    }
}
