//! Backfilling a Unit's Weekly Prices from its source's history request (ADR-0043's backfill
//! amendment): the weeks a Unit wants, the gaps among them, the request for them, reading a reply's
//! daily points and folding them into weeks. Like the rest of fetching, every provider rule is here
//! and the transport only carries bytes, so the rules are tested on captured replies.

use std::collections::HashSet;

use chrono::{DateTime, Duration, NaiveDate, Utc};
use lib_core::Money;
use secrecy::SecretString;
use serde_json::Value;

use crate::fetching::{FetchFailure, PriceRequest, Reading, TransportOutcome, fill, read, request};
use crate::{Endpoint, HistoryEndpoint, SeriesShape, UnitRow, WeeklyPrice};

/// One day's price from a history reply, dated by the provider's trading day. A close-only point
/// has no open, high or low.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Point {
    pub day: NaiveDate,
    /// The instant it is stamped with, when its date is a timestamp or a datetime rather than a
    /// bare date: what an Hourly Price is bucketed by.
    pub at: Option<DateTime<Utc>>,
    pub close: Money,
    pub open: Option<Money>,
    pub high: Option<Money>,
    pub low: Option<Money>,
}

/// What a history reply held: its points, none at all being a range the provider can't supply,
/// or why it failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum HistoryReading {
    Points(Vec<Point>),
    Failed(FetchFailure),
}

/// The day a Unit's history reaches back to: the earliest Opened date of its Accounts,
/// `earliest_opened`, else the day it was created; `None` when neither is known.
pub(crate) fn floor(unit: &UnitRow, earliest_opened: Option<NaiveDate>) -> Option<NaiveDate> {
    earliest_opened.or(unit.created_on)
}

/// The first and last week (each its Monday) a Unit with `floor` wants on `today`: from the week
/// holding the floor to last week, never the current week, which the scheduled fetch owns. A
/// source with a history `depth` in days reaches back only the whole weeks inside it counted from
/// the current week, so no request is sent that is bound to be refused. `None` when no week is
/// wanted.
pub(crate) fn wanted_weeks(
    floor: NaiveDate,
    today: NaiveDate,
    depth: Option<u32>,
) -> Option<(NaiveDate, NaiveDate)> {
    let current = WeeklyPrice::week_of(today);
    let last = current - Duration::weeks(1);
    let mut first = WeeklyPrice::week_of(floor);
    if let Some(days) = depth {
        let reach = current - Duration::weeks(i64::from(days / 7).saturating_sub(1));
        first = first.max(reach);
    }
    (first <= last).then_some((first, last))
}

/// The weeks from `first` to `last` that `unit` has no Weekly Price for, oldest first, leaving out
/// the `empty` ones its provider already answered with nothing.
pub(crate) fn gaps(
    unit: &UnitRow,
    (first, last): (NaiveDate, NaiveDate),
    empty: &HashSet<NaiveDate>,
) -> Vec<NaiveDate> {
    let priced: HashSet<NaiveDate> = unit.prices.iter().map(|price| price.week).collect();
    let mut weeks = Vec::new();
    let mut week = first;
    while week <= last {
        if !priced.contains(&week) && !empty.contains(&week) {
            weeks.push(week);
        }
        week += Duration::weeks(1);
    }
    weeks
}

/// The request asking `endpoint`'s `history` for `symbol` in `quote` (the quote Unit's code) from
/// `from` to `to`, both days included, carrying the endpoint's headers and `key` as its price
/// request does. `{from}` and `{to}` fill as ISO dates, and `{from_unix}` and `{to_unix}` as Unix
/// seconds from a day before `from` to the end of `to`: a market east of UTC stamps a day's bar on
/// the UTC day before. The points outside the asked weeks are dropped when folded.
pub(crate) fn history_request(
    endpoint: &Endpoint,
    history: &HistoryEndpoint,
    symbol: &str,
    quote: &str,
    (from, to): (NaiveDate, NaiveDate),
    key: Option<SecretString>,
) -> PriceRequest {
    let unix = |day: NaiveDate| day.and_time(chrono::NaiveTime::MIN).and_utc().timestamp();
    let path_and_query = fill(&history.request, symbol, quote)
        .replace("{from_unix}", &unix(from - Duration::days(1)).to_string())
        .replace("{to_unix}", &unix(to + Duration::days(1)).to_string())
        .replace("{from}", &from.format("%Y-%m-%d").to_string())
        .replace("{to}", &to.format("%Y-%m-%d").to_string());
    request(endpoint, &path_and_query, key)
}

/// Reads `outcome` onto the daily points of `history` for `symbol` in `quote`, or onto one of the
/// [`FetchFailure`]s. A reply whose series holds no readable point is a range the provider can't
/// supply, not a failure; one that isn't JSON, or has no series where its path says, is a
/// Source error.
pub(crate) fn read_history(
    outcome: &TransportOutcome,
    history: &HistoryEndpoint,
    symbol: &str,
    quote: &str,
) -> HistoryReading {
    let body = match outcome {
        TransportOutcome::Answered { status, body, .. } if (200..=299).contains(status) => body,
        // Every other answer reads as a price's would: an empty price path finds no price, so only
        // the status and transport decide.
        TransportOutcome::Answered { .. } | TransportOutcome::Failed(_) => {
            return match read(outcome, "") {
                Reading::Failed(failure) => HistoryReading::Failed(failure),
                Reading::Priced(_) => HistoryReading::Failed(FetchFailure::SourceError),
            };
        }
    };
    let Ok(json) = serde_json::from_str::<Value>(body) else {
        return HistoryReading::Failed(FetchFailure::SourceError);
    };
    let path = |text: &str| fill(text, symbol, quote);
    let Some(series) = json.pointer(&path(&history.series_path)) else {
        return HistoryReading::Failed(FetchFailure::SourceError);
    };
    let points = match &history.shape {
        SeriesShape::Points => {
            let Some(points) = series.as_array() else {
                return HistoryReading::Failed(FetchFailure::SourceError);
            };
            let (date, price) = (path(&history.date_path), path(&history.price_path));
            points
                .iter()
                .filter_map(|point| {
                    let stamp = point.pointer(&date)?;
                    Some(Point {
                        day: day_of(stamp, 0)?,
                        at: instant_of(stamp),
                        close: money(point.pointer(&price)?)?,
                        open: None,
                        high: None,
                        low: None,
                    })
                })
                .collect()
        }
        SeriesShape::Keyed => {
            let Some(points) = series.as_object() else {
                return HistoryReading::Failed(FetchFailure::SourceError);
            };
            let (date, price) = (path(&history.date_path), path(&history.price_path));
            points
                .iter()
                .filter_map(|(key, point)| {
                    let key = Value::String(key.clone());
                    let stamp = if date.is_empty() {
                        &key
                    } else {
                        point.pointer(&date)?
                    };
                    Some(Point {
                        day: day_of(stamp, 0)?,
                        at: instant_of(stamp),
                        close: money(point.pointer(&price)?)?,
                        open: None,
                        high: None,
                        low: None,
                    })
                })
                .collect()
        }
        SeriesShape::Columns {
            open_path,
            high_path,
            low_path,
            offset_path,
        } => {
            let column = |text: &str| {
                series
                    .pointer(&path(text))
                    .and_then(Value::as_array)
                    .cloned()
                    .unwrap_or_default()
            };
            // A range before the listing's first trade comes back with no timestamps at all.
            let dates = column(&history.date_path);
            let closes = column(&history.price_path);
            let (opens, highs, lows) = (column(open_path), column(high_path), column(low_path));
            let offset = series
                .pointer(&path(offset_path))
                .and_then(Value::as_i64)
                .unwrap_or(0);
            let at = |column: &[Value], index: usize| column.get(index).and_then(money);
            dates
                .iter()
                .enumerate()
                .filter_map(|(index, date)| {
                    Some(Point {
                        day: day_of(date, offset)?,
                        at: instant_of(date),
                        close: at(&closes, index)?,
                        open: at(&opens, index),
                        high: at(&highs, index),
                        low: at(&lows, index),
                    })
                })
                .collect()
        }
    };
    HistoryReading::Points(points)
}

/// A point's day: an ISO date or datetime's date part, or a number of Unix seconds, or
/// milliseconds above 10¹¹, moved by `offset` seconds to the provider's trading day.
fn day_of(value: &Value, offset: i64) -> Option<NaiveDate> {
    match value {
        Value::String(text) => text.get(..10)?.parse().ok(),
        Value::Number(number) => {
            let raw = number.as_f64()?;
            #[expect(
                clippy::cast_possible_truncation,
                reason = "a timestamp's fraction of a second never moves its day"
            )]
            let seconds = if raw > 1e11 { raw / 1000.0 } else { raw } as i64;
            DateTime::<Utc>::from_timestamp(seconds.checked_add(offset)?, 0)
                .map(|at| at.date_naive())
        }
        _ => None,
    }
}

/// A point's instant: a number of Unix seconds, or milliseconds above 10¹¹, or an RFC 3339
/// datetime. `None` for a bare date, which names a day but no hour.
fn instant_of(value: &Value) -> Option<DateTime<Utc>> {
    match value {
        Value::String(text) => DateTime::parse_from_rfc3339(text.trim())
            .ok()
            .map(|at| at.with_timezone(&Utc)),
        Value::Number(number) => {
            let raw = number.as_f64()?;
            #[expect(
                clippy::cast_possible_truncation,
                reason = "a timestamp's fraction of a second never moves its hour"
            )]
            let seconds = if raw > 1e11 { raw / 1000.0 } else { raw } as i64;
            DateTime::<Utc>::from_timestamp(seconds, 0)
        }
        _ => None,
    }
}

/// A price in a reply: a number, or a string holding one. `None` for anything else, such as the
/// `null` a market holiday leaves in a column.
fn money(value: &Value) -> Option<Money> {
    match value {
        Value::Number(number) => number.to_string().parse().ok(),
        Value::String(text) => text.trim().parse().ok(),
        _ => None,
    }
}

/// `points` folded into one Weekly Price per ISO week, written at `at`, oldest first: open from
/// the first point (its open, else its close), close from the last, high and low from the
/// extremes of every point's high and low, or its close where it has none. A week with no points
/// gets none.
pub(crate) fn fold(mut points: Vec<Point>, at: DateTime<Utc>) -> Vec<WeeklyPrice> {
    // Stable, so several points on one day keep the reply's order and the last is the close.
    points.sort_by_key(|point| point.day);
    let mut weeks: Vec<WeeklyPrice> = Vec::new();
    for point in points {
        let week = WeeklyPrice::week_of(point.day);
        let high = point.high.clone().unwrap_or_else(|| point.close.clone());
        let low = point.low.clone().unwrap_or_else(|| point.close.clone());
        match weeks.last_mut() {
            Some(folded) if folded.week == week => {
                folded.high = folded.high.take().map(|was| was.max(high));
                folded.low = folded.low.take().map(|was| was.min(low));
                folded.close = point.close;
            }
            _ => {
                let open = point.open.clone().unwrap_or_else(|| point.close.clone());
                weeks.push(WeeklyPrice {
                    week,
                    high: Some(high.max(open.clone()).max(point.close.clone())),
                    low: Some(low.min(open.clone()).min(point.close.clone())),
                    open: Some(open),
                    close: point.close,
                    written_at: at,
                });
            }
        }
    }
    for week in &mut weeks {
        // The close may sit outside the extremes a provider rounded its high and low to.
        week.high = week.high.take().map(|high| high.max(week.close.clone()));
        week.low = week.low.take().map(|low| low.min(week.close.clone()));
    }
    weeks
}

#[cfg(test)]
mod tests {
    use lib_core::{UnitId, UnitKind};

    use super::*;
    use crate::{COINGECKO, FRANKFURTER, Method, Precision, PriceLink, YAHOO_FINANCE, built_in};

    fn day(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).expect("a real date")
    }

    fn money(text: &str) -> Money {
        text.parse().expect("a decimal")
    }

    fn at() -> DateTime<Utc> {
        day(2026, 10, 21)
            .and_hms_opt(9, 0, 0)
            .expect("a real time")
            .and_utc()
    }

    fn history(id: lib_core::PriceSourceId) -> (&'static Endpoint, &'static HistoryEndpoint) {
        let endpoint = &built_in(id).expect("a built-in").endpoint;
        (
            endpoint,
            endpoint.history.as_deref().expect("a history request"),
        )
    }

    fn answered(body: &str) -> TransportOutcome {
        TransportOutcome::Answered {
            status: 200,
            body: body.into(),
            retry_after: None,
        }
    }

    fn week(monday: NaiveDate, open: &str, high: &str, low: &str, close: &str) -> WeeklyPrice {
        WeeklyPrice {
            week: monday,
            close: money(close),
            open: Some(money(open)),
            high: Some(money(high)),
            low: Some(money(low)),
            written_at: at(),
        }
    }

    fn folded(
        id: lib_core::PriceSourceId,
        body: &str,
        symbol: &str,
        quote: &str,
    ) -> Vec<WeeklyPrice> {
        let (_, history) = history(id);
        match read_history(&answered(body), history, symbol, quote) {
            HistoryReading::Points(points) => fold(points, at()),
            HistoryReading::Failed(failure) => panic!("the reply reads: {failure:?}"),
        }
    }

    #[test]
    fn coingeckos_history_request_asks_its_daily_range_in_iso_dates() {
        let (endpoint, history) = history(COINGECKO);

        let request = history_request(
            endpoint,
            history,
            "Bitcoin",
            "AUD",
            (day(2026, 9, 28), day(2026, 10, 18)),
            None,
        );

        assert_eq!(request.method, Method::Get);
        assert_eq!(
            request.url,
            "https://api.coingecko.com/api/v3/coins/bitcoin/market_chart/range?vs_currency=aud&from=2026-09-28&to=2026-10-18&interval=daily"
        );
    }

    #[test]
    fn yahoos_history_request_asks_daily_bars_from_a_day_early_in_unix_seconds() {
        let (endpoint, history) = history(YAHOO_FINANCE);

        let request = history_request(
            endpoint,
            history,
            "VAS.AX",
            "AUD",
            (day(2026, 9, 28), day(2026, 10, 18)),
            None,
        );

        // 27 Sep and 19 Oct 2026, 00:00 UTC.
        assert_eq!(
            request.url,
            "https://query1.finance.yahoo.com/v8/finance/chart/VAS.AX?period1=1790467200&period2=1792368000&interval=1d"
        );
        assert_eq!(request.headers.len(), 1, "its User-Agent goes too");
    }

    #[test]
    fn frankfurters_history_request_is_v2s_time_series() {
        let (endpoint, history) = history(FRANKFURTER);

        let request = history_request(
            endpoint,
            history,
            "EUR",
            "AUD",
            (day(2026, 9, 28), day(2026, 10, 18)),
            None,
        );

        assert_eq!(
            request.url,
            "https://api.frankfurter.dev/v2/rates?base=EUR&quotes=AUD&from=2026-09-28&to=2026-10-18"
        );
    }

    #[test]
    fn coingeckos_daily_closes_fold_into_weeks() {
        let weeks = folded(
            COINGECKO,
            include_str!("../../fixtures/history/coingecko_market_chart_range.json"),
            "bitcoin",
            "AUD",
        );

        assert_eq!(weeks.len(), 4, "three whole weeks and the Monday after");
        assert_eq!(
            weeks[1],
            week(
                day(2026, 10, 5),
                "124399.39645776771",
                "124399.39645776771",
                "117377.39161078232",
                "118725.50249262212",
            )
        );
    }

    #[test]
    fn yahoos_columns_fold_by_trading_day_taking_each_bars_open_high_and_low() {
        let weeks = folded(
            YAHOO_FINANCE,
            include_str!("../../fixtures/history/yahoo_chart_daily.json"),
            "VAS.AX",
            "AUD",
        );

        let mondays: Vec<_> = weeks.iter().map(|week| week.week).collect();
        assert_eq!(
            mondays,
            [
                day(2026, 9, 28),
                day(2026, 10, 5),
                day(2026, 10, 12),
                day(2026, 10, 19)
            ],
            "Monday's bar, stamped 23:00 UTC on Sunday, falls in its own week"
        );
        assert_eq!(
            weeks[0],
            week(
                day(2026, 9, 28),
                "108.69999694824219",
                "110.37999725341797",
                "106.63999938964844",
                "107.5199966430664",
            )
        );
        assert_eq!(
            weeks[1],
            week(
                day(2026, 10, 5),
                "107.86000061035156",
                "108.4000015258789",
                "107.19999694824219",
                "107.91999816894531",
            )
        );
    }

    #[test]
    fn frankfurters_rates_fold_into_weeks() {
        let weeks = folded(
            FRANKFURTER,
            include_str!("../../fixtures/history/frankfurter_rates.json"),
            "EUR",
            "AUD",
        );

        assert_eq!(weeks.len(), 3);
        assert_eq!(
            weeks[1],
            week(day(2026, 10, 5), "1.6154", "1.6154", "1.6092", "1.6092")
        );
    }

    #[test]
    fn a_week_with_no_points_gets_no_weekly_price() {
        let point = |on: NaiveDate, close: &str| Point {
            day: on,
            at: None,
            close: money(close),
            open: None,
            high: None,
            low: None,
        };

        let weeks = fold(
            vec![
                point(day(2026, 10, 16), "3"),
                point(day(2026, 9, 29), "1"),
                point(day(2026, 10, 13), "2"),
            ],
            at(),
        );

        assert_eq!(
            weeks,
            [
                week(day(2026, 9, 28), "1", "1", "1", "1"),
                week(day(2026, 10, 12), "2", "3", "2", "3"),
            ],
            "sorted by day, the week of 5 October left empty"
        );
    }

    #[test]
    fn a_range_before_a_listing_reads_as_no_points_not_a_failure() {
        let (_, history) = history(YAHOO_FINANCE);
        let body = r#"{"chart":{"result":[{"meta":{"gmtoffset":36000},"indicators":{"quote":[{}]}}],"error":null}}"#;

        assert_eq!(
            read_history(&answered(body), history, "VAS.AX", "AUD"),
            HistoryReading::Points(Vec::new())
        );
    }

    #[test]
    fn a_reply_without_its_series_is_a_source_error_and_statuses_read_as_a_price_does() {
        let (_, history) = history(COINGECKO);
        let read = |outcome: TransportOutcome| read_history(&outcome, history, "bitcoin", "AUD");

        assert_eq!(
            read(answered(r#"{"error":"coin not found"}"#)),
            HistoryReading::Failed(FetchFailure::SourceError)
        );
        assert_eq!(
            read(answered("<html>")),
            HistoryReading::Failed(FetchFailure::SourceError)
        );
        assert_eq!(
            read(TransportOutcome::Answered {
                status: 429,
                body: String::new(),
                retry_after: None,
            }),
            HistoryReading::Failed(FetchFailure::RateLimited)
        );
        assert_eq!(
            read(TransportOutcome::Answered {
                status: 404,
                body: String::new(),
                retry_after: None,
            }),
            HistoryReading::Failed(FetchFailure::NotFound)
        );
    }

    #[test]
    fn a_keyed_series_reads_its_dates_from_the_keys() {
        let history = HistoryEndpoint {
            request: "/history/{symbol}".into(),
            series_path: "/rates".into(),
            date_path: "".into(),
            price_path: "/{quote}".into(),
            shape: SeriesShape::Keyed,
        };
        let body = r#"{"rates":{"2026-10-06":{"AUD":1.61},"2026-10-05T00:00:00Z":{"AUD":"1.62"}}}"#;

        let HistoryReading::Points(points) = read_history(&answered(body), &history, "EUR", "AUD")
        else {
            panic!("the reply reads");
        };

        assert_eq!(
            fold(points, at()),
            [week(day(2026, 10, 5), "1.62", "1.62", "1.61", "1.61")]
        );
    }

    /// A Custom source's history, as a person types it on the form.
    fn custom(
        request: &str,
        series: &str,
        date: &str,
        price: &str,
        shape: SeriesShape,
    ) -> HistoryEndpoint {
        HistoryEndpoint {
            request: request.to_string().into(),
            series_path: series.to_string().into(),
            date_path: date.to_string().into(),
            price_path: price.to_string().into(),
            shape,
        }
    }

    #[test]
    fn a_custom_history_request_fills_iso_dates_and_unix_seconds() {
        let endpoint = Endpoint {
            address: "https://prices.example".to_string().into(),
            request: "/{symbol}".to_string().into(),
            price_path: "/price".to_string().into(),
            auth: crate::PriceSourceAuth::None,
            headers: Vec::new().into(),
            batch: None,
            history: None,
        };
        let history = custom(
            "/h/{symbol}?in={quote}&from={from}&to={to}&s={from_unix}&e={to_unix}",
            "",
            "/d",
            "/p",
            SeriesShape::Points,
        );

        let request = history_request(
            &endpoint,
            &history,
            "ABC",
            "AUD",
            (day(2026, 9, 28), day(2026, 10, 18)),
            None,
        );

        assert_eq!(
            request.url,
            "https://prices.example/h/ABC?in=AUD&from=2026-09-28&to=2026-10-18&s=1790467200&e=1792368000"
        );
    }

    #[test]
    fn a_custom_points_series_dates_each_point_by_iso_date_datetime_seconds_or_milliseconds() {
        let history = custom("/h", "/data", "/t", "/close", SeriesShape::Points);
        // 6, 7 and 8 October 2026 as an ISO datetime, Unix seconds and milliseconds.
        let body = r#"{"data":[
            {"t":"2026-10-05","close":1},
            {"t":"2026-10-06T23:30:00+10:00","close":"2"},
            {"t":1791331200,"close":3},
            {"t":1791417600000,"close":4},
            {"t":"not a date","close":9},
            {"t":"2026-10-09","close":null}
        ]}"#;

        let HistoryReading::Points(points) = read_history(&answered(body), &history, "ABC", "AUD")
        else {
            panic!("the reply reads");
        };
        let days: Vec<_> = points.iter().map(|point| point.day).collect();

        assert_eq!(
            days,
            [
                day(2026, 10, 5),
                day(2026, 10, 6),
                day(2026, 10, 7),
                day(2026, 10, 8)
            ],
            "an unreadable date or price drops only its point"
        );
        assert_eq!(
            fold(points, at()),
            [week(day(2026, 10, 5), "1", "4", "1", "4")],
            "close only: the week's open, high and low fold from the closes"
        );
    }

    #[test]
    fn a_custom_coingecko_shaped_series_reads_pairs_at_slash_0_and_slash_1() {
        let history = custom("/h", "/prices", "/0", "/1", SeriesShape::Points);
        let body = r#"{"prices":[[1791158400000,10],[1791244800000,12],[1791244800000,11]]}"#;

        let HistoryReading::Points(points) = read_history(&answered(body), &history, "x", "AUD")
        else {
            panic!("the reply reads");
        };

        assert_eq!(
            fold(points, at()),
            [week(day(2026, 10, 5), "10", "12", "10", "11")],
            "two points on one day fold as a week does, the last the close"
        );
    }

    fn unit(prices: &[NaiveDate]) -> UnitRow {
        let mut unit = UnitRow::new(
            "EUR".into(),
            "Euro".into(),
            UnitKind::Currency,
            Precision::of_kind(UnitKind::Currency),
            PriceLink {
                source_id: FRANKFURTER,
                symbol: "EUR".into(),
                quote: UnitId::SEED_AUD,
            },
        );
        unit.prices = prices
            .iter()
            .map(|monday| week(*monday, "1", "1", "1", "1"))
            .collect();
        unit
    }

    #[test]
    fn the_floor_is_the_earliest_opened_account_else_the_creation_date() {
        let mut eur = unit(&[]);
        assert_eq!(floor(&eur, None), None);
        eur.created_on = Some(day(2024, 3, 1));
        assert_eq!(floor(&eur, None), Some(day(2024, 3, 1)));
        assert_eq!(
            floor(&eur, Some(day(2019, 6, 1))),
            Some(day(2019, 6, 1)),
            "an Account opened before the Unit was created"
        );
    }

    #[test]
    fn the_wanted_weeks_run_from_the_floors_week_to_last_week() {
        let today = day(2026, 10, 21);

        assert_eq!(
            wanted_weeks(day(2019, 6, 5), today, None),
            Some((day(2019, 6, 3), day(2026, 10, 12)))
        );
        assert_eq!(
            wanted_weeks(day(2026, 10, 19), today, None),
            None,
            "a floor in the current week wants nothing"
        );
    }

    #[test]
    fn a_history_depth_reaches_back_only_the_whole_weeks_inside_it_whatever_the_day() {
        for today in [day(2026, 10, 19), day(2026, 10, 21), day(2026, 10, 25)] {
            let (first, last) =
                wanted_weeks(day(2019, 6, 5), today, Some(365)).expect("weeks wanted");
            assert_eq!(first, day(2025, 10, 27), "{today}");
            assert_eq!(last, day(2026, 10, 12));
            assert!(today - first <= Duration::days(365));
        }
    }

    #[test]
    fn the_gaps_are_the_wanted_weeks_with_no_price_and_not_known_empty() {
        let eur = unit(&[day(2026, 9, 28)]);
        let empty = HashSet::from([day(2026, 9, 14)]);

        assert_eq!(
            gaps(&eur, (day(2026, 9, 7), day(2026, 10, 12)), &empty),
            [
                day(2026, 9, 7),
                day(2026, 9, 21),
                day(2026, 10, 5),
                day(2026, 10, 12)
            ]
        );
    }
}
