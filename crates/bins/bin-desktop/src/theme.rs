//! Design tokens for the desktop shell chrome. Colours come from the resolved Colour Theme
//! (`color`, ADR-0025); type scale and spacing stay fixed by the Modernist design system
//! (`docs/ux/desktop/01-shell/README.md`'s "Design Tokens" table), as a Colour
//! Theme doesn't carry them (`docs/colour-themes-design.md` "Not part of a Colour Theme").
//!
//! Radius is deliberately not a constant here: the handoff's rule is `0` everywhere, no
//! exceptions, which is already `gpui`'s default for an unstyled `div` -- there's nothing to
//! encode, and adding a `RADIUS` constant would invite someone to eventually pass it a
//! non-zero value.

use gpui::{FontWeight, Pixels, px};

/// The resolved Colour Theme's colours, one accessor per Colour Role and calculated colour
/// (`docs/colour-themes-design.md`), read from the `crate::colours::Colours` Global.
pub mod color {
    use gpui::{App, Rgba};
    use lib_colour_theme::ResolvedColours;

    use crate::colours::{resolved, to_gpui};

    macro_rules! accessors {
        ($($(#[$doc:meta])* $name:ident => |$c:ident| $value:expr;)*) => {
            $(
                $(#[$doc])*
                pub fn $name(cx: &App) -> Rgba {
                    let $c: &ResolvedColours = resolved(cx);
                    to_gpui($value)
                }
            )*
        };
    }

    // Translucent `foreground` colours stay translucent, so they draw over any surface.
    accessors! {
        foreground => |c| c.roles.foreground;
        background => |c| c.roles.background;
        /// The primary action, focus ring, matched substrings and other emphasis; never an
        /// unconditional background field ("Accent discipline").
        accent => |c| c.roles.accent;
        /// The text-entry caret only.
        cursor => |c| c.roles.cursor;
        /// Secondary text.
        muted => |c| c.roles.muted;
        positive => |c| c.roles.positive;
        negative => |c| c.roles.negative;
        hover => |c| c.hover;
        /// Hairlines between rows and table rules.
        hairline => |c| c.hairline;
        rail_divider => |c| c.rail_divider;
        /// An unfocused input or button border.
        border => |c| c.border;
        /// The 2px rule between shell bands and columns.
        structural_rule => |c| c.structural_rule;
        /// The segmented control's divider.
        divider => |c| c.divider;
        /// A dialog's full-viewport dimmer.
        scrim => |c| c.scrim;
        palette_shadow => |c| c.palette_shadow;
        dialog_shadow => |c| c.dialog_shadow;
        /// Panel bands, status line, disabled fields.
        chrome => |c| c.chrome;
        inset_track => |c| c.inset_track;
        /// Placeholders, jump keys, meta.
        faint_text => |c| c.faint_text;
        /// `accent` as text on `background`, at the text contrast rule.
        accent_text => |c| c.text_shades.accent;
        positive_text => |c| c.text_shades.positive;
        negative_text => |c| c.text_shades.negative;
        /// The selected row's fill.
        selection_background => |c| c.selection.background;
        selection_text => |c| c.selection.text;
        selection_muted => |c| c.selection.muted;
        selection_accent_text => |c| c.selection.text_shades.accent;
        selection_positive_text => |c| c.selection.text_shades.positive;
        selection_negative_text => |c| c.selection.text_shades.negative;
        /// The accent tint background (the Units "base" flag pill).
        accent_tint => |c| c.accent_tint;
        accent_tint_text => |c| c.accent_tint_text;
        info_toast_background => |c| c.info_toast.background;
        info_toast_text => |c| c.info_toast.text;
        info_toast_border => |c| c.info_toast.border;
        info_toast_bar => |c| c.info_toast.bar;
    }

    /// A Toast Kind's leading bar and glyph: its role corrected to 3:1 against chrome (ADR-0026).
    pub fn toast_mark(kind: lib_toast::ToastKind, cx: &App) -> Rgba {
        let marks = resolved(cx).toast_marks;
        to_gpui(match kind {
            lib_toast::ToastKind::Info => marks.info,
            lib_toast::ToastKind::Success => marks.success,
            lib_toast::ToastKind::Warning => marks.warning,
            lib_toast::ToastKind::Error => marks.error,
        })
    }

    /// Chart series `index` (0-based), repeating after the fifth.
    pub fn chart_series(index: usize, cx: &App) -> Rgba {
        to_gpui(resolved(cx).chart_series_colour(index))
    }
}

/// Type scale. Archivo throughout, bundled offline (see `crate::main`'s `add_fonts` call) --
/// weights are `gpui::FontWeight`s so they plug directly into `Styled::font_weight`.
pub mod type_scale {
    use super::*;

    pub const FONT_FAMILY: &str = "Archivo";

    pub const WEIGHT_REGULAR: FontWeight = FontWeight::NORMAL;
    pub const WEIGHT_SEMIBOLD: FontWeight = FontWeight::SEMIBOLD;
    /// The handoff's "800" weight.
    pub const WEIGHT_BOLD: FontWeight = FontWeight::EXTRA_BOLD;

    pub const HERO_FIGURE: Pixels = px(38.0);
    pub const VIEW_TITLE: Pixels = px(19.0);
    pub const PALETTE_QUERY: Pixels = px(15.0);
    pub const BODY: Pixels = px(13.0);
    pub const INPUT: Pixels = px(12.5);
    pub const META: Pixels = px(11.5);
    pub const JUMP_KEY: Pixels = px(11.0);
    pub const SECTION_KICKER: Pixels = px(10.0);

    /// `letter-spacing: .11em` on section kickers -- an em multiplier, not a `Pixels` value,
    /// so it's resolved against whichever size it's applied to (`SECTION_KICKER` today)
    /// rather than being a fixed length on its own.
    pub const SECTION_KICKER_LETTER_SPACING_EM: f32 = 0.11;
}

/// Spacing. Fixed-pixel padding and gaps for the shell chrome's own bands and rows -- not a
/// general-purpose spacing scale, just the values the handoff names.
pub mod spacing {
    use super::*;

    /// Rail row padding: `7px 14px` (vertical, horizontal).
    pub const RAIL_ROW: (Pixels, Pixels) = (px(7.0), px(14.0));
    /// Context-rail row padding: `9px 14px`.
    pub const CONTEXT_RAIL_ROW: (Pixels, Pixels) = (px(9.0), px(14.0));
    /// Command palette row padding: `8px 16px`.
    pub const PALETTE_ROW: (Pixels, Pixels) = (px(8.0), px(16.0));

    /// View padding: `20px 24px 0` (top, left/right, bottom).
    pub const VIEW_PADDING_TOP: Pixels = px(20.0);
    pub const VIEW_PADDING_X: Pixels = px(24.0);
    pub const VIEW_PADDING_BOTTOM: Pixels = px(0.0);

    /// Status line band gap.
    pub const STATUS_GAP: Pixels = px(14.0);
    /// Top bar band gap.
    pub const TOPBAR_GAP: Pixels = px(12.0);

    /// Dashboard figure-row gaps: the primary figure's own gap, then the secondary-figures
    /// group's gap.
    pub const FIGURE_GAP_PRIMARY: Pixels = px(36.0);
    pub const FIGURE_GAP_SECONDARY: Pixels = px(28.0);

    /// General section gaps.
    pub const SECTION_GAP_SMALL: Pixels = px(24.0);
    pub const SECTION_GAP_LARGE: Pixels = px(40.0);
}
