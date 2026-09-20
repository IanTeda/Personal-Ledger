//! The Settings **Display** preferences applied to real data: date format, decimal and thousands
//! separators, the status and Flagged glyphs, and row density. `gpui`-free, and taking each
//! preference as a plain argument (the enums in `settings.rs`, never `Shell`), so every view
//! renders from one place and the Display section's PREVIEW table keeps matching what the app
//! actually shows.
//!
//! The Settings preview has its own primitive-argument helpers (`settings::format_preview_date` and
//! `format_preview_amount`); these take the real types (`NaiveDate`, `Money`, `TransactionStatus`)
//! and share the same marks and month names, so the two cannot disagree.

use chrono::{Datelike, NaiveDate};
use lib_core::{Money, TransactionStatus};

use crate::settings::{
    DateFormat, DecimalSeparator, MONTH_ABBREVIATIONS, RowDensity, StatusGlyphs,
};

/// `12 sep 2026`, `12/09/2026` (day first) or `2026-09-12`, per the Date format preference.
pub fn date(date: NaiveDate, format: DateFormat) -> String {
    let (year, month, day) = (date.year(), date.month(), date.day());
    match format {
        DateFormat::DayMonthYear => {
            format!(
                "{day:02} {} {year}",
                MONTH_ABBREVIATIONS[month as usize - 1]
            )
        }
        DateFormat::Slash => format!("{day:02}/{month:02}/{year}"),
        DateFormat::Iso => format!("{year:04}-{month:02}-{day:02}"),
    }
}

/// [`date`] for a narrow column: in the day-month-year style the year is dropped when it is
/// `today`'s (`12 sep`, then `28 jan 2025` for an earlier year); the other two styles have no short
/// form and read in full. A column sized for the longest of these fits every style.
pub fn date_compact(date: NaiveDate, format: DateFormat, today: NaiveDate) -> String {
    match format {
        DateFormat::DayMonthYear if date.year() == today.year() => {
            format!(
                "{:02} {}",
                date.day(),
                MONTH_ABBREVIATIONS[date.month() as usize - 1]
            )
        }
        _ => self::date(date, format),
    }
}

/// An amount as `(is_negative, text)`: thousands grouped with the preferred mark, the preferred
/// decimal mark, the amount's **own** fractional digits (a fund's `0.4120` keeps four places; a
/// zero keeps the scale it was entered with), and U+2212 for a minus. The flag lets the caller
/// apply the negative-balance token so a negative is never colour alone. A zero is never negative,
/// even if its text is `-0.00`.
pub fn amount(money: &Money, separator: DecimalSeparator) -> (bool, String) {
    let (thousands, decimal) = separator.marks();
    let text = money.0.to_string();
    let (signed, digits) = match text.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, text.as_str()),
    };
    let negative = signed && digits.chars().any(|c| matches!(c, '1'..='9'));
    let (integer, fraction) = match digits.split_once('.') {
        Some((integer, fraction)) => (integer, Some(fraction.to_string())),
        // `BigDecimal` prints a zero as a bare `0` whatever scale it carries, so an empty `0.00`
        // amount would lose its places; put them back from the amount's own scale.
        None => match usize::try_from(money.0.fractional_digit_count()) {
            Ok(scale) if scale > 0 => (digits, Some("0".repeat(scale))),
            _ => (digits, None),
        },
    };

    let grouped = if integer.chars().all(|c| c.is_ascii_digit()) {
        let mut out = String::with_capacity(integer.len() + integer.len() / 3);
        for (index, digit) in integer.chars().enumerate() {
            if index > 0 && (integer.len() - index) % 3 == 0 {
                out.push(thousands);
            }
            out.push(digit);
        }
        out
    } else {
        integer.to_string()
    };

    let mut display = String::new();
    if negative {
        display.push('\u{2212}');
    }
    display.push_str(&grouped);
    if let Some(fraction) = fraction {
        display.push(decimal);
        display.push_str(&fraction);
    }
    (negative, display)
}

/// [`amount`] with a `+` in front of a positive, non-zero amount -- the Transactions AMOUNT column
/// is "signed", and the Display preview shows income as `+4,210.00`.
pub fn signed_amount(money: &Money, separator: DecimalSeparator) -> (bool, String) {
    let (negative, text) = amount(money, separator);
    let non_zero = text.chars().any(|c| matches!(c, '1'..='9'));
    if !negative && non_zero {
        (false, format!("+{text}"))
    } else {
        (negative, text)
    }
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

    #[test]
    fn dates_follow_each_of_the_three_styles_with_the_day_first() {
        let d = day(2026, 9, 5);
        assert_eq!(date(d, DateFormat::DayMonthYear), "05 sep 2026");
        assert_eq!(date(d, DateFormat::Slash), "05/09/2026");
        assert_eq!(date(d, DateFormat::Iso), "2026-09-05");
    }

    #[test]
    fn every_month_has_its_abbreviation() {
        let months: Vec<_> = (1..=12)
            .map(|m| date(day(2026, m, 1), DateFormat::DayMonthYear))
            .collect();
        assert_eq!(months[0], "01 jan 2026");
        assert_eq!(months[7], "01 aug 2026");
        assert_eq!(months[11], "01 dec 2026");
    }

    #[test]
    fn dates_agree_with_the_settings_previews_helper() {
        for format in DateFormat::ALL {
            assert_eq!(
                date(day(2026, 9, 12), format),
                crate::settings::format_preview_date(2026, 9, 12, format)
            );
        }
    }

    #[test]
    fn a_compact_date_drops_this_years_year_in_the_day_month_year_style_only() {
        let today = day(2026, 9, 19);
        assert_eq!(
            date_compact(day(2026, 9, 12), DateFormat::DayMonthYear, today),
            "12 sep"
        );
        assert_eq!(
            date_compact(day(2025, 1, 28), DateFormat::DayMonthYear, today),
            "28 jan 2025"
        );
        assert_eq!(
            date_compact(day(2026, 9, 12), DateFormat::Slash, today),
            "12/09/2026"
        );
        assert_eq!(
            date_compact(day(2026, 9, 12), DateFormat::Iso, today),
            "2026-09-12"
        );
    }

    #[test]
    fn amounts_use_the_chosen_separators() {
        let m = money("-1234567.89");
        assert_eq!(
            amount(&m, DecimalSeparator::CommaThousands),
            (true, "\u{2212}1,234,567.89".to_string())
        );
        assert_eq!(
            amount(&m, DecimalSeparator::DotThousands),
            (true, "\u{2212}1.234.567,89".to_string())
        );
        assert_eq!(
            amount(&m, DecimalSeparator::SpaceThousands),
            (true, "\u{2212}1 234 567.89".to_string())
        );
    }

    #[test]
    fn amounts_keep_their_own_scale() {
        let comma = DecimalSeparator::CommaThousands;
        assert_eq!(amount(&money("0.4120"), comma).1, "0.4120");
        assert_eq!(amount(&money("1240"), comma).1, "1,240");
        assert_eq!(amount(&money("240.00"), comma).1, "240.00");
        assert_eq!(amount(&money("0.00"), comma).1, "0.00");
        assert_eq!(
            amount(&money("0.4120"), DecimalSeparator::DotThousands).1,
            "0,4120"
        );
    }

    #[test]
    fn a_zero_is_never_negative_and_a_negative_carries_a_real_minus() {
        let comma = DecimalSeparator::CommaThousands;
        assert!(!amount(&money("-0.00"), comma).0);
        assert_eq!(
            amount(&money("-2318.44"), comma),
            (true, "\u{2212}2,318.44".to_string())
        );
        assert_eq!(amount(&money("999"), comma).1, "999");
        assert_eq!(amount(&money("1000"), comma).1, "1,000");
    }

    #[test]
    fn amounts_agree_with_the_settings_previews_helper_apart_from_the_plus() {
        for separator in DecimalSeparator::ALL {
            let expected = crate::settings::format_preview_amount(-864_050, separator);
            assert_eq!(amount(&money("-8640.50"), separator).1, expected);
        }
    }

    #[test]
    fn a_signed_amount_marks_only_positive_non_zero_amounts() {
        let comma = DecimalSeparator::CommaThousands;
        assert_eq!(
            signed_amount(&money("4210.00"), comma),
            (false, "+4,210.00".to_string())
        );
        assert_eq!(
            signed_amount(&money("-86.40"), comma),
            (true, "\u{2212}86.40".to_string())
        );
        assert_eq!(
            signed_amount(&money("0.00"), comma),
            (false, "0.00".to_string())
        );
        let preview = crate::settings::format_preview_amount(421_000, comma);
        assert_eq!(signed_amount(&money("4210.00"), comma).1, preview);
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
