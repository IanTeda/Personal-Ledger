//! Where every price response lands (ADR-0062's amendment "a response lands through one call, and
//! fetching never writes"): [`PriceFetching::land`] works out which kind of request it answers,
//! keeps fetching's own bookkeeping (backoffs, `Retry-After` holds, empty and failed weeks, the
//! next history request) and hands back a [`Landing`] naming what to write and what to tell. It is
//! lent the Units read-only, to check each is still active and on the link it was asked on; the
//! writing Interactors write and log, and the Presenter tells.

use chrono::{DateTime, NaiveDate, TimeZone, Utc};
use lib_core::{Money, PriceSourceId, UnitId};

use super::backfill::HistoryAnswer;
use super::{
    FetchFailure, Landed, PriceFetching, PriceResponse, Reading, TestResult, read, round_failure,
};
use crate::protocols::UnitRepository;
use crate::{PriceLink, WeeklyPrice};

/// What a landed response asks of its caller.
pub(crate) enum Landing {
    /// Nothing to write or tell: finer prices, a dropped test, a form's test (held for the form
    /// to read), a failed history request, or a round still waiting on its other responses.
    Nothing,
    /// A test answered by a Toast, boxed as it far outweighs the other variants.
    Test(Box<TestResult>),
    /// A round's prices to write. `outcome` is `Some` only for a Fetch now, whose Toast it words,
    /// and `backfill` names the Units a Fetch now priced, whose past weeks are filled next: empty
    /// whenever `outcome` is `None`.
    Prices {
        prices: Vec<FetchedPrice>,
        outcome: Option<RoundOutcome>,
        backfill: Vec<UnitId>,
    },
    /// A Unit's backfilled weeks to write, asked on `link`.
    History {
        unit: UnitId,
        link: PriceLink,
        weeks: Vec<WeeklyPrice>,
    },
}

/// A price fetched for a Unit, on the link it was asked on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FetchedPrice {
    pub unit: UnitId,
    pub link: PriceLink,
    pub close: Money,
}

/// How a round went, for the Toast a round a person asked for raises.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RoundOutcome {
    /// The source asked.
    pub source: Option<PriceSourceId>,
    pub written: Vec<FetchedPrice>,
    /// The Units the round failed for, and why, round failures included.
    pub failed: Vec<(UnitId, FetchFailure)>,
    /// The first round failure, which backed the source off.
    pub round_failure: Option<FetchFailure>,
}

impl PriceFetching {
    /// Lands `response` at `now`, whatever it answers, on the Clock's day `today`, a finer
    /// prices response's hours read in `zone`. `units` are only read.
    pub(crate) fn land<Tz: TimeZone>(
        &mut self,
        response: PriceResponse,
        now: DateTime<Utc>,
        today: NaiveDate,
        zone: &Tz,
        units: &dyn UnitRepository,
    ) -> Landing {
        if self.tests.is_test(&response) {
            return self.land_test(&response, now);
        }
        if self.fine.is_fine(&response) {
            self.land_fine(&response, now, today, zone);
            return Landing::Nothing;
        }
        if self.backfill.is_history(&response) {
            return self.land_history(&response, now);
        }
        self.land_round(response, now, units)
    }

    /// Reads a test's `response`, holding a form's for the form and sending its history request
    /// once its price passed. Only a test answered by a Toast comes back.
    fn land_test(&mut self, response: &PriceResponse, now: DateTime<Utc>) -> Landing {
        let Some((result, history)) = self.tests.land(response, now) else {
            return Landing::Nothing;
        };
        if let Some(history) = history {
            self.send_test_history(response.request, &result.asked, history, now);
        }
        if result.asked.toast {
            Landing::Test(Box::new(result))
        } else {
            Landing::Nothing
        }
    }

    /// Reads a finer prices `response` into the cache, or notes its failure for the view alone. A
    /// 429 holds its source until its `Retry-After`; Offline and Rate limited back the source off,
    /// as a history request's do.
    fn land_fine<Tz: TimeZone>(
        &mut self,
        response: &PriceResponse,
        now: DateTime<Utc>,
        today: NaiveDate,
        zone: &Tz,
    ) {
        let Some(source) = self.fine.source_of(response.request) else {
            return;
        };
        let hold = self.hold_on_429(source, &response.outcome, now);
        if let Some((settings, until)) = self.fine.land(response, now, today, zone, hold) {
            self.last.back_off_quietly(source, settings, until);
        }
    }

    /// Reads a history `response` onto the weeks it asked, noting the ones that came back empty,
    /// or its failure, then sends its source's next history request. A 429 holds the source until
    /// its `Retry-After`, as a round's does.
    fn land_history(&mut self, response: &PriceResponse, now: DateTime<Utc>) -> Landing {
        let Some(call) = self.backfill.land(response.request) else {
            return Landing::Nothing;
        };
        self.hold_on_429(call.source, &response.outcome, now);
        let landing = match self.backfill.answered(&call, &response.outcome, now) {
            HistoryAnswer::Weeks(weeks) => Landing::History {
                unit: call.unit,
                link: call.link.clone(),
                weeks,
            },
            HistoryAnswer::Failed(reason) => {
                self.history_failed(&call, reason, now);
                Landing::Nothing
            }
        };
        self.start_history(call.source, now);
        landing
    }

    /// Holds `response` in its round until the last lands, then reads the whole round: each
    /// price for a Unit still active and on the link it was asked on, and how each Unit and the
    /// source fared, backing off a failed source or Unit and clearing a fetched one.
    fn land_round(
        &mut self,
        response: PriceResponse,
        now: DateTime<Utc>,
        units: &dyn UnitRepository,
    ) -> Landing {
        let Some(landed) = self.hold_in_round(response, now) else {
            return Landing::Nothing;
        };
        let (source, settings) = landed.source;
        let mut prices = Vec::new();
        let mut failed = Vec::new();
        for response in landed.responses {
            let request = response.request.0;
            let Some(sent) = self.rounds.take(response.request) else {
                tracing::debug!(request, "Dropped a price response for no request in flight");
                continue;
            };
            for call in sent.units {
                let unit_id = call.unit;
                let reading = read(&response.outcome, &call.price_path);
                let current = units
                    .unit(unit_id)
                    .is_some_and(|unit| unit.is_active && unit.link == call.link);
                if !current {
                    tracing::info!(
                        %unit_id,
                        "Skipped a fetched price for a Unit deleted, deactivated or relinked since"
                    );
                    // A round failure still fails the source, whichever Unit it came back for.
                    if let Reading::Failed(failure) = reading
                        && failure.is_round()
                    {
                        failed.push((unit_id, failure));
                    }
                    continue;
                }
                match reading {
                    Reading::Priced(close) => {
                        self.record_unit(unit_id, call.link.clone(), &settings, now, None);
                        prices.push(FetchedPrice {
                            unit: unit_id,
                            link: call.link,
                            close,
                        });
                    }
                    Reading::Failed(failure) => {
                        tracing::warn!(%unit_id, request, ?failure, "A price fetch failed");
                        self.record_unit(unit_id, call.link, &settings, now, Some(failure));
                        failed.push((unit_id, failure));
                    }
                }
            }
        }
        let round_failure = failed
            .iter()
            .map(|(_, failure)| *failure)
            .find(|failure| failure.is_round());
        self.record_source(source, &settings, now, round_failure);
        if !landed.asked {
            return Landing::Prices {
                prices,
                outcome: None,
                backfill: Vec::new(),
            };
        }
        let backfill = prices.iter().map(|price| price.unit).collect();
        let outcome = RoundOutcome {
            source: Some(source),
            written: prices.clone(),
            failed,
            round_failure,
        };
        Landing::Prices {
            prices,
            outcome: Some(outcome),
            backfill,
        }
    }

    /// Holds `response` in its round, handing back the whole round once its last response lands.
    /// A round failure stops the round: its calls still queued never go out, and it lands once the
    /// calls already out have. A 429 also holds the source until its `Retry-After`, read at `now`.
    /// A response for no round in flight is dropped.
    fn hold_in_round(&mut self, response: PriceResponse, now: DateTime<Utc>) -> Option<Landed> {
        let Some((round, source)) = self.rounds.round_of(response.request) else {
            self.rounds.take(response.request);
            tracing::debug!(
                request = response.request.0,
                "Dropped a price response for no request in flight"
            );
            return None;
        };
        let failure = round_failure(&response.outcome);
        if let Some(until) = self.hold_on_429(source, &response.outcome, now) {
            tracing::warn!(price_source_id = %source, %until, "A price source asked to wait");
        }
        let pending = self.rounds.land(round, response, failure.is_some());
        // The round's calls still queued never go; what is left pending is out already.
        let dropped = self.calls.remove(&pending);
        if !dropped.is_empty() {
            tracing::warn!(
                price_source_id = %source,
                dropped = dropped.len(),
                ?failure,
                "Stopped a price round before its queued calls went out"
            );
            self.rounds.forget(round, &dropped);
        }
        let landed = self.rounds.finish(round)?;
        self.release_parked(source, now);
        Some(landed)
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use chrono::Duration;
    use lib_core::UnitKind;

    use super::*;
    use crate::fake::{FakeTransport, InMemoryPorts};
    use crate::fetching::{FetchOutcome, FineCall, FineKey, Grain, SourceSettings};
    use crate::fetching::{TransportError, TransportOutcome, history_request, window};
    use crate::interactor::{
        Backfill, BackfillUnits, FetchDue, FetchNow, FetchSource, RecordFetchedPrices,
        RecordPriceHistory, TestPriceSource,
    };
    use crate::protocols::UnitsPorts;
    use crate::{
        BuiltInOverride, COINGECKO, FRANKFURTER, Precision, PriceSourceRecord, UnitRow, Updates,
    };

    const FRANKFURTER_RATES: &str = include_str!("../../fixtures/history/frankfurter_rates.json");

    fn start() -> DateTime<Utc> {
        NaiveDate::from_ymd_opt(2026, 10, 21)
            .and_then(|day| day.and_hms_opt(9, 0, 0))
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

    fn answered(status: u16, body: &str, retry_after: Option<&str>) -> TransportOutcome {
        TransportOutcome::Answered {
            status,
            body: body.into(),
            retry_after: retry_after.map(str::to_string),
        }
    }

    fn offline(_: &str) -> TransportOutcome {
        TransportOutcome::Failed(TransportError::Offline)
    }

    fn priced(_: &str) -> TransportOutcome {
        answered(200, r#"{"rate":1.5}"#, None)
    }

    fn throttled(retry_after: Option<&str>) -> TransportOutcome {
        answered(429, "", retry_after)
    }

    /// A 404 for `code`'s request, a price for every other.
    fn missing(code: &'static str) -> impl Fn(&str) -> TransportOutcome {
        move |url| {
            if url.contains(&format!("/{code}/")) {
                answered(404, r#"{"message":"not found"}"#, None)
            } else {
                priced(url)
            }
        }
    }

    /// One Open Ledger's fetching over the sample Units, landing every response through
    /// [`PriceFetching::land`] and writing what it hands back through the writing Interactors, as
    /// the Presenter does.
    struct Ledger {
        ports: InMemoryPorts,
        transport: FakeTransport,
        fetching: PriceFetching,
        now: DateTime<Utc>,
    }

    impl Ledger {
        fn new(units: Vec<UnitRow>) -> Self {
            let mut ports = InMemoryPorts::sample(start());
            ports.units.units.extend(units);
            let transport = FakeTransport::default();
            let fetching = PriceFetching::new(Rc::new(transport.clone()));
            Self {
                ports,
                transport,
                fetching,
                now: start(),
            }
        }

        fn today(&self) -> NaiveDate {
            self.now.date_naive()
        }

        fn later(&mut self, by: Duration) {
            self.now += by;
        }

        fn frankfurter(&self) -> usize {
            self.urls()
                .iter()
                .filter(|url| url.contains("frankfurter"))
                .count()
        }

        /// The scheduled check, then the failed history requests due again, as the Presenter
        /// runs them; returns how many requests went to Frankfurter.
        fn check(&mut self) -> usize {
            let before = self.frankfurter();
            let (today, now) = (self.today(), self.now);
            FetchDue { today, now }.run(&self.ports, &mut self.fetching);
            let retries = self.fetching.history_retries(now);
            if !retries.is_empty() {
                Backfill {
                    units: BackfillUnits::Retrying(retries),
                    today,
                    now,
                }
                .run(&self.ports, &mut self.fetching);
            }
            self.frankfurter() - before
        }

        fn backfill(&mut self) {
            Backfill {
                units: BackfillUnits::Every,
                today: self.today(),
                now: self.now,
            }
            .run(&self.ports, &mut self.fetching);
        }

        fn fetch_now(&mut self, code: &str) {
            let id = self.unit(code).id;
            FetchNow { id }
                .run(&self.ports, &mut self.fetching, self.now)
                .expect("sent");
        }

        fn fetch_source(&mut self, id: PriceSourceId) -> crate::Result<usize> {
            FetchSource { id }.run(&self.ports, &mut self.fetching, self.now)
        }

        fn release(&mut self) {
            self.fetching.release(self.now);
        }

        /// Lands `response` and writes what it hands back.
        fn land(&mut self, response: PriceResponse) -> Landing {
            let (now, today) = (self.now, self.today());
            let landing = self
                .fetching
                .land(response, now, today, &Utc, &self.ports.units);
            match &landing {
                Landing::Prices { prices, .. } => {
                    RecordFetchedPrices {
                        prices: prices.clone(),
                        today,
                        at: now,
                    }
                    .run(&mut self.ports.units);
                }
                Landing::History { unit, link, weeks } => {
                    RecordPriceHistory {
                        unit: *unit,
                        link: link.clone(),
                        weeks: weeks.clone(),
                    }
                    .run(&mut self.ports.units);
                }
                Landing::Nothing | Landing::Test(_) => {}
            }
            landing
        }

        /// Answers every round's request still out, oldest first, each with `outcome(url)`,
        /// returning what each landed as.
        fn answer(&mut self, outcome: impl Fn(&str) -> TransportOutcome) -> Vec<Landing> {
            let out: Vec<_> = self
                .transport
                .requests()
                .into_iter()
                .filter(|(id, _)| self.fetching.in_flight_request(*id))
                .collect();
            out.into_iter()
                .map(|(id, request)| {
                    let response = self.transport.answer(id, outcome(&request.url));
                    self.land(response)
                })
                .collect()
        }

        /// Answers the newest request with `outcome`.
        fn answer_last(&mut self, outcome: TransportOutcome) -> Landing {
            let (id, _) = self.transport.requests().pop().expect("a request out");
            let response = self.transport.answer(id, outcome);
            self.land(response)
        }

        fn sent(&self) -> usize {
            self.transport.requests().len()
        }

        fn urls(&self) -> Vec<String> {
            self.transport.urls()
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

        fn opened(&mut self, code: &str, on: NaiveDate) {
            let id = self.unit(code).id;
            self.ports.openings.by_unit.insert(id, on);
        }

        fn unit_failure(&self, code: &str) -> Option<FetchFailure> {
            let unit = self.unit(code);
            let catalogue = self.ports.source_catalogue();
            let source = catalogue.source(unit.link.source_id)?;
            self.fetching
                .unit_fetch(&unit, &source)
                .and_then(|last| last.failure())
        }

        fn source_failure(&self, id: PriceSourceId) -> Option<FetchFailure> {
            let catalogue = self.ports.source_catalogue();
            let source = catalogue.source(id)?;
            self.fetching
                .source_fetch(&source)
                .and_then(|last| last.failure())
        }

        fn retry_at(&self, id: PriceSourceId) -> Option<DateTime<Utc>> {
            let catalogue = self.ports.source_catalogue();
            let source = catalogue.source(id)?;
            match self.fetching.source_fetch(&source)?.outcome {
                FetchOutcome::Failed { retry_at, .. } => Some(retry_at),
                FetchOutcome::Fetched => None,
            }
        }

        fn history_failure(&self, code: &str) -> Option<(FetchFailure, DateTime<Utc>)> {
            let unit = self.unit(code);
            let catalogue = self.ports.source_catalogue();
            let source = catalogue.source(unit.link.source_id)?;
            self.fetching.history_failure(&unit, &source)
        }

        fn retime(&mut self, updates: Updates) {
            self.ports
                .price_sources
                .records
                .push(PriceSourceRecord::BuiltIn(BuiltInOverride {
                    id: FRANKFURTER,
                    name: "Frankfurter".into(),
                    updates,
                }));
        }

        /// Asks for `code`'s Daily Prices in this week's window, as the Unit prices view does.
        fn ask_daily(&mut self, code: &str) {
            let unit = self.unit(code);
            let catalogue = self.ports.source_catalogue();
            let source = catalogue.source(unit.link.source_id).expect("listed");
            let endpoint = source.endpoint().expect("fetches");
            let history = source.fine_history(Grain::Day).expect("has history");
            let (first, last) = window(Grain::Day, self.today(), 0);
            let request = history_request(
                endpoint,
                history,
                &unit.link.symbol,
                "AUD",
                (first, last),
                None,
            );
            let call = FineCall {
                key: FineKey {
                    unit: unit.id,
                    grain: Grain::Day,
                    first,
                },
                last,
                link: unit.link.clone(),
                settings: SourceSettings::of(&source),
                history: history.clone(),
                quote: "AUD".into(),
                request: Some(request),
            };
            self.fetching.ask_fine(call, self.now);
        }
    }

    /// The round's one landing that isn't [`Landing::Nothing`]: its prices, outcome and backfill.
    fn round(landings: Vec<Landing>) -> (Vec<FetchedPrice>, Option<RoundOutcome>, Vec<UnitId>) {
        let mut rounds = landings.into_iter().filter_map(|landing| match landing {
            Landing::Prices {
                prices,
                outcome,
                backfill,
            } => Some((prices, outcome, backfill)),
            Landing::Nothing | Landing::Test(_) | Landing::History { .. } => None,
        });
        let landed = rounds.next().expect("the round landed");
        assert!(rounds.next().is_none(), "the round lands once");
        landed
    }

    fn codes(ledger: &Ledger, prices: &[FetchedPrice]) -> Vec<String> {
        prices
            .iter()
            .map(|price| {
                ledger
                    .ports
                    .units
                    .units
                    .iter()
                    .find(|unit| unit.id == price.unit)
                    .map(|unit| unit.code.clone())
                    .unwrap_or_default()
            })
            .collect()
    }

    #[test]
    fn a_round_lands_once_its_last_response_is_back_and_a_scheduled_one_has_no_outcome() {
        let mut ledger = Ledger::new(vec![currency("EUR"), currency("USD")]);
        assert_eq!(ledger.check(), 2);

        let landings = ledger.answer(priced);

        assert!(matches!(landings[0], Landing::Nothing), "held for the rest");
        let (prices, outcome, backfill) = round(landings);
        assert_eq!(codes(&ledger, &prices), ["EUR", "USD"]);
        assert_eq!(outcome, None, "a scheduled round tells no one");
        assert!(backfill.is_empty());
        assert_eq!(ledger.fetching.in_flight(), 0);
    }

    #[test]
    fn a_round_failure_backs_off_the_source_and_its_units_for_the_sources_interval() {
        let mut ledger = Ledger::new(vec![currency("EUR")]);
        assert_eq!(ledger.check(), 1);

        let (prices, outcome, _) = round(ledger.answer(offline));

        assert!(prices.is_empty());
        assert_eq!(outcome, None);
        assert_eq!(
            ledger.source_failure(FRANKFURTER),
            Some(FetchFailure::Offline)
        );
        assert_eq!(ledger.unit_failure("EUR"), Some(FetchFailure::Offline));
        ledger.later(Duration::hours(23));
        assert_eq!(ledger.check(), 0, "Frankfurter backs off a day");
        ledger.later(Duration::hours(1));
        assert_eq!(ledger.check(), 1);
    }

    #[test]
    fn a_unit_failure_backs_off_that_unit_alone_and_the_next_success_clears_it() {
        let mut ledger = Ledger::new(vec![currency("EUR"), currency("USD")]);
        ledger.retime(Updates::Every15Minutes);
        ledger.check();
        ledger.answer(missing("EUR"));
        assert_eq!(ledger.unit_failure("EUR"), Some(FetchFailure::NotFound));
        assert_eq!(ledger.unit_failure("USD"), None);
        assert_eq!(ledger.source_failure(FRANKFURTER), None);

        ledger.ports.units.units.push(currency("GBP"));
        ledger.later(Duration::minutes(1));
        assert_eq!(ledger.check(), 1, "gbp, while eur backs off");
        ledger.answer(priced);

        ledger.later(Duration::minutes(15));
        assert_eq!(ledger.check(), 3, "eur's back-off is over");
        ledger.answer(priced);
        assert_eq!(ledger.unit_failure("EUR"), None);
        let unit = ledger.unit("EUR");
        let catalogue = ledger.ports.source_catalogue();
        let source = catalogue.source(FRANKFURTER).expect("frankfurter");
        assert_eq!(
            ledger
                .fetching
                .unit_fetch(&unit, &source)
                .map(|last| last.outcome),
            Some(FetchOutcome::Fetched)
        );
    }

    /// 62 Units on Frankfurter, fetched now: its call rate sends 60, and the fourth answers 429.
    fn stopped_round() -> (Ledger, Vec<Landing>) {
        let currencies: Vec<UnitRow> = (0..62)
            .map(|index| currency(&format!("C{index:02}")))
            .collect();
        let mut ledger = Ledger::new(currencies);
        assert_eq!(ledger.fetch_source(FRANKFURTER), Ok(62));
        assert_eq!(ledger.sent(), 60, "two wait for Frankfurter's call rate");
        let landings = ledger.answer(|url| {
            if url.ends_with("/C03/AUD") {
                throttled(Some("30"))
            } else {
                priced(url)
            }
        });
        (ledger, landings)
    }

    #[test]
    fn a_round_failure_mid_round_stops_it_and_lands_what_it_fetched_in_one_go() {
        let (ledger, landings) = stopped_round();

        let (prices, outcome, backfill) = round(landings);
        assert_eq!(prices.len(), 59);
        let outcome = outcome.expect("a fetch now has an outcome");
        assert_eq!(outcome.source, Some(FRANKFURTER));
        assert_eq!(outcome.written, prices);
        assert_eq!(outcome.round_failure, Some(FetchFailure::RateLimited));
        assert_eq!(backfill.len(), 59, "every Unit it priced");
        assert_eq!(
            ledger.source_failure(FRANKFURTER),
            Some(FetchFailure::RateLimited)
        );
        assert_eq!(ledger.unit_failure("C03"), Some(FetchFailure::RateLimited));
        assert_eq!(
            ledger.unit_failure("C60"),
            None,
            "never reached, never failed"
        );
        assert!(ledger.unit("C60").prices.is_empty());
    }

    #[test]
    fn a_stopped_round_drops_its_queued_calls() {
        let (mut ledger, _) = stopped_round();

        assert_eq!(ledger.fetching.in_flight(), 0);
        assert!(!ledger.fetching.fetching(ledger.unit("C61").id));
        ledger.later(Duration::minutes(2));
        ledger.release();
        assert_eq!(
            ledger.sent(),
            60,
            "the stopped round's queued calls never go"
        );
    }

    #[test]
    fn a_429_backs_off_for_the_longer_of_the_usual_rule_and_its_retry_after() {
        let mut ledger = Ledger::new(vec![]);
        ledger.fetch_now("BTC");
        ledger.answer(|_| throttled(Some("3600")));
        assert_eq!(
            ledger.retry_at(COINGECKO),
            Some(start() + Duration::hours(1)),
            "an hour outlasts CoinGecko's 15 minutes"
        );

        let mut ledger = Ledger::new(vec![]);
        ledger.fetch_now("BTC");
        ledger.answer(|_| throttled(Some("30")));
        assert_eq!(
            ledger.retry_at(COINGECKO),
            Some(start() + Duration::minutes(15)),
            "15 minutes outlast 30 seconds"
        );
    }

    #[test]
    fn a_429_holds_its_source_whatever_kind_of_request_it_answered() {
        type Throttle = fn(&mut Ledger);
        let kinds: [(&str, Throttle); 3] = [
            ("a round", |ledger| {
                ledger.fetch_now("EUR");
                ledger.answer(|_| throttled(Some("60")));
            }),
            ("a history request", |ledger| {
                ledger.opened("EUR", day(2026, 9, 28));
                ledger.backfill();
                ledger.answer_last(throttled(Some("60")));
            }),
            ("a finer prices request", |ledger| {
                ledger.ask_daily("EUR");
                ledger.answer_last(throttled(Some("60")));
            }),
        ];
        for (kind, throttle) in kinds {
            let mut ledger = Ledger::new(vec![currency("EUR")]);
            throttle(&mut ledger);
            let sent = ledger.sent();

            ledger.fetch_now("EUR");
            assert_eq!(ledger.sent(), sent, "{kind}: held by its Retry-After");
            ledger.later(Duration::seconds(59));
            ledger.release();
            assert_eq!(ledger.sent(), sent, "{kind}: still held");
            ledger.later(Duration::seconds(1));
            ledger.release();
            assert_eq!(
                ledger.sent(),
                sent + 1,
                "{kind}: sent once the wait is over"
            );
        }
    }

    #[test]
    fn a_finer_prices_request_parked_behind_its_sources_round_goes_once_the_round_lands() {
        let mut ledger = Ledger::new(vec![currency("EUR")]);
        assert_eq!(ledger.check(), 1);

        ledger.ask_daily("EUR");
        assert_eq!(ledger.sent(), 1, "it waits for the round");
        ledger.answer(priced);

        let urls = ledger.urls();
        assert_eq!(urls.len(), 2);
        assert!(
            urls[1].contains("/v2/rates?"),
            "the parked request goes out: {}",
            urls[1]
        );
    }

    #[test]
    fn failures_are_never_landed_as_prices() {
        let mut ledger = Ledger::new(vec![currency("EUR"), currency("USD")]);
        // Frankfurter's round alone, so the sample's btc never comes due beside it.
        for unit in &mut ledger.ports.units.units {
            unit.is_active &= unit.link.source_id != COINGECKO;
        }
        let sources = ledger.ports.price_sources.clone();
        ledger.check();
        let (prices, _, _) = round(ledger.answer(missing("EUR")));
        assert_eq!(codes(&ledger, &prices), ["USD"]);

        ledger.later(Duration::days(1));
        ledger.check();
        let (prices, _, _) = round(ledger.answer(offline));
        assert!(prices.is_empty());

        assert_eq!(ledger.ports.price_sources, sources);
        assert_eq!(ledger.unit("EUR").prices, Vec::new());
    }

    #[test]
    fn a_unit_changed_since_its_round_went_out_lands_no_price() {
        let mut ledger = Ledger::new(vec![currency("EUR")]);
        ledger.fetch_now("EUR");
        let eur = ledger.unit("EUR").id;
        let unit = ledger
            .ports
            .units
            .units
            .iter_mut()
            .find(|unit| unit.id == eur);
        unit.expect("eur").link.symbol = "EURO".into();

        let (prices, outcome, backfill) = round(ledger.answer(priced));

        assert!(prices.is_empty());
        assert_eq!(outcome.map(|outcome| outcome.written), Some(Vec::new()));
        assert!(backfill.is_empty());
    }

    #[test]
    fn a_failed_history_request_retries_every_ten_minutes_until_its_weeks_land() {
        let mut ledger = Ledger::new(vec![currency("EUR")]);
        ledger.opened("EUR", day(2026, 9, 28));
        ledger.backfill();

        let landing = ledger.answer_last(TransportOutcome::Failed(TransportError::Offline));

        assert!(
            matches!(landing, Landing::Nothing),
            "a failure writes nothing"
        );
        assert_eq!(
            ledger.history_failure("EUR"),
            Some((FetchFailure::Offline, start() + Duration::minutes(10)))
        );
        assert_eq!(ledger.unit_failure("EUR"), None, "never a Last fetch");
        let history = |ledger: &Ledger| {
            ledger
                .urls()
                .iter()
                .filter(|url| url.contains("/v2/rates?"))
                .count()
        };
        ledger.later(Duration::minutes(9));
        ledger.check();
        assert_eq!(
            history(&ledger),
            1,
            "offline backs Frankfurter off for its rounds too"
        );

        ledger.later(Duration::minutes(1));
        ledger.check();
        assert_eq!(history(&ledger), 2, "ten minutes on, it asks again");
        let landing = ledger.answer_last(answered(200, FRANKFURTER_RATES, None));

        let Landing::History { unit, weeks, .. } = landing else {
            panic!("the history lands its weeks");
        };
        assert_eq!(unit, ledger.unit("EUR").id);
        assert_eq!(
            weeks.iter().map(|week| week.week).collect::<Vec<_>>(),
            [day(2026, 9, 28), day(2026, 10, 5), day(2026, 10, 12)]
        );
        assert_eq!(ledger.history_failure("EUR"), None);
    }

    #[test]
    fn a_rate_limited_history_waits_the_longer_of_ten_minutes_and_its_retry_after() {
        let mut ledger = Ledger::new(vec![currency("EUR")]);
        ledger.opened("EUR", day(2026, 9, 28));
        ledger.backfill();

        ledger.answer_last(throttled(Some("1800")));

        assert_eq!(
            ledger.history_failure("EUR"),
            Some((FetchFailure::RateLimited, start() + Duration::minutes(30)))
        );
    }

    #[test]
    fn a_history_retry_stops_once_last_week_has_a_price() {
        let mut ledger = Ledger::new(vec![currency("EUR")]);
        ledger.opened("EUR", day(2026, 9, 28));
        ledger.backfill();
        ledger.answer_last(TransportOutcome::Failed(TransportError::Timeout));
        let synced = WeeklyPrice {
            week: day(2026, 10, 12),
            close: "1.6".parse().expect("a decimal"),
            open: None,
            high: None,
            low: None,
            written_at: start(),
        };
        let eur = ledger.unit("EUR").id;
        let unit = ledger
            .ports
            .units
            .units
            .iter_mut()
            .find(|unit| unit.id == eur);
        unit.expect("eur").prices.push(synced);

        ledger.later(Duration::minutes(10));
        ledger.check();

        assert_eq!(
            ledger
                .urls()
                .iter()
                .filter(|url| url.contains("/v2/rates?"))
                .count(),
            1,
            "older gaps wait for the next open"
        );
        assert_eq!(ledger.history_failure("EUR"), None);
    }

    #[test]
    fn a_test_answered_by_a_toast_lands_as_one_and_a_forms_is_held_for_the_form() {
        let mut ledger = Ledger::new(vec![]);
        let catalogue = ledger.ports.source_catalogue();
        let endpoint = catalogue
            .source(FRANKFURTER)
            .and_then(|source| source.endpoint().cloned())
            .expect("frankfurter fetches");
        drop(catalogue);
        let test = |toast| TestPriceSource {
            source: Some(FRANKFURTER),
            source_name: "Frankfurter".into(),
            endpoint: endpoint.clone(),
            symbol: "EUR".into(),
            quote: UnitId::SEED_AUD,
            quote_code: "AUD".into(),
            key: None,
            toast,
            unsaved: false,
        };

        test(true)
            .run(&mut ledger.fetching, ledger.now)
            .expect("sent");
        let landing = ledger.answer_last(answered(200, r#"{"rate":1.6}"#, None));
        assert!(
            matches!(landing, Landing::Test(_)),
            "its Toast is the caller's to raise"
        );

        let form = test(false)
            .run(&mut ledger.fetching, ledger.now)
            .expect("sent");
        let landing = ledger.answer_last(answered(200, r#"{"rate":1.6}"#, None));
        assert!(matches!(landing, Landing::Nothing));
        assert!(
            ledger.fetching.test_result(form).is_some(),
            "held for the form"
        );
    }
}
