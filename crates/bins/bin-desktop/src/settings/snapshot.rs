//! What the integration tests read back of the Settings pages this View draws (ADR-0030), built by
//! the Settings Entity from its own state (ADR-0037). The open dialog and the status message are
//! `Shell`'s, so its accessors fill them in.

use super::state::SettingsView;
use crate::settings::SettingsFocus;

/// The Settings shell's state: the page on show and the dialog open over it. Each page's own state
/// is its own snapshot (`settings_<page>_snapshot`), so a change to one domain edits one struct.
#[doc(hidden)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingsSnapshot {
    /// The page on show, as its `SettingsPage` variant name.
    pub page: String,
    /// Whether the page, not the index rail, holds keyboard focus.
    pub page_focused: bool,
    /// The open dialog (`AddAccount`, `EditPayee`, `DeleteUnit` and so on); only one page's
    /// dialog is ever open.
    pub dialog: Option<String>,
    /// The dialog's name (or Unit code) draft, and its typed-back confirmation for a Delete.
    pub dialog_name: Option<String>,
    pub dialog_confirm: Option<String>,
    /// The name of the Category an Add Category dialog was opened under, if any.
    pub dialog_parent: Option<String>,
    /// The status line's message, where the stubbed buttons report.
    pub status_message: Option<String>,
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

/// The Tracing page: the chosen level (`TracingLevel` variant name), how many entries the filter
/// shows, their bodies newest first (`subsystem: message key=value`), how many it hides, and the
/// list's top row.
#[doc(hidden)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingsTracingSnapshot {
    pub tracing_level: String,
    pub log_line_count: usize,
    pub log_lines: Vec<String>,
    pub log_hidden_count: usize,
    pub log_scroll_top: usize,
}

impl SettingsView {
    /// The page on show and where focus sits. The dialog and status message are left empty.
    pub fn snapshot(&self) -> SettingsSnapshot {
        SettingsSnapshot {
            page: format!("{:?}", self.selected_section()),
            page_focused: self.focus() == SettingsFocus::Page,
            dialog: None,
            dialog_name: None,
            dialog_confirm: None,
            dialog_parent: None,
            status_message: None,
        }
    }

    pub fn display_snapshot(&self) -> SettingsDisplaySnapshot {
        let display = self.display();
        SettingsDisplaySnapshot {
            display_field: self.display_field(),
            colour_theme_focus: self.colour_theme_focus(),
            date_style: display.date_style.map(|style| format!("{style:?}")),
            row_density: format!("{:?}", display.row_density),
            status_glyphs: format!("{:?}", display.status_glyphs),
            start_sidebar_minimised: display.start_sidebar_minimised,
            toasts_on: display.toasts_on,
        }
    }

    pub fn tracing_snapshot(&self) -> SettingsTracingSnapshot {
        SettingsTracingSnapshot {
            tracing_level: format!("{:?}", self.log().level()),
            log_line_count: self.log().visible().len(),
            log_lines: self
                .log()
                .visible()
                .iter()
                .map(|entry| crate::settings::tracing_log::body(entry))
                .collect(),
            log_hidden_count: self.log().hidden_count(),
            log_scroll_top: self.log_list().logical_scroll_top().item_ix,
        }
    }
}
