//! Typed date input for each shipped Locale.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "integration test crate: a failed setup should fail the test"
)]

use chrono::NaiveDate;
use lib_core::DateStyle;
use lib_locale::format::{
    DateField, DateInputError, DateInputOptions, date_error_message, format_date_input, parse_date,
    parse_date_with,
};
use lib_locale::{Locale, with_locale};

fn day(year: i32, month: u32, date: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(year, month, date).expect("test date is valid")
}

const TODAY: (i32, u32, u32) = (2026, 9, 20);

fn today() -> NaiveDate {
    day(TODAY.0, TODAY.1, TODAY.2)
}

fn parse(locale: Locale, text: &str) -> Result<NaiveDate, DateInputError> {
    with_locale(locale, || parse_date(text, today()))
}

#[test]
fn each_locales_short_form_is_read_in_its_own_order() {
    assert_eq!(parse(Locale::EnAu, "3/9/2026"), Ok(day(2026, 9, 3)));
    assert_eq!(parse(Locale::EnGb, "03/09/2026"), Ok(day(2026, 9, 3)));
    assert_eq!(parse(Locale::EnUs, "9/3/2026"), Ok(day(2026, 9, 3)));
}

#[test]
fn the_locale_alone_resolves_day_month_ambiguity() {
    assert_eq!(parse(Locale::EnAu, "03/09/2026"), Ok(day(2026, 9, 3)));
    assert_eq!(parse(Locale::EnUs, "03/09/2026"), Ok(day(2026, 3, 9)));
    assert_eq!(
        parse(Locale::EnUs, "13/09/2026"),
        Err(DateInputError::OutOfRange(DateField::Month))
    );
    assert_eq!(parse(Locale::EnAu, "13/09/2026"), Ok(day(2026, 9, 13)));
}

#[test]
fn two_digit_years_are_rejected() {
    for locale in [Locale::EnUs, Locale::EnGb, Locale::EnAu] {
        assert_eq!(
            parse(locale, "3/9/26"),
            Err(DateInputError::BadYear),
            "{locale}"
        );
    }
}

#[test]
fn iso_is_always_accepted() {
    for locale in [Locale::EnUs, Locale::EnGb, Locale::EnAu, Locale::EnXa] {
        assert_eq!(parse(locale, "2026-09-03"), Ok(day(2026, 9, 3)), "{locale}");
        assert_eq!(parse(locale, "2026-9-3"), Ok(day(2026, 9, 3)), "{locale}");
    }
}

#[test]
fn an_iso_style_makes_iso_the_only_numeric_form() {
    let options = DateInputOptions {
        style: Some(DateStyle::Iso),
        allow_yearless: false,
    };
    with_locale(Locale::EnAu, || {
        assert_eq!(
            parse_date_with("2026-09-03", today(), &options),
            Ok(day(2026, 9, 3))
        );
        assert_eq!(
            parse_date_with("3/9/2026", today(), &options),
            Err(DateInputError::Unrecognised)
        );
        assert_eq!(parse_date_with("today", today(), &options), Ok(today()));
    });
}

#[test]
fn relative_words_work_in_english_and_from_messages() {
    for locale in [Locale::EnUs, Locale::EnGb, Locale::EnAu, Locale::EnXa] {
        assert_eq!(parse(locale, "today"), Ok(today()), "{locale}");
        assert_eq!(parse(locale, "Yesterday"), Ok(day(2026, 9, 19)), "{locale}");
        assert_eq!(
            parse(locale, " tomorrow "),
            Ok(day(2026, 9, 21)),
            "{locale}"
        );
    }
}

#[test]
fn yearless_input_is_opt_in() {
    let allowed = DateInputOptions {
        style: None,
        allow_yearless: true,
    };
    with_locale(Locale::EnAu, || {
        assert_eq!(
            parse_date("3/9", today()),
            Err(DateInputError::YearRequired)
        );
        assert_eq!(
            parse_date_with("3/9", today(), &allowed),
            Ok(day(2026, 9, 3))
        );
    });
    with_locale(Locale::EnUs, || {
        assert_eq!(
            parse_date_with("9/3", today(), &allowed),
            Ok(day(2026, 9, 3))
        );
    });
}

#[test]
fn invalid_input_gives_typed_errors() {
    assert_eq!(parse(Locale::EnAu, ""), Err(DateInputError::Empty));
    assert_eq!(parse(Locale::EnAu, "   "), Err(DateInputError::Empty));
    assert_eq!(
        parse(Locale::EnAu, "someday"),
        Err(DateInputError::Unrecognised)
    );
    assert_eq!(
        parse(Locale::EnAu, "3-9-2026"),
        Err(DateInputError::Unrecognised)
    );
    assert_eq!(
        parse(Locale::EnAu, "3/9/2/1"),
        Err(DateInputError::Unrecognised)
    );
    assert_eq!(parse(Locale::EnAu, "3/9/2"), Err(DateInputError::BadYear));
    assert_eq!(parse(Locale::EnAu, "3/9/202"), Err(DateInputError::BadYear));
    assert_eq!(
        parse(Locale::EnAu, "31/9/2026"),
        Err(DateInputError::OutOfRange(DateField::Day))
    );
    assert_eq!(
        parse(Locale::EnAu, "29/2/2026"),
        Err(DateInputError::OutOfRange(DateField::Day))
    );
    assert_eq!(parse(Locale::EnAu, "29/2/2028"), Ok(day(2028, 2, 29)));
    assert_eq!(
        parse(Locale::EnAu, "1/13/2026"),
        Err(DateInputError::OutOfRange(DateField::Month))
    );
    assert_eq!(
        parse(Locale::EnAu, "2026-13-01"),
        Err(DateInputError::OutOfRange(DateField::Month))
    );
    assert_eq!(
        parse(Locale::EnAu, "٣/٩/٢٠٢٦"),
        Err(DateInputError::Unrecognised)
    );
}

#[test]
fn error_messages_carry_the_expected_format_as_arguments() {
    let options = DateInputOptions::default();
    let message =
        |locale, error| with_locale(locale, || date_error_message(&error, today(), &options));
    assert_eq!(
        message(Locale::EnAu, DateInputError::Unrecognised),
        "Enter a date like 20/9/2026 or 2026-09-20, or a word such as today, yesterday, tomorrow."
    );
    assert_eq!(
        message(Locale::EnUs, DateInputError::Unrecognised),
        "Enter a date like 9/20/2026 or 2026-09-20, or a word such as today, yesterday, tomorrow."
    );
    assert_eq!(
        message(Locale::EnAu, DateInputError::OutOfRange(DateField::Month)),
        "That is not a valid month for a date."
    );
    assert_eq!(
        message(Locale::EnAu, DateInputError::Empty),
        "Enter a date."
    );
    assert_eq!(
        message(Locale::EnAu, DateInputError::BadYear),
        "Use a four-digit year."
    );
    assert_eq!(
        message(Locale::EnAu, DateInputError::YearRequired),
        "Include the year."
    );

    let iso = DateInputOptions {
        style: Some(DateStyle::Iso),
        allow_yearless: false,
    };
    assert_eq!(
        with_locale(Locale::EnAu, || date_error_message(
            &DateInputError::Unrecognised,
            today(),
            &iso
        )),
        "Enter a date like 2026-09-20, or a word such as today, yesterday, tomorrow."
    );
}

#[test]
fn typed_input_text_reads_back_as_the_same_date() {
    let day = day(2026, 12, 3);
    let cases = [
        (Locale::EnAu, None, "3/12/2026"),
        (Locale::EnGb, Some(DateStyle::Long), "3/12/2026"),
        (Locale::EnUs, Some(DateStyle::Short), "12/3/2026"),
        (Locale::EnAu, Some(DateStyle::Iso), "2026-12-03"),
        (Locale::EnXa, None, "12/3/2026"),
    ];
    for (locale, style, expected) in cases {
        with_locale(locale, || {
            let text = format_date_input(day, style);
            assert_eq!(text, expected, "{locale} {style:?}");
            let options = DateInputOptions {
                style,
                allow_yearless: false,
            };
            assert_eq!(
                parse_date_with(&text, today(), &options),
                Ok(day),
                "{locale}"
            );
        });
    }
}
