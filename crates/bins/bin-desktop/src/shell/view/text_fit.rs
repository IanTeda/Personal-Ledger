//! Fitting the Shell's text to a narrow window (ADR-0042): the status line drops whole hints from its
//! end. The decisions are pure over a `measure` function; [`text_width`] is the real one.

use gpui::{Font, FontWeight, Hsla, Pixels, TextRun, Window, font, px};

use crate::theme::type_scale;

/// How wide `text` draws in Archivo at `size` and `weight`.
pub fn text_width(window: &Window, text: &str, size: Pixels, weight: FontWeight) -> Pixels {
    let run = TextRun {
        len: text.len(),
        font: Font {
            weight,
            ..font(type_scale::FONT_FAMILY)
        },
        color: Hsla::default(),
        background_color: None,
        underline: None,
        strikethrough: None,
    };
    window
        .text_system()
        .shape_line(text.to_string().into(), size, &[run], None)
        .width
}

/// How many of the hints, `widths` wide each, fit in `max` with `gap` between neighbours: the
/// first ones, whole, so hints drop from the end.
pub fn hints_that_fit(widths: &[Pixels], gap: Pixels, max: Pixels) -> usize {
    let mut used = px(0.0);
    for (index, width) in widths.iter().enumerate() {
        let next = used + *width + if index > 0 { gap } else { px(0.0) };
        if next > max {
            return index;
        }
        used = next;
    }
    widths.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hints_drop_from_the_end_whole() {
        let widths = [px(10.0), px(10.0), px(10.0)];
        assert_eq!(hints_that_fit(&widths, px(2.0), px(34.0)), 3);
        assert_eq!(hints_that_fit(&widths, px(2.0), px(33.0)), 2);
        assert_eq!(hints_that_fit(&widths, px(2.0), px(9.0)), 0);
    }
}
