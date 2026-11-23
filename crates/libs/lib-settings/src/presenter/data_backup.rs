//! Settings › Data & backup's Presenter: the mockup's placeholder values and two stand-in buttons,
//! each of which only says on the status line, through [`Notify`], that it is out of scope for this
//! map, and the Price Source API keys' facts (#1392): the encryption key's fingerprint, *Encryption
//! key · 3F9A-21C0* or *Not set*, so a person can check two devices share one secret, and Remove all
//! API keys behind a confirm Dialog ([`RemoveApiKeysPresenter`]), the way out when the secret is
//! lost. The secret itself is never shown, and nothing offers to copy it (ADR-0043).

use std::rc::Rc;

use lib_toast::Notice;
use lib_viper::dialog::{DialogKeys, dialog_keymap};
use lib_viper::keymap::{KeyBindingConfig, Keymap};
use lib_viper::presenter::{PagePresenter, Presenter};
use lib_viper::view::Hint;

use super::remove_api_keys::{RemoveApiKeysOutcome, RemoveApiKeysPresenter};
use crate::keymap::{REMOVE_API_KEYS, data_backup_keymap, settings_plain_page_legend};
use crate::msg;
use crate::protocols::{ApiKeys, DataBackupRouter};

/// The mockup's own placeholder values, which the Data & backup index summary quotes too.
pub(crate) const PLACEHOLDER_LOCATION: &str = "~/.ledger/personal";
pub(crate) const PLACEHOLDER_LAST: &str = "12 sep 2026 \u{b7} 23:10";

/// What the person asks of Settings › Data & backup.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataBackupEvent {
    BackupNow,
    ExportLedger,
    /// `x` or the Remove all API keys button: asks first, since every key is deleted for good.
    RemoveKeys,
}

/// What the Client hands Data & backup as it builds it.
pub struct DataBackupSetup {
    pub keys: Rc<dyn ApiKeys>,
}

/// One label/value row, with the size its value is drawn at in pixels.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataBackupRow {
    pub label: String,
    pub value: String,
    pub value_size: u8,
}

/// Settings › Data & backup as the Client draws it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataBackupViewModel {
    pub rows: Vec<DataBackupRow>,
    pub backup_now: String,
    pub export_ledger: String,
    pub remove_keys: String,
    /// The page's keys, for the status line and the `?` cheat sheet: the frame's scroll and
    /// focus-index chords, which the Presenter reads from the frame's keymap under the config,
    /// and Remove all API keys.
    pub hints: Vec<Hint>,
}

/// Settings › Data & backup's Presenter. It holds no state: the fingerprint is read from the keys
/// port as drawn.
pub struct DataBackupPresenter {
    keys: Rc<dyn ApiKeys>,
    keymap: Keymap<DataBackupEvent>,
    legend: Vec<Hint>,
    dialog_keys: DialogKeys,
}

impl DataBackupPresenter {
    pub fn new(setup: DataBackupSetup, config: &KeyBindingConfig) -> Self {
        let (keymap, legend, dialog_keys) = Self::keys(config);
        Self {
            keys: setup.keys,
            keymap,
            legend,
            dialog_keys,
        }
    }

    /// The page's keymap, its legend and the Remove all API keys Dialog's keys under `config`, as
    /// [`Self::new`] and [`PagePresenter::rebind`] build them.
    fn keys(config: &KeyBindingConfig) -> (Keymap<DataBackupEvent>, Vec<Hint>, DialogKeys) {
        let keymap = data_backup_keymap(config);
        let mut legend = settings_plain_page_legend(config);
        legend.insert(
            1,
            Hint::new(
                keymap.hint_chord(REMOVE_API_KEYS),
                msg::settings_backup_hint_remove_keys(),
            ),
        );
        debug_assert_eq!(legend.len(), 3, "scroll, remove keys, index");
        (keymap, legend, DialogKeys::of(&dialog_keymap(config)))
    }
}

impl Presenter for DataBackupPresenter {
    type Event = DataBackupEvent;
    type ViewModel = DataBackupViewModel;
    type Router<'r> = dyn DataBackupRouter + 'r;

    fn handle(&mut self, event: DataBackupEvent, router: &mut Self::Router<'_>) {
        let text = match event {
            DataBackupEvent::BackupNow => msg::settings_status_backup_now_not_implemented(),
            DataBackupEvent::ExportLedger => msg::settings_status_export_ledger_not_implemented(),
            DataBackupEvent::RemoveKeys => {
                router.open_remove_api_keys(RemoveApiKeysPresenter::new(
                    self.keys.clone(),
                    self.dialog_keys.clone(),
                ));
                return;
            }
        };
        router.notify(Notice::Status(text));
    }

    fn view_model(&self) -> DataBackupViewModel {
        DataBackupViewModel {
            rows: vec![
                DataBackupRow {
                    label: msg::settings_backup_location(),
                    value: PLACEHOLDER_LOCATION.to_string(),
                    value_size: 11,
                },
                DataBackupRow {
                    label: msg::settings_backup_last(),
                    value: PLACEHOLDER_LAST.to_string(),
                    value_size: 12,
                },
                DataBackupRow {
                    label: msg::settings_backup_encryption_key(),
                    value: self
                        .keys
                        .fingerprint()
                        .unwrap_or_else(msg::settings_backup_encryption_key_not_set),
                    value_size: 12,
                },
            ],
            backup_now: msg::settings_backup_now(),
            export_ledger: msg::settings_backup_export(),
            remove_keys: msg::settings_backup_remove_keys(),
            hints: self.legend.clone(),
        }
    }
}

impl PagePresenter for DataBackupPresenter {
    /// What the Remove all API keys Dialog lands with.
    type Outcome = RemoveApiKeysOutcome;

    /// The page reads the keys as drawn, so a confirmed removal has nothing to tell it.
    fn dialog_closed(_outcome: Option<RemoveApiKeysOutcome>) -> Option<DataBackupEvent> {
        None
    }

    fn keymap(&self) -> &Keymap<DataBackupEvent> {
        &self.keymap
    }

    /// The keymap, the legend, which reads the frame's chords, and the Dialog's keys.
    fn rebind(&mut self, keys: &KeyBindingConfig) {
        (self.keymap, self.legend, self.dialog_keys) = Self::keys(keys);
    }
}

#[cfg(test)]
mod tests {
    use lib_toast::Notify;

    use super::*;
    use crate::fake::InMemoryApiKeys;

    #[derive(Default)]
    struct Recording {
        notices: Vec<Notice>,
        dialogs: usize,
    }

    impl Notify for Recording {
        fn notify(&mut self, notice: Notice) {
            self.notices.push(notice);
        }
    }

    impl DataBackupRouter for Recording {
        fn open_remove_api_keys(&mut self, _dialog: RemoveApiKeysPresenter) {
            self.dialogs += 1;
        }
    }

    fn presenter(fingerprint: Option<&str>) -> DataBackupPresenter {
        DataBackupPresenter::new(
            DataBackupSetup {
                keys: Rc::new(InMemoryApiKeys::new(fingerprint, 1)),
            },
            &KeyBindingConfig::default(),
        )
    }

    #[test]
    fn each_stand_in_button_says_it_is_not_implemented() {
        let mut router = Recording::default();
        let mut presenter = presenter(None);

        presenter.handle(DataBackupEvent::BackupNow, &mut router);
        presenter.handle(DataBackupEvent::ExportLedger, &mut router);

        assert_eq!(
            router.notices,
            [
                Notice::Status("backup now — not implemented".to_string()),
                Notice::Status("export ledger — not implemented".to_string()),
            ]
        );
    }

    #[test]
    fn the_view_model_holds_the_placeholder_rows() {
        let model = presenter(None).view_model();

        assert_eq!(model.rows[0].value, "~/.ledger/personal");
        assert_eq!(model.export_ledger, "Export ledger (CSV)");
    }

    #[test]
    fn the_encryption_key_fact_reads_not_set_without_a_secret_and_the_fingerprint_with_one() {
        let fact = |presenter: &DataBackupPresenter| {
            let row = presenter.view_model().rows.remove(2);
            (row.label, row.value)
        };

        assert_eq!(
            fact(&presenter(None)),
            ("Encryption key".to_string(), "Not set".to_string())
        );
        assert_eq!(
            fact(&presenter(Some("3F9A-21C0"))),
            ("Encryption key".to_string(), "3F9A-21C0".to_string())
        );
    }

    #[test]
    fn remove_all_api_keys_asks_through_its_dialog_and_deletes_nothing_itself() {
        let keys = Rc::new(InMemoryApiKeys::new(Some("3F9A-21C0"), 2));
        let mut presenter = DataBackupPresenter::new(
            DataBackupSetup { keys: keys.clone() },
            &KeyBindingConfig::default(),
        );
        let mut router = Recording::default();

        presenter.handle(DataBackupEvent::RemoveKeys, &mut router);

        assert_eq!(router.dialogs, 1);
        assert_eq!(keys.saved(), 2);
        assert!(router.notices.is_empty());
    }
}
