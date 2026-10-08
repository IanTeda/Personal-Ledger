//! The Settings › Display page's model, `gpui`-free: the date style, row density and status
//! glyph choices and the PREVIEW table's seeded rows and formatting.

use lib_core::DateStyle;

/// The **Display** section's "Date format" segmented control: the nullable date style
/// Preference (ADR-0021). `None` is "Locale default" -- the Locale picks the form -- and only an
/// explicit choice is stored. The Locale itself is not chosen here: it is Configuration, shown
/// read-only beside this control. The number separator has no control, because the Locale owns it.
pub const DATE_STYLE_CHOICES: [Option<DateStyle>; 5] = [
    None,
    Some(DateStyle::Short),
    Some(DateStyle::Medium),
    Some(DateStyle::Long),
    Some(DateStyle::Iso),
];

/// The control's label for a choice, as a Message in the Locale in effect.
pub fn date_style_label(choice: Option<DateStyle>) -> String {
    match choice {
        None => crate::msg::desktop_display_date_style_default(),
        Some(DateStyle::Short) => crate::msg::desktop_display_date_style_short(),
        Some(DateStyle::Medium) => crate::msg::desktop_display_date_style_medium(),
        Some(DateStyle::Long) => crate::msg::desktop_display_date_style_long(),
        Some(DateStyle::Iso) => crate::msg::desktop_display_date_style_iso(),
    }
}

/// The same section's "Row density" segmented control (`regular` the mockup's own `checked`
/// option).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RowDensity {
    Compact,
    #[default]
    Regular,
    Roomy,
}

impl RowDensity {
    pub const ALL: [RowDensity; 3] = [Self::Compact, Self::Regular, Self::Roomy];

    pub fn label(self) -> String {
        match self {
            Self::Compact => crate::msg::desktop_settings_density_compact(),
            Self::Regular => crate::msg::desktop_settings_density_regular(),
            Self::Roomy => crate::msg::desktop_settings_density_roomy(),
        }
    }

    /// The PREVIEW table's own row vertical padding at this density -- the mockup only draws the
    /// `regular` state (`padding:8px 12px`), so `compact`/`roomy` step by the design tokens' own
    /// 4px rhythm (`docs/ux/desktop/16-settings/README.md`'s Spacing token list) either side of it,
    /// the same "reflects the currently-selected values" the README's own field list promises
    /// for this control.
    pub fn preview_row_padding_y(self) -> f32 {
        match self {
            Self::Compact => 4.0,
            Self::Regular => 8.0,
            Self::Roomy => 12.0,
        }
    }
}

/// The same section's "Status glyphs" radio group (dot-style, like the Tracing page's level
/// radios, not segmented) -- `unicode` is the mockup's own `checked` option.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StatusGlyphs {
    #[default]
    Unicode,
    AsciiFallback,
}

impl StatusGlyphs {
    pub const ALL: [StatusGlyphs; 2] = [Self::Unicode, Self::AsciiFallback];

    pub fn label(self) -> String {
        match self {
            Self::Unicode => {
                crate::msg::desktop_settings_glyphs_unicode("\u{25cb} \u{25d0} \u{25cf} \u{2691}")
            }
            Self::AsciiFallback => crate::msg::desktop_settings_glyphs_ascii("o / x !"),
        }
    }
}

/// One PREVIEW-table row's transaction status (`docs/ux/desktop/16-settings/README.md`'s own three
/// drawn rows: cleared, pending, flagged) -- distinct from [`StatusGlyphs`], which picks *how* a
/// status renders, not *which* status a row has.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreviewStatus {
    Cleared,
    Pending,
    Flagged,
}

impl PreviewStatus {
    /// The glyph shown in the PREVIEW table's leftmost column at the given [`StatusGlyphs`]
    /// style. The ascii fallback's own three symbols aren't individually labelled in the
    /// mockup's copy ("o / x !"), so this keeps that same left-to-right order: pending stays the
    /// "open/unmarked" `o`, cleared takes the unambiguous `x`, flagged keeps `!`.
    pub fn glyph(self, style: StatusGlyphs) -> &'static str {
        match (self, style) {
            (Self::Cleared, StatusGlyphs::Unicode) => "\u{25cf}",
            (Self::Pending, StatusGlyphs::Unicode) => "\u{25d0}",
            (Self::Flagged, StatusGlyphs::Unicode) => "\u{2691}",
            (Self::Cleared, StatusGlyphs::AsciiFallback) => "x",
            (Self::Pending, StatusGlyphs::AsciiFallback) => "o",
            (Self::Flagged, StatusGlyphs::AsciiFallback) => "!",
        }
    }
}

/// One PREVIEW-table row -- the mockup's own three seeded rows
/// (`docs/ux/desktop/16-settings/README.md`'s "2a resting state" markup), keyed on a real
/// `(year, month, day)` and integer cents rather than pre-formatted strings so
/// [`format_preview_date`]/[`format_preview_amount`] can re-render them under any selected
/// [`DateFormat`]/[`DecimalSeparator`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DisplayPreviewRow {
    pub status: PreviewStatus,
    pub year: u32,
    pub month: u32,
    pub day: u32,
    pub payee: &'static str,
    pub amount_cents: i64,
}

/// The mockup's own three seeded rows, in its own order.
pub const DEFAULT_DISPLAY_PREVIEW_ROWS: &[DisplayPreviewRow] = &[
    DisplayPreviewRow {
        status: PreviewStatus::Cleared,
        year: 2026,
        month: 9,
        day: 12,
        payee: "Woolworths Metro",
        amount_cents: -8640,
    },
    DisplayPreviewRow {
        status: PreviewStatus::Pending,
        year: 2026,
        month: 9,
        day: 10,
        payee: "Telstra",
        amount_cents: -9900,
    },
    DisplayPreviewRow {
        status: PreviewStatus::Flagged,
        year: 2026,
        month: 9,
        day: 9,
        payee: "Dept of Education",
        amount_cents: 421_000,
    },
];

/// Formats a `(year, month, day)` in the selected date style, for the PREVIEW table, through the
/// same Locale formatting the rest of the app uses so the two cannot disagree.
pub fn format_preview_date(year: u32, month: u32, day: u32, style: Option<DateStyle>) -> String {
    i32::try_from(year)
        .ok()
        .and_then(|year| chrono::NaiveDate::from_ymd_opt(year, month, day))
        .map(|date| crate::view::format::date(date, style))
        .unwrap_or_else(|| "???".to_string())
}

/// Formats `amount_cents` as the PREVIEW table shows it (`"+4,210.00"`, `"\u{2212}86.40"`): an
/// explicit `+` for a positive amount and the Locale's grouping.
pub fn format_preview_amount(amount_cents: i64) -> String {
    let sign = if amount_cents < 0 { "-" } else { "" };
    let text = format!(
        "{sign}{}.{:02}",
        amount_cents.unsigned_abs() / 100,
        amount_cents.unsigned_abs() % 100
    );
    text.parse::<lib_core::Money>()
        .map(|money| crate::view::format::signed_amount(&money).1)
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_date_style_choices_start_with_the_locale_default() {
        assert_eq!(DATE_STYLE_CHOICES[0], None);
        assert_eq!(DATE_STYLE_CHOICES.len(), 5);
    }

    #[test]
    fn the_date_style_labels_are_messages() {
        crate::locale::init_for_tests();
        let labels: Vec<String> = DATE_STYLE_CHOICES
            .into_iter()
            .map(date_style_label)
            .collect();
        assert_eq!(labels, ["Locale", "Short", "Medium", "Long", "ISO"]);
    }

    #[test]
    fn row_density_defaults_to_regular() {
        assert_eq!(RowDensity::default(), RowDensity::Regular);
    }

    #[test]
    fn status_glyphs_defaults_to_unicode() {
        assert_eq!(StatusGlyphs::default(), StatusGlyphs::Unicode);
    }

    #[test]
    fn default_display_preview_rows_matches_the_mockups_own_three_seeded_rows() {
        let payees: Vec<_> = DEFAULT_DISPLAY_PREVIEW_ROWS
            .iter()
            .map(|row| row.payee)
            .collect();
        assert_eq!(
            payees,
            vec!["Woolworths Metro", "Telstra", "Dept of Education"]
        );
    }

    #[test]
    fn format_preview_date_follows_the_locale_and_style() {
        use lib_locale::{Locale, with_locale};
        with_locale(Locale::EnAu, || {
            assert_eq!(format_preview_date(2026, 9, 12, None), "12 Sept 2026");
            assert_eq!(
                format_preview_date(2026, 9, 12, Some(DateStyle::Long)),
                "12 September 2026"
            );
            assert_eq!(
                format_preview_date(2026, 9, 12, Some(DateStyle::Iso)),
                "2026-09-12"
            );
        });
        with_locale(Locale::EnUs, || {
            assert_eq!(
                format_preview_date(2026, 9, 12, Some(DateStyle::Short)),
                "9/12/26"
            );
        });
        assert_eq!(format_preview_date(2026, 13, 40, None), "???");
    }

    #[test]
    fn format_preview_amount_matches_the_mockups_own_values() {
        assert_eq!(format_preview_amount(-8640), "\u{2212}86.40");
        assert_eq!(format_preview_amount(421_000), "+4,210.00");
    }

    #[test]
    fn preview_status_glyph_matches_the_selected_style() {
        assert_eq!(
            PreviewStatus::Cleared.glyph(StatusGlyphs::Unicode),
            "\u{25cf}"
        );
        assert_eq!(
            PreviewStatus::Cleared.glyph(StatusGlyphs::AsciiFallback),
            "x"
        );
        assert_eq!(
            PreviewStatus::Flagged.glyph(StatusGlyphs::Unicode),
            "\u{2691}"
        );
        assert_eq!(
            PreviewStatus::Flagged.glyph(StatusGlyphs::AsciiFallback),
            "!"
        );
    }

    #[test]
    fn local_enum_labels_come_from_the_catalogue() {
        crate::locale::init_for_tests();
        assert_eq!(RowDensity::Roomy.label(), "roomy");
        assert_eq!(
            StatusGlyphs::AsciiFallback.label(),
            "ascii fallback \u{2014} o / x !"
        );
    }
}
