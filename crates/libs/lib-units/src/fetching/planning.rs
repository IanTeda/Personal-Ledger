//! Planning a backfill (ADR-0043's backfill amendment): which Units want history, which of their
//! past weeks, and the request asking for them, queued oldest gap first. A method on
//! [`PriceFetching`] since it spans the call queue, the last fetches and the history requests.

use std::collections::HashMap;

use chrono::{DateTime, Duration, NaiveDate, Utc};
use lib_core::UnitId;

use super::history::{floor, gaps, history_request, wanted_weeks};
use super::{HistoryCall, PriceFetching, SourceSettings, key_missing};
use crate::protocols::UnitsPorts;
use crate::{UnitRow, Updates, WeeklyPrice, units_by_code};

/// Which Units a backfill asks history for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BackfillUnits {
    /// Every Unit wanting history, as the Ledger opens: never one on a Manual only source.
    Every,
    /// The Units a person's fetch now just priced, a Manual only source's included.
    Asked(Vec<UnitId>),
    /// The Units whose history request failed and is due again, while last week is still empty;
    /// older gaps wait for the next open.
    Due,
}

impl PriceFetching {
    /// Asks each of `units`' sources for the past weeks it has no Weekly Price for: from the week
    /// holding its floor, the earliest Opened date of its Accounts or else its creation, to last
    /// week on `today`, within its source's history depth, leaving out the weeks its provider
    /// already answered with nothing. Only sends, at `now`: each Unit's weeks land through
    /// [`Self::land`] for the writer. Returns how many Units' history was queued.
    ///
    /// Never backfills a Unit on Manual entry, a deactivated Unit, the base Unit or a Unit on a
    /// source with no history request. One request per source is in flight at a time, the Unit
    /// with the oldest gap first, each paced by the source's call rate behind the calls a person
    /// asked for. A source needing a key this Client doesn't have fails the Unit's history Key
    /// missing unsent.
    pub fn backfill(
        &mut self,
        units: BackfillUnits,
        ports: &dyn UnitsPorts,
        today: NaiveDate,
        now: DateTime<Utc>,
    ) -> usize {
        let (only, asked, retrying) = match units {
            BackfillUnits::Every => (None, false, false),
            BackfillUnits::Asked(ids) => (Some(ids), true, false),
            BackfillUnits::Due => {
                let due = self.history_retries(now);
                if due.is_empty() {
                    return 0;
                }
                (Some(due), false, true)
            }
        };
        let base = ports.base_unit().base_unit();
        let catalogue = ports.source_catalogue();
        let units = ports.units().units();
        let last_week = WeeklyPrice::week_of(today) - Duration::weeks(1);
        let mut calls: Vec<HistoryCall> = Vec::new();
        // Each source's key, opened once however many of its Units want history.
        let mut keys = HashMap::new();
        for unit in units_by_code(&units) {
            if only.as_ref().is_some_and(|ids| !ids.contains(&unit.id)) {
                continue;
            }
            if retrying {
                // Tried again or not, the failure is spent: one that fails again is noted anew.
                self.clear_history_failure(unit.id);
            }
            if !unit.is_active || unit.id == base || self.history_wanted(unit.id) {
                continue;
            }
            let Some(source) = catalogue.source(unit.link.source_id) else {
                continue;
            };
            let Some(endpoint) = source.endpoint() else {
                continue;
            };
            let Some(history) = endpoint.history.as_deref() else {
                continue;
            };
            if source.updates == Updates::ManualOnly && !asked && !retrying {
                continue;
            }
            let Some(weeks) = self.unit_gaps(ports, unit, today, source.history_depth()) else {
                continue;
            };
            if retrying && weeks.last() != Some(&last_week) {
                continue;
            }
            let Some(quote) = units.iter().find(|quote| quote.id == unit.link.quote) else {
                tracing::warn!(unit_id = %unit.id, "Skipped backfilling a Unit whose quote Unit is gone");
                continue;
            };
            let (Some(&from), Some(&to)) = (weeks.first(), weeks.last()) else {
                continue;
            };
            let key = keys
                .entry(source.id)
                .or_insert_with(|| ports.keys().key(source.id))
                .clone();
            let mut call = HistoryCall {
                unit: unit.id,
                code: unit.code.clone(),
                link: unit.link.clone(),
                source: source.id,
                settings: SourceSettings::of(&source),
                history: history.clone(),
                quote: quote.code.clone(),
                weeks,
                asked,
                request: None,
            };
            if key_missing(endpoint, key.as_ref()) {
                tracing::warn!(price_source_id = %source.id, unit_id = %unit.id, "A Unit's history needs an API key this Client doesn't have");
                self.history_key_missing(&call, now);
                continue;
            }
            call.request = Some(history_request(
                endpoint,
                history,
                &unit.link.symbol,
                &quote.code,
                (from, to + Duration::days(6)),
                key,
            ));
            calls.push(call);
        }
        // Oldest gap first, across every source's Units.
        calls.sort_by_key(|call| call.weeks.first().copied());
        let mut queued = 0;
        for call in calls {
            if self.queue_history(call, now) {
                queued += 1;
            }
        }
        if queued == 0 {
            tracing::debug!("No Unit's history is wanted");
        }
        queued
    }

    /// `unit`'s empty weeks on `today`, oldest first, on a source whose history reaches `depth`
    /// days back; `None` when it wants none.
    fn unit_gaps(
        &self,
        ports: &dyn UnitsPorts,
        unit: &UnitRow,
        today: NaiveDate,
        depth: Option<u32>,
    ) -> Option<Vec<NaiveDate>> {
        let floor = floor(unit, ports.openings().earliest_opened(unit.id))?;
        let wanted = wanted_weeks(floor, today, depth)?;
        let weeks = gaps(unit, wanted, &self.empty_weeks(unit));
        (!weeks.is_empty()).then_some(weeks)
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use chrono::Utc;
    use lib_core::UnitKind;

    use super::*;
    use crate::fake::{FakeTransport, InMemoryPorts};
    use crate::fetching::{Landing, TransportOutcome};
    use crate::interactor::{FetchNow, RecordFetchedPrices, RecordPriceHistory};
    use crate::{
        BuiltInOverride, COINGECKO, FRANKFURTER, Precision, PriceLink, PriceSourceRecord,
        YAHOO_FINANCE,
    };

    const FRANKFURTER_RATES: &str = include_str!("../../fixtures/history/frankfurter_rates.json");

    fn now() -> DateTime<Utc> {
        day(2026, 10, 21)
            .and_hms_opt(9, 0, 0)
            .expect("a real time")
            .and_utc()
    }

    fn day(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).expect("a real date")
    }

    fn currency(code: &str) -> UnitRow {
        UnitRow::new(
            code.into(),
            code.into(),
            UnitKind::Currency,
            Precision::of_kind(UnitKind::Currency),
            PriceLink {
                source_id: FRANKFURTER,
                symbol: code.into(),
                quote: UnitId::SEED_AUD,
            },
        )
    }

    fn on(source: lib_core::PriceSourceId, code: &str, symbol: &str) -> UnitRow {
        let mut unit = currency(code);
        unit.link.source_id = source;
        unit.link.symbol = symbol.into();
        unit
    }

    fn answered(body: &str) -> TransportOutcome {
        TransportOutcome::Answered {
            status: 200,
            body: body.into(),
            retry_after: None,
        }
    }

    fn rates() -> TransportOutcome {
        answered(FRANKFURTER_RATES)
    }

    /// One Open Ledger's fetching over the sample with `units` added, writing what lands through
    /// the writing Interactors and backfilling a fetch now's priced Units, as the Presenter does.
    struct Ledger {
        ports: InMemoryPorts,
        transport: FakeTransport,
        fetching: PriceFetching,
    }

    fn ledger(units: Vec<UnitRow>) -> Ledger {
        let mut ports = InMemoryPorts::sample(now());
        ports.units.units.extend(units);
        let transport = FakeTransport::default();
        let fetching = PriceFetching::new(Rc::new(transport.clone()));
        Ledger {
            ports,
            transport,
            fetching,
        }
    }

    impl Ledger {
        fn today(&self) -> NaiveDate {
            now().date_naive()
        }

        fn backfill(&mut self) -> usize {
            let today = self.today();
            self.fetching
                .backfill(BackfillUnits::Every, &self.ports, today, now())
        }

        /// Accounts in the Unit `code`, the earliest opened on `on`.
        fn opened(&mut self, code: &str, on: NaiveDate) {
            let id = self.unit(code).id;
            self.ports.openings.by_unit.insert(id, on);
        }

        fn unit(&self, code: &str) -> UnitRow {
            self.ports
                .units
                .units
                .iter()
                .find(|unit| unit.code == code)
                .cloned()
                .expect("listed")
        }

        fn edit(&mut self, code: &str, edit: impl FnOnce(&mut UnitRow)) {
            edit(
                self.ports
                    .units
                    .units
                    .iter_mut()
                    .find(|unit| unit.code == code)
                    .expect("listed"),
            );
        }

        fn urls(&self) -> Vec<String> {
            self.transport.urls()
        }

        fn weeks(&self, code: &str) -> Vec<NaiveDate> {
            self.unit(code)
                .prices
                .iter()
                .map(|price| price.week)
                .collect()
        }

        /// Answers request `id` with `outcome` and writes what lands.
        fn answer_id(&mut self, id: crate::fetching::RequestId, outcome: TransportOutcome) {
            let response = self.transport.answer(id, outcome);
            let today = self.today();
            let landing = self
                .fetching
                .land(response, now(), today, &Utc, &self.ports.units);
            match landing {
                Landing::Prices {
                    prices, backfill, ..
                } => {
                    RecordFetchedPrices {
                        prices,
                        today,
                        at: now(),
                    }
                    .run(&mut self.ports.units);
                    if !backfill.is_empty() {
                        self.fetching.backfill(
                            BackfillUnits::Asked(backfill),
                            &self.ports,
                            today,
                            now(),
                        );
                    }
                }
                Landing::History { unit, link, weeks } => {
                    RecordPriceHistory { unit, link, weeks }.run(&mut self.ports.units);
                }
                Landing::Nothing | Landing::Test(_) => {}
            }
        }

        /// Answers the newest request with `outcome`.
        fn answer(&mut self, outcome: TransportOutcome) {
            let (id, _) = self.transport.requests().pop().expect("a request out");
            self.answer_id(id, outcome);
        }

        /// Adds a Custom source with `history`, returning its id.
        fn custom_source(
            &mut self,
            history: Option<crate::HistoryEndpoint>,
        ) -> lib_core::PriceSourceId {
            crate::add_custom_source(
                &mut self.ports.price_sources.records,
                "Rates".into(),
                crate::Endpoint {
                    address: "https://rates.example".to_string().into(),
                    request: "/{symbol}".to_string().into(),
                    price_path: "/price".to_string().into(),
                    auth: crate::PriceSourceAuth::None,
                    headers: Vec::new().into(),
                    batch: None,
                    history: history.map(crate::HistoryRef::from),
                },
                Updates::Daily,
            )
            .id
        }
    }

    #[test]
    fn a_unit_with_accounts_opened_in_2019_asks_back_to_then_and_fills_what_comes() {
        let mut ledger = ledger(vec![currency("EUR")]);
        ledger.opened("EUR", day(2019, 6, 5));

        assert_eq!(ledger.backfill(), 1);

        assert_eq!(
            ledger.urls(),
            [
                "https://api.frankfurter.dev/v2/rates?base=EUR&quotes=AUD&from=2019-06-03&to=2026-10-18"
            ],
            "from the week of the first Opened date to the end of last week"
        );
        ledger.answer(rates());

        assert_eq!(
            ledger.weeks("EUR"),
            [day(2026, 9, 28), day(2026, 10, 5), day(2026, 10, 12)]
        );
        assert_eq!(
            ledger.unit("EUR").prices[1].close,
            "1.6092".parse().expect("a decimal"),
            "each week closes on its last day's rate"
        );

        ledger.backfill();
        assert_eq!(
            ledger.urls().len(),
            1,
            "the weeks that came back empty aren't asked again until the link changes"
        );
        ledger.edit("EUR", |eur| eur.link.symbol = "eur".into());
        ledger.backfill();
        assert_eq!(ledger.urls().len(), 2, "a relinked Unit asks again");
    }

    #[test]
    fn a_custom_source_backfills_from_a_list_of_points_or_a_series_keyed_by_date() {
        let mut ledger = ledger(Vec::new());
        let points = ledger.custom_source(Some(crate::HistoryEndpoint {
            request: "/points/{symbol}?from={from_unix}&to={to_unix}"
                .to_string()
                .into(),
            series_path: "/prices".to_string().into(),
            date_path: "/0".to_string().into(),
            price_path: "/1".to_string().into(),
            shape: crate::SeriesShape::Points,
        }));
        let keyed = ledger.custom_source(Some(crate::HistoryEndpoint {
            request: "/keyed/{symbol}?from={from}&to={to}".to_string().into(),
            series_path: "/rates".to_string().into(),
            date_path: "".into(),
            price_path: "/{quote}".to_string().into(),
            shape: crate::SeriesShape::Keyed,
        }));
        ledger
            .ports
            .units
            .units
            .extend([on(points, "EUR", "EUR"), on(keyed, "USD", "USD")]);
        ledger.edit("EUR", |eur| eur.created_on = Some(day(2026, 10, 7)));
        ledger.edit("USD", |usd| usd.created_on = Some(day(2026, 10, 7)));

        ledger.backfill();
        let urls = ledger.urls();
        assert!(
            urls.contains(
                &"https://rates.example/points/EUR?from=1791072000&to=1792368000".to_string()
            ),
            "{urls:?}"
        );
        assert!(
            urls.contains(
                &"https://rates.example/keyed/USD?from=2026-10-05&to=2026-10-18".to_string()
            ),
            "{urls:?}"
        );
        for (id, request) in ledger.transport.requests() {
            let body = if request.url.contains("/points/") {
                // 6 and 13 October 2026, in milliseconds.
                r#"{"prices":[[1791244800000,1.61],[1791849600000,1.62]]}"#
            } else {
                r#"{"rates":{"2026-10-06":{"AUD":1.51},"2026-10-13T00:00:00Z":{"AUD":"1.52"}}}"#
            };
            ledger.answer_id(id, answered(body));
        }

        assert_eq!(ledger.weeks("EUR"), [day(2026, 10, 5), day(2026, 10, 12)]);
        assert_eq!(ledger.weeks("USD"), [day(2026, 10, 5), day(2026, 10, 12)]);
        assert_eq!(
            ledger.unit("USD").prices[1].close,
            "1.52".parse().expect("a decimal")
        );
    }

    #[test]
    fn a_custom_source_with_no_history_request_never_backfills() {
        let mut ledger = ledger(Vec::new());
        let plain = ledger.custom_source(None);
        ledger.ports.units.units.push(on(plain, "EUR", "EUR"));
        ledger.edit("EUR", |eur| eur.created_on = Some(day(2026, 9, 30)));

        assert_eq!(ledger.backfill(), 0);

        assert!(ledger.urls().is_empty());
        assert!(!ledger.fetching.history_fetching(ledger.unit("EUR").id));
    }

    #[test]
    fn with_no_accounts_the_floor_is_the_units_creation_and_with_neither_nothing_is_asked() {
        let mut ledger = ledger(vec![currency("EUR"), currency("USD")]);
        ledger.edit("EUR", |eur| eur.created_on = Some(day(2026, 9, 30)));

        ledger.backfill();

        assert_eq!(
            ledger.urls(),
            [
                "https://api.frankfurter.dev/v2/rates?base=EUR&quotes=AUD&from=2026-09-28&to=2026-10-18"
            ],
            "usd has no Accounts and no creation date"
        );
    }

    #[test]
    fn a_hand_set_week_and_one_synced_meanwhile_are_never_overwritten() {
        let mut ledger = ledger(vec![currency("EUR")]);
        ledger.opened("EUR", day(2026, 9, 28));
        let hand_set = WeeklyPrice {
            week: day(2026, 9, 28),
            close: "9.99".parse().expect("a decimal"),
            open: None,
            high: None,
            low: None,
            written_at: now() - Duration::days(20),
        };
        ledger.edit("EUR", |eur| eur.prices.push(hand_set.clone()));

        ledger.backfill();
        assert_eq!(
            ledger.urls(),
            [
                "https://api.frankfurter.dev/v2/rates?base=EUR&quotes=AUD&from=2026-10-05&to=2026-10-18"
            ],
            "only the empty weeks are asked for"
        );
        let synced = WeeklyPrice {
            week: day(2026, 10, 12),
            close: "8.88".parse().expect("a decimal"),
            ..hand_set.clone()
        };
        ledger.edit("EUR", |eur| eur.prices.push(synced.clone()));
        ledger.answer(rates());

        let eur = ledger.unit("EUR");
        assert_eq!(eur.prices.len(), 3);
        assert!(eur.prices.contains(&hand_set), "the hand-set week stays");
        assert!(
            eur.prices.contains(&synced),
            "the week synced meanwhile wins"
        );
    }

    #[test]
    fn coingecko_asks_only_as_far_back_as_its_history_depth() {
        let mut eth = on(COINGECKO, "ETH", "ethereum");
        eth.kind = UnitKind::Crypto;
        let mut ledger = ledger(vec![eth]);
        ledger.opened("ETH", day(2019, 6, 5));

        ledger.backfill();

        let urls = ledger.urls();
        let eth: Vec<_> = urls.iter().filter(|url| url.contains("ethereum")).collect();
        assert_eq!(
            eth,
            [
                "https://api.coingecko.com/api/v3/coins/ethereum/market_chart/range?vs_currency=aud&from=2025-10-27&to=2026-10-18&interval=daily"
            ]
        );
    }

    #[test]
    fn manual_entry_deactivated_base_and_manual_only_units_never_backfill_on_open() {
        let mut gone = currency("GBP");
        gone.is_active = false;
        let mut manual = currency("NZD");
        manual.link = PriceLink::manual(UnitId::SEED_AUD);
        let mut ledger = ledger(vec![gone, manual, on(YAHOO_FINANCE, "BHP", "BHP.AX")]);
        for code in ["GBP", "NZD", "AUD", "BHP"] {
            ledger.opened(code, day(2026, 9, 28));
        }
        ledger
            .ports
            .price_sources
            .records
            .push(PriceSourceRecord::BuiltIn(BuiltInOverride {
                id: YAHOO_FINANCE,
                name: "Yahoo Finance".into(),
                updates: Updates::ManualOnly,
            }));

        ledger.backfill();
        assert_eq!(ledger.urls(), Vec::<String>::new());

        FetchNow {
            id: ledger.unit("BHP").id,
        }
        .run(&ledger.ports, &mut ledger.fetching, now())
        .expect("sent");
        ledger.answer(answered(include_str!(
            "../../fixtures/prices/yahoo_chart.json"
        )));
        assert!(
            ledger
                .urls()
                .last()
                .is_some_and(|url| url.contains("period1=")),
            "a Manual only source backfills through fetch now"
        );
    }

    #[test]
    fn one_history_request_per_source_at_a_time_oldest_gap_first() {
        let mut ledger = ledger(vec![
            currency("EUR"),
            currency("USD"),
            on(COINGECKO, "ETH", "ethereum"),
        ]);
        ledger.opened("EUR", day(2026, 9, 28));
        ledger.opened("USD", day(2026, 9, 14));
        ledger.opened("ETH", day(2026, 9, 28));

        assert_eq!(ledger.backfill(), 3);

        let urls = ledger.urls();
        assert_eq!(urls.len(), 2, "one to Frankfurter, one to CoinGecko");
        assert!(urls[0].contains("base=USD"), "usd's gap is the oldest");
        assert!(urls[1].contains("ethereum"), "sources run side by side");
        assert!(ledger.fetching.history_fetching(ledger.unit("USD").id));

        let (usd, _) = ledger.transport.requests()[0].clone();
        ledger.answer_id(usd, rates());
        assert!(ledger.urls()[2].contains("base=EUR"), "then eur's");
    }
}
