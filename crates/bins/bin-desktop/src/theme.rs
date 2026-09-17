//! Fixed design tokens for the desktop shell chrome, from the Modernist design system
//! (`docs/ux/desktop/Shell & Navigation/README.md`'s "Design Tokens" table). These are
//! literal constants, not a themeable palette -- the handoff has no dark-mode variant and
//! states "introduce no values outside this set" as a system rule. `gpui-component`'s own
//! `ActiveTheme` stays scoped to the chart/table widgets it already renders (see ADR-0016);
//! shell code should reach for the constants here instead.
//!
//! Radius is deliberately not a constant here: the handoff's rule is `0` everywhere, no
//! exceptions, which is already `gpui`'s default for an unstyled `div` -- there's nothing to
//! encode, and adding a `RADIUS` constant would invite someone to eventually pass it a
//! non-zero value.

use gpui::{FontWeight, Pixels, Rgba, px};

/// Same conversion `gpui::rgb`/`gpui::rgba` do, redeclared as `const fn` so the palette below
/// can be real compile-time constants -- `gpui`'s own versions aren't `const fn`.
const fn rgb(hex: u32) -> Rgba {
    let bytes = hex.to_be_bytes();
    Rgba {
        r: bytes[1] as f32 / 255.0,
        g: bytes[2] as f32 / 255.0,
        b: bytes[3] as f32 / 255.0,
        a: 1.0,
    }
}

/// See [`rgb`]. `hex` is packed `0xRRGGBBAA`, matching `gpui::rgba`'s own convention.
const fn rgba(hex: u32) -> Rgba {
    let bytes = hex.to_be_bytes();
    Rgba {
        r: bytes[0] as f32 / 255.0,
        g: bytes[1] as f32 / 255.0,
        b: bytes[2] as f32 / 255.0,
        a: bytes[3] as f32 / 255.0,
    }
}

/// The Modernist palette. Every color the shell chrome may use, and nothing else.
pub mod color {
    use super::*;

    pub const GROUND: Rgba = rgb(0xf3f2f2);
    pub const CHROME: Rgba = rgb(0xeae9e9);
    pub const INSET_TRACK: Rgba = rgb(0xe2e0df);
    pub const INK: Rgba = rgb(0x201e1d);
    pub const INK_SECONDARY: Rgba = rgb(0x605d5d);
    pub const INK_TERTIARY: Rgba = rgb(0x9b9797);
    /// Ink on dark (inverted rows, e.g. a selected rail row).
    pub const INK_ON_DARK: Rgba = rgb(0xf3f2f2);
    pub const INK_ON_DARK_SECONDARY: Rgba = rgb(0xbab6b6);
    pub const INK_ON_DARK_TERTIARY: Rgba = rgb(0xd7d3d3);
    /// Hairline between rows within a rail.
    pub const HAIRLINE: Rgba = rgb(0xd7d3d3);
    /// The lighter hairline used for table rules.
    pub const HAIRLINE_LIGHT: Rgba = rgb(0xeae9e9);
    /// The 2px structural rule between shell bands/columns: `rgba(32,30,29,.38)`.
    pub const STRUCTURAL_RULE: Rgba = rgba(0x201e1d61);
    /// `Border`: `rgba(32,30,29,.30)` -- an unfocused input/button border (the handoffs'
    /// design-token tables list this role separately from [`Self::STRUCTURAL_RULE`]'s `.38`,
    /// even though earlier code duplicated its value as an inline `gpui::rgba(0x201e1d4d)`
    /// rather than a shared constant -- see `crate::explorer`'s own header/Cancel button).
    pub const BORDER: Rgba = rgba(0x201e1d4d);
    /// `Dimmer`: the same `rgba(32,30,29,.30)` value as [`Self::BORDER`], under its own name
    /// for the Settings dialog component's full-viewport scrim
    /// (`docs/ux/desktop/Settings/README.md`'s "Dimmer" row) -- today's other floating
    /// overlays (`crate::palette::Palette`, `crate::explorer::FileExplorer`) dim the shell by
    /// reducing its own opacity instead (`Shell::render`'s `content_opacity`), so this is the
    /// first real use of an overlay scrim in this crate.
    pub const DIMMER: Rgba = BORDER;
    /// The in-rail group divider: `rgba(32,30,29,.20)`.
    pub const RAIL_DIVIDER: Rgba = rgba(0x201e1d33);
    /// Row/result hover tint: `rgba(32,30,29,.06)`.
    pub const HOVER_TINT: Rgba = rgba(0x201e1d0f);

    /// Reserved for the primary action, the Reconcile badge, over-budget state, the block
    /// caret, matched substrings, the focus ring, and the COMMAND mode badge -- never a
    /// background field in the shell (the handoff's "Accent discipline").
    pub const ACCENT: Rgba = rgb(0xec3013);
    /// Accent for a matched substring on a dark (selected) row.
    pub const ACCENT_ON_DARK: Rgba = rgb(0xff9783);
    /// `ACCENT` doesn't clear 4.5:1 contrast on `GROUND` at body size and below -- use this
    /// instead for accent-colored *text* at 13px or smaller.
    pub const ACCENT_TEXT: Rgba = rgb(0xae1800);

    /// The command palette's own drop shadow: `0 12px 32px rgba(45,43,43,.30)` -- the handoff's
    /// "Shadows" section names this as the shell's one exception to "nothing else elevates," so
    /// it earns its own token rather than reusing `STRUCTURAL_RULE`'s different ink/alpha.
    pub const PALETTE_SHADOW: Rgba = rgba(0x2d2b2b4d);
    /// The "1e" file explorer dialog's own drop shadow: `0 16px 48px rgba(32,30,29,.40)`, named
    /// inline in the handoff's mockup markup rather than its "Shadows" table (that table
    /// predates the file explorer) -- a second named exception alongside `PALETTE_SHADOW`.
    pub const DIALOG_SHADOW: Rgba = rgba(0x201e1d66);
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
