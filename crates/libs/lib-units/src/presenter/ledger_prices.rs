//! The Ledger prices module both Clients host, one per Open Ledger: it owns the Ledger's price
//! fetching and the feed's response channel, runs the open check and backfill as the Ledger opens,
//! and lands each response the Client asks it to, so the round it completes is written as that
//! input's one commit and every View reading the Units repaints. It pumps the queued calls out
//! when the Client's poll asks. It draws nothing; a Fetch now that wrote a price says so in a
//! Success Toast, one that failed in a Warning Toast, and a scheduled round says nothing at all,
//! whether it landed or failed. Each Client keeps its own minute timer, clock and commit.
//!
//! It also backfills (ADR-0043's backfill amendment): on Ledger open, after a person's fetch now
//! and again for a failed history request on the minute check, it asks each Unit's source for the
//! past weeks it has no Weekly Price for, and writes each Unit's weeks as that response's one
//! commit. A backfill never toasts, and nothing here touches the status line (ADR-0048): a fetch now's
//! history shows in the Weekly Prices table.

use std::rc::Rc;
use std::sync::mpsc::{Receiver, TryRecvError};

use chrono::{DateTime, Local, NaiveDate, Utc};
use lib_toast::{Notice, Notify, ToastKind};

use super::Clock;
use super::fetch_text::{
    Asked, round_failed_toast, round_stopped_toast, some_failed_toast, unit_failed_toast,
    wait_begun_toast,
};
use super::price_source_test::PriceSourceTest;
use super::units_pane::amount;
use crate::fetching::{
    BackfillUnits, Landing, PriceFetching, PriceResponse, PriceTransport, SharedFetching,
};
use crate::interactor::{FetchDue, RecordFetchedPrices, RecordPriceHistory, RoundOutcome};
use crate::msg;
use crate::protocols::SharedPorts;

/// What [`LedgerPrices::land_next`] found on the feed's channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Received {
    /// One response landed, and its writes wait on the Client's commit.
    Landed,
    /// Nothing is waiting; the Client looks again on its next poll.
    Empty,
    /// The feed has gone, so nothing more will arrive and the Client stops polling.
    Gone,
}

pub struct LedgerPrices {
    ports: SharedPorts,
    fetching: SharedFetching,
    responses: Receiver<PriceResponse>,
    clock: Clock,
}

impl LedgerPrices {
    /// A Ledger's prices as it opens (app start, `:open`, `:new`): builds its fetching over
    /// `transport`, whose answers arrive on `responses`, then runs the scheduled check and the
    /// backfill, once. The Client then runs [`Self::check`] every [`PRICE_CHECK`] while the
    /// Ledger stays open.
    pub fn open(
        ports: SharedPorts,
        transport: Rc<dyn PriceTransport>,
        responses: Receiver<PriceResponse>,
        clock: Clock,
    ) -> Self {
        let mut prices = Self::unopened(ports, transport, responses, clock);
        prices.opened();
        prices
    }

    /// The Ledger's prices before the open check, so a test arranges the Ledger first.
    pub(crate) fn unopened(
        ports: SharedPorts,
        transport: Rc<dyn PriceTransport>,
        responses: Receiver<PriceResponse>,
        clock: Clock,
    ) -> Self {
        Self {
            ports,
            fetching: PriceFetching::shared(transport),
            responses,
            clock,
        }
    }

    /// The open check and backfill [`Self::open`] runs.
    pub(crate) fn opened(&mut self) {
        self.check();
        self.backfill();
    }

    /// The fetching every Units Presenter on this Ledger shares, for the Units page to fetch now,
    /// test and ask finer prices through.
    pub fn fetching(&self) -> SharedFetching {
        self.fetching.clone()
    }

    /// Lands at most one response waiting on the feed's channel, raising what it tells on
    /// `notify`. The Client commits after a [`Received::Landed`]; on [`Received::Empty`] and
    /// [`Received::Gone`] nothing was written.
    pub fn land_next(&mut self, notify: &mut dyn Notify) -> Received {
        match self.responses.try_recv() {
            Ok(response) => {
                self.land(response, notify);
                Received::Landed
            }
            Err(TryRecvError::Empty) => Received::Empty,
            Err(TryRecvError::Disconnected) => Received::Gone,
        }
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
    pub fn check(&mut self) {
        let now = (self.clock)();
        let today = local_day(now);
        let ports = self.ports.borrow();
        let mut fetching = self.fetching.borrow_mut();
        FetchDue { today, now }.run(&*ports, &mut fetching);
        fetching.backfill(BackfillUnits::Due, &*ports, today, now);
    }

    /// The backfill on Ledger open (app start, `:open`, `:new`): asks every Unit's source for the
    /// past weeks it has no Weekly Price for, silently. The sync effort calls it too, once a sync
    /// completes.
    pub fn backfill(&mut self) {
        let now = (self.clock)();
        self.fetching.borrow_mut().backfill(
            BackfillUnits::Every,
            &*self.ports.borrow(),
            local_day(now),
            now,
        );
    }

    /// Lands `response` through fetching and does what the [`Landing`] asks: writes a round's
    /// prices or a Unit's backfilled weeks, raises a test's Toast, tells a Fetch now's outcome and
    /// backfills the Units it priced. A scheduled round and a backfill raise nothing, and a form's
    /// test is held for the form to show. A fetched price is never undoable, so it raises no
    /// `Notice::Changed`.
    fn land(&mut self, response: PriceResponse, router: &mut dyn Notify) {
        let now = (self.clock)();
        let today = local_day(now);
        let landing = {
            let ports = self.ports.borrow();
            self.fetching
                .borrow_mut()
                .land(response, now, today, &Local, ports.units())
        };
        match landing {
            Landing::Nothing => {}
            Landing::Test(result) => {
                router.notify(PriceSourceTest::landed_toast(
                    &result,
                    &*self.ports.borrow(),
                ));
            }
            Landing::Prices {
                prices,
                outcome,
                backfill,
            } => {
                RecordFetchedPrices {
                    prices,
                    today,
                    at: now,
                }
                .run(self.ports.borrow_mut().split_mut().units);
                if let Some(outcome) = outcome {
                    self.tell(&outcome, router);
                }
                // A fetch now goes on to fill the past weeks of each Unit it priced.
                if !backfill.is_empty() {
                    self.fetching.borrow_mut().backfill(
                        BackfillUnits::Asked(backfill),
                        &*self.ports.borrow(),
                        today,
                        now,
                    );
                }
            }
            Landing::History { unit, link, weeks } => {
                // Never a Toast: the Weekly Prices table shows a fetch now's history in place.
                RecordPriceHistory { unit, link, weeks }
                    .run(self.ports.borrow_mut().split_mut().units);
            }
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

/// How often a Client runs the scheduled check while a Ledger stays open, after the one it runs
/// on opening (ADR-0043's "where and when fetching runs").
pub const PRICE_CHECK: std::time::Duration = std::time::Duration::from_secs(60);

/// The day `now` falls on here, which every week a fetch writes or asks for is counted from.
fn local_day(now: DateTime<Utc>) -> NaiveDate {
    now.with_timezone(&Local).date_naive()
}

#[cfg(test)]
mod tests {
    use chrono::{DateTime, Utc};
    use lib_core::{UnitId, UnitKind};
    use lib_toast::Notice;

    use super::*;
    use crate::fake::{FakeTransport, InMemoryPorts, LedgerPricesFixture};
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
        let mut client = LedgerPricesFixture::new(sample, now());
        let ports = client.shared();
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

        client.prices().check();
        let requests = client.transport().requests();
        assert_eq!(requests.len(), 2, "eur and usd, one Frankfurter round");

        let eur = client.transport().answer(
            requests[0].0,
            crate::TransportOutcome::Answered {
                status: 200,
                body: r#"{"rate":1.65}"#.into(),
                retry_after: None,
            },
        );
        client.land(eur);
        assert!(!priced("EUR"), "held until the round's last response lands");

        let usd = client.transport().answer_last(200, r#"{"rate":1.52}"#);
        client.land(usd);

        assert!(
            priced("EUR") && priced("USD"),
            "the whole round is written at once"
        );
        assert!(
            client.notices().is_empty(),
            "a scheduled round raises no Toast and no status"
        );
        assert_eq!(client.fetching().borrow().in_flight(), 0);
    }

    /// What [`LedgerPrices::land_next`] finds on the feed: one response at a time, nothing, or a
    /// feed gone.
    mod receiving {
        use super::*;
        use crate::presenter::Received;

        fn client() -> LedgerPricesFixture {
            let mut sample = InMemoryPorts::sample(now());
            sample.units.units.push(currency("EUR"));
            LedgerPricesFixture::new(sample, now())
        }

        #[test]
        fn an_empty_feed_lands_nothing() {
            let mut client = client();

            assert_eq!(client.land_next().0, Received::Empty);
        }

        #[test]
        fn a_waiting_response_lands_once_and_leaves_the_feed_empty() {
            let mut client = client();
            client.prices().check();
            let eur = client.transport().answer_last(200, r#"{"rate":1.65}"#);
            client.land(eur);

            assert_eq!(
                client.fetching().borrow().in_flight(),
                0,
                "the round landed"
            );
            assert_eq!(
                client.land_next().0,
                Received::Empty,
                "at most one per call"
            );
        }

        #[test]
        fn a_feed_that_has_gone_says_so() {
            let mut client = client();
            client.hang_up();

            assert_eq!(client.land_next().0, Received::Gone);
        }
    }

    /// One test per [`Landing`] variant: `land_next` writes, tells and backfills as it asks.
    mod dispatch {
        use super::*;
        use crate::fake::InMemoryPorts;
        use crate::interactor::FetchNow;
        use crate::presenter::price_source_test::{FormTest, PriceSourceTest};
        use crate::protocols::UnitsPorts;

        const FRANKFURTER_RATES: &str =
            include_str!("../../fixtures/history/frankfurter_rates.json");

        fn client() -> LedgerPricesFixture {
            crate::presenter::test_support::locale();
            let mut sample = InMemoryPorts::sample(now());
            let eur = currency("EUR");
            sample.openings.by_unit.insert(
                eur.id,
                NaiveDate::from_ymd_opt(2026, 9, 28).expect("a real date"),
            );
            sample.units.units.push(eur);
            LedgerPricesFixture::new(sample, now())
        }

        fn eur(client: &LedgerPricesFixture) -> UnitRow {
            client
                .ports()
                .borrow()
                .units
                .units
                .iter()
                .find(|unit| unit.code == "EUR")
                .cloned()
                .expect("listed")
        }

        /// Tests Frankfurter for EUR as a form does, holding the test so its result stays.
        fn form_test(client: &LedgerPricesFixture) -> PriceSourceTest {
            let endpoint = {
                let ports = client.ports().borrow();
                let catalogue = ports.source_catalogue();
                catalogue
                    .source(FRANKFURTER)
                    .and_then(|source| source.endpoint().cloned())
                    .expect("frankfurter fetches")
            };
            let mut test = PriceSourceTest::new(client.fetching().clone(), client.clock());
            test.run(FormTest {
                source: Some(FRANKFURTER),
                source_name: "Frankfurter".into(),
                endpoint,
                symbol: "EUR".into(),
                quote: UnitId::SEED_AUD,
                quote_code: "AUD".into(),
                key: None,
                unsaved: false,
            });
            test
        }

        #[test]
        fn nothing_writes_and_tells_nothing() {
            let mut client = client();
            let before = client.ports().borrow().clone();
            let form = form_test(&client);

            let response = client.transport().answer_last(200, r#"{"rate":1.6}"#);
            client.land(response);

            assert!(client.notices().is_empty());
            assert_eq!(*client.ports().borrow(), before);
            assert!(
                form.view(&*client.ports().borrow(), None).is_some(),
                "held for the form"
            );
        }

        #[test]
        fn a_test_raises_its_one_toast() {
            let mut client = client();
            let eur = eur(&client);
            let refused = PriceSourceTest::in_background(
                client.fetching(),
                &client.clock(),
                &*client.ports().borrow(),
                &eur,
            );
            assert_eq!(refused, None);

            let response = client.transport().answer_last(200, r#"{"rate":1.6}"#);
            client.land(response);

            assert!(
                matches!(client.notices(), [Notice::Toast(_, _)]),
                "{:?}",
                client.notices()
            );
        }

        #[test]
        fn prices_are_written_told_and_backfilled() {
            let mut client = client();
            FetchNow {
                id: eur(&client).id,
            }
            .run(
                &*client.ports().borrow(),
                &mut client.fetching().borrow_mut(),
                client.now(),
            )
            .expect("sent");

            let response = client.transport().answer_last(200, r#"{"rate":1.6}"#);
            client.land(response);

            assert_eq!(eur(&client).prices.len(), 1, "written");
            assert_eq!(
                client.notices(),
                [Notice::Toast(
                    ToastKind::Success,
                    "Fetched EUR at 1.60 AUD".into()
                )],
                "told"
            );
            assert!(
                client
                    .transport()
                    .urls()
                    .last()
                    .is_some_and(|url| url.contains("/v2/rates?")),
                "its past weeks are asked for next"
            );
        }

        #[test]
        fn history_is_written_silently() {
            let mut client = client();
            client.prices().backfill();

            let response = client.transport().answer_last(200, FRANKFURTER_RATES);
            client.land(response);

            assert_eq!(eur(&client).prices.len(), 3, "its three past weeks");
            assert!(client.notices().is_empty());
        }
    }

    mod failures {
        use chrono::Duration;

        use super::*;
        use crate::fake::InMemoryPorts;
        use crate::protocols::UnitsPorts;
        use crate::{
            BuiltInOverride, CustomSource, Endpoint, FetchFailure, KeyPlacement, PriceSourceAuth,
            PriceSourceId, PriceSourceRecord, TransportError, TransportOutcome, Updates,
        };
        use secrecy::ExposeSecret as _;

        /// The sample with `units` added, on the fixture's clock and fetching.
        fn client(units: Vec<UnitRow>) -> LedgerPricesFixture {
            crate::presenter::test_support::locale();
            let mut sample = InMemoryPorts::sample(now());
            sample.units.units.extend(units);
            LedgerPricesFixture::new(sample, now())
        }

        /// What these tests ask of the fixture beyond its own interface.
        trait Client {
            /// Runs the scheduled check and returns how many requests it sent to Frankfurter (the
            /// sample's btc comes due on CoinGecko too).
            fn check(&mut self) -> usize;
            /// Answers every request still out, oldest first, each with `outcome(url)`.
            fn answer(&mut self, outcome: impl Fn(&str) -> TransportOutcome);
            fn fetch_now(&mut self, code: &str);
            fn fetch_source(&mut self, id: PriceSourceId) -> crate::Result<usize>;
            fn sent(&self) -> usize;
            fn unit(&self, code: &str) -> UnitRow;
            fn unit_failure(&self, code: &str) -> Option<FetchFailure>;
            fn source_failure(&self, id: PriceSourceId) -> Option<FetchFailure>;
            fn retime(&mut self, updates: Updates);
        }

        impl Client for LedgerPricesFixture {
            fn check(&mut self) -> usize {
                let frankfurter = |transport: &FakeTransport| {
                    transport
                        .urls()
                        .iter()
                        .filter(|url| url.contains("frankfurter"))
                        .count()
                };
                let before = frankfurter(self.transport());
                self.prices().check();
                frankfurter(self.transport()) - before
            }

            fn answer(&mut self, outcome: impl Fn(&str) -> TransportOutcome) {
                let out: Vec<_> = self
                    .transport()
                    .requests()
                    .into_iter()
                    .filter(|(id, _)| self.fetching().borrow().in_flight_request(*id))
                    .collect();
                for (id, request) in out {
                    let response = self.transport().answer(id, outcome(&request.url));
                    self.land(response);
                }
            }

            fn fetch_now(&mut self, code: &str) {
                let id = self.unit(code).id;
                crate::interactor::FetchNow { id }
                    .run(
                        &*self.ports().borrow(),
                        &mut self.fetching().borrow_mut(),
                        self.now(),
                    )
                    .expect("sent");
            }

            fn fetch_source(&mut self, id: PriceSourceId) -> crate::Result<usize> {
                crate::interactor::FetchSource { id }.run(
                    &*self.ports().borrow(),
                    &mut self.fetching().borrow_mut(),
                    self.now(),
                )
            }

            fn sent(&self) -> usize {
                self.transport().requests().len()
            }

            fn unit(&self, code: &str) -> UnitRow {
                self.ports()
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
                let ports = self.ports().borrow();
                let catalogue = ports.source_catalogue();
                let source = catalogue.source(unit.link.source_id)?;
                self.fetching()
                    .borrow()
                    .unit_fetch(&unit, &source)
                    .and_then(|last| last.failure())
            }

            fn source_failure(&self, id: PriceSourceId) -> Option<FetchFailure> {
                let ports = self.ports().borrow();
                let catalogue = ports.source_catalogue();
                let source = catalogue.source(id)?;
                self.fetching()
                    .borrow()
                    .source_fetch(&source)
                    .and_then(|last| last.failure())
            }

            fn retime(&mut self, updates: Updates) {
                self.ports()
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
        fn editing_a_units_link_clears_its_failure_and_makes_it_due_at_once() {
            let mut client = client(vec![currency("EUR")]);
            client.check();
            client.answer(missing("EUR"));
            client.later(Duration::minutes(1));
            assert_eq!(client.check(), 0);

            client
                .ports()
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
                .ports()
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
                .ports()
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
                .ports()
                .borrow_mut()
                .keys
                .keys
                .insert(id, crate::fake::InMemoryKey::Saved("k3y".into()));
            client.fetching().borrow_mut().key_stored(id);

            assert_eq!(client.source_failure(id), None);
            assert_eq!(client.unit_failure("EUR"), None);
            client.prices().check();
            let (_, sent) = client
                .transport()
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
            let mut ports = client.ports().borrow_mut();
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

            client.prices().check();

            assert_eq!(client.source_failure(id), Some(FetchFailure::KeyMissing));
            let requests = client.transport().requests();
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
        fn a_fetch_now_stopped_part_way_counts_what_it_fetched_in_one_warning() {
            let currencies: Vec<UnitRow> = (0..62)
                .map(|index| currency(&format!("C{index:02}")))
                .collect();
            let mut client = client(currencies);
            assert_eq!(client.fetch_source(FRANKFURTER), Ok(62));
            client.answer(|url| {
                if url.ends_with("/C03/AUD") {
                    throttled(Some("30"))
                } else {
                    priced(url)
                }
            });

            assert_eq!(
                client.notices(),
                [Notice::Toast(
                    ToastKind::Warning,
                    "Fetched 59 prices from Frankfurter; then rate limited, try again later".into()
                )]
            );
            assert_eq!(
                client.unit("C58").prices.len(),
                1,
                "what it fetched is written"
            );
        }

        #[test]
        fn a_round_stopped_before_it_wrote_anything_keeps_its_failure_toast() {
            let mut client = client(vec![currency("EUR")]);
            client.fetch_now("EUR");
            client.answer(|_| throttled(None));

            assert_eq!(
                client.notices(),
                [Notice::Toast(
                    ToastKind::Warning,
                    "Couldn't fetch from Frankfurter: rate limited, try again later".into()
                )]
            );
        }

        #[test]
        fn fetch_now_waits_out_an_unexpired_retry_after_with_one_info_toast_then_sends() {
            let mut client = client(vec![]);
            client.fetch_now("BTC");
            client.answer(|_| throttled(Some("40")));
            client.take_notices();

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
                    .notices()
                    .iter()
                    .any(|notice| matches!(notice, Notice::Status(_))),
                "the landing puts nothing on the status line: {:?}",
                client.notices()
            );
        }

        #[test]
        fn a_round_queued_behind_the_call_rate_is_in_flight_and_says_fetching_not_waiting() {
            let mut client = client(vec![]);
            for _ in 0..5 {
                client.fetch_now("BTC");
                client.answer(priced_btc);
            }
            client.take_notices();

            assert_eq!(client.fetch_source(crate::COINGECKO), Ok(1));
            assert_eq!(client.sent(), 5, "CoinGecko's five a minute are spent");
            assert!(
                client
                    .fetching()
                    .borrow()
                    .source_asked_for(crate::COINGECKO)
            );
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
            let client = LedgerPricesFixture::new(client.ports().borrow().clone(), now());

            assert_eq!(client.source_failure(FRANKFURTER), None);
        }

        #[test]
        fn a_fetch_now_that_failed_raises_one_warning_naming_what_failed() {
            let mut client = client(vec![currency("EUR")]);
            client.fetch_now("EUR");
            client.answer(offline);
            client.fetch_now("EUR");
            client.answer(missing("EUR"));

            assert_eq!(
                client.notices(),
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
        use chrono::{Duration, NaiveDate};

        use super::*;
        use crate::TransportOutcome;
        use crate::fake::InMemoryPorts;

        const FRANKFURTER_RATES: &str =
            include_str!("../../fixtures/history/frankfurter_rates.json");

        fn day(y: i32, m: u32, d: u32) -> NaiveDate {
            NaiveDate::from_ymd_opt(y, m, d).expect("a real date")
        }

        /// The sample with `units` added, on the fixture's clock and fetching.
        fn client(units: Vec<UnitRow>) -> LedgerPricesFixture {
            crate::presenter::test_support::locale();
            let mut sample = InMemoryPorts::sample(now());
            sample.units.units.extend(units);
            LedgerPricesFixture::new(sample, now())
        }

        /// What these tests ask of the fixture beyond its own interface.
        trait Client {
            /// Accounts in the Unit `code`, the earliest opened on `on`.
            fn opened(&mut self, code: &str, on: NaiveDate);
            fn unit(&self, code: &str) -> UnitRow;
            fn edit(&mut self, code: &str, edit: impl FnOnce(&mut UnitRow));
            fn urls(&self) -> Vec<String>;
            /// Answers the newest request with `outcome`.
            fn answer(&mut self, outcome: TransportOutcome);
            fn weeks(&self, code: &str) -> Vec<NaiveDate>;
        }

        impl Client for LedgerPricesFixture {
            fn opened(&mut self, code: &str, on: NaiveDate) {
                let id = self.unit(code).id;
                self.ports().borrow_mut().openings.by_unit.insert(id, on);
            }

            fn unit(&self, code: &str) -> UnitRow {
                self.ports()
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
                    self.ports()
                        .borrow_mut()
                        .units
                        .units
                        .iter_mut()
                        .find(|unit| unit.code == code)
                        .expect("listed"),
                );
            }

            fn urls(&self) -> Vec<String> {
                self.transport().urls()
            }

            fn answer(&mut self, outcome: TransportOutcome) {
                let (id, _) = self.transport().requests().pop().expect("a request out");
                let response = self.transport().answer(id, outcome);
                self.land(response);
            }

            fn weeks(&self, code: &str) -> Vec<NaiveDate> {
                self.unit(code)
                    .prices
                    .iter()
                    .map(|price| price.week)
                    .collect()
            }
        }

        fn rates() -> TransportOutcome {
            TransportOutcome::Answered {
                status: 200,
                body: FRANKFURTER_RATES.into(),
                retry_after: None,
            }
        }

        #[test]
        fn the_ledger_opening_asks_each_units_past_weeks() {
            let mut client = client(vec![currency("EUR")]);
            client.opened("EUR", day(2026, 9, 28));

            client.prices().opened();
            client.answer(rates());

            assert!(
                client.urls().iter().any(|url| url.contains("/v2/rates?")),
                "{:?}",
                client.urls()
            );
            assert_eq!(client.weeks("EUR").len(), 3, "its three past weeks");
            assert!(
                client.notices().is_empty(),
                "an open's backfill says nothing"
            );
        }

        #[test]
        fn a_fetch_nows_priced_units_go_on_to_ask_their_past_weeks() {
            let mut client = client(vec![currency("EUR")]);
            client.opened("EUR", day(2026, 9, 28));
            crate::interactor::FetchNow {
                id: client.unit("EUR").id,
            }
            .run(
                &*client.ports().borrow(),
                &mut client.fetching().borrow_mut(),
                client.now(),
            )
            .expect("sent");

            client.answer(TransportOutcome::Answered {
                status: 200,
                body: r#"{"rate":1.6}"#.into(),
                retry_after: None,
            });

            assert!(
                client
                    .urls()
                    .last()
                    .is_some_and(|url| url.contains("/v2/rates?")),
                "{:?}",
                client.urls()
            );
        }

        #[test]
        fn the_minute_check_asks_a_failed_history_again_once_its_wait_is_over() {
            let mut client = client(vec![currency("EUR")]);
            client.opened("EUR", day(2026, 9, 28));
            // A fresh price this week, so the minute check sends no round of its own.
            client.edit("EUR", |eur| {
                eur.record_price("1.6".parse().expect("a decimal"), now().date_naive(), now());
            });
            let history = |client: &LedgerPricesFixture| {
                client
                    .urls()
                    .into_iter()
                    .filter(|url| url.contains("/v2/rates?"))
                    .count()
            };
            client.prices().backfill();
            client.answer(TransportOutcome::Failed(crate::TransportError::Offline));
            assert_eq!(history(&client), 1);

            client.later(Duration::minutes(9));
            client.prices().check();
            assert_eq!(history(&client), 1, "not before its wait is over");

            client.later(Duration::minutes(1));
            client.prices().check();
            assert_eq!(history(&client), 2, "asked again");
        }

        #[test]
        fn a_fetch_now_and_its_backfill_send_no_status_from_start_to_landing() {
            let mut client = client(vec![currency("EUR")]);
            client.opened("EUR", day(2026, 9, 28));
            crate::interactor::FetchNow {
                id: client.unit("EUR").id,
            }
            .run(
                &*client.ports().borrow(),
                &mut client.fetching().borrow_mut(),
                client.now(),
            )
            .expect("sent");
            client.pump();

            client.answer(TransportOutcome::Answered {
                status: 200,
                body: r#"{"rate":1.6}"#.into(),
                retry_after: None,
            });
            client.pump();
            client.answer(rates());
            client.pump();

            assert_eq!(
                client.notices(),
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
