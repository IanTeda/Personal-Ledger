//! The Settings › About page (ADR-0037's amendment, one folder per page): static project info.
//! [`AboutPage`] renders itself (`render.rs`) and has no state or events.

mod render;

use gpui::EventEmitter;

use crate::{
    settings::{SettingsPage, pages::Page},
    view::event::ViewEvent,
};

impl EventEmitter<ViewEvent> for AboutPage {}

/// The About page.
#[derive(Debug, Default)]
pub struct AboutPage;

impl Page for AboutPage {
    const PAGE: SettingsPage = SettingsPage::About;
}
