//! Where a price response lands, and what the Client's minute timer and response poll reach: the
//! Client hands each response here through its input door, so the round it completes is written
//! as that input's one commit and every View reading the Units repaints, and pumps the queued
//! calls out from the same poll. It draws nothing; a Fetch now that wrote a price says so in a
//! Success Toast, one that failed in a Warning Toast, and a scheduled round says nothing at all,
//! whether it landed or failed.
//!
//! It also backfills (ADR-0043's backfill amendment): on Ledger open, after a person's fetch now
//! and again for a failed history request on the minute check, it asks each Unit's source for the
//! past weeks it has no Weekly Price for, and writes each Unit's weeks as that response's one
//! commit. A backfill never toasts, and nothing here touches the status line (ADR-0048): a fetch now's
//! history shows in the Weekly Prices table.

use chrono::{DateTime, Local, NaiveDate, Utc};
use lib_toast::{Notice, Notify, ToastKind};

use super::Clock;
use super::fetch_text::{
    Asked, round_failed_toast, round_stopped_toast, some_failed_toast, test_toast,
    unit_failed_toast, wait_begun_toast,
};
use super::units_pane::amount;
use crate::fetching::{HistoryReading, fold, read_history};
use crate::fetching::{PriceResponse, SharedFetching};
use crate::interactor::{
    Backfill, BackfillUnits, FetchDue, RecordFetchedPrices, RecordPriceHistory, RoundOutcome,
};
use crate::msg;
use crate::protocols::SharedPorts;

pub struct FetchedPricesPresenter {
    ports: SharedPorts,
    fetching: SharedFetching,
    clock: Clock,
}

impl FetchedPricesPresenter {
    pub fn new(ports: SharedPorts, fetching: SharedFetching, clock: Clock) -> Self {
        Self {
            ports,
            fetching,
            clock,
        }
    }

    /// What a Client runs as a Ledger opens (app start, `:open`, `:new`): the scheduled check,
    /// then the backfill, once. The Client then runs [`Self::fetch_due`] every [`PRICE_CHECK`]
    /// while the Ledger stays open.
    pub fn opened(&mut self) {
        self.fetch_due();
        self.backfill();
    }

    /// Releases the queued calls at the Clock's now: sends what each source's call rate and any
    /// `Retry-After` allow, raising one *Waiting for CoinGecko, 40 s…* Info Toast as a call a
    /// person asked for begins to wait, unless the page already did. Each Client calls it from its
    /// response poll; nothing here waits or sleeps.
    pub fn pump(&mut self, router: &mut dyn Notify) {
        let now = (self.clock)();
        self.fetching.borrow_mut().release(now);
        let toast = wait_begun_toast(&*self.ports.borrow(), &mut self.fetching.borrow_mut(), now);
        if let Some(toast) = toast {
            router.notify(toast);
        }
    }

    /// The scheduled check, on Ledger open and every minute after: starts a silent round on each
    /// source with a Unit due, leaving a source with a round in flight alone, and tries a failed
    /// history request again once its wait is over. It never starts a backfill of its own.
    pub fn fetch_due(&mut self) {
        let now = (self.clock)();
        let today = local_day(now);
        FetchDue { today, now }.run(&*self.ports.borrow(), &mut self.fetching.borrow_mut());
        let retries = self.fetching.borrow().history_retries(now);
        if !retries.is_empty() {
            Backfill {
                units: BackfillUnits::Retrying(retries),
                today,
                now,
            }
            .run(&*self.ports.borrow(), &mut self.fetching.borrow_mut());
        }
    }

    /// The backfill on Ledger open (app start, `:open`, `:new`): asks every Unit's source for the
    /// past weeks it has no Weekly Price for, silently. The sync effort calls it too, once a sync
    /// completes.
    pub fn backfill(&mut self) {
        let now = (self.clock)();
        Backfill {
            units: BackfillUnits::Every,
            today: local_day(now),
            now,
        }
        .run(&*self.ports.borrow(), &mut self.fetching.borrow_mut());
    }

    /// Holds `response` until the rest of its round lands, then writes every price the round
    /// carries for a Unit that still wants it. A round a person asked for raises a Toast naming
    /// each price; a scheduled one raises nothing. A fetched price is never undoable, so it raises
    /// no `Notice::Changed`.
    ///
    /// A test's response writes nothing: `t` on a row raises its one Toast, and a form's is held
    /// for the form to show.
    pub fn handle(&mut self, response: PriceResponse, router: &mut dyn Notify) {
        if self.fetching.borrow().is_test(&response) {
            let landed = self
                .fetching
                .borrow_mut()
                .land_test(&response, (self.clock)());
            if let Some(result) = landed.filter(|result| result.asked.toast) {
                let quote = self.ports.borrow().units().unit(result.asked.quote);
                router.notify(test_toast(&result, quote.as_ref()));
            }
            return;
        }
        let now = (self.clock)();
        let today = local_day(now);
        if self.fetching.borrow().is_fine(&response) {
            // Shown in place by the Unit prices view and never written, so nothing to tell.
            self.fetching
                .borrow_mut()
                .land_fine(&response, now, today, &Local);
            return;
        }
        if self.fetching.borrow().is_history(&response) {
            self.record_history(&response);
            return;
        }
        let Some(landed) = self.fetching.borrow_mut().land(response, now) else {
            return;
        };
        let asked = landed.asked;
        let outcome = {
            let mut ports = self.ports.borrow_mut();
            RecordFetchedPrices { landed }.run(
                ports.split_mut().units,
                &mut self.fetching.borrow_mut(),
                today,
                now,
            )
        };
        if asked {
            self.tell(&outcome, router);
            // A fetch now goes on to fill the past weeks of each Unit it priced.
            let priced: Vec<_> = outcome.written.iter().map(|price| price.unit).collect();
            if !priced.is_empty() {
                Backfill {
                    units: BackfillUnits::Asked(priced),
                    today,
                    now,
                }
                .run(&*self.ports.borrow(), &mut self.fetching.borrow_mut());
            }
        }
    }

    /// Writes the weeks a history `response` brings into its Unit's empty weeks, noting the weeks
    /// it had nothing for, or its failure, then sends the source's next history request. Never a
    /// Toast: the Weekly Prices table shows a fetch now's history in place.
    fn record_history(&mut self, response: &PriceResponse) {
        let now = (self.clock)();
        let Some(call) = self.fetching.borrow_mut().land_history(response, now) else {
            return;
        };
        let reading = read_history(
            &response.outcome,
            &call.history,
            &call.link.symbol,
            &call.quote,
        );
        match reading {
            HistoryReading::Points(points) => {
                let asked: std::collections::HashSet<_> = call.weeks.iter().copied().collect();
                let weeks: Vec<_> = fold(points, now)
                    .into_iter()
                    .filter(|week| asked.contains(&week.week))
                    .collect();
                let answered: std::collections::HashSet<_> =
                    weeks.iter().map(|week| week.week).collect();
                let missing: Vec<_> = call
                    .weeks
                    .iter()
                    .filter(|week| !answered.contains(*week))
                    .copied()
                    .collect();
                self.fetching.borrow_mut().history_empty(&call, missing);
                let mut ports = self.ports.borrow_mut();
                RecordPriceHistory {
                    unit: call.unit,
                    link: call.link.clone(),
                    weeks,
                }
                .run(ports.split_mut().units);
            }
            HistoryReading::Failed(reason) => {
                self.fetching
                    .borrow_mut()
                    .history_failed(&call, reason, now);
            }
        }
        self.fetching.borrow_mut().start_history(call.source, now);
    }

    /// A Fetch now's outcome, one Toast per round: a Success Toast naming the price, or counting
    /// the prices from the source, when everything landed, else one Warning Toast naming what
    /// failed.
    fn tell(&self, outcome: &RoundOutcome, router: &mut dyn Notify) {
        let ports = self.ports.borrow();
        let catalogue = ports.source_catalogue();
        let source = outcome
            .source
            .and_then(|id| catalogue.source(id))
            .map(|source| source.name.to_string())
            .unwrap_or_default();
        let warn = |text: String| Notice::Toast(ToastKind::Warning, text);
        if let Some(reason) = outcome.round_failure {
            // A round stopped before it wrote anything says only why; one stopped part way also
            // counts what it wrote.
            let text = match outcome.written.len() {
                0 => round_failed_toast(reason, &source),
                fetched => round_stopped_toast(fetched, reason, &source),
            };
            router.notify(warn(text));
            return;
        }
        match (outcome.written.as_slice(), outcome.failed.as_slice()) {
            ([], [(unit, reason)]) => {
                let Some(unit) = ports.units().unit(*unit) else {
                    return;
                };
                let quote = ports
                    .units()
                    .unit(unit.link.quote)
                    .map(|quote| quote.code)
                    .unwrap_or_default();
                let asked = Asked {
                    symbol: &unit.link.symbol,
                    quote: &quote,
                };
                router.notify(warn(unit_failed_toast(&unit.code, *reason, &source, asked)));
            }
            (written, []) if written.len() > 1 => {
                router.notify(Notice::Toast(
                    ToastKind::Success,
                    msg::units_toast_fetch_round_fetched(
                        lib_viper::view::count(written.len()),
                        &source,
                    ),
                ));
            }
            (written, []) => {
                for price in written {
                    let Some(unit) = ports.units().unit(price.unit) else {
                        continue;
                    };
                    let quote = ports.units().unit(unit.link.quote);
                    router.notify(Notice::Toast(
                        ToastKind::Success,
                        msg::units_toast_price_fetched(
                            &unit.code,
                            &amount(&price.close, quote.as_ref()),
                        ),
                    ));
                }
            }
            (written, failed) => {
                let reasons: Vec<_> = failed.iter().map(|(_, reason)| *reason).collect();
                router.notify(warn(some_failed_toast(written.len(), &reasons, &source)));
            }
        }
    }
}

/// How often a Client runs the scheduled check while a Ledger stays open, after the one it runs
/// on opening (ADR-0043's "where and when fetching runs").
pub const PRICE_CHECK: std::time::Duration = std::time::Duration::from_secs(60);

/// The day `now` falls on here, which every week a fetch writes or asks for is counted from.
fn local_day(now: DateTime<Utc>) -> NaiveDate {
    now.with_timezone(&Local).date_naive()
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::rc::Rc;

    use chrono::{DateTime, Utc};
    use lib_core::{UnitId, UnitKind};
    use lib_toast::Notice;

    use super::*;
    use crate::fake::{FakeTransport, InMemoryPorts};
    use crate::fetching::{PriceFetching, SharedFetching};
    use crate::protocols::SharedPorts;
    use crate::{FRANKFURTER, Precision, PriceLink, UnitRow, WeeklyPrice};

    fn now() -> DateTime<Utc> {
        NaiveDate::from_ymd_opt(2026, 10, 21)
            .and_then(|day| day.and_hms_opt(9, 0, 0))
            .expect("a real time")
            .and_utc()
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

    #[test]
    fn a_scheduled_round_is_written_once_its_last_response_lands_and_tells_no_one() {
        let mut sample = InMemoryPorts::sample(now());
        sample
            .units
            .units
            .extend([currency("EUR"), currency("USD")]);
        let ports: SharedPorts = Rc::new(RefCell::new(sample));
        let transport = FakeTransport::default();
        let fetching = PriceFetching::shared(Rc::new(transport.clone()));
        let mut presenter =
            FetchedPricesPresenter::new(ports.clone(), fetching.clone(), Rc::new(now));
        let mut notices: Vec<Notice> = Vec::new();
        let priced = |code: &str| {
            let ports = ports.borrow();
            let unit = ports
                .units()
                .units()
                .into_iter()
                .find(|unit| unit.code == code)
                .expect("listed");
            unit.prices.iter().any(|price| {
                price.week == WeeklyPrice::week_of(now().with_timezone(&Local).date_naive())
            })
        };

        presenter.fetch_due();
        let requests = transport.requests();
        assert_eq!(requests.len(), 2, "eur and usd, one Frankfurter round");

        let eur = transport.answer(
            requests[0].0,
            crate::TransportOutcome::Answered {
                status: 200,
                body: r#"{"rate":1.65}"#.into(),
                retry_after: None,
            },
        );
        presenter.handle(eur, &mut notices);
        assert!(!priced("EUR"), "held until the round's last response lands");

        let usd = transport.answer_last(200, r#"{"rate":1.52}"#);
        presenter.handle(usd, &mut notices);

        assert!(
            priced("EUR") && priced("USD"),
            "the whole round is written at once"
        );
        assert!(
            notices.is_empty(),
            "a scheduled round raises no Toast and no status"
        );
        assert_eq!(fetching.borrow().in_flight(), 0);
    }

    mod failures {
        use std::cell::Cell;

        use chrono::Duration;

        use super::*;
        use crate::fake::InMemoryPorts;
        use crate::protocols::UnitsPorts;
        use crate::{
            BuiltInOverride, CustomSource, Endpoint, FetchFailure, FetchOutcome, KeyPlacement,
            PriceSourceAuth, PriceSourceId, PriceSourceRecord, TransportError, TransportOutcome,
            Updates,
        };
        use secrecy::ExposeSecret as _;

        /// The sample with `units` added, a clock a test moves, and the fetching beside it.
        struct Client {
            ports: Rc<RefCell<InMemoryPorts>>,
            transport: FakeTransport,
            fetching: SharedFetching,
            at: Rc<Cell<DateTime<Utc>>>,
            presenter: FetchedPricesPresenter,
            notices: Vec<Notice>,
        }

        fn client(units: Vec<UnitRow>) -> Client {
            crate::presenter::test_support::locale();
            let mut sample = InMemoryPorts::sample(now());
            sample.units.units.extend(units);
            let (ports, shared) = sample.shared();
            let transport = FakeTransport::default();
            let fetching = PriceFetching::shared(Rc::new(transport.clone()));
            let at = Rc::new(Cell::new(now()));
            let clock = {
                let at = at.clone();
                Rc::new(move || at.get())
            };
            Client {
                presenter: FetchedPricesPresenter::new(shared, fetching.clone(), clock),
                ports,
                transport,
                fetching,
                at,
                notices: Vec::new(),
            }
        }

        impl Client {
            fn later(&mut self, by: Duration) {
                self.at.set(self.at.get() + by);
            }

            /// Runs the scheduled check and returns how many requests it sent to Frankfurter (the
            /// sample's btc comes due on CoinGecko too).
            fn check(&mut self) -> usize {
                let frankfurter = |transport: &FakeTransport| {
                    transport
                        .urls()
                        .iter()
                        .filter(|url| url.contains("frankfurter"))
                        .count()
                };
                let before = frankfurter(&self.transport);
                self.presenter.fetch_due();
                frankfurter(&self.transport) - before
            }

            /// Answers every request still out, oldest first, each with `outcome(url)`.
            fn answer(&mut self, outcome: impl Fn(&str) -> TransportOutcome) {
                let out: Vec<_> = self
                    .transport
                    .requests()
                    .into_iter()
                    .filter(|(id, _)| self.fetching.borrow().in_flight_request(*id))
                    .collect();
                for (id, request) in out {
                    let response = self.transport.answer(id, outcome(&request.url));
                    self.presenter.handle(response, &mut self.notices);
                }
            }

            fn fetch_now(&mut self, code: &str) {
                let id = self.unit(code).id;
                crate::interactor::FetchNow { id }
                    .run(
                        &*self.ports.borrow(),
                        &mut self.fetching.borrow_mut(),
                        self.at.get(),
                    )
                    .expect("sent");
            }

            fn fetch_source(&mut self, id: PriceSourceId) -> crate::Result<usize> {
                crate::interactor::FetchSource { id }.run(
                    &*self.ports.borrow(),
                    &mut self.fetching.borrow_mut(),
                    self.at.get(),
                )
            }

            fn pump(&mut self) -> Vec<Notice> {
                let mut notices = Vec::new();
                self.presenter.pump(&mut notices);
                notices
            }

            fn sent(&self) -> usize {
                self.transport.requests().len()
            }

            /// `source`'s last round's retry time.
            fn retry_at(&self, id: PriceSourceId) -> Option<DateTime<Utc>> {
                let ports = self.ports.borrow();
                let catalogue = ports.source_catalogue();
                let source = catalogue.source(id)?;
                match self.fetching.borrow().source_fetch(&source)?.outcome {
                    FetchOutcome::Failed { retry_at, .. } => Some(retry_at),
                    FetchOutcome::Fetched => None,
                }
            }

            fn unit(&self, code: &str) -> UnitRow {
                self.ports
                    .borrow()
                    .units
                    .units
                    .iter()
                    .find(|unit| unit.code == code)
                    .cloned()
                    .expect("listed")
            }

            fn unit_failure(&self, code: &str) -> Option<FetchFailure> {
                let unit = self.unit(code);
                let ports = self.ports.borrow();
                let catalogue = ports.source_catalogue();
                let source = catalogue.source(unit.link.source_id)?;
                self.fetching
                    .borrow()
                    .unit_fetch(&unit, &source)
                    .and_then(|last| last.failure())
            }

            fn source_failure(&self, id: PriceSourceId) -> Option<FetchFailure> {
                let ports = self.ports.borrow();
                let catalogue = ports.source_catalogue();
                let source = catalogue.source(id)?;
                self.fetching
                    .borrow()
                    .source_fetch(&source)
                    .and_then(|last| last.failure())
            }

            fn retime(&mut self, updates: Updates) {
                self.ports
                    .borrow_mut()
                    .price_sources
                    .records
                    .push(PriceSourceRecord::BuiltIn(BuiltInOverride {
                        id: FRANKFURTER,
                        name: "Frankfurter".into(),
                        updates,
                    }));
            }
        }

        fn offline(_: &str) -> TransportOutcome {
            TransportOutcome::Failed(TransportError::Offline)
        }

        fn priced(_: &str) -> TransportOutcome {
            TransportOutcome::Answered {
                status: 200,
                body: r#"{"rate":1.5}"#.into(),
                retry_after: None,
            }
        }

        /// A 404 for `code`'s request, a price for every other.
        fn missing(code: &'static str) -> impl Fn(&str) -> TransportOutcome {
            move |url| {
                if url.contains(&format!("/{code}/")) {
                    TransportOutcome::Answered {
                        status: 404,
                        body: r#"{"message":"not found"}"#.into(),
                        retry_after: None,
                    }
                } else {
                    priced(url)
                }
            }
        }

        #[test]
        fn a_round_failure_backs_off_the_source_for_its_interval_silently() {
            let mut client = client(vec![currency("EUR")]);
            assert_eq!(client.check(), 1);
            client.answer(offline);

            assert!(
                client.notices.is_empty(),
                "a scheduled failure raises nothing"
            );
            assert_eq!(
                client.source_failure(FRANKFURTER),
                Some(FetchFailure::Offline)
            );
            assert_eq!(client.unit_failure("EUR"), Some(FetchFailure::Offline));
            client.later(Duration::hours(23));
            assert_eq!(client.check(), 0, "Frankfurter backs off a day");
            client.later(Duration::hours(1));
            assert_eq!(client.check(), 1);
        }

        #[test]
        fn a_unit_failure_backs_off_that_unit_alone_and_the_next_success_clears_it() {
            let mut client = client(vec![currency("EUR"), currency("USD")]);
            client.retime(Updates::Every15Minutes);
            client.check();
            client.answer(missing("EUR"));
            assert_eq!(client.unit_failure("EUR"), Some(FetchFailure::NotFound));
            assert_eq!(client.unit_failure("USD"), None);
            assert_eq!(client.source_failure(FRANKFURTER), None);

            client.ports.borrow_mut().units.units.push(currency("GBP"));
            client.later(Duration::minutes(1));
            assert_eq!(client.check(), 1, "gbp, while eur backs off");
            client.answer(priced);

            client.later(Duration::minutes(15));
            assert_eq!(client.check(), 3, "eur's back-off is over");
            client.answer(priced);
            assert_eq!(client.unit_failure("EUR"), None);
            let unit = client.unit("EUR");
            let ports = client.ports.borrow();
            let catalogue = ports.source_catalogue();
            let source = catalogue.source(FRANKFURTER).expect("frankfurter");
            assert_eq!(
                client
                    .fetching
                    .borrow()
                    .unit_fetch(&unit, &source)
                    .map(|last| last.outcome),
                Some(FetchOutcome::Fetched)
            );
        }

        #[test]
        fn editing_a_units_link_clears_its_failure_and_makes_it_due_at_once() {
            let mut client = client(vec![currency("EUR")]);
            client.check();
            client.answer(missing("EUR"));
            client.later(Duration::minutes(1));
            assert_eq!(client.check(), 0);

            client
                .ports
                .borrow_mut()
                .units
                .units
                .iter_mut()
                .find(|unit| unit.code == "EUR")
                .expect("eur")
                .link
                .symbol = "EURO".into();

            assert_eq!(client.unit_failure("EUR"), None);
            assert_eq!(client.check(), 1);
        }

        #[test]
        fn editing_the_sources_updates_clears_its_failure_and_its_units() {
            let mut client = client(vec![currency("EUR")]);
            client.check();
            client.answer(offline);
            client.later(Duration::minutes(1));
            assert_eq!(client.check(), 0);

            client.retime(Updates::Hourly);

            assert_eq!(client.source_failure(FRANKFURTER), None);
            assert_eq!(client.unit_failure("EUR"), None);
            assert_eq!(client.check(), 1);
        }

        #[test]
        fn deactivating_a_unit_drops_its_failure() {
            let mut client = client(vec![currency("EUR")]);
            client.check();
            client.answer(missing("EUR"));
            client
                .ports
                .borrow_mut()
                .units
                .units
                .iter_mut()
                .find(|unit| unit.code == "EUR")
                .expect("eur")
                .is_active = false;

            assert_eq!(client.unit_failure("EUR"), None);
        }

        #[test]
        fn a_source_needing_a_key_fails_key_missing_unsent_until_a_key_is_stored() {
            let id = PriceSourceId::new();
            let mut eur = currency("EUR");
            eur.link.source_id = id;
            let mut client = client(vec![eur]);
            client
                .ports
                .borrow_mut()
                .price_sources
                .records
                .push(PriceSourceRecord::Custom(CustomSource {
                    id,
                    name: "Rates".into(),
                    endpoint: Endpoint {
                        address: "https://rates.example".into(),
                        request: "/{symbol}".into(),
                        price_path: "/rate".into(),
                        auth: PriceSourceAuth::ApiKey {
                            placement: KeyPlacement::Header,
                            name: "x-key".into(),
                            optional: false,
                        },
                        headers: Vec::new().into(),
                        batch: None,
                        history: None,
                    },
                    updates: Updates::Hourly,
                }));

            assert_eq!(client.check(), 0, "nothing is sent without a key");
            assert_eq!(client.source_failure(id), Some(FetchFailure::KeyMissing));
            assert_eq!(client.unit_failure("EUR"), Some(FetchFailure::KeyMissing));

            client
                .ports
                .borrow_mut()
                .keys
                .keys
                .insert(id, crate::fake::InMemoryKey::Saved("k3y".into()));
            client.fetching.borrow_mut().key_stored(id);

            assert_eq!(client.source_failure(id), None);
            assert_eq!(client.unit_failure("EUR"), None);
            client.presenter.fetch_due();
            let (_, sent) = client
                .transport
                .requests()
                .into_iter()
                .find(|(_, request)| request.url.contains("rates.example"))
                .expect("the stored key lets it send");
            let key = sent.key.expect("the key goes with it");
            assert_eq!(
                (key.name.as_str(), key.value.expose_secret()),
                ("x-key", "k3y")
            );
        }

        #[test]
        fn a_key_that_wont_decrypt_here_reads_as_key_missing_and_coingecko_asks_keyless() {
            let id = PriceSourceId::new();
            let mut eur = currency("EUR");
            eur.link.source_id = id;
            let mut btc = currency("BTC");
            btc.link.source_id = crate::COINGECKO;
            let mut client = client(vec![eur, btc]);
            let keyed = Endpoint {
                address: "https://rates.example".into(),
                request: "/{symbol}".into(),
                price_path: "/rate".into(),
                auth: PriceSourceAuth::ApiKey {
                    placement: KeyPlacement::Param,
                    name: "apikey".into(),
                    optional: false,
                },
                headers: Vec::new().into(),
                batch: None,
                history: None,
            };
            let mut ports = client.ports.borrow_mut();
            ports
                .price_sources
                .records
                .push(PriceSourceRecord::Custom(CustomSource {
                    id,
                    name: "Rates".into(),
                    endpoint: keyed,
                    updates: Updates::Hourly,
                }));
            for source in [id, crate::COINGECKO] {
                ports
                    .keys
                    .keys
                    .insert(source, crate::fake::InMemoryKey::Unreadable);
            }
            drop(ports);

            client.presenter.fetch_due();

            assert_eq!(client.source_failure(id), Some(FetchFailure::KeyMissing));
            let requests = client.transport.requests();
            assert_eq!(requests.len(), 1, "only CoinGecko is asked");
            assert!(requests[0].1.url.contains("coingecko"));
            assert_eq!(requests[0].1.key, None, "keyless");
        }

        fn throttled(retry_after: Option<&str>) -> TransportOutcome {
            TransportOutcome::Answered {
                status: 429,
                body: String::new(),
                retry_after: retry_after.map(str::to_string),
            }
        }

        #[test]
        fn a_round_failure_mid_round_stops_it_and_writes_what_it_fetched_in_one_go() {
            let currencies: Vec<UnitRow> = (0..62)
                .map(|index| currency(&format!("C{index:02}")))
                .collect();
            let mut client = client(currencies);
            assert_eq!(client.fetch_source(FRANKFURTER), Ok(62));
            assert_eq!(client.sent(), 60, "two wait for Frankfurter's call rate");

            let rate_limited = client.transport.requests()[3].0;
            client.answer(|url| {
                if url.ends_with("/C03/AUD") {
                    throttled(Some("30"))
                } else {
                    priced(url)
                }
            });
            assert!(!client.fetching.borrow().in_flight_request(rate_limited));

            assert_eq!(
                client.notices,
                [Notice::Toast(
                    ToastKind::Warning,
                    "Fetched 59 prices from Frankfurter; then rate limited, try again later".into()
                )]
            );
            assert_eq!(
                client.source_failure(FRANKFURTER),
                Some(FetchFailure::RateLimited)
            );
            assert_eq!(client.unit_failure("C03"), Some(FetchFailure::RateLimited));
            assert_eq!(
                client.unit_failure("C60"),
                None,
                "never reached, never failed"
            );
            assert!(client.unit("C60").prices.is_empty());
            assert_eq!(client.fetching.borrow().in_flight(), 0);
            client.later(Duration::minutes(2));
            client.pump();
            assert_eq!(
                client.sent(),
                60,
                "the stopped round's queued calls never go"
            );
        }

        #[test]
        fn a_round_stopped_before_it_wrote_anything_keeps_its_failure_toast() {
            let mut client = client(vec![currency("EUR")]);
            client.fetch_now("EUR");
            client.answer(|_| throttled(None));

            assert_eq!(
                client.notices,
                [Notice::Toast(
                    ToastKind::Warning,
                    "Couldn't fetch from Frankfurter: rate limited, try again later".into()
                )]
            );
        }

        #[test]
        fn a_429_backs_off_for_the_longer_of_the_usual_rule_and_its_retry_after() {
            let mut client = client(vec![]);
            client.fetch_now("BTC");
            client.answer(|_| throttled(Some("3600")));
            assert_eq!(
                client.retry_at(crate::COINGECKO),
                Some(now() + Duration::hours(1)),
                "an hour outlasts CoinGecko's 15 minutes"
            );

            let mut client = self::client(vec![]);
            client.fetch_now("BTC");
            client.answer(|_| throttled(Some("30")));
            assert_eq!(
                client.retry_at(crate::COINGECKO),
                Some(now() + Duration::minutes(15)),
                "15 minutes outlast 30 seconds"
            );
        }

        #[test]
        fn fetch_now_waits_out_an_unexpired_retry_after_with_one_info_toast_then_sends() {
            let mut client = client(vec![]);
            client.fetch_now("BTC");
            client.answer(|_| throttled(Some("40")));
            client.notices.clear();

            client.fetch_now("BTC");
            assert_eq!(
                client.sent(),
                1,
                "held by the Retry-After, not the back-off"
            );
            assert_eq!(
                client.pump(),
                [Notice::Toast(
                    ToastKind::Info,
                    "Waiting for CoinGecko, 40 s\u{2026}".into()
                )]
            );
            client.later(Duration::milliseconds(1500));
            assert_eq!(client.pump(), [], "no countdown");
            assert_eq!(client.sent(), 1);

            client.later(Duration::milliseconds(38_500));
            assert_eq!(client.pump(), [], "the send says nothing");
            assert_eq!(client.sent(), 2, "the call goes out once the wait is over");
            client.answer(priced_btc);
            assert!(
                !client
                    .notices
                    .iter()
                    .any(|notice| matches!(notice, Notice::Status(_))),
                "the landing puts nothing on the status line: {:?}",
                client.notices
            );
        }

        #[test]
        fn a_round_queued_behind_the_call_rate_is_in_flight_and_says_fetching_not_waiting() {
            let mut client = client(vec![]);
            for _ in 0..5 {
                client.fetch_now("BTC");
                client.answer(priced_btc);
            }
            client.notices.clear();

            assert_eq!(client.fetch_source(crate::COINGECKO), Ok(1));
            assert_eq!(client.sent(), 5, "CoinGecko's five a minute are spent");
            assert!(client.fetching.borrow().source_asked_for(crate::COINGECKO));
            assert_eq!(
                client.fetch_source(crate::COINGECKO),
                Err(crate::Error::AlreadyFetching)
            );
            assert_eq!(client.pump(), [], "no Retry-After, so no Waiting");

            client.later(Duration::seconds(60));
            client.pump();
            assert_eq!(client.sent(), 6);
        }

        fn priced_btc(_: &str) -> TransportOutcome {
            TransportOutcome::Answered {
                status: 200,
                body: r#"{"bitcoin":{"aud":92568}}"#.into(),
                retry_after: None,
            }
        }

        #[test]
        fn closing_the_ledger_drops_every_failure() {
            let mut client = client(vec![currency("EUR")]);
            client.check();
            client.answer(offline);

            // A Client builds its fetching afresh for each Open Ledger.
            client.fetching = PriceFetching::shared(Rc::new(client.transport.clone()));

            assert_eq!(client.source_failure(FRANKFURTER), None);
        }

        #[test]
        fn failures_are_never_written_to_the_ledger() {
            let mut client = client(vec![currency("EUR"), currency("USD")]);
            let before = client.ports.borrow().clone();
            client.check();
            client.answer(missing("EUR"));
            client.later(Duration::days(1));
            client.check();
            client.answer(offline);

            let after = client.ports.borrow().clone();
            assert_eq!(after.price_sources, before.price_sources);
            assert_eq!(client.unit("EUR").prices, Vec::new());
            assert_eq!(
                after.units.units.len(),
                before.units.units.len(),
                "only usd's fetched price was written"
            );
        }

        #[test]
        fn a_fetch_now_that_failed_raises_one_warning_naming_what_failed() {
            let mut client = client(vec![currency("EUR")]);
            client.fetch_now("EUR");
            client.answer(offline);
            client.fetch_now("EUR");
            client.answer(missing("EUR"));

            assert_eq!(
                client.notices,
                [
                    Notice::Toast(
                        ToastKind::Warning,
                        "Couldn't fetch from Frankfurter: offline, try again later".into()
                    ),
                    Notice::Toast(
                        ToastKind::Warning,
                        "Couldn't fetch EUR: Frankfurter has no EUR in AUD".into()
                    ),
                ]
            );
        }
    }

    mod backfill {
        use std::cell::Cell;

        use chrono::{Duration, NaiveDate};

        use super::*;
        use crate::fake::InMemoryPorts;
        use crate::protocols::UnitsPorts;
        use crate::{
            BuiltInOverride, COINGECKO, FetchFailure, PriceSourceRecord, TransportError,
            TransportOutcome, Updates, YAHOO_FINANCE,
        };

        const FRANKFURTER_RATES: &str =
            include_str!("../../fixtures/history/frankfurter_rates.json");

        fn day(y: i32, m: u32, d: u32) -> NaiveDate {
            NaiveDate::from_ymd_opt(y, m, d).expect("a real date")
        }

        /// The sample with `units` added, a clock a test moves, and the fetching beside it.
        struct Client {
            ports: Rc<RefCell<InMemoryPorts>>,
            transport: FakeTransport,
            fetching: SharedFetching,
            at: Rc<Cell<DateTime<Utc>>>,
            presenter: FetchedPricesPresenter,
            notices: Vec<Notice>,
        }

        fn client(units: Vec<UnitRow>) -> Client {
            crate::presenter::test_support::locale();
            let mut sample = InMemoryPorts::sample(now());
            sample.units.units.extend(units);
            let (ports, shared) = sample.shared();
            let transport = FakeTransport::default();
            let fetching = PriceFetching::shared(Rc::new(transport.clone()));
            let at = Rc::new(Cell::new(now()));
            let clock = {
                let at = at.clone();
                Rc::new(move || at.get())
            };
            Client {
                presenter: FetchedPricesPresenter::new(shared, fetching.clone(), clock),
                ports,
                transport,
                fetching,
                at,
                notices: Vec::new(),
            }
        }

        impl Client {
            fn later(&mut self, by: Duration) {
                self.at.set(self.at.get() + by);
            }

            /// Accounts in the Unit `code`, the earliest opened on `on`.
            fn opened(&mut self, code: &str, on: NaiveDate) {
                let id = self.unit(code).id;
                self.ports.borrow_mut().openings.by_unit.insert(id, on);
            }

            fn unit(&self, code: &str) -> UnitRow {
                self.ports
                    .borrow()
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
                        .borrow_mut()
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

            /// Answers the newest request with `outcome`.
            fn answer(&mut self, outcome: TransportOutcome) {
                let (id, _) = self.transport.requests().pop().expect("a request out");
                let response = self.transport.answer(id, outcome);
                self.presenter.handle(response, &mut self.notices);
            }

            fn weeks(&self, code: &str) -> Vec<NaiveDate> {
                self.unit(code)
                    .prices
                    .iter()
                    .map(|price| price.week)
                    .collect()
            }

            fn history_failure(&self, code: &str) -> Option<(FetchFailure, DateTime<Utc>)> {
                let unit = self.unit(code);
                let ports = self.ports.borrow();
                let catalogue = ports.source_catalogue();
                let source = catalogue.source(unit.link.source_id)?;
                self.fetching.borrow().history_failure(&unit, &source)
            }
        }

        fn rates() -> TransportOutcome {
            TransportOutcome::Answered {
                status: 200,
                body: FRANKFURTER_RATES.into(),
                retry_after: None,
            }
        }

        fn on(source: lib_core::PriceSourceId, code: &str, symbol: &str) -> UnitRow {
            let mut unit = currency(code);
            unit.link.source_id = source;
            unit.link.symbol = symbol.into();
            unit
        }

        #[test]
        fn a_unit_with_accounts_opened_in_2019_asks_back_to_then_and_fills_what_comes() {
            let mut client = client(vec![currency("EUR")]);
            client.opened("EUR", day(2019, 6, 5));

            client.presenter.backfill();

            assert_eq!(
                client.urls(),
                [
                    "https://api.frankfurter.dev/v2/rates?base=EUR&quotes=AUD&from=2019-06-03&to=2026-10-18"
                ],
                "from the week of the first Opened date to the end of last week"
            );
            client.answer(rates());

            assert_eq!(
                client.weeks("EUR"),
                [day(2026, 9, 28), day(2026, 10, 5), day(2026, 10, 12)]
            );
            assert!(client.notices.is_empty(), "an open's backfill says nothing");
            assert_eq!(
                client.unit("EUR").prices[1].close,
                "1.6092".parse().expect("a decimal"),
                "each week closes on its last day's rate"
            );

            client.presenter.backfill();
            assert_eq!(
                client.urls().len(),
                1,
                "the weeks that came back empty aren't asked again until the link changes"
            );
            client.edit("EUR", |eur| eur.link.symbol = "eur".into());
            client.presenter.backfill();
            assert_eq!(client.urls().len(), 2, "a relinked Unit asks again");
        }

        /// Adds a Custom source with `history`, returning its id.
        fn custom_source(
            client: &Client,
            history: Option<crate::HistoryEndpoint>,
        ) -> lib_core::PriceSourceId {
            crate::add_custom_source(
                &mut client.ports.borrow_mut().price_sources.records,
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

        fn answered(body: &str) -> TransportOutcome {
            TransportOutcome::Answered {
                status: 200,
                body: body.into(),
                retry_after: None,
            }
        }

        #[test]
        fn a_custom_source_backfills_from_a_list_of_points_or_a_series_keyed_by_date() {
            let mut client = client(Vec::new());
            let points = custom_source(
                &client,
                Some(crate::HistoryEndpoint {
                    request: "/points/{symbol}?from={from_unix}&to={to_unix}"
                        .to_string()
                        .into(),
                    series_path: "/prices".to_string().into(),
                    date_path: "/0".to_string().into(),
                    price_path: "/1".to_string().into(),
                    shape: crate::SeriesShape::Points,
                }),
            );
            let keyed = custom_source(
                &client,
                Some(crate::HistoryEndpoint {
                    request: "/keyed/{symbol}?from={from}&to={to}".to_string().into(),
                    series_path: "/rates".to_string().into(),
                    date_path: "".into(),
                    price_path: "/{quote}".to_string().into(),
                    shape: crate::SeriesShape::Keyed,
                }),
            );
            client
                .ports
                .borrow_mut()
                .units
                .units
                .extend([on(points, "EUR", "EUR"), on(keyed, "USD", "USD")]);
            client.edit("EUR", |eur| eur.created_on = Some(day(2026, 10, 7)));
            client.edit("USD", |usd| usd.created_on = Some(day(2026, 10, 7)));

            client.presenter.backfill();
            let urls = client.urls();
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
            for (id, request) in client.transport.requests() {
                let body = if request.url.contains("/points/") {
                    // 6 and 13 October 2026, in milliseconds.
                    r#"{"prices":[[1791244800000,1.61],[1791849600000,1.62]]}"#
                } else {
                    r#"{"rates":{"2026-10-06":{"AUD":1.51},"2026-10-13T00:00:00Z":{"AUD":"1.52"}}}"#
                };
                let response = client.transport.answer(id, answered(body));
                client.presenter.handle(response, &mut client.notices);
            }

            assert_eq!(client.weeks("EUR"), [day(2026, 10, 5), day(2026, 10, 12)]);
            assert_eq!(client.weeks("USD"), [day(2026, 10, 5), day(2026, 10, 12)]);
            assert_eq!(
                client.unit("USD").prices[1].close,
                "1.52".parse().expect("a decimal")
            );
        }

        #[test]
        fn a_custom_source_with_no_history_request_never_backfills() {
            let mut client = client(Vec::new());
            let plain = custom_source(&client, None);
            client
                .ports
                .borrow_mut()
                .units
                .units
                .push(on(plain, "EUR", "EUR"));
            client.edit("EUR", |eur| eur.created_on = Some(day(2026, 9, 30)));

            client.presenter.backfill();

            assert!(client.urls().is_empty());
            assert!(
                !client
                    .fetching
                    .borrow()
                    .history_fetching(client.unit("EUR").id)
            );
        }

        #[test]
        fn with_no_accounts_the_floor_is_the_units_creation_and_with_neither_nothing_is_asked() {
            let mut client = client(vec![currency("EUR"), currency("USD")]);
            client.edit("EUR", |eur| eur.created_on = Some(day(2026, 9, 30)));

            client.presenter.backfill();

            assert_eq!(
                client.urls(),
                [
                    "https://api.frankfurter.dev/v2/rates?base=EUR&quotes=AUD&from=2026-09-28&to=2026-10-18"
                ],
                "usd has no Accounts and no creation date"
            );
        }

        #[test]
        fn a_hand_set_week_and_one_synced_meanwhile_are_never_overwritten() {
            let mut client = client(vec![currency("EUR")]);
            client.opened("EUR", day(2026, 9, 28));
            let hand_set = WeeklyPrice {
                week: day(2026, 9, 28),
                close: "9.99".parse().expect("a decimal"),
                open: None,
                high: None,
                low: None,
                written_at: now() - Duration::days(20),
            };
            client.edit("EUR", |eur| eur.prices.push(hand_set.clone()));

            client.presenter.backfill();
            assert_eq!(
                client.urls(),
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
            client.edit("EUR", |eur| eur.prices.push(synced.clone()));
            client.answer(rates());

            let eur = client.unit("EUR");
            assert_eq!(eur.prices.len(), 3);
            assert!(eur.prices.contains(&hand_set), "the hand-set week stays");
            assert!(
                eur.prices.contains(&synced),
                "the week synced meanwhile wins"
            );
        }

        #[test]
        fn coingecko_asks_only_as_far_back_as_its_history_depth() {
            let mut btc = on(COINGECKO, "ETH", "ethereum");
            btc.kind = UnitKind::Crypto;
            let mut client = client(vec![btc]);
            client.opened("ETH", day(2019, 6, 5));

            client.presenter.backfill();

            let urls = client.urls();
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
            let mut client = client(vec![gone, manual, on(YAHOO_FINANCE, "BHP", "BHP.AX")]);
            for code in ["GBP", "NZD", "AUD", "BHP"] {
                client.opened(code, day(2026, 9, 28));
            }
            client
                .ports
                .borrow_mut()
                .price_sources
                .records
                .push(PriceSourceRecord::BuiltIn(BuiltInOverride {
                    id: YAHOO_FINANCE,
                    name: "Yahoo Finance".into(),
                    updates: Updates::ManualOnly,
                }));

            client.presenter.backfill();
            assert_eq!(client.urls(), Vec::<String>::new());

            crate::interactor::FetchNow {
                id: client.unit("BHP").id,
            }
            .run(
                &*client.ports.borrow(),
                &mut client.fetching.borrow_mut(),
                client.at.get(),
            )
            .expect("sent");
            client.answer(TransportOutcome::Answered {
                status: 200,
                body: include_str!("../../fixtures/prices/yahoo_chart.json").into(),
                retry_after: None,
            });
            assert!(
                client
                    .urls()
                    .last()
                    .is_some_and(|url| url.contains("period1=")),
                "a Manual only source backfills through fetch now"
            );
        }

        #[test]
        fn one_history_request_per_source_at_a_time_oldest_gap_first() {
            let mut client = client(vec![
                currency("EUR"),
                currency("USD"),
                on(COINGECKO, "ETH", "ethereum"),
            ]);
            client.opened("EUR", day(2026, 9, 28));
            client.opened("USD", day(2026, 9, 14));
            client.opened("ETH", day(2026, 9, 28));

            client.presenter.backfill();

            let urls = client.urls();
            assert_eq!(urls.len(), 2, "one to Frankfurter, one to CoinGecko");
            assert!(urls[0].contains("base=USD"), "usd's gap is the oldest");
            assert!(urls[1].contains("ethereum"), "sources run side by side");
            assert!(
                client
                    .fetching
                    .borrow()
                    .history_fetching(client.unit("USD").id)
            );

            let (usd, _) = client.transport.requests()[0].clone();
            let response = client.transport.answer(usd, rates());
            client.presenter.handle(response, &mut client.notices);
            assert!(client.urls()[2].contains("base=EUR"), "then eur's");
        }

        #[test]
        fn a_failure_retries_every_ten_minutes_until_last_week_has_a_price() {
            let mut client = client(vec![currency("EUR")]);
            client.opened("EUR", day(2026, 9, 28));
            client.presenter.backfill();
            client.answer(TransportOutcome::Failed(TransportError::Offline));

            assert_eq!(
                client.history_failure("EUR"),
                Some((FetchFailure::Offline, now() + Duration::minutes(10)))
            );
            assert!(client.notices.is_empty(), "never a Toast");
            let catalogue_failure = {
                let unit = client.unit("EUR");
                let ports = client.ports.borrow();
                let catalogue = ports.source_catalogue();
                let source = catalogue.source(FRANKFURTER).expect("frankfurter");
                client.fetching.borrow().unit_fetch(&unit, &source).copied()
            };
            assert_eq!(catalogue_failure, None, "never fetch failed or Last fetch");

            client.later(Duration::minutes(9));
            client.presenter.fetch_due();
            let history = |client: &Client| {
                client
                    .urls()
                    .iter()
                    .filter(|url| url.contains("/v2/rates?"))
                    .count()
            };
            assert_eq!(
                history(&client),
                1,
                "offline backs Frankfurter off for its rounds too"
            );

            client.later(Duration::minutes(1));
            client.presenter.fetch_due();
            assert_eq!(history(&client), 2, "ten minutes on, it asks again");
            client.answer(rates());
            assert_eq!(client.history_failure("EUR"), None);
        }

        #[test]
        fn a_rate_limited_history_waits_the_longer_of_ten_minutes_and_its_retry_after() {
            let mut client = client(vec![currency("EUR")]);
            client.opened("EUR", day(2026, 9, 28));
            client.presenter.backfill();
            client.answer(TransportOutcome::Answered {
                status: 429,
                body: String::new(),
                retry_after: Some("1800".into()),
            });

            assert_eq!(
                client.history_failure("EUR"),
                Some((FetchFailure::RateLimited, now() + Duration::minutes(30)))
            );
        }

        #[test]
        fn a_retry_stops_once_last_week_has_a_price() {
            let mut client = client(vec![currency("EUR")]);
            client.opened("EUR", day(2026, 9, 28));
            client.presenter.backfill();
            client.answer(TransportOutcome::Failed(TransportError::Timeout));
            let synced = WeeklyPrice {
                week: day(2026, 10, 12),
                close: "1.6".parse().expect("a decimal"),
                open: None,
                high: None,
                low: None,
                written_at: now(),
            };
            client.edit("EUR", |eur| eur.prices.push(synced));

            client.later(Duration::minutes(10));
            client.presenter.fetch_due();

            assert_eq!(
                client
                    .urls()
                    .iter()
                    .filter(|url| url.contains("/v2/rates?"))
                    .count(),
                1,
                "older gaps wait for the next open"
            );
            assert_eq!(client.history_failure("EUR"), None);
        }

        #[test]
        fn a_fetch_now_and_its_backfill_send_no_status_from_start_to_landing() {
            let mut client = client(vec![currency("EUR")]);
            client.opened("EUR", day(2026, 9, 28));
            crate::interactor::FetchNow {
                id: client.unit("EUR").id,
            }
            .run(
                &*client.ports.borrow(),
                &mut client.fetching.borrow_mut(),
                client.at.get(),
            )
            .expect("sent");
            client.presenter.pump(&mut client.notices);

            client.answer(TransportOutcome::Answered {
                status: 200,
                body: r#"{"rate":1.6}"#.into(),
                retry_after: None,
            });
            client.presenter.pump(&mut client.notices);
            client.answer(rates());
            client.presenter.pump(&mut client.notices);

            assert_eq!(
                client.notices,
                [Notice::Toast(
                    ToastKind::Success,
                    "Fetched EUR at 1.60 AUD".into()
                )],
                "the landing Toast alone; the history is carried by the Weekly Prices table"
            );
            assert_eq!(
                client.weeks("EUR").len(),
                4,
                "three past weeks and this one"
            );
        }
    }
}
