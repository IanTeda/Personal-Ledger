//! The Settings **Display** preferences applied to real data: the date style (dates and amounts
//! otherwise follow the Locale through `lib-locale`), the status and Flagged glyphs, and row
//! density. `gpui`-free, and taking each
//! preference as a plain argument (the enums in `settings.rs`, never `Shell`), so every view
//! renders from one place and the Display section's PREVIEW table keeps matching what the app
//! actually shows.
//!
//! The Settings preview has its own primitive-argument helpers (`settings::format_preview_date` and
//! `format_preview_amount`); these take the real types (`NaiveDate`, `Money`, `TransactionStatus`)
//! and route through the same Locale formatting, so the two cannot disagree.

use chrono::NaiveDate;
use lib_core::{DateStyle, Money, TransactionStatus};
use lib_locale::format::{AmountStyle, format_amount, format_date};

use crate::settings::{RowDensity, StatusGlyphs};

/// A date in the chosen style, or in the Locale's default form when there is none. `Iso` is
/// always `2026-09-12`; the others follow the Locale (`12 Sept 2026` in `en-AU`).
pub fn date(date: NaiveDate, style: Option<DateStyle>) -> String {
    format_date(date, style)
}

/// An amount as `(is_negative, text)` at the amount's **own** fractional digits (a fund's `0.4120`
/// keeps four places; a zero keeps the scale it was entered with). The flag lets the caller apply
/// the negative-balance token so a negative is never colour alone.
pub fn amount(money: &Money) -> (bool, String) {
    let formatted = format_amount(money, AmountStyle::own());
    (formatted.is_negative, formatted.text)
}

/// [`amount`] with a `+` in front of a positive, non-zero amount -- the Transactions AMOUNT column
/// is "signed", and the Display preview shows income as `+4,210.00`.
pub fn signed_amount(money: &Money) -> (bool, String) {
    let formatted = format_amount(money, AmountStyle::own().with_plus());
    (formatted.is_negative, formatted.text)
}

/// The status glyph: Open ○, Cleared ◐, Reconciled ● in the unicode style. The ascii fallback keeps
/// the Display preview's own mapping for the glyphs they share (◐ is `o`, ● is `x`) and gives Open
/// the remaining mark, `-`.
pub fn status_glyph(status: &TransactionStatus, style: StatusGlyphs) -> &'static str {
    match (status, style) {
        (TransactionStatus::Open, StatusGlyphs::Unicode) => "\u{25cb}",
        (TransactionStatus::Cleared, StatusGlyphs::Unicode) => "\u{25d0}",
        (TransactionStatus::Reconciled, StatusGlyphs::Unicode) => "\u{25cf}",
        (TransactionStatus::Open, StatusGlyphs::AsciiFallback) => "-",
        (TransactionStatus::Cleared, StatusGlyphs::AsciiFallback) => "o",
        (TransactionStatus::Reconciled, StatusGlyphs::AsciiFallback) => "x",
    }
}

/// The Flagged mark, shown separately from the status (Flagged is independent of it): ⚑, or `!`.
pub fn flag_glyph(style: StatusGlyphs) -> &'static str {
    match style {
        StatusGlyphs::Unicode => "\u{2691}",
        StatusGlyphs::AsciiFallback => "!",
    }
}

/// The status bar's glyph legend, in the chosen style:
/// `○ open · ◐ cleared · ● reconciled · ⚑ flagged`.
pub fn status_legend(style: StatusGlyphs) -> String {
    let statuses = [
        (TransactionStatus::Open, "open"),
        (TransactionStatus::Cleared, "cleared"),
        (TransactionStatus::Reconciled, "reconciled"),
    ];
    let mut parts: Vec<String> = statuses
        .iter()
        .map(|(status, name)| format!("{} {name}", status_glyph(status, style)))
        .collect();
    parts.push(format!("{} flagged", flag_glyph(style)));
    parts.join(" \u{b7} ")
}

/// The height of one line of table text: 13px type at the shell's body line height.
pub const ROW_LINE_HEIGHT_PX: f32 = 16.0;

/// A table row's vertical padding: the Transactions bundle's own 9px is the regular density, and
/// compact and roomy step 4px either side, the same step the Display preview uses.
pub fn row_padding_y_px(density: RowDensity) -> f32 {
    match density {
        RowDensity::Compact => 5.0,
        RowDensity::Regular => 9.0,
        RowDensity::Roomy => 13.0,
    }
}

/// A table row's total height in pixels for the chosen density -- what a virtualised list needs
/// (every row the same height), and it changes with the preference.
pub fn row_height_px(density: RowDensity) -> f32 {
    row_padding_y_px(density) * 2.0 + ROW_LINE_HEIGHT_PX
}

#[cfg(test)]
mod tests {
    use super::*;

    fn money(text: &str) -> Money {
        text.parse().unwrap()
    }

    fn day(year: i32, month: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(year, month, day).unwrap()
    }

    use lib_locale::{Locale, with_locale};

    #[test]
    fn dates_follow_the_locale_and_the_chosen_style() {
        let d = day(2026, 9, 5);
        with_locale(Locale::EnAu, || {
            assert_eq!(date(d, None), "5 Sept 2026");
            assert_eq!(date(d, Some(DateStyle::Short)), "5/9/26");
            assert_eq!(date(d, Some(DateStyle::Long)), "5 September 2026");
            assert_eq!(date(d, Some(DateStyle::Iso)), "2026-09-05");
        });
        with_locale(Locale::EnUs, || {
            assert_eq!(date(d, None), "Sep 5, 2026");
        });
    }

    #[test]
    fn dates_agree_with_the_settings_previews_helper() {
        with_locale(Locale::EnGb, || {
            for style in crate::settings::DATE_STYLE_CHOICES {
                assert_eq!(
                    date(day(2026, 9, 12), style),
                    crate::settings::format_preview_date(2026, 9, 12, style)
                );
            }
        });
    }

    #[test]
    fn amounts_keep_their_own_scale() {
        assert_eq!(amount(&money("0.4120")).1, "0.4120");
        assert_eq!(amount(&money("1240")).1, "1,240");
        assert_eq!(amount(&money("240.00")).1, "240.00");
        assert_eq!(amount(&money("0.00")).1, "0.00");
    }

    #[test]
    fn amounts_agree_with_the_settings_previews_helper_apart_from_the_plus() {
        let expected = crate::settings::format_preview_amount(-864_050);
        assert_eq!(amount(&money("-8640.50")).1, expected);
    }

    #[test]
    fn a_signed_amount_marks_only_positive_non_zero_amounts() {
        assert_eq!(
            signed_amount(&money("4210.00")),
            (false, "+4,210.00".to_string())
        );
        assert!(signed_amount(&money("-86.40")).0);
        assert_eq!(signed_amount(&money("0.00")), (false, "0.00".to_string()));
        assert_eq!(
            signed_amount(&money("4210.00")).1,
            crate::settings::format_preview_amount(421_000)
        );
    }

    #[test]
    fn status_glyphs_match_the_transactions_bundle_in_unicode() {
        let unicode = StatusGlyphs::Unicode;
        assert_eq!(status_glyph(&TransactionStatus::Open, unicode), "\u{25cb}");
        assert_eq!(
            status_glyph(&TransactionStatus::Cleared, unicode),
            "\u{25d0}"
        );
        assert_eq!(
            status_glyph(&TransactionStatus::Reconciled, unicode),
            "\u{25cf}"
        );
        assert_eq!(flag_glyph(unicode), "\u{2691}");
    }

    #[test]
    fn the_ascii_fallback_keeps_the_display_previews_marks_for_shared_glyphs() {
        let ascii = StatusGlyphs::AsciiFallback;
        assert_eq!(status_glyph(&TransactionStatus::Cleared, ascii), "o");
        assert_eq!(status_glyph(&TransactionStatus::Reconciled, ascii), "x");
        assert_eq!(flag_glyph(ascii), "!");
        assert_eq!(status_glyph(&TransactionStatus::Open, ascii), "-");
        for style in StatusGlyphs::ALL {
            let glyphs = [
                status_glyph(&TransactionStatus::Open, style),
                status_glyph(&TransactionStatus::Cleared, style),
                status_glyph(&TransactionStatus::Reconciled, style),
                flag_glyph(style),
            ];
            let mut unique = glyphs.to_vec();
            unique.sort_unstable();
            unique.dedup();
            assert_eq!(unique.len(), 4, "every mark is distinct in {style:?}");
        }
    }

    #[test]
    fn the_legend_names_every_status_and_the_flag_in_the_chosen_style() {
        assert_eq!(
            status_legend(StatusGlyphs::Unicode),
            "\u{25cb} open \u{b7} \u{25d0} cleared \u{b7} \u{25cf} reconciled \u{b7} \u{2691} flagged"
        );
        assert_eq!(
            status_legend(StatusGlyphs::AsciiFallback),
            "- open \u{b7} o cleared \u{b7} x reconciled \u{b7} ! flagged"
        );
    }

    #[test]
    fn row_height_grows_with_density_and_regular_is_the_bundles_nine_pixel_padding() {
        assert_eq!(row_padding_y_px(RowDensity::Regular), 9.0);
        assert_eq!(row_height_px(RowDensity::Compact), 26.0);
        assert_eq!(row_height_px(RowDensity::Regular), 34.0);
        assert_eq!(row_height_px(RowDensity::Roomy), 42.0);
        assert!(row_height_px(RowDensity::Compact) < row_height_px(RowDensity::Regular));
        assert!(row_height_px(RowDensity::Regular) < row_height_px(RowDensity::Roomy));
    }

    #[test]
    fn the_density_step_matches_the_display_previews_step() {
        for density in RowDensity::ALL {
            let preview = density.preview_row_padding_y();
            let regular_preview = RowDensity::Regular.preview_row_padding_y();
            let regular = row_padding_y_px(RowDensity::Regular);
            assert_eq!(
                row_padding_y_px(density) - regular,
                preview - regular_preview
            );
        }
    }
}
