//! The Settings › Data & backup page (ADR-0037's amendment, one folder per page): `lib-settings`'
//! Data & backup Presenter in a gpui Entity, drawn from its view model through `PageClicks`
//! (`render.rs`). Both buttons are permanently out of scope for this map, so each only says so on
//! the status line, as a Notice.

mod render;

use gpui::{App, Context, EventEmitter};
use lib_settings::{DataBackupEvent, DataBackupPresenter, DataBackupViewModel, SettingsFrameEvent};
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

impl EventEmitter<ViewEvent> for DataBackupPage {}

/// The page has no rows, so it never emits a row click.
impl EventEmitter<PageRowClicked> for DataBackupPage {}

/// The Data & backup page: its Presenter, which holds no state, and the frame's keys for its legend.
pub struct DataBackupPage {
    presenter: DataBackupPresenter,
    keymap: Keymap<SettingsFrameEvent>,
}

impl DataBackupPage {
    /// The page, its legend read from the frame's `keymap`.
    pub fn new(keymap: Keymap<SettingsFrameEvent>) -> Self {
        Self {
            presenter: DataBackupPresenter::new(),
            keymap,
        }
    }

    /// The frame's keys under `[keybindings]`, for a rebind.
    pub fn set_keymap(&mut self, keymap: Keymap<SettingsFrameEvent>) {
        self.keymap = keymap;
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

    fn hints(&self, _cx: &App) -> Vec<Hint> {
        settings_plain_page_hints(&self.keymap)
    }
}
