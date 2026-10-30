//! The Settings › Data & backup page (ADR-0037's amendment, one folder per page): a read-only column
//! of label/value rows and two stand-in buttons. [`DataBackupPage`] renders itself (`render.rs`).
//! Both buttons are permanently out of scope for this map, so a click only says so on the status
//! line, as [`ViewEvent::SetStatus`].

mod render;

use gpui::{Context, EventEmitter};

use crate::{
    settings::{SettingsPage, page::PageRowClicked, pages::Page},
    view::event::ViewEvent,
};

impl EventEmitter<ViewEvent> for DataBackupPage {}

/// The page has no rows, so it never emits a row click.
impl EventEmitter<PageRowClicked> for DataBackupPage {}

/// The Data & backup page. It holds no state: the values it shows are the mockup's own.
#[derive(Debug, Default)]
pub struct DataBackupPage;

impl DataBackupPage {
    /// "Backup now" has nothing behind it, and says so on the status line.
    fn click_backup_now(&mut self, cx: &mut Context<'_, Self>) {
        cx.emit(ViewEvent::SetStatus(Some(
            crate::msg::desktop_status_backup_now_not_implemented(),
        )));
    }

    /// "Export ledger (CSV)" has nothing behind it, and says so on the status line.
    fn click_export_ledger(&mut self, cx: &mut Context<'_, Self>) {
        cx.emit(ViewEvent::SetStatus(Some(
            crate::msg::desktop_status_export_ledger_not_implemented(),
        )))
    }
}

impl Page for DataBackupPage {
    const PAGE: SettingsPage = SettingsPage::DataBackup;
}
