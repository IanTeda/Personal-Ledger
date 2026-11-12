//! Settings › Sync server's Presenter: a read-only column of the mockup's placeholder values and a
//! "Sync now" stand-in. Sync now is permanently out of scope for this map, so its event only says
//! so on the status line, through [`Notify`].

use std::convert::Infallible;

use lib_toast::{Notice, Notify};
use lib_viper::keymap::{KeyBindingConfig, Keymap};
use lib_viper::presenter::{PagePresenter, Presenter};
use lib_viper::view::Hint;

use crate::msg;

/// The mockup's own placeholder values, which the Sync server index summary quotes too.
pub(crate) const PLACEHOLDER_URL: &str = "sync.ledger.localhost";
pub(crate) const PLACEHOLDER_LAST: &str = "14 sep 2026 \u{b7} 09:14";

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
    /// The page's keys, for the status line and the `?` cheat sheet: the frame's scroll and
    /// focus-index chords, which only the Client can read from its keymap, so it hands them over
    /// ([`SyncServerPresenter::set_legend`]).
    pub hints: Vec<Hint>,
}

/// Settings › Sync server's Presenter. It holds no state, and its only events are its buttons' clicks,
/// so its keymap binds nothing.
#[derive(Debug)]
pub struct SyncServerPresenter {
    keymap: Keymap<SyncServerEvent>,
    legend: Vec<Hint>,
}

impl SyncServerPresenter {
    pub fn new() -> Self {
        Self {
            keymap: Keymap::new(&[], &KeyBindingConfig::default()),
            legend: Vec::new(),
        }
    }

    /// Replaces the legend, for start-up and a `[keybindings]` change.
    pub fn set_legend(&mut self, legend: Vec<Hint>) {
        self.legend = legend;
    }
}

impl Default for SyncServerPresenter {
    fn default() -> Self {
        Self::new()
    }
}

impl Presenter for SyncServerPresenter {
    type Event = SyncServerEvent;
    type ViewModel = SyncServerViewModel;
    type Router<'r> = dyn Notify + 'r;

    fn handle(&mut self, event: SyncServerEvent, router: &mut Self::Router<'_>) {
        match event {
            SyncServerEvent::SyncNow => router.notify(Notice::Status(
                msg::settings_status_sync_now_not_implemented(),
            )),
        }
    }

    fn view_model(&self) -> SyncServerViewModel {
        SyncServerViewModel {
            url_label: msg::settings_sync_url(),
            url: PLACEHOLDER_URL.to_string(),
            status_label: msg::settings_sync_status(),
            connected: msg::settings_sync_connected(),
            last_label: msg::settings_sync_last(),
            last: PLACEHOLDER_LAST.to_string(),
            sync_now: msg::settings_sync_now(),
            hints: self.legend.clone(),
        }
    }
}

impl PagePresenter for SyncServerPresenter {
    /// Sync server opens no Dialog.
    type Outcome = Infallible;

    /// Sync server opens no Dialog, so there is nothing to tell it.
    fn dialog_closed(_outcome: Option<Infallible>) -> Option<SyncServerEvent> {
        None
    }

    fn keymap(&self) -> &Keymap<SyncServerEvent> {
        &self.keymap
    }

    /// The legend, which reads the frame's chords.
    fn rebind(&mut self, keys: &KeyBindingConfig) {
        self.legend =
            crate::keymap::settings_plain_page_hints(&crate::keymap::settings_frame_keymap(keys));
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
