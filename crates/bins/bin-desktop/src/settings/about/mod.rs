//! The Settings › About page (ADR-0037's amendment, one folder per page): static project info.
//! `lib-settings`' About Presenter in a gpui Entity, given the version from this crate's `env!`.
//! [`AboutPage`] renders itself (`render.rs`) and has no events.

mod render;

use gpui::{App, EventEmitter};
use lib_settings::{AboutPresenter, AboutSetup, AboutViewModel, SettingsFrameEvent};
use lib_shell::Hint;
use lib_viper::keymap::Keymap;
use lib_viper::presenter::Presenter;

use crate::{
    settings::{
        SettingsPage,
        hints::settings_plain_page_hints,
        pages::{Page, PageRowClicked},
    },
    view::event::ViewEvent,
};

impl EventEmitter<ViewEvent> for AboutPage {}

/// The page has no rows, so it never emits a row click.
impl EventEmitter<PageRowClicked> for AboutPage {}

/// The About page.
pub struct AboutPage {
    presenter: AboutPresenter,
    /// The frame's keys, which the legend's chords are read from.
    keymap: Keymap<SettingsFrameEvent>,
}

impl AboutPage {
    /// The page for a build of `version`, its legend read from the frame's `keymap`.
    pub fn new(version: &str, keymap: Keymap<SettingsFrameEvent>) -> Self {
        Self {
            presenter: AboutPresenter::new(AboutSetup {
                version: version.to_string(),
            }),
            keymap,
        }
    }

    /// The frame's keys under `[keybindings]`, for a rebind.
    pub fn set_keymap(&mut self, keymap: Keymap<SettingsFrameEvent>) {
        self.keymap = keymap;
    }

    pub fn view_model(&self) -> AboutViewModel {
        self.presenter.view_model()
    }
}

impl Page for AboutPage {
    const PAGE: SettingsPage = SettingsPage::About;

    fn hints(&self, _cx: &App) -> Vec<Hint> {
        settings_plain_page_hints(&self.keymap)
    }
}
