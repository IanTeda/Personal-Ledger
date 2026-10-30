//! What the integration tests read back of the Settings pages this View draws (ADR-0030), built by
//! the Settings Entity from its own state (ADR-0037). The open dialog and the status message are
//! `Root`'s, so its accessors fill them in.

use gpui::App;

use super::state::SettingsView;
use crate::settings::{SettingsFocus, display::DisplayPage, pages::Page};

/// A Settings page's snapshot, read from the page it names, so `Root::settings_page_snapshot` and
/// the test harness reach every page through one path keyed on the snapshot's type rather than a
/// forwarder per page (ADR-0037's Settings page identity amendment).
pub trait PageSnapshot: Sized {
    /// The page the snapshot is read from.
    type Page: Page;

    fn read(page: &Self::Page, cx: &App) -> Self;
}

/// The Settings shell's state: the page on show and the dialog open over it. Each page's own state
/// is its own [`PageSnapshot`], so a change to one domain edits one struct.
#[doc(hidden)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingsSnapshot {
    /// The page on show, as its `SettingsPage` variant name.
    pub page: String,
    /// Whether the page, not the index rail, holds keyboard focus.
    pub page_focused: bool,
    /// The status line's message, where the stubbed buttons report.
    pub status_message: Option<String>,
    /// The page the open Medium page picker has highlighted, the one `enter` opens.
    pub picker_chosen: Option<String>,
}

/// The Display page: the focused control above the Colour Theme grid and the focused card, then
/// each control's value (enum variant names; `date_style` is `None` for the default).
#[doc(hidden)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingsDisplaySnapshot {
    pub display_field: Option<usize>,
    pub colour_theme_focus: Option<usize>,
    pub date_style: Option<String>,
    pub row_density: String,
    pub status_glyphs: String,
    pub start_sidebar_minimised: bool,
    pub toasts_on: bool,
}

impl SettingsView {
    /// The page on show and where focus sits. The status message is left empty.
    pub fn snapshot(&self) -> SettingsSnapshot {
        SettingsSnapshot {
            page: format!("{:?}", self.selected_section()),
            page_focused: self.focus() == SettingsFocus::Page,
            status_message: None,
            picker_chosen: self.picker().map(|picker| format!("{:?}", picker.chosen())),
        }
    }
}

impl PageSnapshot for SettingsDisplaySnapshot {
    type Page = DisplayPage;

    fn read(page: &DisplayPage, _cx: &App) -> Self {
        let display = page.preferences();
        SettingsDisplaySnapshot {
            display_field: page.field_focus(),
            colour_theme_focus: page.colour_theme_focus(),
            date_style: display.date_style.map(|style| format!("{style:?}")),
            row_density: format!("{:?}", display.row_density),
            status_glyphs: format!("{:?}", display.status_glyphs),
            start_sidebar_minimised: display.start_sidebar_minimised,
            toasts_on: display.toasts_on,
        }
    }
}
