//! Settings › Sync server's Presenter: a read-only column of the mockup's placeholder values and a
//! "Sync now" stand-in. Sync now is permanently out of scope for this map, so its event only says
//! so on the status line, through [`Notify`].

use lib_toast::{Notice, Notify};

use crate::msg;

/// What the person asks of Settings › Sync server.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncServerEvent {
    /// The "Sync now" button.
    SyncNow,
}

/// Settings › Sync server as the Client draws it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncServerViewModel {
    pub url_label: String,
    pub url: String,
    pub status_label: String,
    pub connected: String,
    pub last_label: String,
    pub last: String,
    pub sync_now: String,
}

/// Settings › Sync server's Presenter. It holds no state.
#[derive(Debug, Default)]
pub struct SyncServerPresenter;

impl SyncServerPresenter {
    pub fn new() -> Self {
        Self
    }

    pub fn handle(&mut self, event: SyncServerEvent, router: &mut dyn Notify) {
        match event {
            SyncServerEvent::SyncNow => router.notify(Notice::Status(
                msg::settings_status_sync_now_not_implemented(),
            )),
        }
    }

    pub fn view_model(&self) -> SyncServerViewModel {
        SyncServerViewModel {
            url_label: msg::settings_sync_url(),
            url: "sync.ledger.localhost".to_string(),
            status_label: msg::settings_sync_status(),
            connected: msg::settings_sync_connected(),
            last_label: msg::settings_sync_last(),
            last: "14 sep 2026 \u{b7} 09:14".to_string(),
            sync_now: msg::settings_sync_now(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sync_now_says_it_is_not_implemented() {
        let mut notices: Vec<Notice> = Vec::new();

        SyncServerPresenter::new().handle(SyncServerEvent::SyncNow, &mut notices);

        assert_eq!(
            notices,
            [Notice::Status("sync now — not implemented".to_string())]
        );
    }

    #[test]
    fn the_view_model_holds_the_placeholder_values() {
        let model = SyncServerPresenter::new().view_model();

        assert_eq!(model.url, "sync.ledger.localhost");
        assert_eq!(model.sync_now, "Sync now");
        assert_eq!(model.connected, "connected");
    }
}
