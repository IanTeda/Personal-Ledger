//! Reading a provider's response: the price at its path, or the failure it is, and how long a
//! 429's `Retry-After` asks to wait.

use chrono::{DateTime, Duration, Utc};
use lib_core::Money;

use super::{FetchFailure, TransportError, TransportOutcome};

/// How long a 429 waits when its `Retry-After` is missing, unreadable or already past.
pub const RETRY_AFTER_DEFAULT: Duration = Duration::seconds(60);

/// How long a 429's raw `Retry-After` asks to wait from `now`: whole seconds, or an HTTP date.
/// Missing, unreadable or already past, it is [`RETRY_AFTER_DEFAULT`].
pub(super) fn retry_after_wait(raw: Option<&str>, now: DateTime<Utc>) -> Duration {
    let Some(raw) = raw.map(str::trim) else {
        return RETRY_AFTER_DEFAULT;
    };
    if let Ok(seconds) = raw.parse::<u32>() {
        return Duration::seconds(i64::from(seconds));
    }
    match DateTime::parse_from_rfc2822(raw) {
        Ok(at) if at.with_timezone(&Utc) > now => at.with_timezone(&Utc) - now,
        Ok(_) | Err(_) => RETRY_AFTER_DEFAULT,
    }
}

/// What a response held: a price, or why not.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Reading {
    Priced(Money),
    Failed(FetchFailure),
}

/// The round failure `outcome` is, if it is one: what stops a round whatever Unit it was for.
pub(super) fn round_failure(outcome: &TransportOutcome) -> Option<FetchFailure> {
    match read(outcome, "") {
        Reading::Failed(failure) if failure.is_round() => Some(failure),
        Reading::Priced(_) | Reading::Failed(_) => None,
    }
}

/// Reads `outcome` onto a price at `price_path`, or onto one of the [`FetchFailure`]s.
pub(super) fn read(outcome: &TransportOutcome, price_path: &str) -> Reading {
    let failed = Reading::Failed;
    match outcome {
        TransportOutcome::Failed(TransportError::Offline | TransportError::Timeout) => {
            failed(FetchFailure::Offline)
        }
        TransportOutcome::Failed(TransportError::Tls | TransportError::Other) => {
            failed(FetchFailure::SourceError)
        }
        TransportOutcome::Answered { status, body, .. } => match status {
            200..=299 => {
                if serde_json::from_str::<serde_json::Value>(body).is_err() {
                    failed(FetchFailure::SourceError)
                } else {
                    parse_price(body, price_path)
                        .map_or(failed(FetchFailure::NoPrice), Reading::Priced)
                }
            }
            401 | 403 => failed(FetchFailure::KeyRejected),
            429 => failed(FetchFailure::RateLimited),
            // A provider answers an unknown symbol or quote with a 404, or a 400 or 422 for one it
            // can't read.
            400..=499 => failed(FetchFailure::NotFound),
            _ => failed(FetchFailure::SourceError),
        },
    }
}

/// The price at JSON pointer `path` in `body`: a number, or a string holding one. `None` for a
/// body that isn't JSON or has no price there.
pub(crate) fn parse_price(body: &str, path: &str) -> Option<Money> {
    let json: serde_json::Value = serde_json::from_str(body).ok()?;
    super::history::money(json.pointer(path)?)
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;
    use lib_core::PriceSourceId;

    use super::*;
    use crate::fetching::price_request;
    use crate::{COINGECKO, FRANKFURTER, YAHOO_FINANCE, built_in};

    /// Replies captured from each built-in, read at the price path its request fills.
    mod captured {
        use super::*;

        fn priced(source: PriceSourceId, symbol: &str, quote: &str, body: &str) -> Reading {
            let endpoint = &built_in(source).expect("a built-in").endpoint;
            let (_, path) = price_request(endpoint, symbol, quote, None);
            read(
                &TransportOutcome::Answered {
                    status: 200,
                    body: body.into(),
                    retry_after: None,
                },
                &path,
            )
        }

        fn money(text: &str) -> Reading {
            Reading::Priced(text.parse().expect("a decimal"))
        }

        #[test]
        fn coingeckos_simple_price_reply_is_priced() {
            let body = include_str!("../../fixtures/prices/coingecko_simple_price.json");
            assert_eq!(priced(COINGECKO, "bitcoin", "AUD", body), money("92568"));
        }

        #[test]
        fn coingeckos_empty_reply_for_an_unknown_coin_is_no_price() {
            assert_eq!(
                priced(COINGECKO, "notacoin", "AUD", "{}"),
                Reading::Failed(FetchFailure::NoPrice)
            );
        }

        #[test]
        fn yahoos_chart_reply_is_priced_at_its_regular_market_price() {
            let body = include_str!("../../fixtures/prices/yahoo_chart.json");
            assert_eq!(priced(YAHOO_FINANCE, "BHP.AX", "AUD", body), money("52.52"));
        }

        #[test]
        fn yahoos_not_found_reply_is_not_found() {
            let body = include_str!("../../fixtures/prices/yahoo_chart_not_found.json");
            let endpoint = &built_in(YAHOO_FINANCE).expect("a built-in").endpoint;
            let (_, path) = price_request(endpoint, "NOPE.AX", "AUD", None);
            assert_eq!(
                read(
                    &TransportOutcome::Answered {
                        status: 404,
                        body: body.into(),
                        retry_after: None,
                    },
                    &path
                ),
                Reading::Failed(FetchFailure::NotFound)
            );
        }

        #[test]
        fn frankfurters_rate_reply_is_priced() {
            let body = include_str!("../../fixtures/prices/frankfurter_rate.json");
            assert_eq!(priced(FRANKFURTER, "EUR", "AUD", body), money("1.6513"));
        }
    }

    #[test]
    fn a_price_is_read_at_its_path_as_a_number_or_a_string() {
        let money = |text: &str| text.parse::<Money>().expect("a decimal");
        assert_eq!(
            parse_price(r#"{"base":"EUR","quote":"AUD","rate":1.6512}"#, "/rate"),
            Some(money("1.6512"))
        );
        assert_eq!(
            parse_price(r#"{"a":{"b":"0.25"}}"#, "/a/b"),
            Some(money("0.25"))
        );
    }

    #[test]
    fn no_price_is_read_from_a_missing_path_a_non_number_or_a_body_that_isnt_json() {
        assert_eq!(parse_price(r#"{"rate":1.5}"#, "/rates/AUD"), None);
        assert_eq!(parse_price(r#"{"rate":true}"#, "/rate"), None);
        assert_eq!(parse_price(r#"{"rate":"n/a"}"#, "/rate"), None);
        assert_eq!(parse_price("<html>", "/rate"), None);
    }

    #[test]
    fn each_transport_outcome_reads_onto_its_reason_at_its_level() {
        let answered = |status: u16, body: &str| TransportOutcome::Answered {
            status,
            body: body.into(),
            retry_after: None,
        };
        let cases = [
            (
                TransportOutcome::Failed(TransportError::Offline),
                FetchFailure::Offline,
            ),
            (
                TransportOutcome::Failed(TransportError::Timeout),
                FetchFailure::Offline,
            ),
            (
                TransportOutcome::Failed(TransportError::Tls),
                FetchFailure::SourceError,
            ),
            (answered(429, ""), FetchFailure::RateLimited),
            (answered(401, ""), FetchFailure::KeyRejected),
            (answered(403, ""), FetchFailure::KeyRejected),
            (answered(503, "<html>"), FetchFailure::SourceError),
            (answered(200, "<html>"), FetchFailure::SourceError),
            (
                answered(404, r#"{"message":"not found"}"#),
                FetchFailure::NotFound,
            ),
            (answered(422, ""), FetchFailure::NotFound),
            (answered(200, r#"{"rates":{}}"#), FetchFailure::NoPrice),
        ];
        for (outcome, reason) in cases {
            assert_eq!(
                read(&outcome, "/rate"),
                Reading::Failed(reason),
                "{outcome:?}"
            );
        }
        assert_eq!(
            read(&answered(200, r#"{"rate":1.5}"#), "/rate"),
            Reading::Priced("1.5".parse().expect("a decimal"))
        );
        let round = [
            FetchFailure::Offline,
            FetchFailure::RateLimited,
            FetchFailure::KeyMissing,
            FetchFailure::KeyRejected,
            FetchFailure::SourceError,
        ];
        assert!(round.iter().all(|reason| reason.is_round()));
        assert!(!FetchFailure::NotFound.is_round());
        assert!(!FetchFailure::NoPrice.is_round());
    }

    mod retry_after {
        use super::*;

        fn now() -> DateTime<Utc> {
            NaiveDate::from_ymd_opt(2015, 10, 21)
                .and_then(|day| day.and_hms_opt(7, 27, 0))
                .expect("a real time")
                .and_utc()
        }

        #[test]
        fn seconds_wait_that_many_seconds() {
            assert_eq!(retry_after_wait(Some("40"), now()), Duration::seconds(40));
            assert_eq!(
                retry_after_wait(Some(" 120 "), now()),
                Duration::seconds(120)
            );
        }

        #[test]
        fn an_http_date_waits_until_it() {
            assert_eq!(
                retry_after_wait(Some("Wed, 21 Oct 2015 07:28:00 GMT"), now()),
                Duration::seconds(60)
            );
            assert_eq!(
                retry_after_wait(Some("Wed, 21 Oct 2015 07:37:30 GMT"), now()),
                Duration::seconds(630)
            );
        }

        #[test]
        fn missing_unreadable_or_past_waits_60_seconds() {
            for raw in [
                None,
                Some("soon"),
                Some("-5"),
                Some("Wed, 21 Oct 2015 07:00:00 GMT"),
            ] {
                assert_eq!(
                    retry_after_wait(raw, now()),
                    Duration::seconds(60),
                    "{raw:?}"
                );
            }
        }
    }
}
