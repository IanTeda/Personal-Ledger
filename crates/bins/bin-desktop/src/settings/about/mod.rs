//! The Settings › About page (ADR-0037's amendment, one folder per page): static project info.
//! `lib-settings`' About Presenter in a gpui Entity, given the version from this crate's `env!`.
//! [`AboutPage`] renders itself (`render.rs`) and has no events.

mod render;

use gpui::EventEmitter;
use lib_settings::{AboutPresenter, AboutSetup, AboutViewModel};

use crate::{
    settings::{
        SettingsPage,
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
}

impl AboutPage {
    /// The page for a build of `version`.
    pub fn new(version: &str) -> Self {
        Self {
            presenter: AboutPresenter::new(AboutSetup {
                version: version.to_string(),
            }),
        }
    }

    pub fn view_model(&self) -> AboutViewModel {
        self.presenter.view_model()
    }
}

impl Page for AboutPage {
    const PAGE: SettingsPage = SettingsPage::About;
}
