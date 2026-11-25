//! Fetching a Unit's price from its Price Source (ADR-0062): the plain transport port the Client's
//! adapter (`lib-price-feed`) answers, the request it is handed and the response it hands back,
//! and [`PriceFetching`], the per-Client bookkeeping of what is in flight and how each source's and
//! Unit's last fetch went. Every call to a source waits its turn in [`PriceFetching`]'s queue, paced
//! by the source's [`CallRate`] and any `Retry-After` it answered with (ADR-0043's batching and
//! throttling amendment), and goes out when [`PriceFetching`] is next released.
//!
//! Every piece of provider knowledge stays here: filling an [`Endpoint`]'s placeholders into a
//! request and reading the price at its price path. The transport only carries bytes, so these
//! rules are tested with no network.

use std::cell::RefCell;
use std::rc::Rc;

use chrono::{DateTime, NaiveDate, Utc};
use lib_core::{PriceSourceId, UnitId};

mod backfill;
mod call_queue;
mod fine;
mod fine_prices;
mod history;
mod landing;
mod last_fetch;
mod planning;
mod price_tests;
mod reading;
mod request;
mod rounds;
mod transport;

use backfill::Backfill;
pub use backfill::HISTORY_RETRY;
pub(crate) use backfill::HistoryCall;
use call_queue::CallQueue;
pub use call_queue::CallRate;
pub use fine::{FINE_FRESH, Grain};
pub(crate) use fine::{FineKey, FinePrice, fine_prices, window};
use fine_prices::FinePrices;
pub(crate) use fine_prices::{FineCall, FineState};
pub(crate) use history::history_request;
use history::{HistoryReading, read_history};
pub(crate) use landing::Landing;
pub(crate) use landing::{FetchedPrice, RoundOutcome};
use last_fetch::LastFetches;
pub(crate) use last_fetch::back_off;
pub use last_fetch::{FetchOutcome, LastFetch, MIN_BACK_OFF};
pub(crate) use planning::BackfillUnits;
use price_tests::PriceTests;
pub(crate) use price_tests::{
    HistoryTestOutcome, HistoryTestResult, TestHistory, TestInFlight, TestOutcome, TestResult,
};
pub use reading::RETRY_AFTER_DEFAULT;
use reading::round_failure;
pub(crate) use reading::{Reading, read, retry_after_wait};
pub(crate) use request::{Wanted, fill, is_due, key_missing, price_request, request, round_calls};
use rounds::Landed;
use rounds::Rounds;
pub(crate) use rounds::{InFlight, RoundId, UnitCall};
pub use transport::{
    PriceRequest, PriceResponse, PriceTransport, RequestId, RequestKey, TransportError,
    TransportOutcome,
};

use crate::{Endpoint, PriceLink, PriceSource, UnitRow, Updates};

/// Why a fetch failed: the fixed set the transport's outcome is read onto, so no rule here sees a
/// status code. The first five are round failures (nothing came back for the source); the last
/// two are Unit failures (the round landed without that Unit).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FetchFailure {
    /// The provider couldn't be reached, or took too long.
    Offline,
    /// The provider asked to be called less often (429).
    RateLimited,
    /// The source needs an API key this Client doesn't have.
    KeyMissing,
    /// The provider refused the key (401, 403).
    KeyRejected,
    /// The provider answered with an error (5xx), or a reply not in the expected shape.
    SourceError,
    /// The provider doesn't know the symbol or quote.
    NotFound,
    /// The price path found nothing in the reply.
    NoPrice,
}

impl FetchFailure {
    /// Whether it failed the whole round, backing off the source, rather than one Unit.
    pub fn is_round(self) -> bool {
        !matches!(self, Self::NotFound | Self::NoPrice)
    }
}

/// What of a source a failure was against: editing any of it clears the failure, since the edit
/// may be the fix.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SourceSettings {
    endpoint: Option<Endpoint>,
    updates: Updates,
}

impl SourceSettings {
    pub(crate) fn of(source: &PriceSource<'_>) -> Self {
        Self {
            endpoint: source.endpoint().cloned(),
            updates: source.updates,
        }
    }

    /// Whether these are still `source`'s settings, compared in place rather than by cloning its
    /// endpoint into a second [`SourceSettings`].
    pub(crate) fn matches(&self, source: &PriceSource<'_>) -> bool {
        self.updates == source.updates && self.endpoint.as_ref() == source.endpoint()
    }
}

/// Whether `unit`'s last price, on a source updating every `updates`, is Out of date at `now`
/// (`GLOSSARY.md`): written more than twice the interval ago. Never for Manual only, a Unit with
/// no price or (with no `updates`) one on Manual entry. Worked out from the Ledger, so every
/// Client agrees.
pub(crate) fn is_out_of_date(unit: &UnitRow, updates: Option<Updates>, now: DateTime<Utc>) -> bool {
    let Some(interval) = updates.and_then(Updates::interval) else {
        return false;
    };
    unit.last_price()
        .is_some_and(|price| now - price.written_at > interval * 2)
}

/// The fetching state one Client keeps per open Ledger, beside its ports, and drops when the
/// Ledger closes: never persisted (ADR-0062). The requests and rounds in flight, and each source's
/// and Unit's last fetch with its back-off, so closing the Ledger clears every failure. Six parts
/// hold it, none holding or calling another: every rule spanning two stays a method here.
pub struct PriceFetching {
    calls: CallQueue,
    rounds: Rounds,
    last: LastFetches,
    tests: PriceTests,
    backfill: Backfill,
    fine: FinePrices,
}

/// [`PriceFetching`] as the Presenters share it, single-threaded as both Clients' UI threads are.
pub type SharedFetching = Rc<RefCell<PriceFetching>>;

impl PriceFetching {
    pub(crate) fn new(transport: Rc<dyn PriceTransport>) -> Self {
        Self {
            calls: CallQueue::new(transport),
            rounds: Rounds::default(),
            last: LastFetches::default(),
            tests: PriceTests::default(),
            backfill: Backfill::default(),
            fine: FinePrices::default(),
        }
    }

    pub(crate) fn shared(transport: Rc<dyn PriceTransport>) -> SharedFetching {
        Rc::new(RefCell::new(Self::new(transport)))
    }

    /// How many of the rounds' requests are queued or out.
    pub fn in_flight(&self) -> usize {
        self.rounds.in_flight()
    }

    /// Whether request `id` is still queued or out.
    pub(crate) fn in_flight_request(&self, id: RequestId) -> bool {
        self.rounds.in_flight_request(id)
    }

    /// Whether a request for `unit` is queued or out.
    pub fn fetching(&self, unit: UnitId) -> bool {
        self.rounds.fetching(unit)
    }

    /// Whether `source` has a round queued or out, which a scheduled check leaves alone.
    pub(crate) fn source_in_flight(&self, source: PriceSourceId) -> bool {
        self.rounds.source_in_flight(source)
    }

    /// Whether `unit` is in a round a person asked for that is still out, which its row shows as
    /// *Fetching…*.
    pub fn unit_asked_for(&self, unit: UnitId) -> bool {
        self.rounds.unit_asked_for(unit)
    }

    /// Whether `source` has a round a person asked for still out, which its row shows as
    /// *Fetching…*.
    pub fn source_asked_for(&self, source: PriceSourceId) -> bool {
        self.rounds.source_asked_for(source)
    }

    /// Queues a test's `request` at `now`, remembering what it asked until it lands. Never in a
    /// round, so it neither waits for nor holds up a fetch's response, but it is a call a person
    /// asked for, paced like any other.
    pub(crate) fn send_test(
        &mut self,
        request: PriceRequest,
        asked: TestInFlight,
        now: DateTime<Utc>,
    ) -> RequestId {
        let id = self.calls.enqueue(asked.source, true, request);
        self.tests.sent(id, asked);
        self.release(now);
        id
    }

    /// Sends every queued call its lane allows at `now`: the asked-for first, then the scheduled,
    /// oldest first within each, while the lane's window has room and no `Retry-After` holds it.
    /// A lane that can't send one call sends none after it, so no call overtakes another on its
    /// lane but an asked-for one. Returns how many went out.
    pub(crate) fn release(&mut self, now: DateTime<Utc>) -> usize {
        let sent = self.calls.release(now);
        self.tests.went_out(&sent, now);
        sent.len()
    }

    /// The source a call a person asked for waits on, and the whole seconds left of its
    /// `Retry-After` at `now`, rounded up. `None` while no such call is held, a queued one only
    /// waiting its call rate included.
    pub(crate) fn waiting(&self, now: DateTime<Utc>) -> Option<(PriceSourceId, i64)> {
        self.calls.waiting(now, |id| self.tests.asks(id))
    }

    /// [`Self::waiting`] the first time it is asked of a hold, `None` after: a wait is announced
    /// once as it begins, never counted down.
    pub(crate) fn wait_begun(&mut self, now: DateTime<Utc>) -> Option<(PriceSourceId, i64)> {
        self.calls.wait_begun(now, |id| self.tests.asks(id))
    }

    /// Queues test `test`'s history request, once its price passed, as a call a person asked for.
    fn send_test_history(
        &mut self,
        test: RequestId,
        asked: &TestInFlight,
        history: TestHistory,
        now: DateTime<Utc>,
    ) {
        let request = history_request(
            &history.endpoint,
            &history.history,
            &asked.symbol,
            &asked.quote_code,
            history.range,
            asked.key.clone(),
        );
        let id = self.calls.enqueue(asked.source, true, request);
        self.tests.history_sent(id, test, history);
        self.release(now);
    }

    /// Whether test `id` is still out.
    pub(crate) fn testing(&self, id: RequestId) -> bool {
        self.tests.testing(id)
    }

    /// Whether a test of stored source `source` is out.
    pub(crate) fn source_testing(&self, source: PriceSourceId) -> bool {
        self.tests.source_testing(source)
    }

    /// Test `id`'s result, once it has landed.
    pub(crate) fn test_result(&self, id: RequestId) -> Option<&TestResult> {
        self.tests.result(id)
    }

    /// Forgets test `id`: its form closed or tested again. A response still to come lands as no
    /// one's and is dropped.
    pub(crate) fn drop_test(&mut self, id: RequestId) {
        self.tests.drop_test(id);
    }

    /// Starts a round on `source`, `asked` when a person asked for it. Every request sent into it
    /// is held until the last lands; a round sent nothing never lands, so open one only to send.
    pub(crate) fn open_round(&mut self, source: &PriceSource<'_>, asked: bool) -> RoundId {
        self.rounds
            .open(source.id, SourceSettings::of(source), asked)
    }

    /// Starts a round on `source` for `wanted`, `asked` when a person asked for it, sending its
    /// calls at `now`, and hands back their ids: none, opening no round, when the source makes no
    /// call for them. A source needing a key `key` doesn't supply sends nothing: the source and
    /// each wanted Unit fail Key missing instead.
    ///
    /// # Errors
    /// [`crate::Error::KeyMissing`] for a missing key, [`crate::Error::NotFetched`] for a source
    /// that doesn't fetch.
    pub(crate) fn start_round(
        &mut self,
        source: &PriceSource<'_>,
        wanted: &[Wanted],
        key: Option<&secrecy::SecretString>,
        asked: bool,
        now: DateTime<Utc>,
    ) -> crate::Result<Vec<RequestId>> {
        let endpoint = source.endpoint().ok_or(crate::Error::NotFetched)?;
        let settings = SourceSettings::of(source);
        if key_missing(endpoint, key) {
            let failure = Some(FetchFailure::KeyMissing);
            self.record_source(source.id, &settings, now, failure);
            for one in wanted {
                self.record_unit(one.unit, one.link.clone(), &settings, now, failure);
            }
            return Err(crate::Error::KeyMissing);
        }
        let calls = round_calls(endpoint, wanted, key, &settings);
        if calls.is_empty() {
            return Ok(Vec::new());
        }
        let round = self.open_round(source, asked);
        Ok(calls
            .into_iter()
            .map(|(request, sent)| self.send(round, request, sent, now))
            .collect())
    }

    /// Queues `request` for `sent` in `round`, remembering it until its response comes back, and
    /// sends what the call rate allows at `now`.
    pub(crate) fn send(
        &mut self,
        round: RoundId,
        request: PriceRequest,
        sent: InFlight,
        now: DateTime<Utc>,
    ) -> RequestId {
        let (lane, asked) = match self.rounds.source_of(round) {
            Some((source, asked)) => (Some(source), asked),
            None => (None, false),
        };
        let id = self.calls.enqueue(lane, asked, request);
        self.rounds.sent(round, id, sent);
        self.release(now);
        id
    }

    /// `unit`'s last fetch on `source` while it still holds: the Unit is active and neither its
    /// link nor its source's settings changed since. Editing either, or deactivating the Unit,
    /// drops it.
    pub fn unit_fetch(&self, unit: &UnitRow, source: &PriceSource<'_>) -> Option<&LastFetch> {
        self.last.unit_fetch(unit, source)
    }

    /// `source`'s last round while it still holds: its settings haven't changed since.
    pub fn source_fetch(&self, source: &PriceSource<'_>) -> Option<&LastFetch> {
        self.last.source_fetch(source)
    }

    /// A key was stored or replaced for `source`: its Key missing and Key rejected failures, and
    /// its Units', clear, so the next check tries it again.
    pub fn key_stored(&mut self, source: PriceSourceId) {
        self.last.key_stored(source);
    }

    /// Asks for a series as the Unit prices view wants it, at `now`: a person's call, so it skips
    /// the source's back-off but waits out a `Retry-After`, and waits behind a round on its source
    /// already in flight rather than being refused. Forgets the series' last failure and any series
    /// the Unit fetched on another link. Asks nothing while the same series is already asked.
    pub(crate) fn ask_fine(&mut self, call: FineCall, now: DateTime<Utc>) {
        self.fine.forget(&call);
        if self.fine.asked(call.key) {
            return;
        }
        if self.source_in_flight(call.source()) {
            tracing::debug!(price_source_id = %call.source(), "A finer prices request waits for its source's round");
            self.fine.park(call);
            return;
        }
        self.send_fine(call, now);
    }

    fn send_fine(&mut self, mut call: FineCall, now: DateTime<Utc>) {
        let Some(request) = call.request.take() else {
            return;
        };
        let source = call.source();
        tracing::debug!(price_source_id = %source, unit_id = %call.key.unit, grain = ?call.key.grain, "Asking a price source for finer prices");
        let id = self.calls.enqueue(Some(source), true, request);
        self.fine.sent(id, call);
        self.release(now);
    }

    /// Sends the finer prices requests that waited for `source`'s round, now it has landed.
    fn release_parked(&mut self, source: PriceSourceId, now: DateTime<Utc>) {
        for call in self.fine.release_parked(source) {
            self.send_fine(call, now);
        }
    }

    /// Notes that a series couldn't be asked for `reason` without sending, as a key this Client
    /// doesn't have.
    pub(crate) fn fine_refused(&mut self, key: FineKey, link: PriceLink, reason: FetchFailure) {
        self.fine.refused(key, link, reason);
    }

    /// Whether `source` has a finer prices request waiting or out, which a scheduled check counts
    /// as in flight.
    fn fine_in_flight(&self, source: PriceSourceId) -> bool {
        self.fine.in_flight(source)
    }

    /// Whether the series `key` is cached on `link` and still fresh at `now`: a past window for the
    /// session, one holding today for [`FINE_FRESH`].
    pub(crate) fn fine_fresh(&self, key: FineKey, link: &PriceLink, now: DateTime<Utc>) -> bool {
        self.fine.fresh(key, link, now)
    }

    /// What the view has of the series `key` on `link` at `now`.
    pub(crate) fn fine_state(
        &self,
        key: FineKey,
        link: &PriceLink,
        now: DateTime<Utc>,
    ) -> FineState<'_> {
        let (queued, hold) = match self.fine.out_call(key) {
            Some((id, source)) => (self.calls.queued(id), self.calls.held_until(Some(source))),
            None => (false, None),
        };
        self.fine.state(key, link, now, queued, hold)
    }

    /// Whether `unit` on `source` is due a scheduled fetch on `today` at `now`: the source fetches,
    /// has no round or finer prices request in flight and isn't backing off, the Unit isn't
    /// backing off, and it was fetched on another link than it has now or is due by its source's
    /// Updates.
    pub(crate) fn due(
        &self,
        unit: &UnitRow,
        source: &PriceSource<'_>,
        today: NaiveDate,
        now: DateTime<Utc>,
    ) -> bool {
        source.endpoint().is_some()
            && !self.source_in_flight(source.id)
            && !self.fine_in_flight(source.id)
            && !self.source_backing_off(source, now)
            && !self.unit_backing_off(unit, source, now)
            && (self.relinked(unit) || is_due(unit, source.updates, today, now))
    }

    /// Whether `source` failed a round, or a history request found it offline or rate limited,
    /// and is still backing off at `now`.
    pub(crate) fn source_backing_off(&self, source: &PriceSource<'_>, now: DateTime<Utc>) -> bool {
        self.last.source_backing_off(source, now)
    }

    /// Queues `call` behind its source's history requests, oldest gap first, and sends the next
    /// if the source has none in flight. Returns false, queuing nothing, for a Unit whose history
    /// is already queued or out.
    fn queue_history(&mut self, call: HistoryCall, now: DateTime<Utc>) -> bool {
        let source = call.source;
        if !self.backfill.queue(call) {
            return false;
        }
        self.start_history(source, now);
        true
    }

    /// Sends `source`'s next waiting history request, unless one is already in flight: one at a
    /// time per source, through the call queue like any call.
    fn start_history(&mut self, source: PriceSourceId, now: DateTime<Utc>) {
        let Some((call, request)) = self.backfill.next(source) else {
            return;
        };
        tracing::debug!(
            price_source_id = %source,
            unit_id = %call.unit,
            weeks = call.weeks.len(),
            "Asking a price source for a Unit's history"
        );
        let id = self.calls.enqueue(Some(source), call.asked, request);
        self.backfill.sent(id, call);
        self.release(now);
    }

    /// Holds `source` until its `Retry-After`, read at `now`, when `outcome` is a 429, whatever
    /// kind of request it answers. The hold's end, or `None` for any other outcome.
    fn hold_on_429(
        &mut self,
        source: PriceSourceId,
        outcome: &TransportOutcome,
        now: DateTime<Utc>,
    ) -> Option<DateTime<Utc>> {
        let TransportOutcome::Answered {
            status: 429,
            retry_after,
            ..
        } = outcome
        else {
            return None;
        };
        let until = now + retry_after_wait(retry_after.as_deref(), now);
        self.calls.hold(Some(source), until);
        Some(until)
    }

    /// Notes that `call` failed for `reason` at `at`: it is tried again after [`HISTORY_RETRY`],
    /// or its source's `Retry-After` if longer. Offline and Rate limited back the source off for
    /// everyone and fail the Units waiting behind it the same way, since they would fail too.
    fn history_failed(&mut self, call: &HistoryCall, reason: FetchFailure, at: DateTime<Utc>) {
        let hold = self.calls.held_until(Some(call.source));
        if let Some(until) = self.backfill.history_failed(call, reason, at, hold) {
            self.last
                .back_off_quietly(call.source, call.settings.clone(), until);
        }
    }

    /// Whether `unit`'s history request is queued or out.
    pub fn history_fetching(&self, unit: UnitId) -> bool {
        self.backfill.fetching(unit)
    }

    /// Why `unit`'s last history request on `source` failed and when it is tried again, while it
    /// still holds: the Unit is active and neither its link nor its source's settings changed.
    pub fn history_failure(
        &self,
        unit: &UnitRow,
        source: &PriceSource<'_>,
    ) -> Option<(FetchFailure, DateTime<Utc>)> {
        self.backfill.failure(unit, source)
    }

    /// Whether `unit` failed and is still backing off at `now`.
    fn unit_backing_off(
        &self,
        unit: &UnitRow,
        source: &PriceSource<'_>,
        now: DateTime<Utc>,
    ) -> bool {
        self.last.unit_backing_off(unit, source, now)
    }

    /// Whether this Client fetched `unit` on another link than it has now: editing its source,
    /// symbol or quote makes it due straight away.
    fn relinked(&self, unit: &UnitRow) -> bool {
        self.last.relinked(unit)
    }

    /// Notes how `unit`'s fetch on `link` went at `at`, backing it off on a failure.
    fn record_unit(
        &mut self,
        unit: UnitId,
        link: PriceLink,
        settings: &SourceSettings,
        at: DateTime<Utc>,
        failure: Option<FetchFailure>,
    ) {
        self.last.record_unit(unit, link, settings, at, failure);
    }

    /// Notes how `source`'s round went at `at`: failed on a round failure, which backs it off, or
    /// fetched, which clears the last.
    fn record_source(
        &mut self,
        source: PriceSourceId,
        settings: &SourceSettings,
        at: DateTime<Utc>,
        failure: Option<FetchFailure>,
    ) {
        let hold = self.calls.held_until(Some(source));
        self.last.record_source(source, settings, at, failure, hold);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::FRANKFURTER;

    mod out_of_date {
        use chrono::Duration;

        use super::*;
        use crate::{Precision, UnitKind};

        fn now() -> DateTime<Utc> {
            NaiveDate::from_ymd_opt(2026, 10, 21)
                .and_then(|day| day.and_hms_opt(9, 0, 0))
                .expect("a real time")
                .and_utc()
        }

        fn written(at: DateTime<Utc>) -> UnitRow {
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
            unit.record_price("1.65".parse().expect("a decimal"), at.date_naive(), at);
            unit
        }

        #[test]
        fn a_price_is_out_of_date_past_twice_its_sources_interval() {
            for (updates, twice) in [
                (Updates::Every15Minutes, Duration::minutes(30)),
                (Updates::Hourly, Duration::hours(2)),
                (Updates::Daily, Duration::hours(48)),
            ] {
                let fresh = written(now() - twice);
                let old = written(now() - twice - Duration::seconds(1));
                assert!(!is_out_of_date(&fresh, Some(updates), now()), "{updates:?}");
                assert!(is_out_of_date(&old, Some(updates), now()), "{updates:?}");
            }
        }

        #[test]
        fn manual_only_manual_entry_and_an_unpriced_unit_are_never_out_of_date() {
            let ancient = written(now() - Duration::weeks(10));
            assert!(!is_out_of_date(&ancient, Some(Updates::ManualOnly), now()));
            assert!(!is_out_of_date(&ancient, None, now()));
            let mut unpriced = ancient;
            unpriced.prices.clear();
            assert!(!is_out_of_date(&unpriced, Some(Updates::Daily), now()));
        }
    }
}
