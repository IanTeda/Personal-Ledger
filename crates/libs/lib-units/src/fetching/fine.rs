//! Daily and Hourly Prices (`GLOSSARY.md`): the grains finer than a week that the Unit prices view
//! asks a Unit's source for, to show and never to store. Their windows, and folding a history
//! reply's points into one price per day or per hour. A day is the provider's own (the day its
//! points are dated by); an hour is the person's local hour, read through the Clock.

use chrono::{Duration, NaiveDate, NaiveDateTime, NaiveTime, Timelike as _};
use lib_core::{Clock, Money, UnitId};

use super::history::Point;

/// How finely the Unit prices view shows a Unit's prices: its Weekly Prices from the Ledger, or
/// the Daily or Hourly Prices its source offers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Grain {
    Week,
    Day,
    Hour,
}

impl Grain {
    /// The grain after this one among `offered`, Week → Day → Hour → Week, skipping the grains a
    /// source lacks; itself when it is the only one.
    pub fn next_in(self, offered: &[Grain]) -> Grain {
        let order = [Grain::Week, Grain::Day, Grain::Hour];
        let at = order.iter().position(|grain| *grain == self).unwrap_or(0);
        (1..=order.len())
            .map(|step| order[(at + step) % order.len()])
            .find(|grain| offered.contains(grain))
            .unwrap_or(self)
    }
}

/// How long a window that holds today stays fresh before showing it fetches again. A window that
/// ended before today stays for the Ledger's session.
pub const FINE_FRESH: Duration = Duration::minutes(15);

/// The days a Day window holds: 13 weeks, about 3 months.
const DAY_WINDOW_DAYS: i64 = 91;
/// The days an Hour window holds.
const HOUR_WINDOW_DAYS: i64 = 7;

/// One Daily or Hourly Price: the start of its day or hour, and a close with an optional open,
/// high and low, as a Weekly Price has.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FinePrice {
    pub start: NaiveDateTime,
    pub open: Option<Money>,
    pub high: Option<Money>,
    pub low: Option<Money>,
    pub close: Money,
}

/// Names one cached series: a Unit's prices at a grain in the window starting `first`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) struct FineKey {
    pub(super) unit: UnitId,
    pub(super) grain: Grain,
    pub(super) first: NaiveDate,
}

/// The first and last day of the Day or Hour window `steps` steps before the one ending `today`:
/// a Day window is 13 weeks stepped a week, an Hour window 7 days stepped a day. Week windows are
/// the Unit prices view's own.
pub(crate) fn window(grain: Grain, today: NaiveDate, steps: u32) -> (NaiveDate, NaiveDate) {
    let (days, step) = match grain {
        Grain::Hour => (HOUR_WINDOW_DAYS, 1),
        Grain::Day | Grain::Week => (DAY_WINDOW_DAYS, 7),
    };
    let last = today - Duration::days(step * i64::from(steps));
    (last - Duration::days(days - 1), last)
}

/// `points` folded into one price per day, or per `clock`'s local hour, from `first` to `last`,
/// oldest first. A period of several points, or of one carrying an open, high or low, is a candle,
/// opening at its first point and closing at its last; a lone close-only point is a tick. A period
/// with no points is absent. An hour needs a point's instant, so a bare date never makes one.
pub(super) fn fine_prices(
    points: Vec<Point>,
    grain: Grain,
    (first, last): (NaiveDate, NaiveDate),
    clock: &Clock,
) -> Vec<FinePrice> {
    let mut keyed: Vec<(NaiveDateTime, Point)> = points
        .into_iter()
        .filter_map(|point| {
            let start = match grain {
                Grain::Hour => {
                    let local = clock.local_time(point.at?);
                    local
                        .date()
                        .and_time(NaiveTime::from_hms_opt(local.hour(), 0, 0)?)
                }
                Grain::Day | Grain::Week => point.day.and_time(NaiveTime::MIN),
            };
            (first..=last)
                .contains(&start.date())
                .then_some((start, point))
        })
        .collect();
    // Stable, so several points in one period keep the reply's order and the last is its close.
    keyed.sort_by_key(|(start, _)| *start);
    let mut prices: Vec<(FinePrice, bool)> = Vec::new();
    for (start, point) in keyed {
        match prices.last_mut() {
            Some((price, candle)) if price.start == start => {
                let high = point.high.clone().unwrap_or_else(|| point.close.clone());
                let low = point.low.clone().unwrap_or_else(|| point.close.clone());
                let close = price.close.clone();
                price.open = Some(price.open.take().unwrap_or(close.clone()));
                price.high = Some(price.high.take().unwrap_or(close.clone()).max(high));
                price.low = Some(price.low.take().unwrap_or(close).min(low));
                price.close = point.close;
                *candle = true;
            }
            _ => {
                let candle = point.open.is_some() || point.high.is_some() || point.low.is_some();
                prices.push((
                    FinePrice {
                        start,
                        open: point.open,
                        high: point.high,
                        low: point.low,
                        close: point.close,
                    },
                    candle,
                ));
            }
        }
    }
    prices
        .into_iter()
        .map(|(mut price, candle)| {
            if candle {
                // A candle has all four, and the close may sit outside a rounded high or low.
                let close = price.close.clone();
                let open = price.open.take().unwrap_or(close.clone());
                price.high = Some(
                    price
                        .high
                        .take()
                        .unwrap_or(close.clone())
                        .max(close.clone())
                        .max(open.clone()),
                );
                price.low = Some(
                    price
                        .low
                        .take()
                        .unwrap_or(close.clone())
                        .min(close)
                        .min(open.clone()),
                );
                price.open = Some(open);
            }
            price
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use chrono::{DateTime, FixedOffset, Utc};

    use super::*;

    fn day(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).expect("a valid date")
    }

    fn money(text: &str) -> Money {
        text.parse().expect("a number")
    }

    fn close_only(on: NaiveDate, at: Option<&str>, close: &str) -> Point {
        Point {
            day: on,
            at: at.map(|at| {
                DateTime::parse_from_rfc3339(at)
                    .expect("an instant")
                    .with_timezone(&Utc)
            }),
            close: money(close),
            open: None,
            high: None,
            low: None,
        }
    }

    #[test]
    fn z_cycles_week_day_hour_skipping_what_a_source_lacks() {
        let all = [Grain::Week, Grain::Day, Grain::Hour];
        assert_eq!(Grain::Week.next_in(&all), Grain::Day);
        assert_eq!(Grain::Day.next_in(&all), Grain::Hour);
        assert_eq!(Grain::Hour.next_in(&all), Grain::Week);
        let daily = [Grain::Week, Grain::Day];
        assert_eq!(Grain::Day.next_in(&daily), Grain::Week);
        assert_eq!(Grain::Week.next_in(&[Grain::Week]), Grain::Week);
    }

    #[test]
    fn a_day_window_is_13_weeks_stepped_a_week_and_an_hour_window_7_days_stepped_a_day() {
        let today = day(2026, 10, 21);
        assert_eq!(
            window(Grain::Day, today, 0),
            (day(2026, 7, 23), day(2026, 10, 21))
        );
        assert_eq!(
            window(Grain::Day, today, 1),
            (day(2026, 7, 16), day(2026, 10, 14))
        );
        assert_eq!(
            window(Grain::Hour, today, 0),
            (day(2026, 10, 15), day(2026, 10, 21))
        );
        assert_eq!(
            window(Grain::Hour, today, 2),
            (day(2026, 10, 13), day(2026, 10, 19))
        );
    }

    #[test]
    fn days_keep_the_providers_day_and_its_last_close_and_a_missing_day_is_absent() {
        let points = vec![
            close_only(day(2026, 10, 19), None, "100"),
            close_only(day(2026, 10, 21), None, "102"),
            // CoinGecko's last point is now, on the same UTC day as its midnight one.
            close_only(day(2026, 10, 21), None, "103"),
            close_only(day(2026, 10, 30), None, "999"),
        ];
        let prices = fine_prices(
            points,
            Grain::Day,
            (day(2026, 10, 19), day(2026, 10, 21)),
            &Clock::fixed(day(2026, 10, 21)),
        );
        let days: Vec<_> = prices.iter().map(|price| price.start.date()).collect();
        assert_eq!(days, [day(2026, 10, 19), day(2026, 10, 21)]);
        assert_eq!(prices[0].open, None, "a lone close-only point is a tick");
        assert_eq!(prices[1].open, Some(money("102")));
        assert_eq!(prices[1].close, money("103"));
    }

    #[test]
    fn hours_bucket_by_the_local_hour_and_fold_bars_into_a_candle() {
        let points = vec![
            close_only(day(2026, 10, 20), Some("2026-10-20T23:05:00Z"), "10"),
            close_only(day(2026, 10, 20), Some("2026-10-20T23:40:00Z"), "12"),
            close_only(day(2026, 10, 21), Some("2026-10-21T00:10:00Z"), "11"),
        ];
        let sydney = FixedOffset::east_opt(11 * 3600).expect("a valid offset");
        // Now only fixes the day: 21 Oct 09:30 UTC is 20:30 the same day in Sydney.
        let clock = Clock::new(
            || {
                day(2026, 10, 21)
                    .and_hms_opt(9, 30, 0)
                    .expect("a real time")
                    .and_utc()
            },
            move |at| at.with_timezone(&sydney).naive_local(),
        );
        let prices = fine_prices(
            points,
            Grain::Hour,
            (day(2026, 10, 21), day(2026, 10, 21)),
            &clock,
        );
        let starts: Vec<_> = prices
            .iter()
            .map(|price| price.start.format("%Y-%m-%d %H:%M").to_string())
            .collect();
        assert_eq!(starts, ["2026-10-21 10:00", "2026-10-21 11:00"]);
        assert_eq!(prices[0].open, Some(money("10")));
        assert_eq!(prices[0].high, Some(money("12")));
        assert_eq!(prices[0].low, Some(money("10")));
        assert_eq!(prices[0].close, money("12"));
        assert_eq!(prices[1].open, None);
    }

    #[test]
    fn a_bare_date_makes_no_hour() {
        let prices = fine_prices(
            vec![close_only(day(2026, 10, 21), None, "1")],
            Grain::Hour,
            (day(2026, 10, 21), day(2026, 10, 21)),
            &Clock::fixed(day(2026, 10, 21)),
        );
        assert!(prices.is_empty());
    }
}
