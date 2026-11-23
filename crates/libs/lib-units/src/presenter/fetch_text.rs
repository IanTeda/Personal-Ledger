//! How a fetch's outcome reads (#1387): a [`FetchFailure`]'s name and detail, the Last fetch fact,
//! and the Warning Toast a Fetch now that failed raises. Shared by the page, its pane and the
//! fetched-prices Presenter so every place says the same thing.

use chrono::{DateTime, Local, Utc};
use lib_locale::format::format_time_minutes;
use lib_toast::{Notice, ToastKind};

use super::ago;
use super::units_pane::amount;
use crate::UnitRow;
use crate::fetching::{
    FetchFailure, FetchOutcome, LastFetch, PriceFetching, TestOutcome, TestResult,
};
use crate::msg;
use crate::protocols::UnitsPorts;

/// The reason's name: `Not found`.
pub(super) fn failure_name(reason: FetchFailure) -> String {
    match reason {
        FetchFailure::Offline => msg::units_fetch_failure_offline(),
        FetchFailure::RateLimited => msg::units_fetch_failure_rate_limited(),
        FetchFailure::KeyMissing => msg::units_fetch_failure_key_missing(),
        FetchFailure::KeyRejected => msg::units_fetch_failure_key_rejected(),
        FetchFailure::SourceError => msg::units_fetch_failure_source_error(),
        FetchFailure::NotFound => msg::units_fetch_failure_not_found(),
        FetchFailure::NoPrice => msg::units_fetch_failure_no_price(),
    }
}

/// What the Unit fetched: its symbol and its quote Unit's code. A source's own Last fetch has
/// none, and shows only round reasons, whose detail names no symbol.
#[derive(Debug, Clone, Copy)]
pub(super) struct Asked<'a> {
    pub symbol: &'a str,
    pub quote: &'a str,
}

/// What the reason means on `source`: `Yahoo Finance has no BHP.AX in AUD`.
pub(super) fn failure_detail(
    reason: FetchFailure,
    source: &str,
    asked: Option<Asked<'_>>,
) -> String {
    let (symbol, quote) = asked.map_or(("", ""), |asked| (asked.symbol, asked.quote));
    match reason {
        FetchFailure::Offline => msg::units_fetch_detail_offline(source),
        FetchFailure::RateLimited => msg::units_fetch_detail_rate_limited(source),
        FetchFailure::KeyMissing => msg::units_fetch_detail_key_missing(source),
        FetchFailure::KeyRejected => msg::units_fetch_detail_key_rejected(source),
        FetchFailure::SourceError => msg::units_fetch_detail_source_error(source),
        FetchFailure::NotFound => msg::units_fetch_detail_not_found(source, symbol, quote),
        FetchFailure::NoPrice => msg::units_fetch_detail_no_price(source, symbol, quote),
    }
}

/// A round failure as a Toast phrases it after the source: `rate limited, try again later`. A
/// Unit failure never fails a round, so it reads as a source error should one arrive.
fn round_phrase(reason: FetchFailure) -> String {
    match reason {
        FetchFailure::Offline => msg::units_fetch_round_offline(),
        FetchFailure::RateLimited => msg::units_fetch_round_rate_limited(),
        FetchFailure::KeyMissing => msg::units_fetch_round_key_missing(),
        FetchFailure::KeyRejected => msg::units_fetch_round_key_rejected(),
        FetchFailure::SourceError | FetchFailure::NotFound | FetchFailure::NoPrice => {
            msg::units_fetch_round_source_error()
        }
    }
}

/// The status line while asked-for rounds are out, naming their sources in the order they list
/// (`Fetching prices from CoinGecko, Yahoo Finance…`), then while a fetch now's history is out
/// (`Fetching price history for BTC from CoinGecko…`); `None` once everything has landed.
fn fetching_status(ports: &dyn UnitsPorts, fetching: &PriceFetching) -> Option<String> {
    let out = fetching.named_sources();
    let names: Vec<String> = ports
        .source_catalogue()
        .sources()
        .iter()
        .filter(|source| out.contains(&source.id))
        .map(|source| source.name.to_string())
        .collect();
    if !names.is_empty() {
        return Some(msg::units_status_fetching(&names.join(", ")));
    }
    // Once its rounds land, a fetch now names the first history it is still waiting on.
    let (code, source) = fetching.asked_history().into_iter().next()?;
    let catalogue = ports.source_catalogue();
    let name = catalogue.source(source)?.name.to_string();
    Some(msg::units_status_fetching_history(&code, &name))
}

/// `Waiting for CoinGecko, 40 s…` while a call a person asked for waits out a `Retry-After` at
/// `now`; `None` while none does.
pub(super) fn waiting_status(
    ports: &dyn UnitsPorts,
    fetching: &PriceFetching,
    now: DateTime<Utc>,
) -> Option<String> {
    let (source, seconds) = fetching.waiting(now)?;
    let catalogue = ports.source_catalogue();
    let name = catalogue.source(source)?.name.to_string();
    Some(msg::units_status_fetch_waiting(&name, &seconds.to_string()))
}

/// What the status line says of fetching at `now`: a `Retry-After` being waited out first, else
/// the asked-for rounds still out; `None` once there is neither.
pub(super) fn fetch_status(
    ports: &dyn UnitsPorts,
    fetching: &PriceFetching,
    now: DateTime<Utc>,
) -> Option<String> {
    waiting_status(ports, fetching, now).or_else(|| fetching_status(ports, fetching))
}

/// `Fetched 2 prices from CoinGecko; then rate limited, try again later`: a round a round failure
/// stopped after it had written some prices.
pub(super) fn round_stopped_toast(fetched: usize, reason: FetchFailure, source: &str) -> String {
    msg::units_toast_fetch_round_stopped(
        lib_viper::view::count(fetched),
        source,
        &round_phrase(reason),
    )
}

/// `Couldn't fetch from CoinGecko: rate limited, try again later`.
pub(super) fn round_failed_toast(reason: FetchFailure, source: &str) -> String {
    msg::units_toast_fetch_round_failed(source, &round_phrase(reason))
}

/// `Couldn't fetch BHP: Yahoo Finance has no BHP.AX in AUD`.
pub(super) fn unit_failed_toast(
    code: &str,
    reason: FetchFailure,
    source: &str,
    asked: Asked<'_>,
) -> String {
    msg::units_toast_fetch_unit_failed(code, &failure_detail(reason, source, Some(asked)))
}

/// `Fetched 3 prices from Yahoo Finance; 1 not found`: what failed named when every failure shares
/// a Unit reason.
pub(super) fn some_failed_toast(fetched: usize, failed: &[FetchFailure], source: &str) -> String {
    let what = match failed {
        [first, rest @ ..] if rest.iter().all(|reason| reason == first) => match first {
            FetchFailure::NotFound => msg::units_fetch_some_not_found(),
            FetchFailure::NoPrice => msg::units_fetch_some_no_price(),
            _ => msg::units_fetch_some_failed(),
        },
        _ => msg::units_fetch_some_failed(),
    };
    msg::units_toast_fetch_some_failed(
        lib_viper::view::count(fetched),
        source,
        &failed.len().to_string(),
        &what,
    )
}

/// The Last fetch fact: `Fetched 4 minutes ago`, or `Failed 4 minutes ago · Not found: Yahoo
/// Finance has no BHP.AX in AUD · tries again in 11 minutes`.
pub(super) fn last_fetch_text(
    last: &LastFetch,
    now: DateTime<Utc>,
    source: &str,
    asked: Option<Asked<'_>>,
) -> String {
    match last.outcome {
        FetchOutcome::Fetched => msg::units_last_fetch_fetched(&ago(last.at, now)),
        FetchOutcome::Failed { reason, retry_at } => msg::units_last_fetch_failed(
            &ago(last.at, now),
            &failure_name(reason),
            &failure_detail(reason, source, asked),
            &retry_text(retry_at, now),
        ),
    }
}

/// How long a test's answer took, in whole milliseconds.
fn millis(result: &TestResult) -> String {
    result.elapsed.num_milliseconds().max(0).to_string()
}

/// What the test asked, for a failure's detail.
fn test_asked(result: &TestResult) -> Asked<'_> {
    Asked {
        symbol: &result.asked.symbol,
        quote: &result.asked.quote_code,
    }
}

/// A landed test as its form shows it: `Price found · 412 ms` over `VAS.AX · 101.84 AUD · tested
/// 14:36`, or a reason's name over its detail. `quote` is the quote Unit, for the price's places.
pub(super) fn test_lines(result: &TestResult, quote: Option<&UnitRow>) -> (String, String) {
    match &result.outcome {
        TestOutcome::Priced(price) => (
            msg::units_price_source_test_found(&millis(result)),
            msg::units_price_source_test_found_detail(
                &result.asked.symbol,
                &amount(price, quote),
                &format_time_minutes(result.at.with_timezone(&Local).time()),
            ),
        ),
        TestOutcome::Failed(reason) => (
            failure_name(*reason),
            failure_detail(*reason, &result.asked.source_name, Some(test_asked(result))),
        ),
    }
}

/// The one Toast `t` on a row answers with: Info on a price, Warning naming the reason.
pub(super) fn test_toast(result: &TestResult, quote: Option<&UnitRow>) -> Notice {
    let source = &result.asked.source_name;
    match &result.outcome {
        TestOutcome::Priced(price) => Notice::Toast(
            ToastKind::Info,
            msg::units_toast_test_found(
                source,
                &result.asked.symbol,
                &amount(price, quote),
                &millis(result),
            ),
        ),
        TestOutcome::Failed(reason) => test_failed_toast(*reason, source, test_asked(result)),
    }
}

/// `CoinGecko test: Not found · CoinGecko has no bitcon in AUD`.
pub(super) fn test_failed_toast(reason: FetchFailure, source: &str, asked: Asked<'_>) -> Notice {
    Notice::Toast(
        ToastKind::Warning,
        msg::units_toast_test_failed(
            source,
            &failure_name(reason),
            &failure_detail(reason, source, Some(asked)),
        ),
    )
}

/// When a backed-off fetch is tried again: `tries again in 11 minutes`, rounded up.
pub(super) fn retry_text(retry_at: DateTime<Utc>, now: DateTime<Utc>) -> String {
    let seconds = (retry_at - now).num_seconds();
    if seconds <= 0 {
        return msg::units_retry_next_check();
    }
    let minutes = (seconds + 59) / 60;
    if minutes < 60 {
        msg::units_retry_minutes(minutes)
    } else {
        msg::units_retry_hours((minutes + 59) / 60)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::presenter::test_support::locale;

    #[test]
    fn a_round_with_some_failures_counts_them_naming_a_shared_reason() {
        locale();
        let not_found = [FetchFailure::NotFound];
        assert_eq!(
            some_failed_toast(3, &not_found, "Yahoo Finance"),
            "Fetched 3 prices from Yahoo Finance; 1 not found"
        );
        let mixed = [FetchFailure::NotFound, FetchFailure::NoPrice];
        assert_eq!(
            some_failed_toast(1, &mixed, "Yahoo Finance"),
            "Fetched 1 price from Yahoo Finance; 2 failed"
        );
    }
}
