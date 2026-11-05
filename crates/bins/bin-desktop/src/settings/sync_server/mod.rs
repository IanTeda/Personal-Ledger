//! The Settings › Sync server page (ADR-0037's amendment, one folder per page): `lib-settings`' Sync
//! server Presenter in a gpui Entity, drawn from its view model through `PageClicks` (`render.rs`).
//! "Sync now" is permanently out of scope for this map, so its event only says so on the status
//! line, as a Notice.

mod render;

use gpui::{App, Context, EventEmitter};
use lib_settings::{SettingsFrameEvent, SyncServerEvent, SyncServerPresenter, SyncServerViewModel};
use lib_shell::Hint;
use lib_viper::keymap::Keymap;

use crate::{
    settings::{
        SettingsPage,
        hints::settings_plain_page_hints,
        pages::{Page, PageRowClicked},
    },
    shell::view::page_clicks::PresenterPage,
    view::event::ViewEvent,
};

impl EventEmitter<ViewEvent> for SyncServerPage {}

/// The page has no rows, so it never emits a row click.
impl EventEmitter<PageRowClicked> for SyncServerPage {}

/// The Sync server page: its Presenter, which holds no state, and the frame's keys for its legend.
pub struct SyncServerPage {
    presenter: SyncServerPresenter,
    keymap: Keymap<SettingsFrameEvent>,
}

impl SyncServerPage {
    /// The page, its legend read from the frame's `keymap`.
    pub fn new(keymap: Keymap<SettingsFrameEvent>) -> Self {
        Self {
            presenter: SyncServerPresenter::new(),
            keymap,
        }
    }

    /// The frame's keys under `[keybindings]`, for a rebind.
    pub fn set_keymap(&mut self, keymap: Keymap<SettingsFrameEvent>) {
        self.keymap = keymap;
    }

    pub fn view_model(&self) -> SyncServerViewModel {
        self.presenter.view_model()
    }

    /// A click as one input, committed once it is handled.
    fn send(&mut self, event: SyncServerEvent, cx: &mut Context<'_, Self>) {
        crate::input::join_or_send(cx, |_| self.handle(event));
        cx.notify();
    }

    fn handle(&mut self, event: SyncServerEvent) {
        self.presenter.handle(event, &mut crate::notices::Queue);
    }
}

/// The page's one click, Sync now.
impl PresenterPage<SyncServerEvent> for SyncServerPage {
    fn send(&mut self, event: SyncServerEvent, cx: &mut Context<'_, Self>) {
        SyncServerPage::send(self, event, cx);
    }
}

impl Page for SyncServerPage {
    const PAGE: SettingsPage = SettingsPage::SyncServer;

    fn hints(&self, _cx: &App) -> Vec<Hint> {
        settings_plain_page_hints(&self.keymap)
    }
}
