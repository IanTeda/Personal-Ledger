//! How a fetch's outcome reads (#1387): a [`FetchFailure`]'s name and detail, the Last fetch fact,
//! and the Warning Toast a Fetch now that failed raises. Shared by the page, its pane and the
//! fetched-prices Presenter so every place says the same thing.

use chrono::{DateTime, Utc};

use super::ago;
use crate::fetching::{FetchFailure, FetchOutcome, LastFetch};
use crate::msg;

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

/// When a backed-off fetch is tried again: `tries again in 11 minutes`, rounded up.
fn retry_text(retry_at: DateTime<Utc>, now: DateTime<Utc>) -> String {
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
