//! Building a provider's requests: filling an Endpoint's placeholders, carrying its key and
//! headers, and splitting a round into calls.

use chrono::{DateTime, NaiveDate, Utc};
use lib_core::UnitId;
use secrecy::SecretString;

use super::{InFlight, PriceRequest, RequestKey, SourceSettings, UnitCall};
use crate::{Endpoint, PriceLink, PriceSourceAuth, UnitRow, Updates, WeeklyPrice};

/// Whether `endpoint` needs an API key and `key`, the one this Client reads for its source, is
/// none: a missing key or one that won't decrypt here. A source whose key is optional, as
/// CoinGecko's is, is asked keyless instead.
pub(crate) fn key_missing(endpoint: &Endpoint, key: Option<&SecretString>) -> bool {
    key.is_none()
        && matches!(
            endpoint.auth,
            PriceSourceAuth::ApiKey {
                optional: false,
                ..
            }
        )
}

/// Whether `unit`, on a source that updates every `updates`, is due a scheduled fetch on `today`
/// at `now`: it has no Weekly Price for the current week, or that price was written at least the
/// interval ago. The current week's price, not the newest written, decides, since a backfilled
/// past week carries a recent `written_at` but no current price. The Ledger alone decides, so a
/// price another device fetched and synced counts as fresh. Manual only is never due.
pub(crate) fn is_due(
    unit: &UnitRow,
    updates: Updates,
    today: NaiveDate,
    now: DateTime<Utc>,
) -> bool {
    let Some(interval) = updates.interval() else {
        return false;
    };
    let week = WeeklyPrice::week_of(today);
    // The current week, when it has a price, is the newest: prices are kept oldest week first.
    unit.prices
        .iter()
        .rev()
        .find(|price| price.week == week)
        .is_none_or(|current| now - current.written_at >= interval)
}

/// `text` with `{symbol}` and `{quote}` filled from a link, and `{symbol_lower}` and
/// `{quote_lower}` with the same in lower case.
pub(crate) fn fill(text: &str, symbol: &str, quote: &str) -> String {
    text.replace("{symbol}", symbol)
        .replace("{quote}", quote)
        .replace("{symbol_lower}", &symbol.to_lowercase())
        .replace("{quote_lower}", &quote.to_lowercase())
}

/// `text` with the list placeholders `{symbols}` and `{quotes}` filled comma-joined, and
/// `{symbols_lower}` and `{quotes_lower}` with the same in lower case.
fn fill_lists(text: &str, symbols: &[&str], quotes: &[&str]) -> String {
    let lower = |list: &[&str]| list.join(",").to_lowercase();
    text.replace("{symbols_lower}", &lower(symbols))
        .replace("{quotes_lower}", &lower(quotes))
        .replace("{symbols}", &symbols.join(","))
        .replace("{quotes}", &quotes.join(","))
}

/// The request asking `endpoint` for `symbol`'s price in `quote` (the quote Unit's code), carrying
/// the endpoint's fixed headers and `key` where its authentication places it, and the price path
/// filled the same way. A source with a batch request asks through it, as a batch of one. With no
/// key, a source whose key is optional is asked keyless.
pub(crate) fn price_request(
    endpoint: &Endpoint,
    symbol: &str,
    quote: &str,
    key: Option<SecretString>,
) -> (PriceRequest, String) {
    let path_and_query = match &endpoint.batch {
        Some(batch) => fill_lists(&batch.request, &[symbol], &[quote]),
        None => fill(&endpoint.request, symbol, quote),
    };
    (
        request(endpoint, &path_and_query, key),
        fill(&endpoint.price_path, symbol, quote),
    )
}

/// One Unit a round wants priced: its id and link, and its quote Unit's code.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Wanted {
    pub unit: UnitId,
    pub link: PriceLink,
    pub quote: String,
}

impl Wanted {
    /// `unit` as a round wants it, its quote Unit found among `units`: `None` when that Unit is
    /// gone.
    pub(crate) fn of(unit: &UnitRow, units: &[UnitRow]) -> Option<Self> {
        let quote = units.iter().find(|quote| quote.id == unit.link.quote)?;
        Some(Self {
            unit: unit.id,
            link: unit.link.clone(),
            quote: quote.code.clone(),
        })
    }
}

/// The calls a round on `endpoint` makes for `wanted`, in order: one per Unit, or, for a source
/// with a batch request, one per chunk of at most its `max_symbols` distinct symbols, each asking
/// every quote its Units need. Each Unit's price is read at its own filled price path.
pub(crate) fn round_calls(
    endpoint: &Endpoint,
    wanted: &[Wanted],
    key: Option<&SecretString>,
    settings: &SourceSettings,
) -> Vec<(PriceRequest, InFlight)> {
    let call = |wanted: &Wanted| UnitCall {
        unit: wanted.unit,
        link: wanted.link.clone(),
        price_path: fill(&endpoint.price_path, &wanted.link.symbol, &wanted.quote),
    };
    let Some(batch) = &endpoint.batch else {
        return wanted
            .iter()
            .map(|one| {
                let path_and_query = fill(&endpoint.request, &one.link.symbol, &one.quote);
                let request = request(endpoint, &path_and_query, key.cloned());
                let sent = InFlight {
                    units: vec![call(one)],
                    settings: settings.clone(),
                };
                (request, sent)
            })
            .collect();
    };
    let symbols = distinct(wanted.iter().map(|one| one.link.symbol.as_str()));
    symbols
        .chunks(batch.max_symbols.max(1))
        .map(|chunk| {
            let units: Vec<&Wanted> = wanted
                .iter()
                .filter(|one| chunk.contains(&one.link.symbol.as_str()))
                .collect();
            let quotes = distinct(units.iter().map(|one| one.quote.as_str()));
            let request = request(
                endpoint,
                &fill_lists(&batch.request, chunk, &quotes),
                key.cloned(),
            );
            let sent = InFlight {
                units: units.into_iter().map(call).collect(),
                settings: settings.clone(),
            };
            (request, sent)
        })
        .collect()
}

/// `items` in their first-seen order, each once.
fn distinct<'a>(items: impl Iterator<Item = &'a str>) -> Vec<&'a str> {
    let mut seen = Vec::new();
    for item in items {
        if !seen.contains(&item) {
            seen.push(item);
        }
    }
    seen
}

/// A GET of `path_and_query` on `endpoint`'s address with its fixed headers, and `key` where its
/// authentication places it.
pub(crate) fn request(
    endpoint: &Endpoint,
    path_and_query: &str,
    key: Option<SecretString>,
) -> PriceRequest {
    let key = match (&endpoint.auth, key) {
        (
            PriceSourceAuth::ApiKey {
                placement, name, ..
            },
            Some(value),
        ) => Some(RequestKey {
            placement: *placement,
            name: name.to_string(),
            value,
        }),
        (PriceSourceAuth::None, _) | (PriceSourceAuth::ApiKey { .. }, None) => None,
    };
    PriceRequest {
        url: format!(
            "{}{}",
            endpoint.address.trim_end_matches('/'),
            path_and_query
        ),
        headers: endpoint
            .headers
            .iter()
            .map(|header| (header.name.to_string(), header.value.to_string()))
            .collect(),
        key,
    }
}

#[cfg(test)]
mod tests {
    use secrecy::ExposeSecret as _;

    use super::*;
    use crate::{COINGECKO, FRANKFURTER, KeyPlacement, RequestHeader, YAHOO_FINANCE, built_in};

    #[test]
    fn frankfurters_request_is_v2_rate_for_the_symbol_in_the_quote() {
        let endpoint = &built_in(FRANKFURTER).expect("a built-in").endpoint;

        let (request, path) = price_request(endpoint, "EUR", "AUD", None);

        assert_eq!(
            request,
            PriceRequest {
                url: "https://api.frankfurter.dev/v2/rate/EUR/AUD".into(),
                headers: Vec::new(),
                key: None,
            }
        );
        assert_eq!(path, "/rate");
    }

    #[test]
    fn coingeckos_request_and_price_path_read_its_lower_case_keys_through_quote_lower() {
        let endpoint = &built_in(COINGECKO).expect("a built-in").endpoint;

        let (request, path) = price_request(endpoint, "Bitcoin", "AUD", None);

        assert_eq!(
            request,
            PriceRequest {
                url: "https://api.coingecko.com/api/v3/simple/price?ids=bitcoin&vs_currencies=aud"
                    .into(),
                headers: Vec::new(),
                key: None,
            },
            "keyless while no key is stored, so no key header"
        );
        assert_eq!(path, "/bitcoin/aud");
    }

    #[test]
    fn coingecko_with_a_demo_key_sends_it_in_its_header() {
        let endpoint = &built_in(COINGECKO).expect("a built-in").endpoint;

        let (request, _) = price_request(endpoint, "bitcoin", "AUD", Some("CG-demo".into()));

        assert_eq!(
            request.key,
            Some(RequestKey {
                placement: KeyPlacement::Header,
                name: "x-cg-demo-api-key".into(),
                value: "CG-demo".into(),
            })
        );
        assert!(!request.url.contains("CG-demo"));
    }

    #[test]
    fn a_keyed_custom_sources_key_goes_in_its_parameter_but_never_in_the_url_or_debug() {
        let endpoint = Endpoint {
            address: "https://prices.example".to_string().into(),
            request: "/q?s={symbol}".to_string().into(),
            price_path: "/price".to_string().into(),
            auth: PriceSourceAuth::ApiKey {
                placement: KeyPlacement::Param,
                name: "apikey".to_string().into(),
                optional: false,
            },
            headers: Vec::new().into(),
            batch: None,
            history: None,
        };

        let (request, _) = price_request(&endpoint, "ABC", "USD", Some("s3cr3t".into()));

        let key = request.key.as_ref().expect("the key goes with it");
        assert_eq!(
            (key.placement, key.name.as_str(), key.value.expose_secret()),
            (KeyPlacement::Param, "apikey", "s3cr3t")
        );
        assert_eq!(request.url, "https://prices.example/q?s=ABC");
        assert!(!format!("{request:?}").contains("s3cr3t"));
    }

    #[test]
    fn a_source_with_no_authentication_sends_no_key_even_if_one_is_saved() {
        let endpoint = &built_in(FRANKFURTER).expect("a built-in").endpoint;

        let (request, _) = price_request(endpoint, "EUR", "AUD", Some("stray".into()));

        assert_eq!(request.key, None);
    }

    #[test]
    fn yahoos_request_carries_its_user_agent() {
        let endpoint = &built_in(YAHOO_FINANCE).expect("a built-in").endpoint;

        let (request, path) = price_request(endpoint, "BHP.AX", "AUD", None);

        assert_eq!(
            request.url,
            "https://query1.finance.yahoo.com/v8/finance/chart/BHP.AX?interval=1d&range=1d"
        );
        assert_eq!(
            request.headers,
            [(
                "User-Agent".to_string(),
                format!("Personal-Ledger/{}", env!("CARGO_PKG_VERSION"))
            )]
        );
        assert_eq!(path, "/chart/result/0/meta/regularMarketPrice");
    }

    #[test]
    fn a_custom_sources_headers_are_sent_as_entered_in_order() {
        let endpoint = Endpoint {
            address: "https://prices.example/".to_string().into(),
            request: "/q/{symbol}/{quote_lower}".to_string().into(),
            price_path: "/{symbol_lower}".to_string().into(),
            auth: PriceSourceAuth::None,
            headers: vec![
                RequestHeader {
                    name: "Accept".to_string().into(),
                    value: "application/json".to_string().into(),
                },
                RequestHeader {
                    name: "X-Client".to_string().into(),
                    value: "ledger; v1".to_string().into(),
                },
            ]
            .into(),
            batch: None,
            history: None,
        };

        let (request, path) = price_request(&endpoint, "ABC", "USD", None);

        assert_eq!(request.url, "https://prices.example/q/ABC/usd");
        assert_eq!(
            request.headers,
            [
                ("Accept".to_string(), "application/json".to_string()),
                ("X-Client".to_string(), "ledger; v1".to_string()),
            ]
        );
        assert_eq!(path, "/abc");
    }

    #[test]
    fn a_required_key_is_missing_but_an_optional_one_is_not() {
        let keyed = |optional| Endpoint {
            address: "https://prices.example".to_string().into(),
            request: "/{symbol}".to_string().into(),
            price_path: "/price".to_string().into(),
            auth: PriceSourceAuth::ApiKey {
                placement: KeyPlacement::Header,
                name: "x-api-key".to_string().into(),
                optional,
            },
            headers: Vec::new().into(),
            batch: None,
            history: None,
        };
        let key = SecretString::from("k3y");
        assert!(key_missing(&keyed(false), None));
        assert!(!key_missing(&keyed(false), Some(&key)));
        assert!(!key_missing(&keyed(true), None));
        assert!(!key_missing(
            &built_in(COINGECKO).expect("a built-in").endpoint,
            None
        ));
    }

    mod due {
        use chrono::Duration;

        use super::*;
        use crate::{Precision, UnitKind};

        fn today() -> NaiveDate {
            NaiveDate::from_ymd_opt(2026, 10, 21).expect("a real date")
        }

        fn now() -> DateTime<Utc> {
            today().and_hms_opt(9, 0, 0).expect("a real time").and_utc()
        }

        fn eur() -> UnitRow {
            UnitRow::new(
                "EUR".into(),
                "Euro".into(),
                UnitKind::Currency,
                Precision::of_kind(UnitKind::Currency),
                PriceLink {
                    source_id: FRANKFURTER,
                    symbol: "EUR".into(),
                    quote: UnitId::SEED_AUD,
                },
            )
        }

        fn priced(today: NaiveDate, written_at: DateTime<Utc>) -> UnitRow {
            let mut unit = eur();
            unit.record_price("1.65".parse().expect("a decimal"), today, written_at);
            unit
        }

        #[test]
        fn a_price_is_due_once_its_sources_interval_has_passed_and_not_a_second_before() {
            for (updates, interval) in [
                (Updates::Every15Minutes, Duration::minutes(15)),
                (Updates::Hourly, Duration::hours(1)),
                (Updates::Daily, Duration::hours(24)),
            ] {
                let fresh = priced(today(), now() - interval + Duration::seconds(1));
                let stale = priced(today(), now() - interval);

                assert!(!is_due(&fresh, updates, today(), now()), "{updates:?}");
                assert!(is_due(&stale, updates, today(), now()), "{updates:?}");
            }
        }

        #[test]
        fn a_unit_with_no_price_for_the_current_week_is_due_even_with_a_fresh_one_last_week() {
            let last_week = today() - Duration::weeks(1);
            let backfilled = priced(last_week, now() - Duration::minutes(1));

            assert!(is_due(&eur(), Updates::Daily, today(), now()));
            assert!(is_due(&backfilled, Updates::Daily, today(), now()));
        }

        #[test]
        fn a_price_another_device_wrote_and_synced_counts_as_fresh() {
            // Written an hour ago by another device: no fetch of this Client's own is behind it.
            let synced = priced(today(), now() - Duration::hours(1));

            assert!(!is_due(&synced, Updates::Daily, today(), now()));
        }

        #[test]
        fn manual_only_is_never_due() {
            assert!(!is_due(&eur(), Updates::ManualOnly, today(), now()));
        }
    }
}
