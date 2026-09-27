//! The resolved Colour Theme as ratatui `Style`s (ADR-0024, ADR-0025). `Shell` owns one
//! [`Colours`] and passes it into every `View` and popup render; views ask for a named style
//! (`c.muted()`, `c.selection()`) rather than writing a `Color::` literal.
//!
//! Two modes: RGB draws the resolved Colour Theme, downgraded to the terminal's
//! [`ColourDepth`]; terminal colours (the `terminal_colours` Configuration switch, or a
//! 16-colour terminal) ignore the Colour Theme and draw the fallback table in
//! `docs/colour-themes-design.md`, so the terminal's own theme wins.
//!
//! The `colour_theme` and `colour_appearance` Preferences live here in memory until the TUI
//! reads `preferences` from the database; null means Modernist and System.

use lib_colour_theme::{
    ColourAppearance, ColourVariant, ResolveInputs, ResolvedColours, Rgba, ThemeOverrides, resolve,
};
use ratatui::{
    buffer::Buffer,
    style::{Color, Modifier, Style},
};

/// How many colours the terminal can draw. Detection lands with #329; until then the TUI
/// assumes truecolor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ColourDepth {
    #[default]
    TrueColor,
    /// Each colour drawn as the nearest of the xterm 256.
    Ansi256,
    /// Too few colours to draw a Colour Theme: the fallback table is used instead.
    Ansi16,
}

/// The styles every view and popup draws with, plus the inputs they were resolved from.
#[derive(Debug, Clone)]
pub struct Colours {
    resolved: ResolvedColours,
    colour_theme: Option<String>,
    colour_appearance: Option<ColourAppearance>,
    overrides: ThemeOverrides,
    system: Option<ColourVariant>,
    terminal_colours: bool,
    depth: ColourDepth,
}

impl Default for Colours {
    fn default() -> Self {
        Self::new(ThemeOverrides::default(), false, ColourDepth::default())
    }
}

impl Colours {
    /// Resolves with null Preferences. System is undetected until #329, so it draws Dark.
    pub fn new(overrides: ThemeOverrides, terminal_colours: bool, depth: ColourDepth) -> Self {
        Self {
            resolved: resolve_logged(ResolveInputs {
                colour_theme: None,
                colour_appearance: None,
                overrides: &overrides,
                system: None,
            }),
            colour_theme: None,
            colour_appearance: None,
            overrides,
            system: None,
            terminal_colours,
            depth,
        }
    }

    fn resolve(&mut self) {
        self.resolved = resolve_logged(ResolveInputs {
            colour_theme: self.colour_theme.as_deref(),
            colour_appearance: self.colour_appearance,
            overrides: &self.overrides,
            system: self.system,
        });
    }

    /// Sets the in-memory `colour_theme` Preference; the next draw shows it.
    pub fn set_colour_theme(&mut self, colour_theme: Option<String>) {
        self.colour_theme = colour_theme;
        self.resolve();
    }

    /// Sets the in-memory `colour_appearance` Preference; the next draw shows it.
    pub fn set_colour_appearance(&mut self, colour_appearance: Option<ColourAppearance>) {
        self.colour_appearance = colour_appearance;
        self.resolve();
    }

    pub fn resolved(&self) -> &ResolvedColours {
        &self.resolved
    }

    /// Whether the fallback table is drawn instead of the Colour Theme.
    fn terminal(&self) -> bool {
        self.terminal_colours || self.depth == ColourDepth::Ansi16
    }

    /// The one place a Colour Theme value becomes a ratatui colour.
    fn colour(&self, colour: Rgba) -> Color {
        let colour = colour.over(self.resolved.roles.background);
        match self.depth {
            ColourDepth::Ansi256 => Color::Indexed(nearest_256(colour)),
            _ => Color::Rgb(colour.r, colour.g, colour.b),
        }
    }

    /// Paints the Colour Theme's `foreground`/`background` into every cell still on the
    /// terminal's defaults, so views and popups (including ones that `Clear` their area)
    /// sit on the Colour Theme without each setting a base style. A no-op in terminal colours.
    pub fn paint_base(&self, buf: &mut Buffer) {
        if self.terminal() {
            return;
        }
        let fg = self.colour(self.resolved.roles.foreground);
        let bg = self.colour(self.resolved.roles.background);
        let area = buf.area;
        for y in area.top()..area.bottom() {
            for x in area.left()..area.right() {
                if let Some(cell) = buf.cell_mut((x, y)) {
                    if cell.fg == Color::Reset {
                        cell.fg = fg;
                    }
                    if cell.bg == Color::Reset {
                        cell.bg = bg;
                    }
                }
            }
        }
    }

    /// Secondary text: the `muted` role, or `DIM` in terminal colours.
    pub fn muted(&self) -> Style {
        if self.terminal() {
            Style::default().add_modifier(Modifier::DIM)
        } else {
            Style::default().fg(self.colour(self.resolved.roles.muted))
        }
    }

    /// Emphasised text on `background`: the `accent` text shade, or red in terminal colours.
    pub fn accent(&self) -> Style {
        if self.terminal() {
            Style::default().fg(Color::Red)
        } else {
            Style::default().fg(self.colour(self.resolved.text_shades.accent))
        }
    }

    /// The [`accent`](Self::accent) colour alone, for widgets that take a `Color`.
    pub fn accent_colour(&self) -> Color {
        self.accent().fg.unwrap_or(Color::Red)
    }

    /// The current or selected row: the calculated selection swap, or `REVERSED` in terminal
    /// colours.
    pub fn selection(&self) -> Style {
        if self.terminal() {
            Style::default().add_modifier(Modifier::REVERSED)
        } else {
            let selection = self.resolved.selection;
            Style::default()
                .fg(self.colour(selection.text))
                .bg(self.colour(selection.background))
        }
    }

    /// The status line bar: `chrome` behind `foreground`, or `REVERSED` in terminal colours.
    pub fn status_bar(&self) -> Style {
        if self.terminal() {
            Style::default().add_modifier(Modifier::REVERSED)
        } else {
            Style::default()
                .fg(self.colour(self.resolved.roles.foreground))
                .bg(self.colour(self.resolved.chrome))
        }
    }
}

// Contrast failures are only possible through `[theme]` overrides; they are drawn anyway.
fn resolve_logged(inputs: ResolveInputs<'_>) -> ResolvedColours {
    let (resolved, failures) = resolve(inputs);
    for failure in failures {
        tracing::warn!(%failure, "Colour Theme pair below its contrast rule");
    }
    resolved
}

/// The nearest xterm 256 index to an opaque colour: the 6×6×6 cube or the 24-step grey
/// ramp, whichever is closer. The first 16 are skipped because the terminal theme redefines
/// them.
fn nearest_256(colour: Rgba) -> u8 {
    const CUBE: [u8; 6] = [0, 95, 135, 175, 215, 255];
    let nearest_step = |channel: u8| -> u8 {
        let mut best = 0;
        for (i, step) in CUBE.iter().enumerate() {
            if channel.abs_diff(*step) < channel.abs_diff(CUBE[best]) {
                best = i;
            }
        }
        best as u8
    };
    let (r, g, b) = (
        nearest_step(colour.r),
        nearest_step(colour.g),
        nearest_step(colour.b),
    );
    let cube = [CUBE[r as usize], CUBE[g as usize], CUBE[b as usize]];

    let average = (u16::from(colour.r) + u16::from(colour.g) + u16::from(colour.b)) / 3;
    let grey_step = (average.saturating_sub(3) / 10).min(23) as u8;
    let grey = 8 + grey_step * 10;

    let distance = |to: [u8; 3]| -> u32 {
        [colour.r, colour.g, colour.b]
            .iter()
            .zip(to)
            .map(|(a, b)| u32::from(a.abs_diff(b)).pow(2))
            .sum()
    };
    if distance([grey; 3]) < distance(cube) {
        232 + grey_step
    } else {
        16 + 36 * r + 6 * g + b
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::layout::Rect;

    #[test]
    fn null_preferences_resolve_to_modernist_dark_until_system_is_detected() {
        let colours = Colours::default();
        assert_eq!(colours.resolved().theme_id, "modernist");
        assert_eq!(colours.resolved().variant, ColourVariant::Dark);
    }

    #[test]
    fn changing_a_preference_re_resolves_the_styles() {
        let mut colours = Colours::default();
        let before = colours.muted();
        colours.set_colour_theme(Some("nord".to_string()));
        colours.set_colour_appearance(Some(ColourAppearance::Light));
        assert_eq!(colours.resolved().theme_id, "nord");
        assert_eq!(colours.resolved().variant, ColourVariant::Light);
        assert_ne!(colours.muted(), before);
    }

    #[test]
    fn terminal_colours_draw_the_fallback_table() {
        let colours = Colours::new(ThemeOverrides::default(), true, ColourDepth::TrueColor);
        assert_eq!(
            colours.muted(),
            Style::default().add_modifier(Modifier::DIM)
        );
        assert_eq!(colours.accent(), Style::default().fg(Color::Red));
        assert_eq!(
            colours.selection(),
            Style::default().add_modifier(Modifier::REVERSED)
        );
    }

    #[test]
    fn a_16_colour_terminal_falls_back_to_terminal_colours() {
        let colours = Colours::new(ThemeOverrides::default(), false, ColourDepth::Ansi16);
        assert_eq!(colours.accent(), Style::default().fg(Color::Red));
    }

    #[test]
    fn a_256_colour_terminal_draws_indexed_colours() {
        let colours = Colours::new(ThemeOverrides::default(), false, ColourDepth::Ansi256);
        assert!(matches!(colours.muted().fg, Some(Color::Indexed(_))));
    }

    #[test]
    fn nearest_256_picks_cube_or_grey() {
        assert_eq!(nearest_256(Rgba::rgb(255, 0, 0)), 196);
        assert_eq!(nearest_256(Rgba::rgb(0, 0, 0)), 16);
        assert_eq!(nearest_256(Rgba::rgb(128, 128, 128)), 244);
    }

    #[test]
    fn paint_base_fills_only_default_cells() {
        let colours = Colours::default();
        let mut buf = Buffer::empty(Rect::new(0, 0, 2, 1));
        buf.set_style(Rect::new(1, 0, 1, 1), Style::default().fg(Color::Red));
        colours.paint_base(&mut buf);
        let bg = colours.colour(colours.resolved().roles.background);
        assert_eq!(buf[(0, 0)].bg, bg);
        assert_eq!(buf[(1, 0)].fg, Color::Red);
        assert_eq!(buf[(1, 0)].bg, bg);
    }

    #[test]
    fn paint_base_is_a_no_op_in_terminal_colours() {
        let colours = Colours::new(ThemeOverrides::default(), true, ColourDepth::TrueColor);
        let mut buf = Buffer::empty(Rect::new(0, 0, 1, 1));
        colours.paint_base(&mut buf);
        assert_eq!(buf[(0, 0)].bg, Color::Reset);
    }
}
