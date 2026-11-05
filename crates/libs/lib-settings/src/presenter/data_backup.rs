//! Settings › Data & backup's Presenter: the mockup's placeholder values and two stand-in buttons.
//! Both are permanently out of scope for this map, so each event only says so on the status line,
//! through [`Notify`].

use lib_toast::{Notice, Notify};

use crate::msg;

/// What the person asks of Settings › Data & backup.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataBackupEvent {
    BackupNow,
    ExportLedger,
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
}

/// Settings › Data & backup's Presenter. It holds no state.
#[derive(Debug, Default)]
pub struct DataBackupPresenter;

impl DataBackupPresenter {
    pub fn new() -> Self {
        Self
    }

    pub fn handle(&mut self, event: DataBackupEvent, router: &mut dyn Notify) {
        let text = match event {
            DataBackupEvent::BackupNow => msg::settings_status_backup_now_not_implemented(),
            DataBackupEvent::ExportLedger => msg::settings_status_export_ledger_not_implemented(),
        };
        router.notify(Notice::Status(text));
    }

    pub fn view_model(&self) -> DataBackupViewModel {
        DataBackupViewModel {
            rows: vec![
                DataBackupRow {
                    label: msg::settings_backup_location(),
                    value: "~/.ledger/personal".to_string(),
                    value_size: 11,
                },
                DataBackupRow {
                    label: msg::settings_backup_last(),
                    value: "12 sep 2026 \u{b7} 23:10".to_string(),
                    value_size: 12,
                },
            ],
            backup_now: msg::settings_backup_now(),
            export_ledger: msg::settings_backup_export(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_button_says_it_is_not_implemented() {
        let mut notices: Vec<Notice> = Vec::new();
        let mut presenter = DataBackupPresenter::new();

        presenter.handle(DataBackupEvent::BackupNow, &mut notices);
        presenter.handle(DataBackupEvent::ExportLedger, &mut notices);

        assert_eq!(
            notices,
            [
                Notice::Status("backup now — not implemented".to_string()),
                Notice::Status("export ledger — not implemented".to_string()),
            ]
        );
    }

    #[test]
    fn the_view_model_holds_the_placeholder_rows() {
        let model = DataBackupPresenter::new().view_model();

        assert_eq!(model.rows[0].value, "~/.ledger/personal");
        assert_eq!(model.rows.len(), 2);
        assert_eq!(model.export_ledger, "Export ledger (CSV)");
    }
}
