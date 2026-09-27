//! Resolution (ADR-0023) and the calculated-colour rules (ADR-0022).
//!
//! Pure on purpose: no I/O and no logging. Each Client watches its own inputs, calls
//! [`resolve`] again on a change, and logs any [`ContrastFailure`] it gets back at `warn`.

use std::collections::BTreeMap;
use std::fmt;

use lib_core::HexColor;

use crate::colour::{Rgba, contrast_ratio, mix};
use crate::theme::{ColourAppearance, ColourRole, ColourTheme, ColourVariant, Palette};

// `foreground` over `background` at these opacities; the proportions the Modernist
// handoffs used, so the default Colour Theme draws as it did before Colour Themes.
const HOVER_OPACITY: f32 = 0.06;
const HAIRLINE_OPACITY: f32 = 0.13;
const RAIL_DIVIDER_OPACITY: f32 = 0.20;
const BORDER_OPACITY: f32 = 0.30;
const SCRIM_OPACITY: f32 = 0.30;
const STRUCTURAL_RULE_OPACITY: f32 = 0.38;
const DIVIDER_OPACITY: f32 = 0.40;
const PALETTE_SHADOW_OPACITY: f32 = 0.30;
const DIALOG_SHADOW_OPACITY: f32 = 0.40;

// `background` moved this far toward `foreground`.
const CHROME_STEP: f32 = 0.045;
const INSET_TRACK_STEP: f32 = 0.08;

/// The faint text tier sits this far from `muted` toward `background`.
const FAINT_TEXT_STEP: f32 = 0.5;

/// Where selected-row muted text starts, as a mix of `background` toward `foreground`,
/// before it is backed off to reach the text contrast rule on `foreground`.
const SELECTION_MUTED_START: f32 = 0.35;

/// `accent` at this opacity over `background` for the accent tint (the "base" flag pill).
const ACCENT_TINT_OPACITY: f32 = 0.08;

/// Contrast every non-text mark must reach against `background`.
pub const NON_TEXT_CONTRAST: f32 = 3.0;

/// The number of distinct chart series colours; a 6th series or later repeats the pattern.
pub const CHART_SERIES_COUNT: usize = 5;

/// Search resolution for the colours that are backed off or corrected until they pass.
const SEARCH_STEPS: u16 = 100;

/// `[theme]` role overrides, as `lib-config` parses them: bare keys apply to both Colour
/// Variants, and `[theme.light]`/`[theme.dark]` keys to one, winning over the bare key.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ThemeOverrides {
    pub both: BTreeMap<ColourRole, HexColor>,
    pub light: BTreeMap<ColourRole, HexColor>,
    pub dark: BTreeMap<ColourRole, HexColor>,
}

impl ThemeOverrides {
    /// The override for `role` in `variant`, the variant-specific key first.
    pub fn get(&self, variant: ColourVariant, role: ColourRole) -> Option<&HexColor> {
        let specific = match variant {
            ColourVariant::Light => &self.light,
            ColourVariant::Dark => &self.dark,
        };
        specific.get(&role).or_else(|| self.both.get(&role))
    }

    /// The roles overridden in `variant`, for the Settings note.
    pub fn roles(&self, variant: ColourVariant) -> Vec<ColourRole> {
        ColourRole::ALL
            .into_iter()
            .filter(|role| self.get(variant, *role).is_some())
            .collect()
    }

    pub fn is_empty(&self) -> bool {
        self.both.is_empty() && self.light.is_empty() && self.dark.is_empty()
    }
}

/// Everything [`resolve`] depends on.
#[derive(Debug, Clone, Copy)]
pub struct ResolveInputs<'a> {
    /// The `colour_theme` Preference. Null, or an id this release doesn't know, draws the
    /// default Colour Theme; check [`ColourTheme::by_id`] to log the unknown case.
    pub colour_theme: Option<&'a str>,
    /// The `colour_appearance` Preference. Null means System.
    pub colour_appearance: Option<ColourAppearance>,
    pub overrides: &'a ThemeOverrides,
    /// The OS or terminal light/dark setting, if one was detected.
    pub system: Option<ColourVariant>,
}

/// Text and non-text colours in their text-shade form: each role corrected until it reaches
/// the text contrast rule against the surface it sits on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextShades {
    pub accent: Rgba,
    pub positive: Rgba,
    pub negative: Rgba,
}

/// The current/selected row: `foreground` and `background` swapped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Selection {
    pub background: Rgba,
    pub text: Rgba,
    pub muted: Rgba,
    pub text_shades: TextShades,
}

/// The info toast: chrome ground, `foreground` text, `muted` border and a thin `foreground`
/// bar on its leading edge.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InfoToast {
    pub background: Rgba,
    pub text: Rgba,
    pub border: Rgba,
    pub bar: Rgba,
}

/// The seven resolved Colour Roles plus every calculated colour. A Client converts what it
/// can draw and ignores the rest (the TUI has no shadows or hover).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResolvedColours {
    /// The Colour Theme drawn: the default when the Preference is null or unknown.
    pub theme_id: &'static str,
    /// The Colour Variant drawn, for "System (currently Dark)".
    pub variant: ColourVariant,
    /// The seven stored Colour Roles, with `[theme]` overrides applied.
    pub roles: Palette,

    // Translucent `foreground`; flatten with `Rgba::over(roles.background)` where needed.
    pub hover: Rgba,
    pub hairline: Rgba,
    pub rail_divider: Rgba,
    pub border: Rgba,
    pub structural_rule: Rgba,
    pub divider: Rgba,
    pub scrim: Rgba,
    pub palette_shadow: Rgba,
    pub dialog_shadow: Rgba,

    /// Panel bands, status line, disabled fields.
    pub chrome: Rgba,
    pub inset_track: Rgba,
    /// Placeholders, jump keys, meta.
    pub faint_text: Rgba,

    /// Text shades on `background`.
    pub text_shades: TextShades,
    pub selection: Selection,
    pub accent_tint: Rgba,
    pub accent_tint_text: Rgba,
    pub info_toast: InfoToast,
    /// Series 1 is `accent`; 2–5 step from `foreground` to the palest mix reaching 3:1.
    pub chart_series: [Rgba; CHART_SERIES_COUNT],
}

impl ResolvedColours {
    /// The colour for chart series `index` (0-based), repeating after the fifth.
    pub fn chart_series_colour(&self, index: usize) -> Rgba {
        self.chart_series[index % CHART_SERIES_COUNT]
    }
}

/// A pair that appears on screen below its contrast rule. Only a `[theme]` override can
/// cause one: every built-in Colour Theme passes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ContrastFailure {
    pub subject: &'static str,
    pub surface: &'static str,
    pub ratio: f32,
    pub required: f32,
}

impl fmt::Display for ContrastFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} on {} is {:.1}:1, needs {}:1",
            self.subject, self.surface, self.ratio, self.required
        )
    }
}

/// Resolves the colours to draw: the Colour Theme from its Preference, the Colour Variant
/// from the Colour Appearance, `[theme]` overrides over that variant, then every calculated
/// colour. Failing pairs are still used and returned as data, never rejected.
pub fn resolve(inputs: ResolveInputs<'_>) -> (ResolvedColours, Vec<ContrastFailure>) {
    let theme = inputs
        .colour_theme
        .and_then(ColourTheme::by_id)
        .unwrap_or_else(|| ColourTheme::default_theme());
    let variant = inputs
        .colour_appearance
        .unwrap_or_default()
        .variant(inputs.system);

    let mut roles = theme.palette(variant);
    for role in ColourRole::ALL {
        if let Some(hex) = inputs.overrides.get(variant, role) {
            let (r, g, b) = hex.components();
            roles.set(role, Rgba::rgb(r, g, b));
        }
    }

    let colours = calculate(theme.id, variant, roles, theme.text_contrast());
    let failures = check_contrast(&colours, theme.text_contrast());
    (colours, failures)
}

fn calculate(
    theme_id: &'static str,
    variant: ColourVariant,
    roles: Palette,
    text_contrast: f32,
) -> ResolvedColours {
    let Palette {
        foreground: fg,
        background: bg,
        accent,
        muted,
        positive,
        negative,
        ..
    } = roles;

    let shades_on = |surface: Rgba| TextShades {
        accent: text_shade(accent, surface, text_contrast),
        positive: text_shade(positive, surface, text_contrast),
        negative: text_shade(negative, surface, text_contrast),
    };

    // Back off from the starting mix toward `background` until it reads on `foreground`.
    let selection_muted = (0..=steps_in(SELECTION_MUTED_START))
        .rev()
        .map(|step| mix(bg, fg, step_amount(step)))
        .find(|c| contrast_ratio(*c, fg) >= text_contrast)
        .unwrap_or(bg);

    let chrome = mix(bg, fg, CHROME_STEP);
    let accent_tint = accent.with_opacity(ACCENT_TINT_OPACITY).over(bg);

    ResolvedColours {
        theme_id,
        variant,
        roles,
        hover: fg.with_opacity(HOVER_OPACITY),
        hairline: fg.with_opacity(HAIRLINE_OPACITY),
        rail_divider: fg.with_opacity(RAIL_DIVIDER_OPACITY),
        border: fg.with_opacity(BORDER_OPACITY),
        structural_rule: fg.with_opacity(STRUCTURAL_RULE_OPACITY),
        divider: fg.with_opacity(DIVIDER_OPACITY),
        scrim: fg.with_opacity(SCRIM_OPACITY),
        palette_shadow: fg.with_opacity(PALETTE_SHADOW_OPACITY),
        dialog_shadow: fg.with_opacity(DIALOG_SHADOW_OPACITY),
        chrome,
        inset_track: mix(bg, fg, INSET_TRACK_STEP),
        faint_text: mix(muted, bg, FAINT_TEXT_STEP),
        text_shades: shades_on(bg),
        selection: Selection {
            background: fg,
            text: bg,
            muted: selection_muted,
            text_shades: shades_on(fg),
        },
        accent_tint,
        accent_tint_text: text_shade(accent, accent_tint, text_contrast),
        info_toast: InfoToast {
            background: chrome,
            text: fg,
            border: muted,
            bar: fg,
        },
        chart_series: chart_series(accent, fg, bg),
    }
}

/// `colour` darkened or lightened, whichever gains contrast on `surface`, until it reaches
/// `target`. Unchanged if it already does.
fn text_shade(colour: Rgba, surface: Rgba, target: f32) -> Rgba {
    // Above this luminance black out-contrasts white, so darkening is the way to go.
    const BLACK_WHITE_CROSSOVER: f32 = 0.179;
    let toward = if surface.relative_luminance() > BLACK_WHITE_CROSSOVER {
        Rgba::BLACK
    } else {
        Rgba::WHITE
    };
    (0..=SEARCH_STEPS)
        .map(|step| mix(colour, toward, step_amount(step)))
        .find(|c| contrast_ratio(*c, surface) >= target)
        .unwrap_or(toward)
}

fn chart_series(accent: Rgba, fg: Rgba, bg: Rgba) -> [Rgba; CHART_SERIES_COUNT] {
    // The palest `foreground`-over-`background` mix that still reaches 3:1.
    let palest = (0..=SEARCH_STEPS)
        .map(step_amount)
        .find(|amount| contrast_ratio(mix(fg, bg, *amount), bg) < NON_TEXT_CONTRAST)
        .map_or(1.0, |first_failing| {
            (first_failing - 1.0 / f32::from(SEARCH_STEPS)).max(0.0)
        });
    let last = (CHART_SERIES_COUNT - 2) as f32;
    let mut series = [accent; CHART_SERIES_COUNT];
    for (i, slot) in series.iter_mut().enumerate().skip(1) {
        *slot = mix(fg, bg, palest * (i - 1) as f32 / last);
    }
    series
}

fn step_amount(step: u16) -> f32 {
    f32::from(step) / f32::from(SEARCH_STEPS)
}

// Rounded down so the search never starts past the proportion it names.
fn steps_in(amount: f32) -> u16 {
    (amount * f32::from(SEARCH_STEPS)) as u16
}

fn check_contrast(c: &ResolvedColours, text: f32) -> Vec<ContrastFailure> {
    let Palette {
        foreground,
        background,
        accent,
        cursor,
        muted,
        positive,
        negative,
    } = c.roles;
    let selected = &c.selection;
    let mut pairs: Vec<(&'static str, Rgba, &'static str, Rgba, f32)> = vec![
        ("foreground", foreground, "background", background, text),
        ("muted", muted, "background", background, text),
        (
            "accent text",
            c.text_shades.accent,
            "background",
            background,
            text,
        ),
        (
            "positive text",
            c.text_shades.positive,
            "background",
            background,
            text,
        ),
        (
            "negative text",
            c.text_shades.negative,
            "background",
            background,
            text,
        ),
        (
            "muted",
            selected.muted,
            "selected row",
            selected.background,
            text,
        ),
        (
            "accent text",
            selected.text_shades.accent,
            "selected row",
            selected.background,
            text,
        ),
        (
            "positive text",
            selected.text_shades.positive,
            "selected row",
            selected.background,
            text,
        ),
        (
            "negative text",
            selected.text_shades.negative,
            "selected row",
            selected.background,
            text,
        ),
        (
            "accent text",
            c.accent_tint_text,
            "accent tint",
            c.accent_tint,
            text,
        ),
        (
            "accent",
            accent,
            "background",
            background,
            NON_TEXT_CONTRAST,
        ),
        (
            "cursor",
            cursor,
            "background",
            background,
            NON_TEXT_CONTRAST,
        ),
        (
            "positive",
            positive,
            "background",
            background,
            NON_TEXT_CONTRAST,
        ),
        (
            "negative",
            negative,
            "background",
            background,
            NON_TEXT_CONTRAST,
        ),
    ];
    const SERIES_NAMES: [&str; CHART_SERIES_COUNT - 1] = [
        "chart series 2",
        "chart series 3",
        "chart series 4",
        "chart series 5",
    ];
    for (name, colour) in SERIES_NAMES.iter().zip(&c.chart_series[1..]) {
        pairs.push((name, *colour, "background", background, NON_TEXT_CONTRAST));
    }

    pairs
        .into_iter()
        .filter_map(|(subject, colour, surface, ground, required)| {
            let ratio = contrast_ratio(colour, ground);
            (ratio < required).then_some(ContrastFailure {
                subject,
                surface,
                ratio,
                required,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn inputs<'a>(
        theme: &'a str,
        appearance: ColourAppearance,
        overrides: &'a ThemeOverrides,
    ) -> ResolveInputs<'a> {
        ResolveInputs {
            colour_theme: Some(theme),
            colour_appearance: Some(appearance),
            overrides,
            system: None,
        }
    }

    fn hex(s: &str) -> HexColor {
        HexColor::parse(s).unwrap()
    }

    #[test]
    fn every_built_in_theme_passes_contrast_in_both_variants() {
        let none = ThemeOverrides::default();
        for theme in ColourTheme::built_in() {
            for appearance in [ColourAppearance::Light, ColourAppearance::Dark] {
                let (colours, failures) = resolve(inputs(theme.id, appearance, &none));
                assert_eq!(colours.theme_id, theme.id);
                assert!(
                    failures.is_empty(),
                    "{} {appearance:?}: {failures:#?}",
                    theme.id
                );
            }
        }
    }

    #[test]
    fn high_contrast_text_reaches_seven_to_one() {
        let none = ThemeOverrides::default();
        for appearance in [ColourAppearance::Light, ColourAppearance::Dark] {
            let (c, _) = resolve(inputs("high_contrast", appearance, &none));
            for text in [c.roles.muted, c.text_shades.accent, c.text_shades.negative] {
                assert!(contrast_ratio(text, c.roles.background) >= 7.0);
            }
        }
    }

    #[test]
    fn null_or_unknown_theme_draws_modernist_and_null_appearance_is_system() {
        let none = ThemeOverrides::default();
        let null = ResolveInputs {
            colour_theme: None,
            colour_appearance: None,
            overrides: &none,
            system: None,
        };
        let (colours, _) = resolve(null);
        assert_eq!(colours.theme_id, "modernist");
        assert_eq!(colours.variant, ColourVariant::Dark);

        let (colours, _) = resolve(ResolveInputs {
            colour_theme: Some("from_a_newer_release"),
            system: Some(ColourVariant::Light),
            ..null
        });
        assert_eq!(colours.theme_id, "modernist");
        assert_eq!(colours.variant, ColourVariant::Light);
    }

    #[test]
    fn overrides_replace_only_the_roles_named() {
        let overrides = ThemeOverrides {
            both: BTreeMap::from([(ColourRole::Accent, hex("#00cc00"))]),
            ..Default::default()
        };
        let none = ThemeOverrides::default();
        for appearance in [ColourAppearance::Light, ColourAppearance::Dark] {
            let (base, _) = resolve(inputs("nord", appearance, &none));
            let (over, _) = resolve(inputs("nord", appearance, &overrides));
            for role in ColourRole::ALL {
                if role == ColourRole::Accent {
                    assert_eq!(over.roles.get(role), Rgba::rgb(0, 0xcc, 0));
                } else {
                    assert_eq!(over.roles.get(role), base.roles.get(role), "{role:?}");
                }
            }
        }
    }

    #[test]
    fn variant_section_applies_to_one_variant_and_wins_over_bare_key() {
        let overrides = ThemeOverrides {
            both: BTreeMap::from([(ColourRole::Accent, hex("#cc0000"))]),
            dark: BTreeMap::from([
                (ColourRole::Accent, hex("#ff6666")),
                (ColourRole::Muted, hex("aaaaaa")),
            ]),
            ..Default::default()
        };
        let none = ThemeOverrides::default();
        let (light, _) = resolve(inputs("modernist", ColourAppearance::Light, &overrides));
        let (dark, _) = resolve(inputs("modernist", ColourAppearance::Dark, &overrides));
        let (base_light, _) = resolve(inputs("modernist", ColourAppearance::Light, &none));

        assert_eq!(light.roles.accent, Rgba::rgb(0xcc, 0, 0));
        assert_eq!(light.roles.muted, base_light.roles.muted);
        assert_eq!(dark.roles.accent, Rgba::rgb(0xff, 0x66, 0x66));
        assert_eq!(dark.roles.muted, Rgba::rgb(0xaa, 0xaa, 0xaa));
        assert_eq!(overrides.roles(ColourVariant::Light), [ColourRole::Accent]);
    }

    #[test]
    fn failing_override_is_used_and_reported() {
        let overrides = ThemeOverrides {
            both: BTreeMap::from([
                (ColourRole::Accent, hex("#f0f0f0")),
                (ColourRole::Muted, hex("#e0e0e0")),
            ]),
            ..Default::default()
        };
        let (colours, failures) = resolve(inputs("modernist", ColourAppearance::Light, &overrides));
        assert_eq!(colours.roles.accent, Rgba::rgb(0xf0, 0xf0, 0xf0));
        let accent = failures.iter().find(|f| f.subject == "accent").unwrap();
        assert_eq!(accent.surface, "background");
        assert_eq!(accent.required, NON_TEXT_CONTRAST);
        assert!(accent.to_string().starts_with("accent on background is 1."));
        assert!(
            failures
                .iter()
                .any(|f| f.subject == "muted" && f.surface == "background")
        );
    }

    #[test]
    fn pale_accent_still_gets_readable_accent_text() {
        let overrides = ThemeOverrides {
            both: BTreeMap::from([(ColourRole::Accent, hex("#ffd0c0"))]),
            ..Default::default()
        };
        let (c, failures) = resolve(inputs("modernist", ColourAppearance::Light, &overrides));
        assert!(contrast_ratio(c.text_shades.accent, c.roles.background) >= 4.5);
        assert!(!failures.iter().any(|f| f.subject == "accent text"));
    }

    #[test]
    fn calculated_colours_follow_their_rules() {
        let none = ThemeOverrides::default();
        let (c, _) = resolve(inputs("modernist", ColourAppearance::Light, &none));
        let Palette {
            foreground: fg,
            background: bg,
            accent,
            muted,
            ..
        } = c.roles;

        assert_eq!(c.border, Rgba { a: 77, ..fg });
        assert_eq!(c.hover, Rgba { a: 15, ..fg });
        assert_eq!(c.chrome, mix(bg, fg, CHROME_STEP));
        assert_eq!(c.faint_text, mix(muted, bg, 0.5));
        assert_eq!((c.selection.background, c.selection.text), (fg, bg));
        assert_eq!(c.info_toast.border, muted);

        assert_eq!(c.chart_series[0], accent);
        assert_eq!(c.chart_series[1], fg);
        assert_eq!(c.chart_series_colour(5), accent);
        let palest = c.chart_series[4];
        assert!(contrast_ratio(palest, bg) >= NON_TEXT_CONTRAST);
        assert!(contrast_ratio(mix(palest, bg, 0.05), bg) < NON_TEXT_CONTRAST);
    }
}
