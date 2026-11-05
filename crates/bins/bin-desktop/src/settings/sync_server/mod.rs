//! The Settings › Sync server page (ADR-0037's amendment, one folder per page): `lib-settings`' Sync
//! server Presenter in a gpui Entity, drawn from its view model through `PageClicks` (`render.rs`).
//! "Sync now" is permanently out of scope for this map, so its event only says so on the status
//! line, as a Notice.

mod render;

use gpui::{Context, EventEmitter};
use lib_settings::{SyncServerEvent, SyncServerPresenter, SyncServerViewModel};

use crate::{
    settings::{
        SettingsPage,
        pages::{Page, PageRowClicked},
    },
    shell::view::page_clicks::PresenterPage,
    view::event::ViewEvent,
};

impl EventEmitter<ViewEvent> for SyncServerPage {}

/// The page has no rows, so it never emits a row click.
impl EventEmitter<PageRowClicked> for SyncServerPage {}

/// The Sync server page: its Presenter, which holds no state.
pub struct SyncServerPage {
    presenter: SyncServerPresenter,
}

impl Default for SyncServerPage {
    fn default() -> Self {
        Self::new()
    }
}

impl SyncServerPage {
    pub fn new() -> Self {
        Self {
            presenter: SyncServerPresenter::new(),
        }
    }

    pub fn view_model(&self) -> SyncServerViewModel {
        self.presenter.view_model()
    }

    /// A click as one input, committed once it is handled.
    fn send(&mut self, event: SyncServerEvent, cx: &mut Context<'_, Self>) {
        if crate::input::sending(cx) {
            self.handle(event);
        } else {
            crate::input::send(cx, |_| self.handle(event));
        }
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
}
