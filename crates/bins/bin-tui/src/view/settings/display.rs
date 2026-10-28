//! Settings › Display (TUI-032): the Appearance and Colour Theme Preferences (ADR-0023), which
//! sync, the date style, the read-only Locale (ADR-0021), terminal colours (ADR-0024), status
//! glyphs, the Client-scoped Toasts Preference (ADR-0027) and the hint bar. Each row carries its
//! own scope glyph, since the page mixes scopes.
//!
//! `enter` on Appearance or Colour Theme opens its existing list popup, which `Shell` hosts, and
//! `enter` or `space` on Toasts or the hint bar flips it. The date style opens in place and holds
//! for the session ([`SettingsValues`](super::field::SettingsValues)): no TUI View formats dates by
//! it yet, so the choice shows on this row, and the card's preview box isn't drawn until one does.
//!
//! Terminal colours, status glyphs and the hint bar are Configuration in `personal-ledger.conf`
//! ([`Configuration`](super::field::Configuration)), which `Shell` writes to the user's file
//! (#904): terminal colours and status glyphs open in place, and the hint bar flips. A key an
//! environment variable or a higher file sets is noted `set by …` on its row, and `enter` on it
//! says why it can't be saved here rather than appearing to save.

use crossterm::event::{KeyCode, KeyEvent};
use lib_colour_theme::ColourAppearance;
use lib_config::StatusGlyphs;
use lib_list_cursor::Step;
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    text::Span,
    widgets::{Paragraph, Wrap},
};

use super::field::{DATE_STYLES, SettingField, on_off};
use super::keys::{self, Binding, Press};
use super::{Context, FieldRow, Open, Scope, list, render_form};
use crate::colours::Colours;
use crate::msg;
use crate::popup::settings::colour::{appearance_label, colour_theme_name, system_hint};
use crate::popup::settings::edit::{Draft, Segment};
use crate::view::Action;

/// The rows, in the page's order.
const ROWS: [SettingField; 8] = [
    SettingField::Appearance,
    SettingField::ColourTheme,
    SettingField::DateStyle,
    SettingField::Locale,
    SettingField::TerminalColours,
    SettingField::StatusGlyphs,
    SettingField::Toasts,
    SettingField::HintBar,
];

#[derive(Debug, Clone, Copy)]
enum Command {
    Row,
    Open,
    Toggle,
}

/// The page's keys and its hint bar, one table (#898).
const KEYS: &[Binding<Command>] = &[
    Binding {
        keys: keys::ROW_KEYS,
        label: "j/k",
        hint: msg::tui_settings_hint_field,
        command: Command::Row,
    },
    Binding {
        keys: &[Press::key(KeyCode::Enter)],
        label: "enter",
        hint: msg::tui_settings_hint_edit,
        command: Command::Open,
    },
    Binding {
        keys: &[Press::char(' ')],
        label: "space",
        hint: msg::tui_settings_hint_toggle,
        command: Command::Toggle,
    },
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

    /// `enter` opens the selected row; on Toasts or the hint bar it, like `space`, flips the
    /// checkbox at once, from `hint_bar` for the hint bar. `space` on any other row does nothing.
    pub(super) fn handle_key(&mut self, key: KeyEvent, hint_bar: bool) -> Option<Action> {
        let field = ROWS[self.selected];
        let flipped = match field {
            SettingField::Toasts => Some(!self.toasts_on),
            SettingField::HintBar => Some(!hint_bar),
            _ => None,
        };
        Some(match keys::lookup(KEYS, key)? {
            Command::Row => {
                self.step(list::motion(key)?);
                Action::NoOp
            }
            Command::Open | Command::Toggle if let Some(on) = flipped => {
                Action::SetSetting(field, Some(on_off(on).to_string()))
            }
            Command::Open => Action::SetSetting(field, None),
            Command::Toggle => Action::NoOp,
        })
    }

    pub(super) fn hints() -> Vec<(String, String)> {
        keys::hints(KEYS)
    }

    pub(super) fn step(&mut self, step: Step) {
        self.selected = lib_list_cursor::step(self.selected, ROWS.len(), step);
    }

    /// Selects `field`'s row, if it is on this page.
    pub(super) fn select(&mut self, field: SettingField) {
        if let Some(index) = ROWS.iter().position(|row| *row == field) {
            self.selected = index;
        }
    }

    /// `focused`, the selected row is reversed; an `editor` open on one of its rows draws in that
    /// row's place. The notes are pinned to the bottom.
    pub(super) fn render(
        &self,
        frame: &mut Frame<'_>,
        area: Rect,
        focused: bool,
        context: &Context<'_>,
        c: &Colours,
    ) {
        let rows: Vec<FieldRow> = ROWS
            .iter()
            .map(|field| self.field(*field, context, c))
            .collect();
        let open = context.editor.map(|editor| Open {
            editor,
            sub_rows: vec![(msg::tui_settings_edit_was(), editor.was().to_string())],
        });
        let rest = render_form(
            frame,
            area,
            &rows,
            focused.then_some(self.selected),
            open,
            c,
        );
        let [_, notes] = Layout::vertical([Constraint::Min(0), Constraint::Length(2)]).areas(rest);
        let config_note = match &context.values.configuration.path {
            Some(path) => msg::tui_settings_display_note_config_writes(&path.display().to_string()),
            None => msg::tui_settings_display_note_config(),
        };
        frame.render_widget(
            Paragraph::new(vec![
                ratatui::text::Line::from(Span::styled(config_note, c.muted())),
                ratatui::text::Line::from(Span::styled(
                    msg::tui_settings_display_note_scope(),
                    c.muted(),
                )),
            ])
            .wrap(Wrap { trim: true }),
            notes,
        );
    }

    /// What opening `field` in place starts from: its draft and the old value's label. `None` for
    /// a row that opens a popup or is read-only.
    pub(super) fn draft(
        &self,
        field: SettingField,
        context: &Context<'_>,
    ) -> Option<(Draft, String)> {
        let configuration = &context.values.configuration;
        match field {
            SettingField::TerminalColours => {
                let on = configuration.terminal_colours;
                Some((
                    choice(&[(false, "off"), (true, "on")], on, |on| {
                        terminal_colours_label(*on)
                    }),
                    terminal_colours_label(on),
                ))
            }
            SettingField::StatusGlyphs => {
                let glyphs = configuration.status_glyphs;
                Some((
                    choice(
                        &[
                            (StatusGlyphs::Unicode, StatusGlyphs::Unicode.key()),
                            (StatusGlyphs::Ascii, StatusGlyphs::Ascii.key()),
                        ],
                        glyphs,
                        |glyphs| status_glyphs_label(*glyphs),
                    ),
                    status_glyphs_label(glyphs),
                ))
            }
            // `:set hint-bar` without a value opens the checkbox in place, as Toasts does.
            SettingField::HintBar => Some((
                Draft::Check(configuration.hint_bar),
                hint_bar_label(configuration.hint_bar),
            )),
            SettingField::DateStyle => {
                let segments = DATE_STYLES
                    .iter()
                    .map(|(value, style)| Segment {
                        value: (*value).to_string(),
                        label: crate::locale::date_style_label(*style),
                    })
                    .collect();
                let chosen = DATE_STYLES
                    .iter()
                    .position(|(_, style)| *style == context.values.date_style)
                    .unwrap_or_default();
                Some((
                    Draft::Choice { segments, chosen },
                    crate::locale::date_style_label(context.values.date_style),
                ))
            }
            // `:set toasts` without a value opens the checkbox in place.
            SettingField::Toasts => {
                Some((Draft::Check(self.toasts_on), toasts_label(self.toasts_on)))
            }
            _ => None,
        }
    }

    /// One row's glyph, value and note. The colour rows stay editable under `terminal_colours`,
    /// since the Preferences still sync, with a note naming it.
    fn field(&self, field: SettingField, context: &Context<'_>, c: &Colours) -> FieldRow {
        let colour_note = if c.terminal_colours() {
            msg::tui_settings_colour_terminal_row_note("terminal_colours")
        } else {
            msg::tui_settings_colour_theme_note()
        };
        let configuration = &context.values.configuration;
        // A Configuration row names what outranks its write, else the file it saves to.
        let config = || {
            field
                .config_key()
                .and_then(|key| configuration.shadow(key))
                .map_or_else(msg::tui_settings_config_file_note, |source| {
                    msg::tui_settings_config_shadowed_note(&source)
                })
        };
        let (glyph, value, note) = match field {
            SettingField::Appearance => {
                let appearance = c.colour_appearance().unwrap_or_default();
                (
                    Scope::Synced,
                    appearance_label(appearance),
                    // The System hint outgrows the value column, so it takes the note.
                    if appearance == ColourAppearance::System {
                        system_hint(c.system())
                    } else {
                        colour_note
                    },
                )
            }
            SettingField::ColourTheme => (
                Scope::Synced,
                colour_theme_name(c.resolved().theme_id),
                colour_note,
            ),
            SettingField::DateStyle => (
                Scope::ThisDevice,
                crate::locale::date_style_label(context.values.date_style),
                String::new(),
            ),
            SettingField::Locale => {
                // The Locale startup resolved and where it came from, never a fixed value.
                let (tag, source) = crate::locale::describe(&crate::locale::info());
                (Scope::ThisDevice, tag, source)
            }
            SettingField::TerminalColours => (
                Scope::ThisDevice,
                terminal_colours_label(configuration.terminal_colours),
                config(),
            ),
            SettingField::StatusGlyphs => (
                Scope::ThisDevice,
                status_glyphs_label(configuration.status_glyphs),
                config(),
            ),
            SettingField::Toasts => (
                Scope::ThisDevice,
                toasts_label(self.toasts_on),
                lib_locale::msg::toast_setting_note(),
            ),
            _ => (
                Scope::ThisDevice,
                hint_bar_label(configuration.hint_bar),
                config(),
            ),
        };
        FieldRow {
            glyph: Some(glyph),
            field,
            value,
            note,
        }
    }
}

/// A two-way choice drafted on `chosen`, each option by the value `:set` takes.
fn choice<T: PartialEq>(options: &[(T, &str)], chosen: T, label: impl Fn(&T) -> String) -> Draft {
    Draft::Choice {
        segments: options
            .iter()
            .map(|(option, value)| Segment {
                value: (*value).to_string(),
                label: label(option),
            })
            .collect(),
        chosen: options
            .iter()
            .position(|(option, _)| *option == chosen)
            .unwrap_or_default(),
    }
}

fn terminal_colours_label(on: bool) -> String {
    if on {
        msg::tui_settings_terminal_colours_palette()
    } else {
        msg::tui_settings_terminal_colours_theme()
    }
}

fn status_glyphs_label(glyphs: StatusGlyphs) -> String {
    match glyphs {
        StatusGlyphs::Unicode => msg::tui_settings_status_glyphs_unicode(),
        StatusGlyphs::Ascii => msg::tui_settings_status_glyphs_ascii(),
    }
}

fn hint_bar_label(on: bool) -> String {
    if on {
        msg::tui_settings_hint_bar_shown()
    } else {
        msg::tui_settings_hint_bar_hidden()
    }
}

fn toasts_label(on: bool) -> String {
    if on {
        lib_locale::msg::toast_setting_on()
    } else {
        lib_locale::msg::toast_setting_off()
    }
}

#[cfg(test)]
mod tests {
    use crossterm::event::KeyModifiers;

    use super::*;
    use crate::view::View;
    use crate::view::settings::field::SettingsValues;
    use crate::view::settings::{SettingsPage, SettingsView};

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn context(values: &SettingsValues) -> Context<'_> {
        Context {
            values,
            units: None,
            editor: None,
        }
    }

    #[test]
    fn enter_opens_each_colour_popup_by_its_field_and_flips_toasts() {
        let mut page = DisplayPage::new();
        assert_eq!(
            page.handle_key(key(KeyCode::Enter), true),
            Some(Action::SetSetting(SettingField::Appearance, None))
        );
        page.handle_key(key(KeyCode::Char('j')), true);
        assert_eq!(
            page.handle_key(key(KeyCode::Enter), true),
            Some(Action::SetSetting(SettingField::ColourTheme, None))
        );
        page.select(SettingField::Toasts);
        assert_eq!(
            page.handle_key(key(KeyCode::Enter), true),
            Some(Action::SetSetting(SettingField::Toasts, Some("off".into())))
        );
        page.set_toasts_on(false);
        assert_eq!(
            page.handle_key(key(KeyCode::Char(' ')), true),
            Some(Action::SetSetting(SettingField::Toasts, Some("on".into())))
        );
        page.select(SettingField::Locale);
        assert_eq!(
            page.handle_key(key(KeyCode::Enter), true),
            Some(Action::SetSetting(SettingField::Locale, None)),
            "the Shell refuses the read-only Locale with its reason"
        );
        assert_eq!(
            page.handle_key(key(KeyCode::Char(' ')), true),
            Some(Action::NoOp),
            "space only toggles Toasts"
        );
    }

    #[test]
    fn the_page_shows_every_row_with_its_scope_glyph() {
        crate::locale::init_for_tests();
        let mut view = SettingsView::new();
        view.update(&Action::OpenSettingsPage(SettingsPage::Display));
        let text = crate::view::settings::tests::render_at(&view, 96, 30);
        for needle in [
            "● appearance",
            "System",
            "● colour theme",
            "Modernist",
            "○ date style",
            "locale default",
            "○ locale",
            "en-US",
            "the default",
            "○ terminal colours",
            "theme rgb",
            "○ status glyphs",
            "unicode",
            "○ Toasts",
            "○ hint bar",
            "config file",
        ] {
            assert!(text.contains(needle), "{needle} missing:\n{text}");
        }
        let values = SettingsValues::default();
        assert_eq!(
            DisplayPage::new()
                .field(
                    SettingField::Appearance,
                    &context(&values),
                    &Colours::default()
                )
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
        let mut values = SettingsValues::default();
        values.configuration.terminal_colours = true;
        let page = DisplayPage::new();
        assert!(
            page.field(SettingField::ColourTheme, &context(&values), &c)
                .note
                .contains("terminal_colours")
        );
        assert_eq!(
            page.field(SettingField::TerminalColours, &context(&values), &c)
                .value,
            "terminal palette"
        );
    }

    #[test]
    fn the_configuration_rows_open_in_place_flip_and_name_what_outranks_them() {
        crate::locale::init_for_tests();
        let mut values = SettingsValues::default();
        let mut page = DisplayPage::new();
        page.select(SettingField::TerminalColours);
        assert_eq!(
            page.handle_key(key(KeyCode::Enter), true),
            Some(Action::SetSetting(SettingField::TerminalColours, None))
        );
        let Some((Draft::Choice { segments, chosen }, was)) =
            page.draft(SettingField::StatusGlyphs, &context(&values))
        else {
            panic!("status glyphs opens a choice");
        };
        assert_eq!(
            segments
                .iter()
                .map(|s| s.value.as_str())
                .collect::<Vec<_>>(),
            ["unicode", "ascii"]
        );
        assert_eq!((chosen, was.as_str()), (0, "unicode"));

        page.select(SettingField::HintBar);
        assert_eq!(
            page.handle_key(key(KeyCode::Char(' ')), true),
            Some(Action::SetSetting(
                SettingField::HintBar,
                Some("off".into())
            ))
        );
        assert_eq!(
            page.handle_key(key(KeyCode::Enter), false),
            Some(Action::SetSetting(SettingField::HintBar, Some("on".into())))
        );

        values.configuration.hint_bar = false;
        values.configuration.shadows = vec![(
            lib_config::ConfigKey::HintBar,
            lib_config::Shadow::Env("PERSONAL_LEDGER_PERSONAL_LEDGER__HINT_BAR".into()),
        )];
        let row = page.field(
            SettingField::HintBar,
            &context(&values),
            &Colours::default(),
        );
        assert_eq!(row.value, "[ ] show key hints");
        assert_eq!(row.note, "set by PERSONAL_LEDGER_PERSONAL_LEDGER__HINT_BAR");
        assert_eq!(
            page.field(
                SettingField::StatusGlyphs,
                &context(&values),
                &Colours::default()
            )
            .note,
            "config file"
        );
    }

    #[test]
    fn the_hint_bar_is_the_keys_the_page_takes() {
        keys::tests::assert_the_hints_are_the_keys(KEYS, DisplayPage::hints(), |key| {
            DisplayPage::new().handle_key(key, true)
        });
    }
}
