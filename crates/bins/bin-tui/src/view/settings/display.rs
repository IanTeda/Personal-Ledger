//! Settings › Display (TUI-032), as far as it is built: the Appearance and Colour Theme
//! Preferences (ADR-0023), which sync, the Client-scoped Toasts Preference (ADR-0027), the date
//! style, and the read-only Locale (ADR-0021). Each row carries its own scope glyph, since the page
//! mixes scopes. Status glyphs, terminal colours and the hint bar join it in Stage 2.

use crossterm::event::{KeyCode, KeyEvent};
use lib_colour_theme::ColourAppearance;
use lib_core::DateStyle;
use lib_list_cursor::Step;
use ratatui::{Frame, layout::Rect};

use super::{FieldRow, Scope, render_fields};
use crate::colours::Colours;
use crate::msg;
use crate::popup::settings::colour::{appearance_label, colour_theme_name, system_hint};
use crate::view::Action;

/// The rows, in the page's order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Row {
    Appearance,
    ColourTheme,
    Toasts,
    DateStyle,
    Locale,
}

const ROWS: [Row; 5] = [
    Row::Appearance,
    Row::ColourTheme,
    Row::Toasts,
    Row::DateStyle,
    Row::Locale,
];

/// Display's state: the selected row, and the Toasts Preference as `Shell` last reported it.
pub(super) struct DisplayPage {
    selected: usize,
    toasts_on: bool,
}

impl DisplayPage {
    pub(super) fn new() -> Self {
        Self {
            selected: 0,
            toasts_on: true,
        }
    }

    pub(super) fn set_toasts_on(&mut self, on: bool) {
        self.toasts_on = on;
    }

    /// `enter` flips Toasts or opens the colour row's list popup; the date style and Locale rows
    /// are read-only until Stage 2.
    pub(super) fn handle_key(&mut self, key: KeyEvent) -> Option<Action> {
        let step = match key.code {
            KeyCode::Char('j') | KeyCode::Down => Step::Next,
            KeyCode::Char('k') | KeyCode::Up => Step::Prev,
            KeyCode::Home => Step::First,
            KeyCode::Char('G') | KeyCode::End => Step::Last,
            KeyCode::Enter => {
                return match ROWS[self.selected] {
                    Row::Appearance => Some(Action::OpenColourAppearancePopup),
                    Row::ColourTheme => Some(Action::OpenColourThemePopup),
                    Row::Toasts => Some(Action::SetToasts(!self.toasts_on)),
                    Row::DateStyle | Row::Locale => None,
                };
            }
            _ => return None,
        };
        self.selected = lib_list_cursor::step(self.selected, ROWS.len(), step);
        Some(Action::NoOp)
    }

    pub(super) fn render(&self, frame: &mut Frame<'_>, area: Rect, c: &Colours) {
        let rows: Vec<FieldRow> = ROWS.iter().map(|row| self.field(*row, c)).collect();
        render_fields(frame, area, &rows, self.selected, c);
    }

    /// One row's glyph, label, value and note. The colour rows stay editable under
    /// `terminal_colours`, since the Preferences still sync, with a note naming it.
    fn field(&self, row: Row, c: &Colours) -> FieldRow {
        let colour_note = if c.terminal_colours() {
            msg::tui_settings_colour_terminal_row_note("terminal_colours")
        } else {
            msg::tui_settings_colour_theme_note()
        };
        match row {
            Row::Appearance => {
                let appearance = c.colour_appearance().unwrap_or_default();
                FieldRow {
                    glyph: Some(Scope::Synced),
                    label: msg::tui_settings_setting_colour_appearance(),
                    value: appearance_label(appearance),
                    // The System hint outgrows the value column, so it takes the note.
                    note: if appearance == ColourAppearance::System {
                        system_hint(c.system())
                    } else {
                        colour_note
                    },
                }
            }
            Row::ColourTheme => FieldRow {
                glyph: Some(Scope::Synced),
                label: msg::tui_settings_setting_colour_theme(),
                value: colour_theme_name(c.resolved().theme_id),
                note: colour_note,
            },
            Row::Toasts => FieldRow {
                glyph: Some(Scope::ThisDevice),
                label: lib_locale::msg::toast_setting_label(),
                value: if self.toasts_on {
                    lib_locale::msg::toast_setting_on()
                } else {
                    lib_locale::msg::toast_setting_off()
                },
                note: lib_locale::msg::toast_setting_note(),
            },
            Row::DateStyle => FieldRow {
                glyph: Some(Scope::ThisDevice),
                label: msg::tui_settings_setting_date_style(),
                // wireframe: not read from the date style Preference until Stage 2.
                value: crate::locale::date_style_label(Some(DateStyle::Short)),
                note: String::new(),
            },
            Row::Locale => {
                // The Locale startup resolved and where it came from, never a fixed value.
                let (tag, source) = crate::locale::describe(&crate::locale::info());
                FieldRow {
                    glyph: Some(Scope::ThisDevice),
                    label: msg::tui_settings_setting_locale(),
                    value: tag,
                    note: source,
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crossterm::event::KeyModifiers;

    use super::*;
    use crate::view::View;
    use crate::view::settings::{SettingsPage, SettingsView};

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    #[test]
    fn enter_opens_each_colour_popup_and_flips_toasts() {
        let mut page = DisplayPage::new();
        assert_eq!(
            page.handle_key(key(KeyCode::Enter)),
            Some(Action::OpenColourAppearancePopup)
        );
        page.handle_key(key(KeyCode::Char('j')));
        assert_eq!(
            page.handle_key(key(KeyCode::Enter)),
            Some(Action::OpenColourThemePopup)
        );
        page.handle_key(key(KeyCode::Char('j')));
        assert_eq!(
            page.handle_key(key(KeyCode::Enter)),
            Some(Action::SetToasts(false))
        );
        page.set_toasts_on(false);
        assert_eq!(
            page.handle_key(key(KeyCode::Enter)),
            Some(Action::SetToasts(true))
        );
        page.handle_key(key(KeyCode::Char('G')));
        assert_eq!(
            page.handle_key(key(KeyCode::Enter)),
            None,
            "the Locale is read-only"
        );
    }

    #[test]
    fn the_page_shows_the_colour_toasts_and_locale_rows() {
        crate::locale::init_for_tests();
        let mut view = SettingsView::new();
        view.update(&Action::OpenSettingsPage(SettingsPage::Display));
        let text = crate::view::settings::tests::render_at(&view, 96, 28);
        for needle in [
            "appearance",
            "System",
            "colour theme",
            "Modernist",
            "Toasts",
            "locale",
            "en-US",
            "the default",
        ] {
            assert!(text.contains(needle), "{needle} missing:\n{text}");
        }
        assert_eq!(
            DisplayPage::new()
                .field(Row::Appearance, &Colours::default())
                .note,
            "System (not detected, using Dark)"
        );
    }

    #[test]
    fn under_terminal_colours_the_colour_rows_name_the_key() {
        crate::locale::init_for_tests();
        let c = Colours::new(
            lib_colour_theme::ThemeOverrides::default(),
            true,
            crate::colours::ColourDepth::TrueColor,
        );
        assert!(
            DisplayPage::new()
                .field(Row::ColourTheme, &c)
                .note
                .contains("terminal_colours")
        );
    }
}
