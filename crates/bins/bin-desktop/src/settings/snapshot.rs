//! What the integration tests read back of the Settings pages this View draws (ADR-0030), built by
//! the Settings Entity from its own state (ADR-0037). The open dialog and the status message are
//! `Root`'s, so its accessors fill them in.

use gpui::App;

use super::state::SettingsView;
use crate::settings::{SettingsFocus, display::DisplayPage};
use crate::shell::view::module_page::{ModulePage, SettingsPageView};

/// A Settings page's snapshot, read from the page it names, so `Root::settings_page_snapshot` and
/// the test harness reach every page through one path keyed on the snapshot's type rather than a
/// forwarder per page (ADR-0037's Settings page identity amendment).
pub trait PageSnapshot: Sized {
    /// The Presenter of the page the snapshot is read from.
    type Presenter: SettingsPageView;

    fn read(page: &ModulePage<Self::Presenter>, cx: &App) -> Self;
}

/// The Settings shell's state: the page on show and the dialog open over it. Each page's own state
/// is its own [`PageSnapshot`], so a change to one domain edits one struct.
#[doc(hidden)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingsSnapshot {
    /// The page on show, or the index's cursor while the index shows, as its `SettingsPage`
    /// variant name.
    pub page: String,
    /// Whether a page shows, with the keyboard in it, rather than the Settings index.
    pub page_focused: bool,
    /// Whether a page shows with the keyboard on the docked index beside it, after a docked row
    /// was clicked while a page showed (Large and X-Large).
    pub dock_focused: bool,
    /// The status line's message, where the stubbed buttons report.
    pub status_message: Option<String>,
    /// The page the open Medium page picker has highlighted, the one `enter` opens.
    pub picker_chosen: Option<String>,
    /// The page the X-Large index's detail pane summarises, `None` wherever no pane docks.
    pub pane: Option<String>,
    /// Whether the pane's **Open** opens its page: false for a This Ledger page with no Ledger
    /// open, and with no pane.
    pub pane_open_enabled: bool,
}

/// The Display page: the focused field above the Colour Theme grid and the focused card, then
/// each control's value (enum variant names; `date_style` is `None` for the default).
#[doc(hidden)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingsDisplaySnapshot {
    pub display_field: Option<lib_settings::DisplayField>,
    pub colour_theme_focus: Option<usize>,
    pub date_style: Option<String>,
    pub row_density: String,
    pub status_glyphs: String,
    pub start_sidebar_minimised: bool,
}

impl SettingsView {
    /// The page on show and where focus sits. The status message is left empty.
    pub fn snapshot(&self, cx: &App) -> SettingsSnapshot {
        let pane = self.pane_summary(cx);
        SettingsSnapshot {
            pane: pane.as_ref().map(|summary| format!("{:?}", summary.page)),
            pane_open_enabled: pane.is_some_and(|summary| summary.open_enabled),
            page: format!("{:?}", self.selected_section()),
            page_focused: self.focus() == SettingsFocus::Page,
            dock_focused: self.focus() == SettingsFocus::Dock,
            status_message: None,
            picker_chosen: self
                .view_model()
                .picker
                .and_then(|picker| picker.chosen)
                .map(|page| format!("{page:?}")),
        }
    }
}

impl PageSnapshot for SettingsDisplaySnapshot {
    type Presenter = lib_settings::DisplayPresenter;

    fn read(page: &DisplayPage, _cx: &App) -> Self {
        let model = page.view_model();
        let display = model.output;
        SettingsDisplaySnapshot {
            display_field: model.fields.focused(),
            colour_theme_focus: model.grid,
            date_style: display.date_style.map(|style| format!("{style:?}")),
            row_density: format!("{:?}", display.row_density),
            status_glyphs: format!("{:?}", display.status_glyphs),
            start_sidebar_minimised: display.start_sidebar_minimised,
        }
    }
}
