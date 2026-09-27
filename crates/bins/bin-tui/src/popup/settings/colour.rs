//! The Display group's Colour Theme and Colour Appearance list popup (`docs/colour-themes-design.md`
//! "Settings", decided on #303): the §4b in-place editor pattern as a list, each Colour Theme row
//! carrying seven `█` swatch cells, one per stored Colour Role. `j`/`k` previews live, Enter
//! keeps, Esc reverts to the Preference the popup opened on.
//!
//! The popup owns only its cursor and that original Preference; the Preference itself lives on
//! `Shell`'s `Colours`, which `Shell` updates as the cursor moves so the whole frame previews.

use lib_colour_theme::{ColourAppearance, ColourTheme, ColourVariant, Palette};
use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Clear, Paragraph, Wrap},
};

use crate::colours::Colours;
use crate::msg;

/// Width of the popup in cells, wide enough for the notes to wrap onto a few lines.
const POPUP_WIDTH: u16 = 64;

/// Width of the option name column, before the swatches.
const NAME_WIDTH: usize = 16;

const APPEARANCES: [ColourAppearance; 3] = [
    ColourAppearance::Light,
    ColourAppearance::Dark,
    ColourAppearance::System,
];

/// Which Preference the popup picks, and the value it held when the popup opened.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ColourPick {
    Theme(Option<String>),
    Appearance(Option<ColourAppearance>),
}

/// A Preference the popup asks `Shell` to set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ColourPreference {
    Theme(Option<String>),
    Appearance(Option<ColourAppearance>),
}

pub struct ColourPopup {
    original: ColourPick,
    cursor: usize,
}

impl ColourPopup {
    /// Opens on the Colour Theme list, the cursor on the Colour Theme drawn now.
    pub fn theme(c: &Colours) -> Self {
        let drawn = c.resolved().theme_id;
        Self {
            original: ColourPick::Theme(c.colour_theme().map(str::to_string)),
            cursor: ColourTheme::built_in()
                .iter()
                .position(|theme| theme.id == drawn)
                .unwrap_or(0),
        }
    }

    /// Opens on the Colour Appearance list, the cursor on the Preference (null is System).
    pub fn appearance(c: &Colours) -> Self {
        let chosen = c.colour_appearance().unwrap_or_default();
        Self {
            original: ColourPick::Appearance(c.colour_appearance()),
            cursor: APPEARANCES
                .iter()
                .position(|appearance| *appearance == chosen)
                .unwrap_or(0),
        }
    }

    fn len(&self) -> usize {
        match self.original {
            ColourPick::Theme(_) => ColourTheme::built_in().len(),
            ColourPick::Appearance(_) => APPEARANCES.len(),
        }
    }

    /// Moves the cursor one row (stopping at the ends) and returns the Preference to preview.
    pub fn step(&mut self, down: bool) -> ColourPreference {
        self.cursor = if down {
            (self.cursor + 1).min(self.len().saturating_sub(1))
        } else {
            self.cursor.saturating_sub(1)
        };
        self.under_cursor()
    }

    fn under_cursor(&self) -> ColourPreference {
        match self.original {
            ColourPick::Theme(_) => ColourPreference::Theme(
                ColourTheme::built_in()
                    .get(self.cursor)
                    .map(|theme| theme.id.to_string()),
            ),
            ColourPick::Appearance(_) => {
                ColourPreference::Appearance(APPEARANCES.get(self.cursor).copied())
            }
        }
    }

    /// The Preference the popup opened on, which Esc restores exactly.
    pub fn original(&self) -> ColourPreference {
        match &self.original {
            ColourPick::Theme(theme) => ColourPreference::Theme(theme.clone()),
            ColourPick::Appearance(appearance) => ColourPreference::Appearance(*appearance),
        }
    }

    pub fn render(&self, frame: &mut Frame<'_>, area: Rect, c: &Colours) {
        let width = POPUP_WIDTH.min(area.width);
        let inner_width = width.saturating_sub(2);

        let (title, options) = match self.original {
            ColourPick::Theme(_) => (
                msg::tui_settings_colour_popup_theme_title(),
                ColourTheme::built_in()
                    .iter()
                    .map(|theme| theme_line(theme.id, c))
                    .collect::<Vec<_>>(),
            ),
            ColourPick::Appearance(_) => (
                msg::tui_settings_colour_popup_appearance_title(),
                APPEARANCES
                    .iter()
                    .map(|appearance| appearance_line(*appearance, c))
                    .collect(),
            ),
        };

        let mut lines = vec![
            Line::styled(title, Style::default().add_modifier(Modifier::BOLD)),
            Line::raw(""),
        ];
        for (index, line) in options.into_iter().enumerate() {
            lines.push(if index == self.cursor {
                line.patch_style(c.selection())
            } else {
                line
            });
        }
        let notes = notes(c);
        if !notes.is_empty() {
            lines.push(Line::raw(""));
            lines.extend(notes);
        }

        // Each line takes as many rows as it wraps onto; an empty line still takes one.
        let rows: usize = lines
            .iter()
            .map(|line| {
                line.width()
                    .div_ceil(usize::from(inner_width.max(1)))
                    .max(1)
            })
            .sum();
        let height = (rows as u16 + 2).min(area.height);
        let paragraph = Paragraph::new(lines).wrap(Wrap { trim: false });
        let popup = Rect {
            x: area.x + (area.width - width) / 2,
            y: area.y + (area.height - height) / 2,
            width,
            height,
        };
        frame.render_widget(Clear, popup);
        let block = Block::bordered();
        let inner = block.inner(popup);
        frame.render_widget(block, popup);
        frame.render_widget(paragraph, inner);
    }
}

/// A Colour Theme's name, a shared Message.
pub fn colour_theme_name(id: &str) -> String {
    match id {
        "high_contrast" => lib_locale::msg::colour_theme_high_contrast(),
        "catppuccin" => lib_locale::msg::colour_theme_catppuccin(),
        "gruvbox" => lib_locale::msg::colour_theme_gruvbox(),
        "nord" => lib_locale::msg::colour_theme_nord(),
        _ => lib_locale::msg::colour_theme_modernist(),
    }
}

pub fn appearance_label(appearance: ColourAppearance) -> String {
    match appearance {
        ColourAppearance::Light => lib_locale::msg::colour_appearance_light(),
        ColourAppearance::Dark => lib_locale::msg::colour_appearance_dark(),
        ColourAppearance::System => lib_locale::msg::colour_appearance_system(),
    }
}

/// The System row's label: "System (currently Dark)", or the undetected form naming the Dark
/// fallback.
pub fn system_hint(system: Option<ColourVariant>) -> String {
    let variant_label = |variant: ColourVariant| {
        appearance_label(match variant {
            ColourVariant::Light => ColourAppearance::Light,
            ColourVariant::Dark => ColourAppearance::Dark,
        })
    };
    match system {
        Some(variant) => lib_locale::msg::colour_appearance_system_current(&variant_label(variant)),
        None => lib_locale::msg::colour_appearance_system_undetected(&variant_label(
            ColourAppearance::System.variant(None),
        )),
    }
}

/// The Colour Appearance's label as a Settings value, System naming what it resolved to.
pub fn appearance_value(appearance: ColourAppearance, c: &Colours) -> String {
    match appearance {
        ColourAppearance::System => system_hint(c.system()),
        _ => appearance_label(appearance),
    }
}

fn theme_line(id: &str, c: &Colours) -> Line<'static> {
    let preview = c.preview(id);
    let mut spans = vec![Span::raw(format!(
        " {:<NAME_WIDTH$}",
        colour_theme_name(id)
    ))];
    spans.extend(
        swatch_roles(&preview.roles)
            .into_iter()
            .map(|colour| Span::styled("█", c.swatch(colour))),
    );
    Line::from(spans)
}

fn appearance_line(appearance: ColourAppearance, c: &Colours) -> Line<'static> {
    Line::raw(format!(" {}", appearance_value(appearance, c)))
}

/// The seven stored Colour Roles, in their `[theme]` key order.
fn swatch_roles(roles: &Palette) -> [lib_colour_theme::Rgba; 7] {
    [
        roles.foreground,
        roles.background,
        roles.accent,
        roles.cursor,
        roles.muted,
        roles.positive,
        roles.negative,
    ]
}

/// The `terminal_colours` note, the `[theme]` override note and a warning per contrast failure;
/// none of them when nothing applies.
pub fn notes(c: &Colours) -> Vec<Line<'static>> {
    let mut notes = Vec::new();
    if c.terminal_colours() {
        notes.push(Line::styled(
            msg::tui_settings_colour_terminal_note("terminal_colours"),
            c.muted(),
        ));
    }
    let roles = c.overrides().roles(c.resolved().variant);
    if !roles.is_empty() {
        let roles = roles
            .iter()
            .map(|role| role.key())
            .collect::<Vec<_>>()
            .join(", ");
        notes.push(Line::styled(
            msg::tui_settings_colour_overrides(&roles),
            c.muted(),
        ));
    }
    notes.extend(c.failures().iter().map(|failure| {
        Line::styled(
            msg::tui_settings_colour_contrast(
                failure.subject,
                failure.surface,
                &format!("{:.1}", failure.ratio),
                &format!("{}", failure.required),
            ),
            c.negative(),
        )
    }));
    notes
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::colours::ColourDepth;
    use lib_colour_theme::ThemeOverrides;
    use ratatui::{Terminal, backend::TestBackend};

    fn drawn(popup: &ColourPopup, c: &Colours) -> String {
        let mut terminal =
            Terminal::new(TestBackend::new(96, 30)).expect("test backend should initialise");
        terminal
            .draw(|frame| popup.render(frame, frame.area(), c))
            .expect("drawing the popup should not error");
        let buffer = terminal.backend().buffer();
        let mut text = String::new();
        for y in 0..buffer.area.height {
            for x in 0..buffer.area.width {
                text.push_str(buffer[(x, y)].symbol());
            }
            text.push('\n');
        }
        text
    }

    #[test]
    fn each_colour_theme_row_carries_seven_swatch_cells() {
        crate::locale::init_for_tests();
        let c = Colours::default();
        let text = drawn(&ColourPopup::theme(&c), &c);
        for theme in ColourTheme::built_in() {
            let line = text
                .lines()
                .find(|line| line.contains(&colour_theme_name(theme.id)))
                .expect("every Colour Theme should have a row");
            assert_eq!(line.matches('█').count(), 7, "{line}");
        }
    }

    #[test]
    fn step_previews_the_next_theme_and_stops_at_the_ends() {
        let c = Colours::default();
        let mut popup = ColourPopup::theme(&c);
        assert_eq!(
            popup.step(true),
            ColourPreference::Theme(Some(ColourTheme::built_in()[1].id.to_string()))
        );
        popup.step(false);
        assert_eq!(
            popup.step(false),
            ColourPreference::Theme(Some(ColourTheme::built_in()[0].id.to_string()))
        );
    }

    #[test]
    fn original_is_the_preference_the_popup_opened_on_null_included() {
        let mut c = Colours::default();
        let mut popup = ColourPopup::theme(&c);
        popup.step(true);
        assert_eq!(popup.original(), ColourPreference::Theme(None));

        c.set_colour_appearance(Some(ColourAppearance::Light));
        let popup = ColourPopup::appearance(&c);
        assert_eq!(
            popup.original(),
            ColourPreference::Appearance(Some(ColourAppearance::Light))
        );
    }

    #[test]
    fn the_system_row_names_the_resolved_variant() {
        crate::locale::init_for_tests();
        let c = Colours::default();
        let text = drawn(&ColourPopup::appearance(&c), &c);
        assert!(text.contains("System (not detected, using Dark)"), "{text}");
    }

    #[test]
    fn terminal_colours_names_its_configuration_key() {
        crate::locale::init_for_tests();
        let c = Colours::new(ThemeOverrides::default(), true, ColourDepth::TrueColor);
        let text = drawn(&ColourPopup::theme(&c), &c);
        assert!(text.contains("terminal_colours is on"), "{text}");
    }

    #[test]
    fn no_notes_without_terminal_colours_or_overrides() {
        let c = Colours::default();
        assert!(notes(&c).is_empty());
    }

    #[test]
    fn an_override_is_named_and_a_failing_pair_warned() {
        crate::locale::init_for_tests();
        let mut overrides = ThemeOverrides::default();
        let grey: lib_core::HexColor = "#777777".parse().expect("valid hex");
        overrides
            .both
            .insert(lib_colour_theme::ColourRole::Accent, grey.clone());
        overrides
            .both
            .insert(lib_colour_theme::ColourRole::Background, grey);
        let c = Colours::new(overrides, false, ColourDepth::TrueColor);
        let text: Vec<String> = notes(&c).iter().map(ToString::to_string).collect();
        assert!(text[0].contains("background, accent") || text[0].contains("accent"));
        assert!(text.iter().any(|line| line.contains("needs")), "{text:?}");
    }
}
