//! The Tags dialogs and the small pieces Settings' Tags page (2j, drawn by `view::settings::tags`)
//! shares with them: the colour swatch and the likely-duplicate count. The standalone 7a page left
//! the primary rail for Settings (#420).

pub mod add_dialog;
pub mod colour_field;
pub mod edit_dialog;
pub mod merge_dialog;
pub mod remove_dialog;
pub mod state;

use std::rc::Rc;

use gpui::{App, Rgba, Window, div, prelude::*, px};
use lib_core::HexColor;

use crate::tags::DuplicateGroup;

/// Called with a Tag's [`Tag::id`].
pub type OnTagClick = Rc<dyn Fn(u32, &mut Window, &mut App)>;
pub type OnPlainClick = Rc<dyn Fn(&mut Window, &mut App)>;

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

/// How many Tags are flagged as likely duplicates: every group member but its suggested target.
pub fn likely_duplicate_count(groups: &[DuplicateGroup]) -> usize {
    groups.iter().map(|group| group.duplicates.len()).sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_non_target_members_count_as_likely_duplicates() {
        let groups = [
            DuplicateGroup {
                target: 1,
                duplicates: vec![2, 3],
            },
            DuplicateGroup {
                target: 4,
                duplicates: vec![5],
            },
        ];
        assert_eq!(likely_duplicate_count(&groups), 3);
        assert_eq!(likely_duplicate_count(&[]), 0);
    }
}
