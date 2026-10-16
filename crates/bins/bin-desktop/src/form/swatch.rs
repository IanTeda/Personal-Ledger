//! A Tag's colour swatch: the Settings Tags table, the Transactions tag cells and the Tags
//! dialogs' colour field all draw it, so it lives with the shared form widgets.

use gpui::{Rgba, div, prelude::*, px};
use lib_core::HexColor;

/// The table swatch: `10×10`, `gap 8px` before the name.
pub const SWATCH_SIZE: gpui::Pixels = px(10.0);

/// A Tag colour as a `gpui` colour. It is the user's data, not a Colour Theme role, so it is drawn
/// as stored under every Colour Variant (#352).
pub fn swatch_colour(colour: &HexColor) -> Rgba {
    let (r, g, b) = colour.components();
    Rgba {
        r: f32::from(r) / 255.0,
        g: f32::from(g) / 255.0,
        b: f32::from(b) / 255.0,
        a: 1.0,
    }
}

/// A `size`-square swatch, or an empty slot of the same size for a colourless Tag so names stay
/// aligned (#352).
pub fn swatch(colour: Option<&HexColor>, size: gpui::Pixels) -> impl IntoElement {
    div()
        .flex_none()
        .size(size)
        .when_some(colour, |this, colour| this.bg(swatch_colour(colour)))
}
