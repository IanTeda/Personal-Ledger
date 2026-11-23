//! Where a price response lands, and what the Client's minute timer and response poll reach: the
//! Client hands each response here through its input door, so the round it completes is written
//! as that input's one commit and every View reading the Units repaints, and pumps the queued
//! calls out from the same poll. It draws nothing; a Fetch now that wrote a price says so in a
//! Success Toast, one that failed in a Warning Toast, and a scheduled round says nothing at all,
//! whether it landed or failed.

use chrono::Local;
use lib_toast::{Notice, Notify, ToastKind};

use super::Clock;
use super::fetch_text::{
    Asked, fetch_status, round_failed_toast, round_stopped_toast, some_failed_toast, test_toast,
    unit_failed_toast, waiting_status,
};
use super::units_pane::amount;
use crate::fetching::{PriceResponse, SharedFetching};
use crate::interactor::{FetchDue, RecordFetchedPrices, RoundOutcome};
use crate::msg;
use crate::protocols::SharedPorts;

pub struct FetchedPricesPresenter {
    ports: SharedPorts,
    fetching: SharedFetching,
    clock: Clock,
    /// The *Waiting for…* status last put up, so a pump says only what changed.
    waiting: Option<String>,
}

impl FetchedPricesPresenter {
    pub fn new(ports: SharedPorts, fetching: SharedFetching, clock: Clock) -> Self {
        Self {
            ports,
            fetching,
            clock,
            waiting: None,
        }
    }

    /// Releases the queued calls at the Clock's now: sends what each source's call rate and any
    /// `Retry-After` allow, and counts *Waiting for CoinGecko, 40 s…* down on the status line,
    /// putting back what is still fetching, or clearing it, once the call goes out. Each Client
    /// calls it from its response poll; nothing here waits or sleeps.
    pub fn pump(&mut self, router: &mut dyn Notify) {
        let now = (self.clock)();
        let sent = self.fetching.borrow_mut().release(now);
        if sent > 0 {
            tracing::debug!(sent, "Sent queued price requests");
        }
        let ports = self.ports.borrow();
        let fetching = self.fetching.borrow();
        let waiting = waiting_status(&*ports, &fetching, now);
        if waiting == self.waiting {
            return;
        }
        let status = waiting
            .clone()
            .or_else(|| fetch_status(&*ports, &fetching, now))
            .unwrap_or_default();
        router.notify(Notice::Status(status));
        self.waiting = waiting;
    }

    /// The scheduled check, on Ledger open and every minute after: starts a silent round on each
    /// source with a Unit due, leaving a source with a round in flight alone.
    pub fn fetch_due(&mut self) {
        let now = (self.clock)();
        FetchDue {
            today: now.with_timezone(&Local).date_naive(),
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
        let Some(landed) = self.fetching.borrow_mut().land(response, now) else {
            return;
        };
        let asked = landed.asked;
        let named = landed.named;
        let outcome = {
            let mut ports = self.ports.borrow_mut();
            RecordFetchedPrices { landed }.run(
                ports.split_mut().units,
                &mut self.fetching.borrow_mut(),
                now.with_timezone(&Local).date_naive(),
                now,
            )
        };
        if asked {
            self.tell(&outcome, router);
        }
        if named {
            // The status names what is still out and clears, with an empty message, once the last
            // asked-for round has landed.
            let status = {
                let ports = self.ports.borrow();
                fetch_status(&*ports, &self.fetching.borrow(), now)
            };
            router.notify(Notice::Status(status.unwrap_or_default()));
        }
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
        chrono::NaiveDate::from_ymd_opt(2026, 10, 21)
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
                [
                    Notice::Toast(
                        ToastKind::Warning,
                        "Fetched 59 prices from Frankfurter; then rate limited, try again later"
                            .into()
                    ),
                    Notice::Status(String::new()),
                ]
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
        fn fetch_now_waits_out_an_unexpired_retry_after_counting_it_down_then_sends() {
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
                [Notice::Status("Waiting for CoinGecko, 40 s\u{2026}".into())]
            );
            client.later(Duration::milliseconds(1500));
            assert_eq!(
                client.pump(),
                [Notice::Status("Waiting for CoinGecko, 39 s\u{2026}".into())]
            );
            assert_eq!(client.pump(), [], "nothing new to say");
            assert_eq!(client.sent(), 1);

            client.later(Duration::milliseconds(38_500));
            assert_eq!(client.pump(), [Notice::Status(String::new())]);
            assert_eq!(client.sent(), 2, "the call goes out once the wait is over");
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
}
