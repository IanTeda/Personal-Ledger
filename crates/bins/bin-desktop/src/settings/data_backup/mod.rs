//! The Settings › Data & backup page (ADR-0037's amendment, one folder per page): `lib-settings`'
//! Data & backup Presenter in a gpui Entity, drawn from its view model through `PageClicks`
//! (`render.rs`). Both buttons are permanently out of scope for this map, so each only says so on
//! the status line, as a Notice.

mod render;

use gpui::{Context, EventEmitter};
use lib_settings::{DataBackupEvent, DataBackupPresenter, DataBackupViewModel};

use crate::{
    settings::{
        SettingsPage,
        pages::{Page, PageRowClicked},
    },
    shell::view::page_clicks::PresenterPage,
    view::event::ViewEvent,
};

impl EventEmitter<ViewEvent> for DataBackupPage {}

/// The page has no rows, so it never emits a row click.
impl EventEmitter<PageRowClicked> for DataBackupPage {}

/// The Data & backup page: its Presenter, which holds no state.
pub struct DataBackupPage {
    presenter: DataBackupPresenter,
}

impl Default for DataBackupPage {
    fn default() -> Self {
        Self::new()
    }
}

impl DataBackupPage {
    pub fn new() -> Self {
        Self {
            presenter: DataBackupPresenter::new(),
        }
    }

    pub fn view_model(&self) -> DataBackupViewModel {
        self.presenter.view_model()
    }

    /// A click as one input, committed once it is handled.
    fn send(&mut self, event: DataBackupEvent, cx: &mut Context<'_, Self>) {
        crate::input::join_or_send(cx, |_| self.handle(event));
        cx.notify();
    }

    fn handle(&mut self, event: DataBackupEvent) {
        self.presenter.handle(event, &mut crate::notices::Queue);
    }
}

/// The page's clicks on Backup now and Export ledger.
impl PresenterPage<DataBackupEvent> for DataBackupPage {
    fn send(&mut self, event: DataBackupEvent, cx: &mut Context<'_, Self>) {
        DataBackupPage::send(self, event, cx);
    }
}

impl Page for DataBackupPage {
    const PAGE: SettingsPage = SettingsPage::DataBackup;
}
