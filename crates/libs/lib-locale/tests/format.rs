//! Expected outputs for the formatting API in each shipped Locale.

use std::str::FromStr;

use bigdecimal::BigDecimal;
use chrono::NaiveDate;
use lib_core::{DateStyle, Money, UnitKind};
use lib_locale::format::{Unit, format_date, format_money, format_number, upper};
use lib_locale::{Locale, with_locale};

fn money(text: &str) -> Money {
    Money::from_str(text).expect("test amount parses")
}

fn date(year: i32, month: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(year, month, day).expect("test date is valid")
}

fn fiat(amount: &str, code: &str, decimal_places: u32, locale: Locale) -> String {
    with_locale(locale, || {
        format_money(
            &money(amount),
            &Unit::new(code, &UnitKind::Fiat, decimal_places),
        )
    })
}

#[test]
fn numbers_group_and_round_in_every_locale() {
    for locale in [Locale::EnUs, Locale::EnGb, Locale::EnAu] {
        with_locale(locale, || {
            assert_eq!(format_number(&money("1234567.891"), 2), "1,234,567.89");
            assert_eq!(format_number(&money("1234.5"), 2), "1,234.50");
            assert_eq!(format_number(&money("999"), 0), "999");
            assert_eq!(format_number(&money("-1234.5"), 2), "-1,234.50");
        });
    }
}

#[test]
fn rounding_is_half_away_from_zero_and_never_negative_zero() {
    with_locale(Locale::EnUs, || {
        assert_eq!(format_number(&money("2.345"), 2), "2.35");
        assert_eq!(format_number(&money("-2.345"), 2), "-2.35");
        assert_eq!(format_number(&money("0.5"), 0), "1");
        assert_eq!(format_number(&money("-0.004"), 2), "0.00");
    });
}

#[test]
fn amounts_stay_exact_beyond_f64() {
    with_locale(Locale::EnUs, || {
        assert_eq!(
            format_number(&money("123456789012345678901234567890.12"), 2),
            "123,456,789,012,345,678,901,234,567,890.12"
        );
        // `Display` for this value is scientific notation, which ICU4X cannot parse.
        let huge = Money(BigDecimal::new(1.into(), -30));
        assert_eq!(
            format_number(&huge, 0),
            "1,000,000,000,000,000,000,000,000,000,000"
        );
    });
}

#[test]
fn fiat_amounts_follow_the_locale_and_disambiguate() {
    assert_eq!(fiat("1234.5", "AUD", 2, Locale::EnAu), "$1,234.50");
    assert_eq!(fiat("1234.5", "AUD", 2, Locale::EnUs), "A$1,234.50");
    assert_eq!(fiat("1234.5", "AUD", 2, Locale::EnGb), "A$1,234.50");
    assert_eq!(fiat("1234.5", "USD", 2, Locale::EnUs), "$1,234.50");
    assert_eq!(fiat("1234.5", "USD", 2, Locale::EnAu), "US$1,234.50");
    assert_eq!(fiat("1234.5", "GBP", 2, Locale::EnGb), "£1,234.50");
    assert_eq!(fiat("-1234.5", "AUD", 2, Locale::EnAu), "-$1,234.50");
}

#[test]
fn the_unit_decides_the_fraction_digits() {
    assert_eq!(fiat("1234.5", "JPY", 0, Locale::EnUs), "¥1,235");
    // A fiat currency's own CLDR digits (two for AUD) win over a Unit configured with more.
    assert_eq!(fiat("1234.5", "AUD", 3, Locale::EnAu), "$1,234.50");
    // A non-fiat Unit's digits are used as configured.
    with_locale(Locale::EnAu, || {
        let gold = Unit::new("XAU", &UnitKind::PreciousMetal, 3);
        assert_eq!(format_money(&money("2.5"), &gold), "XAU 2.500");
    });
}

#[test]
fn non_fiat_units_place_the_code_through_a_message() {
    with_locale(Locale::EnAu, || {
        let btc = Unit::new("BTC", &UnitKind::Crypto, 8);
        assert_eq!(format_money(&money("1234.5"), &btc), "BTC 1,234.50000000");
        let aapl = Unit::new("AAPL", &UnitKind::Stock, 3);
        assert_eq!(format_money(&money("-0.005"), &aapl), "AAPL -0.005");
    });
}

#[test]
fn a_fiat_code_with_no_currency_falls_back_to_a_quantity() {
    with_locale(Locale::EnUs, || {
        let odd = Unit::new("NOT-A-CODE", &UnitKind::Fiat, 2);
        assert_eq!(format_money(&money("5"), &odd), "NOT-A-CODE 5.00");
    });
}

#[test]
fn dates_follow_the_locale_and_style() {
    let day = date(2026, 9, 3);
    let cases = [
        (Locale::EnUs, "9/3/26", "Sep 3, 2026", "September 3, 2026"),
        (
            Locale::EnGb,
            "03/09/2026",
            "3 Sept 2026",
            "3 September 2026",
        ),
        (Locale::EnAu, "3/9/26", "3 Sept 2026", "3 September 2026"),
    ];
    for (locale, short, medium, long) in cases {
        with_locale(locale, || {
            assert_eq!(
                format_date(day, Some(DateStyle::Short)),
                short,
                "{locale} short"
            );
            assert_eq!(
                format_date(day, Some(DateStyle::Medium)),
                medium,
                "{locale} medium"
            );
            assert_eq!(
                format_date(day, Some(DateStyle::Long)),
                long,
                "{locale} long"
            );
            assert_eq!(
                format_date(day, Some(DateStyle::Iso)),
                "2026-09-03",
                "{locale} iso"
            );
            assert_eq!(format_date(day, None), medium, "{locale} default");
        });
    }
}

#[test]
fn en_xa_formats_as_en_us() {
    with_locale(Locale::EnXa, || {
        assert_eq!(
            format_date(date(2026, 9, 3), Some(DateStyle::Short)),
            "9/3/26"
        );
        assert_eq!(format_number(&money("1234.5"), 2), "1,234.50");
        assert_eq!(upper("net position"), "NET POSITION");
    });
}

#[test]
fn upper_uses_full_unicode_casing() {
    with_locale(Locale::EnAu, || {
        assert_eq!(upper("Straße"), "STRASSE");
        assert_eq!(upper("Net position"), "NET POSITION");
    });
}
