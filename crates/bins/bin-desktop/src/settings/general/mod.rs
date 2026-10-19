//! The **General** section (`docs/ux/desktop-mockups/16-settings/README.md`'s "2a resting state", first in
//! scroll order): ledger identity fields plus a "THIS LEDGER" summary panel. Dummy values
//! throughout, matching the mockup's own -- no real `lib_database::Preferences` wiring in this
//! map (see the Desktop Settings Surface map's Destination).

mod render;

use gpui::EventEmitter;

use crate::{
    settings::{SettingsPage, pages::Page},
    view::event::ViewEvent,
};

impl EventEmitter<ViewEvent> for GeneralPage {}

/// The General page. It holds no state: the values it shows are the mockup's own.
#[derive(Debug, Default)]
pub struct GeneralPage;

impl Page for GeneralPage {
    const PAGE: SettingsPage = SettingsPage::General;
}
