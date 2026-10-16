//! The Settings › Display page (ADR-0037's amendment, one folder per page). [`DisplayPage`] owns the
//! Display Preferences (`preferences.rs`), the keyboard focus on its controls and on the Colour Theme
//! grid (`colour_theme.rs`), and renders itself (`render.rs`). Each change reaches `Shell` as
//! [`ViewEvent::DisplayChanged`]; the Settings View forwards the page's keys and focus to it while
//! the page has focus.

pub mod colour_theme;
mod preferences;
mod render;

pub use preferences::*;

use gpui::{Context, EventEmitter, Keystroke};
use lib_colour_theme::ColourTheme;
use lib_core::DateStyle;

use crate::{settings::step_choice, theme::colours::ColourChange, view::event::ViewEvent};

/// The Display page's keyboard-reachable controls above the Colour Theme grid, top to bottom:
/// Date format, Row density, Status glyphs, Start sidebar minimised, Toasts. The grid is the stop
/// after the last one.
pub const DISPLAY_FIELD_COUNT: usize = 5;

/// The Display field that is a checkbox rather than a segmented or radio control.
pub const DISPLAY_FIELD_SIDEBAR: usize = 3;

/// What a key did on the page. [`DisplayKey::FocusIndex`] is the grid's `h` at its first column,
/// which steps focus out to the index rail: a focus change only Settings can make.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisplayKey {
    Unhandled,
    Handled,
    FocusIndex,
}

impl EventEmitter<ViewEvent> for DisplayPage {}

/// The Display page: the Display Preferences its controls set and the other Views and the Chrome
/// read, and the keyboard focus on its controls and Colour Theme grid. Only one of the two is
/// focused at a time, and neither while focus is elsewhere in Settings.
#[derive(Debug, Default)]
pub struct DisplayPage {
    preferences: DisplayPreferences,
    /// The control the keyboard is on, by its `DISPLAY_FIELD_*` index.
    field_focus: Option<usize>,
    /// The Colour Theme card the keyboard is on, while the grid has focus.
    colour_theme_focus: Option<usize>,
}

impl DisplayPage {
    pub fn preferences(&self) -> DisplayPreferences {
        self.preferences
    }

    pub fn field_focus(&self) -> Option<usize> {
        self.field_focus
    }

    pub fn colour_theme_focus(&self) -> Option<usize> {
        self.colour_theme_focus
    }

    /// Sets the Display Preferences from outside the page (launch, `:toasts`), without telling
    /// `Shell`, which made the change itself.
    pub fn set_preferences(&mut self, preferences: DisplayPreferences, cx: &mut Context<'_, Self>) {
        self.preferences = preferences;
        cx.notify();
    }

    /// Focus arrives on the page's first control, as `l`/`enter` from the index does.
    pub fn focus_first(&mut self, cx: &mut Context<'_, Self>) {
        self.field_focus = Some(0);
        cx.notify();
    }

    /// Focus leaves the page's controls and grid, as it does for the index or another page.
    pub fn clear_focus(&mut self, cx: &mut Context<'_, Self>) {
        self.field_focus = None;
        self.colour_theme_focus = None;
        cx.notify();
    }

    /// A movement the grid did not take (`Ctrl-d`/`Ctrl-u`) leaves it, as any other key does.
    pub fn leave_grid(&mut self, cx: &mut Context<'_, Self>) {
        self.colour_theme_focus = None;
        cx.notify();
    }

    /// `Esc` leaves the Colour Theme grid for the control above it. `false` when the grid has no
    /// focus, so the page is left to Settings.
    pub fn escape(&mut self, cx: &mut Context<'_, Self>) -> bool {
        if self.colour_theme_focus.is_none() {
            return false;
        }
        cx.emit(ViewEvent::SetStatus(None));
        self.leave_colour_theme_grid();
        cx.notify();
        true
    }

    /// The page's keys while it has focus: the Colour Theme grid's when it holds focus, else the
    /// controls'. `body_width` is the page body's width, which sets the grid's columns.
    pub fn key(
        &mut self,
        keystroke: &Keystroke,
        body_width: f32,
        cx: &mut Context<'_, Self>,
    ) -> DisplayKey {
        let outcome = if self.colour_theme_focus.is_some() {
            self.colour_theme_key(keystroke, body_width, cx)
        } else {
            self.field_key(keystroke, cx)
        };
        cx.notify();
        outcome
    }

    /// The Display page's form keys: `j`/`k` walk the controls and on past the last into the
    /// Colour Theme grid, `h`/`l` change a segmented or radio control (or clear/tick the
    /// checkbox) in place, `enter`/`space` toggles the checkbox.
    fn field_key(&mut self, keystroke: &Keystroke, cx: &mut Context<'_, Self>) -> DisplayKey {
        let Some(field) = self.field_focus else {
            return DisplayKey::Unhandled;
        };
        if keystroke.modifiers.shift {
            return DisplayKey::Unhandled;
        }
        let delta = match keystroke.key.as_str() {
            "h" | "left" => -1,
            "l" | "right" => 1,
            _ => 0,
        };
        match keystroke.key.as_str() {
            "j" | "down" => {
                if field + 1 >= DISPLAY_FIELD_COUNT {
                    self.field_focus = None;
                    self.colour_theme_focus = Some(colour_theme::chosen_index(cx));
                } else {
                    self.field_focus = Some(field + 1);
                }
                DisplayKey::Handled
            }
            "k" | "up" => {
                self.field_focus = Some(field.saturating_sub(1));
                DisplayKey::Handled
            }
            "h" | "left" | "l" | "right" => {
                self.change_display(
                    |display| match field {
                        0 => {
                            display.date_style =
                                step_choice(&DATE_STYLE_CHOICES, display.date_style, delta);
                        }
                        1 => {
                            display.row_density =
                                step_choice(&RowDensity::ALL, display.row_density, delta);
                        }
                        2 => {
                            display.status_glyphs =
                                step_choice(&StatusGlyphs::ALL, display.status_glyphs, delta);
                        }
                        DISPLAY_FIELD_SIDEBAR => display.start_sidebar_minimised = delta > 0,
                        _ => display.toasts_on = delta < 0,
                    },
                    cx,
                );
                DisplayKey::Handled
            }
            "enter" | "space" => {
                if field == DISPLAY_FIELD_SIDEBAR {
                    self.click_start_sidebar_minimised(cx);
                }
                DisplayKey::Handled
            }
            _ => DisplayKey::Unhandled,
        }
    }

    /// The Colour Theme grid (`docs/colour-themes-design.md` "Settings"): arrows or `h`/`j`/`k`/`l`
    /// move focus, `Enter` selects, and `Tab` leaves it, going on to the next zone. Moving focus
    /// never previews. Any key the grid does not take leaves it.
    fn colour_theme_key(
        &mut self,
        keystroke: &Keystroke,
        body_width: f32,
        cx: &mut Context<'_, Self>,
    ) -> DisplayKey {
        let Some(index) = self.colour_theme_focus else {
            return DisplayKey::Unhandled;
        };
        let key = keystroke.key.as_str();
        match key {
            "tab" => {
                self.colour_theme_focus = None;
                DisplayKey::Unhandled
            }
            "enter" => {
                if let Some(theme) = ColourTheme::built_in().get(index) {
                    apply_colour(ColourChange::Theme(theme.id), cx);
                }
                DisplayKey::Handled
            }
            _ => {
                let columns = colour_theme::grid_columns(body_width);
                let len = ColourTheme::built_in().len();
                // `k` on the top row climbs back to the last control above the grid.
                if matches!(key, "k" | "up") && index < columns {
                    self.leave_colour_theme_grid();
                    return DisplayKey::Handled;
                }
                // `h` at the first column steps back out to the index.
                if matches!(key, "h" | "left") && index % columns.max(1) == 0 {
                    return DisplayKey::FocusIndex;
                }
                self.colour_theme_focus = colour_theme::grid_move(index, len, columns, key);
                if self.colour_theme_focus.is_some() {
                    DisplayKey::Handled
                } else {
                    DisplayKey::Unhandled
                }
            }
        }
    }

    /// Steps from the Colour Theme grid back up to the page's last control.
    fn leave_colour_theme_grid(&mut self) {
        self.colour_theme_focus = None;
        self.field_focus = Some(DISPLAY_FIELD_COUNT - 1);
    }

    /// Applies a change made on the page and hands the result to `Shell`.
    fn change_display(
        &mut self,
        change: impl FnOnce(&mut DisplayPreferences),
        cx: &mut Context<'_, Self>,
    ) {
        change(&mut self.preferences);
        cx.emit(ViewEvent::DisplayChanged(self.preferences));
        cx.notify();
    }

    fn click_date_style(&mut self, style: Option<DateStyle>, cx: &mut Context<'_, Self>) {
        self.change_display(|display| display.date_style = style, cx);
    }

    fn click_row_density(&mut self, density: RowDensity, cx: &mut Context<'_, Self>) {
        self.change_display(|display| display.row_density = density, cx);
    }

    fn click_status_glyphs(&mut self, glyphs: StatusGlyphs, cx: &mut Context<'_, Self>) {
        self.change_display(|display| display.status_glyphs = glyphs, cx);
    }

    /// "Start Sidebar minimised" takes effect at the next launch, so the current rail is left
    /// alone.
    fn click_start_sidebar_minimised(&mut self, cx: &mut Context<'_, Self>) {
        self.change_display(
            |display| display.start_sidebar_minimised = !display.start_sidebar_minimised,
            cx,
        );
    }

    fn click_toasts(&mut self, on: bool, cx: &mut Context<'_, Self>) {
        self.change_display(|display| display.toasts_on = on, cx);
    }

    /// A card click selects its Colour Theme and clears any keyboard focus left on the grid.
    fn click_colour_theme(&mut self, id: &'static str, cx: &mut Context<'_, Self>) {
        self.colour_theme_focus = None;
        apply_colour(ColourChange::Theme(id), cx);
        cx.notify();
    }
}

/// Applies a Colour Theme once the current update is done: the change refreshes every window,
/// which must not happen while `Shell` is mid-update.
fn apply_colour(change: ColourChange, cx: &mut Context<'_, DisplayPage>) {
    cx.defer(move |cx| change.apply(cx));
}
