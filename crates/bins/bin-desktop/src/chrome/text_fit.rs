//! Fitting Chrome text to a narrow window (ADR-0042): the top bar's breadcrumb loses its start so
//! the leaf always shows, and the status line drops whole hints from its end. The decisions are
//! pure over a `measure` function; [`text_width`] is the real one.

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

/// `text` whole if it fits in `max`, otherwise its end after a leading `…`, as much of it as
/// fits. Only the `…` is left when not even one character fits beside it.
pub fn truncate_start(text: &str, max: Pixels, measure: impl Fn(&str) -> Pixels) -> String {
    if measure(text) <= max {
        return text.to_string();
    }
    text.char_indices()
        .skip(1)
        .map(|(start, _)| format!("\u{2026}{}", &text[start..]))
        .find(|candidate| measure(candidate) <= max)
        .unwrap_or_else(|| "\u{2026}".to_string())
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

    /// One pixel per character, so widths read as lengths.
    fn chars(text: &str) -> Pixels {
        px(text.chars().count() as f32)
    }

    #[test]
    fn a_trail_that_fits_is_left_whole() {
        assert_eq!(truncate_start("Settings", px(8.0), chars), "Settings");
    }

    #[test]
    fn a_trail_too_long_loses_its_start_so_the_leaf_shows() {
        assert_eq!(
            truncate_start("Transactions › import.csv", px(11.0), chars),
            "…import.csv"
        );
    }

    #[test]
    fn nothing_but_the_ellipsis_is_left_when_nothing_fits() {
        assert_eq!(truncate_start("Budgets", px(1.0), chars), "…");
    }

    #[test]
    fn hints_drop_from_the_end_whole() {
        let widths = [px(10.0), px(10.0), px(10.0)];
        assert_eq!(hints_that_fit(&widths, px(2.0), px(34.0)), 3);
        assert_eq!(hints_that_fit(&widths, px(2.0), px(33.0)), 2);
        assert_eq!(hints_that_fit(&widths, px(2.0), px(9.0)), 0);
    }
}
