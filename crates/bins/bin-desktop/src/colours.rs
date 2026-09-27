//! The resolved Colour Theme as a `gpui` Global (ADR-0025). Views read it through
//! `crate::theme::color`'s accessors; a change to either Preference re-resolves, replaces the
//! Global, writes the result into `gpui_component::Theme` and redraws every window.
//!
//! The `colour_theme` and `colour_appearance` Preferences live here in memory until the
//! Desktop reads `preferences` from the database; null means Modernist and System.
//!
//! System follows the OS light/dark setting as `gpui` reports it: read once at start, then
//! re-resolved live through the main window's appearance observer (`set_system`).

use gpui::{App, Global, Rgba, WindowAppearance};
use gpui_component::theme::{Theme, ThemeMode};
use lib_colour_theme::{
    ColourAppearance, ColourVariant, ResolveInputs, ResolvedColours, ThemeOverrides, resolve,
};

/// The Global: the colours drawn plus the inputs they were resolved from.
pub struct Colours {
    resolved: ResolvedColours,
    colour_theme: Option<String>,
    colour_appearance: Option<ColourAppearance>,
    overrides: ThemeOverrides,
    system: Option<ColourVariant>,
}

impl Global for Colours {}

impl Colours {
    fn resolve(&mut self) {
        self.resolved = resolve_logged(ResolveInputs {
            colour_theme: self.colour_theme.as_deref(),
            colour_appearance: self.colour_appearance,
            overrides: &self.overrides,
            system: self.system,
        });
    }

    pub fn resolved(&self) -> &ResolvedColours {
        &self.resolved
    }

    pub fn colour_theme(&self) -> Option<&str> {
        self.colour_theme.as_deref()
    }

    pub fn colour_appearance(&self) -> Option<ColourAppearance> {
        self.colour_appearance
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

/// The OS light/dark setting as a Colour Variant. `gpui` always reports one, so System on the
/// Desktop is never "not detected"; the vibrant macOS appearances are still light or dark.
fn system_variant(appearance: WindowAppearance) -> ColourVariant {
    match appearance {
        WindowAppearance::Light | WindowAppearance::VibrantLight => ColourVariant::Light,
        WindowAppearance::Dark | WindowAppearance::VibrantDark => ColourVariant::Dark,
    }
}

/// Installs the Global with null Preferences and the OS setting at start. Call after
/// `gpui_component::init`, which the component theme write-through needs in place.
pub fn init(overrides: ThemeOverrides, cx: &mut App) {
    let system = Some(system_variant(cx.window_appearance()));
    let resolved = resolve_logged(ResolveInputs {
        colour_theme: None,
        colour_appearance: None,
        overrides: &overrides,
        system,
    });
    cx.set_global(Colours {
        resolved,
        colour_theme: None,
        colour_appearance: None,
        overrides,
        system,
    });
    apply_to_component_theme(cx);
}

/// Records a change to the OS light/dark setting. Re-resolves only when it changed, since the
/// observer also fires for the initial report (the Linux portal answers after start).
pub fn set_system(appearance: WindowAppearance, cx: &mut App) {
    let system = Some(system_variant(appearance));
    if cx.global::<Colours>().system != system {
        update(cx, |colours| colours.system = system);
    }
}

/// Sets the in-memory `colour_theme` Preference and redraws.
pub fn set_colour_theme(colour_theme: Option<String>, cx: &mut App) {
    update(cx, |colours| colours.colour_theme = colour_theme);
}

/// Sets the in-memory `colour_appearance` Preference and redraws.
pub fn set_colour_appearance(colour_appearance: Option<ColourAppearance>, cx: &mut App) {
    update(cx, |colours| colours.colour_appearance = colour_appearance);
}

fn update(cx: &mut App, change: impl FnOnce(&mut Colours)) {
    let colours = cx.global_mut::<Colours>();
    change(colours);
    colours.resolve();
    apply_to_component_theme(cx);
    cx.refresh_windows();
}

/// The resolved colours, for `crate::theme::color`'s accessors.
pub fn resolved(cx: &App) -> &ResolvedColours {
    cx.global::<Colours>().resolved()
}

/// Converts the shared crate's RGBA into `gpui`'s. The one place a Colour Theme value
/// becomes a `gpui` colour.
pub fn to_gpui(colour: lib_colour_theme::Rgba) -> Rgba {
    Rgba {
        r: f32::from(colour.r) / 255.0,
        g: f32::from(colour.g) / 255.0,
        b: f32::from(colour.b) / 255.0,
        a: f32::from(colour.a) / 255.0,
    }
}

/// Writes the resolved colours into `gpui_component::Theme` so the charts and tables it
/// draws follow the Colour Theme and its Colour Variant.
fn apply_to_component_theme(cx: &mut App) {
    let c = *resolved(cx);
    let hsla = |colour: lib_colour_theme::Rgba| to_gpui(colour).into();
    let bg = c.roles.background;
    let theme = Theme::global_mut(cx);
    theme.mode = match c.variant {
        ColourVariant::Light => ThemeMode::Light,
        ColourVariant::Dark => ThemeMode::Dark,
    };
    let colors = &mut theme.colors;
    colors.background = hsla(bg);
    colors.foreground = hsla(c.roles.foreground);
    colors.border = hsla(c.border.over(bg));
    colors.muted = hsla(c.chrome);
    colors.muted_foreground = hsla(c.roles.muted);
    colors.accent = hsla(c.hover.over(bg));
    colors.accent_foreground = hsla(c.roles.foreground);
    colors.primary = hsla(c.roles.accent);
    colors.primary_foreground = hsla(bg);
    colors.ring = hsla(c.roles.accent);
    colors.caret = hsla(c.roles.cursor);
    colors.selection = hsla(c.selection.background.with_opacity(0.2).over(bg));
    colors.success = hsla(c.roles.positive);
    colors.danger = hsla(c.roles.negative);
    colors.bullish = hsla(c.roles.positive);
    colors.bearish = hsla(c.roles.negative);
    colors.table = hsla(bg);
    colors.table_head = hsla(c.chrome);
    colors.table_head_foreground = hsla(c.roles.muted);
    colors.table_hover = hsla(c.hover.over(bg));
    colors.table_active = hsla(c.selection.background);
    colors.table_active_border = hsla(c.roles.accent);
    colors.table_even = hsla(bg);
    colors.table_row_border = hsla(c.hairline.over(bg));
    colors.popover = hsla(bg);
    colors.popover_foreground = hsla(c.roles.foreground);
    colors.input = hsla(c.border.over(bg));
    colors.chart_1 = hsla(c.chart_series_colour(0));
    colors.chart_2 = hsla(c.chart_series_colour(1));
    colors.chart_3 = hsla(c.chart_series_colour(2));
    colors.chart_4 = hsla(c.chart_series_colour(3));
    colors.chart_5 = hsla(c.chart_series_colour(4));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn colours() -> Colours {
        let overrides = ThemeOverrides::default();
        Colours {
            resolved: resolve_logged(ResolveInputs {
                colour_theme: None,
                colour_appearance: None,
                overrides: &overrides,
                system: Some(ColourVariant::Light),
            }),
            colour_theme: None,
            colour_appearance: None,
            overrides,
            system: Some(ColourVariant::Light),
        }
    }

    #[test]
    fn null_preferences_resolve_to_modernist_light() {
        let colours = colours();
        assert_eq!(colours.resolved().theme_id, "modernist");
        assert_eq!(colours.resolved().variant, ColourVariant::Light);
    }

    #[test]
    fn changing_a_preference_re_resolves() {
        let mut colours = colours();
        colours.colour_theme = Some("nord".to_string());
        colours.colour_appearance = Some(ColourAppearance::Dark);
        colours.resolve();
        assert_eq!(colours.resolved().theme_id, "nord");
        assert_eq!(colours.resolved().variant, ColourVariant::Dark);
    }

    #[test]
    fn a_system_change_re_resolves_only_under_system() {
        let mut colours = colours();
        colours.system = Some(system_variant(WindowAppearance::VibrantDark));
        colours.resolve();
        assert_eq!(colours.resolved().variant, ColourVariant::Dark);

        colours.colour_appearance = Some(ColourAppearance::Light);
        colours.resolve();
        assert_eq!(colours.resolved().variant, ColourVariant::Light);
    }

    #[test]
    fn to_gpui_scales_each_channel() {
        let colour = to_gpui(lib_colour_theme::Rgba {
            r: 255,
            g: 0,
            b: 51,
            a: 255,
        });
        assert_eq!(
            colour,
            Rgba {
                r: 1.0,
                g: 0.0,
                b: 0.2,
                a: 1.0
            }
        );
    }
}
