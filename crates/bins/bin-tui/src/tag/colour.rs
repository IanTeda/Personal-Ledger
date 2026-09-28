//! A Tag's optional colour: the preset swatches and the popups' shared colour draft
//! (issue #363, following ADR-0015's #352 amendment). The colour is the user's own data, not a
//! Colour Theme role, so it is drawn as stored and never adapts to a Colour Variant.

use lib_colour_theme::Rgba;
use lib_core::{HexColor, HexColorError};
use ratatui::{style::Style, text::Span};

use crate::colours::Colours;

/// The swatch glyph drawn before a coloured Tag's name.
pub const SWATCH: &str = "\u{25cf}";

/// The preset swatches, in the Desktop handoff's order (`bin-desktop`'s `tags::swatches`);
/// any other hex value is allowed too.
pub fn swatches() -> [HexColor; 6] {
    [
        HexColor::from_rgb(0xec, 0x30, 0x13),
        HexColor::from_rgb(0x7a, 0x8a, 0x6b),
        HexColor::from_rgb(0xc9, 0x92, 0x2f),
        HexColor::from_rgb(0x4a, 0x7c, 0x9e),
        HexColor::from_rgb(0x8a, 0x6b, 0xb5),
        HexColor::from_rgb(0x9b, 0x97, 0x97),
    ]
}

/// The swatch for `color`, or a blank cell of the same width when there is none, so names stay
/// aligned whether or not a Tag has a colour.
pub fn swatch_span(color: Option<&HexColor>, c: &Colours) -> Span<'static> {
    match color {
        Some(color) => {
            let (r, g, b) = color.components();
            Span::styled(SWATCH, c.swatch(Rgba::rgb(r, g, b)))
        }
        None => Span::styled(" ", Style::default()),
    }
}

/// The colour field of the new and edit popups. The hex text is the single source of the
/// colour: Space steps through "none" and the presets by writing into it, and typing edits it
/// freely, so a typed value and a picked one can never disagree.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ColourDraft {
    hex: String,
}

impl ColourDraft {
    pub fn new(color: Option<&HexColor>) -> Self {
        Self {
            hex: color
                .map(|color| color.as_str().to_string())
                .unwrap_or_default(),
        }
    }

    pub fn hex(&self) -> &str {
        &self.hex
    }

    /// Space steps none → each preset → none; from a custom value it starts at the first preset.
    /// Any other character that can be part of a hex value is typed.
    pub fn push_char(&mut self, ch: char) {
        if ch == ' ' {
            self.cycle();
        } else if (ch == '#' || ch.is_ascii_hexdigit()) && self.hex.chars().count() < 7 {
            self.hex.push(ch);
        }
    }

    pub fn backspace(&mut self) {
        self.hex.pop();
    }

    pub fn clear(&mut self) {
        self.hex.clear();
    }

    fn cycle(&mut self) {
        let presets = swatches();
        let next = match self.colour() {
            Ok(None) => presets.first(),
            Ok(Some(current)) => match presets.iter().position(|preset| *preset == current) {
                Some(index) => presets.get(index + 1),
                None => presets.first(),
            },
            Err(_) => presets.first(),
        };
        self.hex = next
            .map(|preset| preset.as_str().to_string())
            .unwrap_or_default();
    }

    /// The colour the draft names: `Ok(None)` when empty, an error while the text isn't a
    /// valid hex colour yet (which blocks saving).
    pub fn colour(&self) -> Result<Option<HexColor>, HexColorError> {
        let hex = self.hex.trim();
        if hex.is_empty() {
            return Ok(None);
        }
        HexColor::parse(hex).map(Some)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_empty_draft_is_no_colour() {
        assert_eq!(ColourDraft::default().colour(), Ok(None));
    }

    #[test]
    fn space_cycles_none_through_every_preset_and_back_to_none() {
        let mut draft = ColourDraft::default();
        for preset in swatches() {
            draft.push_char(' ');
            assert_eq!(draft.colour(), Ok(Some(preset)));
        }
        draft.push_char(' ');
        assert_eq!(draft.colour(), Ok(None));
    }

    #[test]
    fn space_from_a_custom_colour_starts_at_the_first_preset() {
        let mut draft = ColourDraft::new(Some(&HexColor::from_rgb(1, 2, 3)));
        draft.push_char(' ');
        assert_eq!(draft.colour(), Ok(Some(swatches()[0].clone())));
    }

    #[test]
    fn typing_a_hex_value_names_that_colour() {
        let mut draft = ColourDraft::default();
        "#12abef".chars().for_each(|ch| draft.push_char(ch));
        assert_eq!(
            draft.colour(),
            Ok(Some(HexColor::from_rgb(0x12, 0xab, 0xef)))
        );
    }

    #[test]
    fn non_hex_characters_are_ignored_and_a_partial_value_is_invalid() {
        let mut draft = ColourDraft::default();
        "#12zz".chars().for_each(|ch| draft.push_char(ch));
        assert_eq!(draft.hex(), "#12");
        assert!(draft.colour().is_err());
    }

    #[test]
    fn backspace_and_clear_edit_the_text() {
        let mut draft = ColourDraft::new(Some(&swatches()[3]));
        draft.backspace();
        assert!(draft.colour().is_err());
        draft.clear();
        assert_eq!(draft.colour(), Ok(None));
    }
}
