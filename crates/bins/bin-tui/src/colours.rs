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
    ColourAppearance, ColourVariant, ContrastFailure, ResolveInputs, ResolvedColours, Rgba,
    ThemeOverrides, resolve,
};
use lib_toast::ToastKind;
use ratatui::{
    buffer::Buffer,
    style::{Color, Modifier, Style},
};

/// How many colours the terminal can draw, detected from the environment in `detect.rs`.
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
    failures: Vec<ContrastFailure>,
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
    /// Resolves with null Preferences and System undetected, so it draws Dark until
    /// [`with_system`](Self::with_system) says otherwise.
    pub fn new(overrides: ThemeOverrides, terminal_colours: bool, depth: ColourDepth) -> Self {
        let (resolved, failures) = resolve_logged(ResolveInputs {
            colour_theme: None,
            colour_appearance: None,
            overrides: &overrides,
            system: None,
        });
        Self {
            resolved,
            failures,
            colour_theme: None,
            colour_appearance: None,
            overrides,
            system: None,
            terminal_colours,
            depth,
        }
    }

    /// Sets the terminal's detected light/dark background, which System follows.
    pub fn with_system(mut self, system: Option<ColourVariant>) -> Self {
        self.system = system;
        self.resolve();
        self
    }

    fn resolve(&mut self) {
        (self.resolved, self.failures) = resolve_logged(ResolveInputs {
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

    /// The in-memory `colour_theme` Preference, null for the default.
    pub fn colour_theme(&self) -> Option<&str> {
        self.colour_theme.as_deref()
    }

    /// The in-memory `colour_appearance` Preference, null for System.
    pub fn colour_appearance(&self) -> Option<ColourAppearance> {
        self.colour_appearance
    }

    /// The colours `colour_theme` would draw with the current Colour Appearance, terminal
    /// background and `[theme]` overrides, for the Settings swatches.
    pub fn preview(&self, colour_theme: &str) -> ResolvedColours {
        resolve(ResolveInputs {
            colour_theme: Some(colour_theme),
            colour_appearance: self.colour_appearance,
            overrides: &self.overrides,
            system: self.system,
        })
        .0
    }

    /// The pairs below their contrast rule in the colours drawn, for the Settings warnings.
    pub fn failures(&self) -> &[ContrastFailure] {
        &self.failures
    }

    pub fn overrides(&self) -> &ThemeOverrides {
        &self.overrides
    }

    /// The terminal's detected light/dark background, for the Settings System hint.
    pub fn system(&self) -> Option<ColourVariant> {
        self.system
    }

    /// Whether the `terminal_colours` Configuration switch is on, for the Settings note. A
    /// 16-colour terminal also draws terminal colours but is not named by that note.
    pub fn terminal_colours(&self) -> bool {
        self.terminal_colours
    }

    /// A Settings swatch cell drawn in `colour`; unstyled on a 16-colour terminal, which
    /// cannot show it.
    pub fn swatch(&self, colour: Rgba) -> Style {
        if self.depth == ColourDepth::Ansi16 {
            Style::default()
        } else {
            Style::default().fg(self.colour(colour))
        }
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

    /// Chart series `index` (0-based), repeating after the fifth. Terminal colours keep the
    /// series rule through the fallback table: red first, then the default foreground
    /// stepping to paler greys.
    pub fn chart_series(&self, index: usize) -> Color {
        if self.terminal() {
            const FALLBACK: [Color; 5] = [
                Color::Red,
                Color::Reset,
                Color::White,
                Color::Gray,
                Color::DarkGray,
            ];
            FALLBACK[index % FALLBACK.len()]
        } else {
            self.colour(self.resolved.chart_series_colour(index))
        }
    }

    /// The drawn text-entry caret `▌`: the `cursor` role, or red in terminal colours.
    pub fn cursor(&self) -> Style {
        if self.terminal() {
            Style::default().fg(Color::Red)
        } else {
            Style::default().fg(self.colour(self.resolved.roles.cursor))
        }
    }

    /// Negative values and destructive consequences on `background`: the `negative` text
    /// shade, or red in terminal colours.
    pub fn negative(&self) -> Style {
        if self.terminal() {
            Style::default().fg(Color::Red)
        } else {
            Style::default().fg(self.colour(self.resolved.text_shades.negative))
        }
    }

    /// [`negative`](Self::negative) on the [`selection`](Self::selection) fill, whose own
    /// text shade keeps the contrast rule there.
    pub fn negative_on_selection(&self) -> Style {
        if self.terminal() {
            Style::default().fg(Color::Red)
        } else {
            Style::default().fg(self.colour(self.resolved.selection.text_shades.negative))
        }
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

    /// The chosen option of a focused choice field: `background` on an `accent` fill, or red
    /// `REVERSED` in terminal colours. Unfocused, the chosen option is a plain
    /// [`selection`](Self::selection).
    pub fn accent_selection(&self) -> Style {
        if self.terminal() {
            Style::default()
                .fg(Color::Red)
                .add_modifier(Modifier::REVERSED)
        } else {
            Style::default()
                .fg(self.colour(self.resolved.roles.background))
                .bg(self.colour(self.resolved.roles.accent))
        }
    }

    /// A Toast's ground and text: `foreground` on chrome, or the terminal's defaults in
    /// terminal colours.
    pub fn toast(&self) -> Style {
        if self.terminal() {
            Style::default()
        } else {
            let toast = self.resolved.info_toast;
            Style::default()
                .fg(self.colour(toast.text))
                .bg(self.colour(toast.background))
        }
    }

    /// A Toast's border: `muted`, or the terminal's default colour in terminal colours.
    pub fn toast_border(&self) -> Style {
        if self.terminal() {
            Style::default()
        } else {
            Style::default().fg(self.colour(self.resolved.info_toast.border))
        }
    }

    /// A Toast Kind's leading bar and glyph: its mark shade (ADR-0026), or the fallback table's
    /// green and red in terminal colours, where Info keeps the default colour.
    pub fn toast_mark(&self, kind: ToastKind) -> Style {
        if self.terminal() {
            match kind {
                ToastKind::Info => Style::default(),
                ToastKind::Success => Style::default().fg(Color::Green),
                ToastKind::Warning | ToastKind::Error => Style::default().fg(Color::Red),
            }
        } else {
            let marks = self.resolved.toast_marks;
            let mark = match kind {
                ToastKind::Info => marks.info,
                ToastKind::Success => marks.success,
                ToastKind::Warning => marks.warning,
                ToastKind::Error => marks.error,
            };
            Style::default().fg(self.colour(mark))
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
fn resolve_logged(inputs: ResolveInputs<'_>) -> (ResolvedColours, Vec<ContrastFailure>) {
    let (resolved, failures) = resolve(inputs);
    for failure in &failures {
        tracing::warn!(%failure, "Colour Theme pair below its contrast rule");
    }
    (resolved, failures)
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
    fn system_follows_the_detected_terminal_background() {
        let colours = Colours::default().with_system(Some(ColourVariant::Light));
        assert_eq!(colours.resolved().variant, ColourVariant::Light);
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
        assert_eq!(colours.cursor(), Style::default().fg(Color::Red));
        assert_eq!(colours.negative(), Style::default().fg(Color::Red));
        assert_eq!(
            colours.accent_selection(),
            Style::default()
                .fg(Color::Red)
                .add_modifier(Modifier::REVERSED)
        );
        assert_eq!(colours.toast(), Style::default());
        assert_eq!(colours.toast_mark(ToastKind::Info), Style::default());
        assert_eq!(
            colours.toast_mark(ToastKind::Success),
            Style::default().fg(Color::Green)
        );
        assert_eq!(
            colours.toast_mark(ToastKind::Warning),
            Style::default().fg(Color::Red)
        );
        assert_eq!(
            colours.toast_mark(ToastKind::Error),
            Style::default().fg(Color::Red)
        );
    }

    #[test]
    fn cursor_and_negative_draw_their_own_roles_not_the_accent() {
        let colours = Colours::default();
        let resolved = colours.resolved();
        assert_eq!(
            colours.cursor().fg,
            Some(colours.colour(resolved.roles.cursor))
        );
        assert_eq!(
            colours.negative().fg,
            Some(colours.colour(resolved.text_shades.negative))
        );
        assert_ne!(colours.negative(), colours.accent());
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
