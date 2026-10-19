//! The Settings › Sync server page (ADR-0037's amendment, one folder per page): a read-only column
//! of label/value rows and a "Sync now" stand-in. [`SyncServerPage`] renders itself (`render.rs`).
//! "Sync now" is permanently out of scope for this map, so its click only says so on the status
//! line, as [`ViewEvent::SetStatus`].

mod render;

use gpui::{Context, EventEmitter};

use crate::{
    settings::{SettingsPage, pages::Page},
    view::event::ViewEvent,
};

impl EventEmitter<ViewEvent> for SyncServerPage {}

/// The Sync server page. It holds no state: the values it shows are the mockup's own.
#[derive(Debug, Default)]
pub struct SyncServerPage;

impl SyncServerPage {
    /// "Sync now" has nothing behind it, and says so on the status line.
    fn click_sync_now(&mut self, cx: &mut Context<'_, Self>) {
        cx.emit(ViewEvent::SetStatus(Some(
            crate::msg::desktop_status_sync_now_not_implemented(),
        )));
    }
}

impl Page for SyncServerPage {
    const PAGE: SettingsPage = SettingsPage::SyncServer;
}
