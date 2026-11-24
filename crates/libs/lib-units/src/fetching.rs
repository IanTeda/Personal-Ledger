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
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use chrono::{DateTime, Duration, NaiveDate, Utc};
use lib_core::{Money, PriceSourceId, UnitId};
use secrecy::{ExposeSecret as _, SecretString};

mod call_queue;
mod fine;
mod history;
mod last_fetch;
mod reading;
mod request;
mod rounds;
mod transport;

use call_queue::CallQueue;
pub use call_queue::CallRate;
pub use fine::{FINE_FRESH, Grain};
pub(crate) use fine::{FineKey, FinePrice, fine_prices, window};
pub(crate) use history::{
    HistoryReading, floor, fold, gaps, history_request, read_history, wanted_weeks,
};
use last_fetch::LastFetches;
pub(crate) use last_fetch::back_off;
pub use last_fetch::{FetchOutcome, LastFetch, MIN_BACK_OFF};
pub use reading::RETRY_AFTER_DEFAULT;
use reading::round_failure;
pub(crate) use reading::{Reading, read, retry_after_wait};
pub(crate) use request::{Wanted, fill, is_due, key_missing, price_request, request, round_calls};
use rounds::Rounds;
pub(crate) use rounds::{InFlight, Landed, RoundId, UnitCall};
pub use transport::{
    Method, PriceRequest, PriceResponse, PriceTransport, RequestId, RequestKey, TransportError,
    TransportOutcome,
};

use crate::{Endpoint, HistoryEndpoint, PriceLink, PriceSource, UnitRow, Updates};

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
/// and Unit's last fetch with its back-off, so closing the Ledger clears every failure.
pub struct PriceFetching {
    calls: CallQueue,
    rounds: Rounds,
    last: LastFetches,
    /// Tests out, by request: never part of a round, so they write nothing when they land.
    tests: HashMap<RequestId, TestInFlight>,
    /// Tests landed for a form that hasn't read them yet, dropped with the form.
    test_results: HashMap<RequestId, TestResult>,
    /// Tests out whose form has closed, whose responses are dropped as they land.
    dropped: HashSet<RequestId>,
    /// A form's test's history requests out, by request, each with the test it belongs to: the
    /// test is out until its history lands too.
    test_histories: HashMap<RequestId, (RequestId, TestHistory)>,
    /// Each source's one history request queued or out, by request (ADR-0043's backfill
    /// amendment): never part of a round, so it writes only its own Unit's weeks.
    history_out: HashMap<RequestId, HistoryCall>,
    /// The history requests waiting for their source's one in flight, oldest gap first.
    history_queue: HashMap<PriceSourceId, Vec<HistoryCall>>,
    /// Each Unit's failed history request, until it is tried again.
    history_failures: HashMap<UnitId, HistoryFailure>,
    /// The weeks each Unit's provider answered with nothing, on the link it answered for: not
    /// asked again until the Ledger reopens or the link changes.
    empty_weeks: HashMap<UnitId, (PriceLink, HashSet<NaiveDate>)>,
    /// The Unit prices view's Daily and Hourly Price requests queued or out, by request.
    fine_out: HashMap<RequestId, FineCall>,
    /// Daily and Hourly Price requests waiting for their source's round to land, oldest first.
    fine_parked: Vec<FineCall>,
    /// The Daily and Hourly Prices fetched this session, never written to the Ledger.
    fine_cache: HashMap<FineKey, FineEntry>,
    /// Why a series' last request failed, until it is asked again: shown in place and nowhere
    /// else, so never a Unit's or source's Last fetch.
    fine_failed: HashMap<FineKey, (PriceLink, FetchFailure)>,
    /// The `Retry-After` hold a *Waiting for…* Toast last announced, so each wait raises one
    /// whichever of the page or the pump notices it first.
    announced_wait: Option<(PriceSourceId, DateTime<Utc>)>,
}

/// One request for a Unit's Daily or Hourly Prices in one window, as the Unit prices view asked.
#[derive(Debug, Clone)]
pub(crate) struct FineCall {
    pub key: FineKey,
    pub last: NaiveDate,
    pub link: PriceLink,
    pub settings: SourceSettings,
    pub history: HistoryEndpoint,
    /// The quote Unit's code, filling the series' paths.
    pub quote: String,
    /// The request, until it joins the call queue.
    pub request: Option<PriceRequest>,
}

impl FineCall {
    fn source(&self) -> PriceSourceId {
        self.link.source_id
    }
}

/// A series fetched this session, with the link it was for and whether its window held today.
#[derive(Debug, Clone)]
struct FineEntry {
    link: PriceLink,
    fetched_at: DateTime<Utc>,
    current: bool,
    prices: Vec<FinePrice>,
}

/// What the Unit prices view has of a series.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FineState<'a> {
    /// Asked and not yet back: waiting out a `Retry-After` for so many whole seconds, else 0.
    Fetching {
        waiting: i64,
    },
    Shown(&'a [FinePrice]),
    Failed(FetchFailure),
    /// Never asked on this link.
    Unasked,
}

/// How long a failed history request waits before it is tried again, or longer when a
/// `Retry-After` asks for it (ADR-0043's backfill amendment).
pub const HISTORY_RETRY: Duration = Duration::minutes(10);

/// One Unit's history request: what it asked for, so the weeks it brings are written only while
/// the Unit still wants them.
#[derive(Debug, Clone)]
pub(crate) struct HistoryCall {
    pub unit: UnitId,
    /// The Unit's code, which the status line names while a person's fetch now waits on it.
    pub code: String,
    pub link: PriceLink,
    pub source: PriceSourceId,
    pub settings: SourceSettings,
    pub history: HistoryEndpoint,
    /// The quote Unit's code, filling the series' paths.
    pub quote: String,
    /// The empty weeks asked for, oldest first: only these are written.
    pub weeks: Vec<NaiveDate>,
    /// Whether a person's fetch now started it, which the status line names.
    pub asked: bool,
    /// The request, until it joins the call queue.
    pub request: Option<PriceRequest>,
}

/// A Unit's failed history request and when it is tried again, with the link and source settings
/// it was for: editing either drops it.
#[derive(Debug, Clone)]
struct HistoryFailure {
    reason: FetchFailure,
    retry_at: DateTime<Utc>,
    link: PriceLink,
    settings: SourceSettings,
}

/// What a test asked (`GLOSSARY.md`'s Price Source, test): one symbol's price, shown and never
/// written.
#[derive(Clone)]
pub(crate) struct TestInFlight {
    /// The stored source tested, `None` for an unsaved one being added.
    pub source: Option<PriceSourceId>,
    /// The name its Toast and detail give it.
    pub source_name: String,
    pub symbol: String,
    pub quote: UnitId,
    pub quote_code: String,
    pub price_path: String,
    /// The key sent, so its echo in the response can be masked.
    pub key: Option<SecretString>,
    pub sent_at: DateTime<Utc>,
    /// `t` on a row: it answers with a Toast rather than in a form.
    pub toast: bool,
    /// Whether the form's fields differed from the stored source when it was sent.
    pub unsaved: bool,
    /// The history request a form's test asks once its price passes; `None` for a source with
    /// none, and for `t` on a row, which answers with one Toast.
    pub history: Option<TestHistory>,
}

/// What a test asks of a source's history (ADR-0043's amendment "a Custom source's history
/// request"): the last 14 days, through the endpoint the price came from.
#[derive(Clone)]
pub(crate) struct TestHistory {
    pub endpoint: Endpoint,
    pub history: HistoryEndpoint,
    /// The days asked, both included.
    pub range: (NaiveDate, NaiveDate),
}

/// How a test's history went: its points, an empty series being No price, or a reason.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum HistoryTestOutcome {
    Points {
        count: usize,
        newest: NaiveDate,
        /// The newest point's close.
        close: Money,
    },
    Failed(FetchFailure),
}

/// A test's landed history.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct HistoryTestResult {
    pub outcome: HistoryTestOutcome,
    /// The status and body the provider answered, the key masked; `None` when nothing came back.
    pub response: Option<(u16, String)>,
}

/// How a test went: a price, or one of the seven reasons.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum TestOutcome {
    Priced(Money),
    Failed(FetchFailure),
}

/// A landed test, as the form or the Toast shows it.
#[derive(Clone)]
pub(crate) struct TestResult {
    pub asked: TestInFlight,
    pub outcome: TestOutcome,
    pub elapsed: Duration,
    pub at: DateTime<Utc>,
    /// The status and body the provider answered, the key masked; `None` when nothing came back.
    pub response: Option<(u16, String)>,
    /// The history the test asked once its price passed, once it has landed.
    pub history: Option<HistoryTestResult>,
}

/// What stands in for a key echoed back in a response.
const MASK: &str = "\u{2022}\u{2022}\u{2022}";

/// `body` with every echo of `key` masked.
fn masked(body: &str, key: Option<&SecretString>) -> String {
    match key.map(|key| key.expose_secret().trim()) {
        Some(key) if !key.is_empty() => body.replace(key, MASK),
        _ => body.to_string(),
    }
}

/// [`PriceFetching`] as the Presenters share it, single-threaded as both Clients' UI threads are.
pub type SharedFetching = Rc<RefCell<PriceFetching>>;

impl PriceFetching {
    pub fn new(transport: Rc<dyn PriceTransport>) -> Self {
        Self {
            calls: CallQueue::new(transport),
            rounds: Rounds::default(),
            last: LastFetches::default(),
            tests: HashMap::new(),
            test_results: HashMap::new(),
            dropped: HashSet::new(),
            test_histories: HashMap::new(),
            history_out: HashMap::new(),
            history_queue: HashMap::new(),
            history_failures: HashMap::new(),
            empty_weeks: HashMap::new(),
            fine_out: HashMap::new(),
            fine_parked: Vec::new(),
            fine_cache: HashMap::new(),
            fine_failed: HashMap::new(),
            announced_wait: None,
        }
    }

    pub fn shared(transport: Rc<dyn PriceTransport>) -> SharedFetching {
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
        self.tests.insert(id, asked);
        self.release(now);
        id
    }

    /// Sends every queued call its lane allows at `now`: the asked-for first, then the scheduled,
    /// oldest first within each, while the lane's window has room and no `Retry-After` holds it.
    /// A lane that can't send one call sends none after it, so no call overtakes another on its
    /// lane but an asked-for one. Returns how many went out.
    pub(crate) fn release(&mut self, now: DateTime<Utc>) -> usize {
        let sent = self.calls.release(now);
        for id in &sent {
            if let Some(test) = self.tests.get_mut(id) {
                // A test times its answer from when it went out, not from when it queued.
                test.sent_at = now;
            }
        }
        sent.len()
    }

    /// The source a call a person asked for waits on, and the whole seconds left of its
    /// `Retry-After` at `now`, rounded up. `None` while no such call is held, a queued one only
    /// waiting its call rate included.
    pub(crate) fn waiting(&self, now: DateTime<Utc>) -> Option<(PriceSourceId, i64)> {
        self.calls.waiting(now, |id| self.tests.contains_key(&id))
    }

    /// [`Self::waiting`] the first time it is asked of a hold, `None` after: a wait is announced
    /// once as it begins, never counted down.
    pub(crate) fn wait_begun(&mut self, now: DateTime<Utc>) -> Option<(PriceSourceId, i64)> {
        let (source, seconds) = self.waiting(now)?;
        let until = self.calls.held_until(Some(source))?;
        if self.announced_wait == Some((source, until)) {
            return None;
        }
        self.announced_wait = Some((source, until));
        Some((source, seconds))
    }

    /// Whether `response` answers a test this Client sent, dropped or not, so it is never read as
    /// a round's.
    pub(crate) fn is_test(&self, response: &PriceResponse) -> bool {
        self.tests.contains_key(&response.request)
            || self.test_histories.contains_key(&response.request)
    }

    /// Reads a test's `response` at `now`, holding it for its form unless it answers with a
    /// Toast. `None` for a response that is no test's, or a dropped one's.
    pub(crate) fn land_test(
        &mut self,
        response: &PriceResponse,
        now: DateTime<Utc>,
    ) -> Option<TestResult> {
        if self.test_histories.contains_key(&response.request) {
            self.land_test_history(response);
            return None;
        }
        let asked = self.tests.remove(&response.request)?;
        if self.dropped.remove(&response.request) {
            return None;
        }
        let outcome = match read(&response.outcome, &asked.price_path) {
            Reading::Priced(price) => TestOutcome::Priced(price),
            Reading::Failed(reason) => TestOutcome::Failed(reason),
        };
        let answered = match &response.outcome {
            TransportOutcome::Answered { status, body, .. } => {
                Some((*status, masked(body, asked.key.as_ref())))
            }
            TransportOutcome::Failed(_) => None,
        };
        let result = TestResult {
            elapsed: now - asked.sent_at,
            at: now,
            outcome,
            response: answered,
            history: None,
            asked,
        };
        if !result.asked.toast {
            self.test_results.insert(response.request, result.clone());
            if matches!(result.outcome, TestOutcome::Priced(_))
                && let Some(history) = result.asked.history.clone()
            {
                self.send_test_history(response.request, &result.asked, history, now);
            }
        }
        Some(result)
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
        self.test_histories.insert(id, (test, history));
        self.release(now);
    }

    /// Reads a test's history `response` onto its test's result, unless its form has dropped it.
    fn land_test_history(&mut self, response: &PriceResponse) {
        let Some((test, history)) = self.test_histories.remove(&response.request) else {
            return;
        };
        if self.dropped.remove(&response.request) {
            return;
        }
        let Some(result) = self.test_results.get_mut(&test) else {
            return;
        };
        let asked = &result.asked;
        let outcome = match read_history(
            &response.outcome,
            &history.history,
            &asked.symbol,
            &asked.quote_code,
        ) {
            HistoryReading::Points(mut points) => {
                // Stable, so the last of several points on one day is its close, as a week folds.
                points.sort_by_key(|point| point.day);
                match points.last() {
                    Some(newest) => HistoryTestOutcome::Points {
                        count: points.len(),
                        newest: newest.day,
                        close: newest.close.clone(),
                    },
                    None => HistoryTestOutcome::Failed(FetchFailure::NoPrice),
                }
            }
            HistoryReading::Failed(reason) => HistoryTestOutcome::Failed(reason),
        };
        let answered = match &response.outcome {
            TransportOutcome::Answered { status, body, .. } => {
                Some((*status, masked(body, asked.key.as_ref())))
            }
            TransportOutcome::Failed(_) => None,
        };
        result.history = Some(HistoryTestResult {
            outcome,
            response: answered,
        });
    }

    /// Whether test `id` is still out.
    pub(crate) fn testing(&self, id: RequestId) -> bool {
        self.tests.contains_key(&id) || self.test_histories.values().any(|(test, _)| *test == id)
    }

    /// Whether a test of stored source `source` is out.
    pub(crate) fn source_testing(&self, source: PriceSourceId) -> bool {
        self.tests
            .iter()
            .any(|(id, asked)| asked.source == Some(source) && !self.dropped.contains(id))
    }

    /// Test `id`'s result, once it has landed.
    pub(crate) fn test_result(&self, id: RequestId) -> Option<&TestResult> {
        self.test_results.get(&id)
    }

    /// Forgets test `id`: its form closed or tested again. A response still to come lands as no
    /// one's and is dropped.
    pub(crate) fn drop_test(&mut self, id: RequestId) {
        self.test_results.remove(&id);
        // Kept among the tests, so its response is still known as a test's, never a round's.
        if self.tests.contains_key(&id) {
            self.dropped.insert(id);
        }
        for (history, (test, _)) in &self.test_histories {
            if *test == id {
                self.dropped.insert(*history);
            }
        }
    }

    /// Starts a round on `source`, `asked` when a person asked for it. Every request sent into it
    /// is held until the last lands; a round sent nothing never lands, so open one only to send.
    pub(crate) fn open_round(&mut self, source: &PriceSource<'_>, asked: bool) -> RoundId {
        self.rounds
            .open(source.id, SourceSettings::of(source), asked)
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

    /// Holds `response` in its round, handing back the whole round once its last response lands.
    /// A round failure stops the round: its calls still queued never go out, and it lands once the
    /// calls already out have. A 429 also holds the source until its `Retry-After`, read at `now`.
    /// A response for no round in flight lands alone, for the writer to drop.
    pub(crate) fn land(&mut self, response: PriceResponse, now: DateTime<Utc>) -> Option<Landed> {
        let Some((round, source)) = self.rounds.round_of(response.request) else {
            return Some(Landed {
                source: None,
                asked: false,
                responses: vec![response],
            });
        };
        let failure = round_failure(&response.outcome);
        if failure == Some(FetchFailure::RateLimited)
            && let TransportOutcome::Answered { retry_after, .. } = &response.outcome
        {
            let until = now + retry_after_wait(retry_after.as_deref(), now);
            tracing::warn!(price_source_id = %source, %until, "A price source asked to wait");
            self.calls.hold(Some(source), until);
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

    /// What request `id` was for, forgetting it: `None` for one this Ledger never sent.
    pub(crate) fn take(&mut self, id: RequestId) -> Option<InFlight> {
        self.rounds.take(id)
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
        let key = call.key;
        self.fine_failed.remove(&key);
        self.fine_cache
            .retain(|cached, entry| cached.unit != key.unit || entry.link == call.link);
        if self.fine_asked(key) {
            return;
        }
        if self.source_in_flight(call.source()) {
            tracing::debug!(price_source_id = %call.source(), "A finer prices request waits for its source's round");
            self.fine_parked.push(call);
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
        self.fine_out.insert(id, call);
        self.release(now);
    }

    /// Sends the finer prices requests that waited for `source`'s round, now it has landed.
    fn release_parked(&mut self, source: PriceSourceId, now: DateTime<Utc>) {
        let (going, staying) = std::mem::take(&mut self.fine_parked)
            .into_iter()
            .partition(|call| call.source() == source);
        self.fine_parked = staying;
        for call in going {
            self.send_fine(call, now);
        }
    }

    /// Notes that a series couldn't be asked for `reason` without sending, as a key this Client
    /// doesn't have.
    pub(crate) fn fine_refused(&mut self, key: FineKey, link: PriceLink, reason: FetchFailure) {
        self.fine_failed.insert(key, (link, reason));
    }

    /// Whether the series `key` is queued, out or waiting for a round.
    pub(crate) fn fine_asked(&self, key: FineKey) -> bool {
        self.fine_out.values().any(|call| call.key == key)
            || self.fine_parked.iter().any(|call| call.key == key)
    }

    /// Whether `source` has a finer prices request waiting or out, which a scheduled check counts
    /// as in flight.
    pub(crate) fn fine_in_flight(&self, source: PriceSourceId) -> bool {
        self.fine_out.values().any(|call| call.source() == source)
            || self.fine_parked.iter().any(|call| call.source() == source)
    }

    /// Whether `response` answers a finer prices request.
    pub(crate) fn is_fine(&self, response: &PriceResponse) -> bool {
        self.fine_out.contains_key(&response.request)
    }

    /// Reads a finer prices `response` at `now` into the cache, its days dated by the provider and
    /// its hours in `zone`, or notes its failure for the view alone. A 429 holds its source until
    /// its `Retry-After`; Offline and Rate limited back the source off, as a history request's do.
    pub(crate) fn land_fine<Tz: chrono::TimeZone>(
        &mut self,
        response: &PriceResponse,
        now: DateTime<Utc>,
        today: NaiveDate,
        zone: &Tz,
    ) {
        let Some(call) = self.fine_out.remove(&response.request) else {
            return;
        };
        let hold = match &response.outcome {
            TransportOutcome::Answered {
                status: 429,
                retry_after,
                ..
            } => {
                let until = now + retry_after_wait(retry_after.as_deref(), now);
                self.calls.hold(Some(call.source()), until);
                Some(until)
            }
            _ => None,
        };
        match read_history(
            &response.outcome,
            &call.history,
            &call.link.symbol,
            &call.quote,
        ) {
            HistoryReading::Points(points) => {
                let prices = fine_prices(points, call.key.grain, (call.key.first, call.last), zone);
                self.fine_cache.insert(
                    call.key,
                    FineEntry {
                        link: call.link,
                        fetched_at: now,
                        current: call.last >= today,
                        prices,
                    },
                );
            }
            HistoryReading::Failed(reason) => {
                tracing::warn!(unit_id = %call.key.unit, price_source_id = %call.source(), ?reason, "A finer prices request failed");
                if matches!(reason, FetchFailure::Offline | FetchFailure::RateLimited) {
                    let backed = now + back_off(call.settings.updates);
                    let until = hold.map_or(backed, |until| until.max(backed));
                    self.last
                        .back_off_quietly(call.source(), call.settings.clone(), until);
                }
                self.fine_failed.insert(call.key, (call.link, reason));
            }
        }
    }

    /// Whether the series `key` is cached on `link` and still fresh at `now`: a past window for the
    /// session, one holding today for [`FINE_FRESH`].
    pub(crate) fn fine_fresh(&self, key: FineKey, link: &PriceLink, now: DateTime<Utc>) -> bool {
        self.fine_cache.get(&key).is_some_and(|entry| {
            entry.link == *link && (!entry.current || now - entry.fetched_at < FINE_FRESH)
        })
    }

    /// What the view has of the series `key` on `link` at `now`.
    pub(crate) fn fine_state(
        &self,
        key: FineKey,
        link: &PriceLink,
        now: DateTime<Utc>,
    ) -> FineState<'_> {
        if self.fine_asked(key) {
            let waiting = self
                .fine_out
                .iter()
                .find(|(_, call)| call.key == key)
                .and_then(|(id, call)| self.calls.queued(*id).then_some(call))
                .and_then(|call| self.calls.held_until(Some(call.source())))
                .map_or(0, |until| {
                    let left = (until - now).num_milliseconds();
                    if left > 0 { (left + 999) / 1000 } else { 0 }
                });
            return FineState::Fetching { waiting };
        }
        if let Some(entry) = self
            .fine_cache
            .get(&key)
            .filter(|entry| entry.link == *link)
        {
            return FineState::Shown(&entry.prices);
        }
        match self.fine_failed.get(&key) {
            Some((failed_on, reason)) if failed_on == link => FineState::Failed(*reason),
            _ => FineState::Unasked,
        }
    }

    /// Whether `source` failed a round, or a history request found it offline or rate limited,
    /// and is still backing off at `now`.
    pub(crate) fn source_backing_off(&self, source: &PriceSource<'_>, now: DateTime<Utc>) -> bool {
        self.last.source_backing_off(source, now)
    }

    /// Queues `call` behind its source's history requests, oldest gap first, and sends the next
    /// if the source has none in flight. Returns false, queuing nothing, for a Unit whose history
    /// is already queued or out.
    pub(crate) fn queue_history(&mut self, call: HistoryCall, now: DateTime<Utc>) -> bool {
        if self.history_wanted(call.unit) {
            return false;
        }
        self.history_failures.remove(&call.unit);
        let source = call.source;
        let queue = self.history_queue.entry(source).or_default();
        queue.push(call);
        queue.sort_by_key(|call| call.weeks.first().copied());
        self.start_history(source, now);
        true
    }

    /// Sends `source`'s next waiting history request, unless one is already in flight: one at a
    /// time per source, through the call queue like any call.
    pub(crate) fn start_history(&mut self, source: PriceSourceId, now: DateTime<Utc>) {
        if self.history_out.values().any(|call| call.source == source) {
            return;
        }
        let Some(queue) = self.history_queue.get_mut(&source) else {
            return;
        };
        if queue.is_empty() {
            self.history_queue.remove(&source);
            return;
        }
        let mut call = queue.remove(0);
        let Some(request) = call.request.take() else {
            return;
        };
        tracing::debug!(
            price_source_id = %source,
            unit_id = %call.unit,
            weeks = call.weeks.len(),
            "Asking a price source for a Unit's history"
        );
        let id = self.calls.enqueue(Some(source), call.asked, request);
        self.history_out.insert(id, call);
        self.release(now);
    }

    /// Whether `response` answers a history request.
    pub(crate) fn is_history(&self, response: &PriceResponse) -> bool {
        self.history_out.contains_key(&response.request)
    }

    /// What history request `response` answers, forgetting it. A 429 holds its source until its
    /// `Retry-After`, read at `now`, as a round's does.
    pub(crate) fn land_history(
        &mut self,
        response: &PriceResponse,
        now: DateTime<Utc>,
    ) -> Option<HistoryCall> {
        let call = self.history_out.remove(&response.request)?;
        if let TransportOutcome::Answered {
            status: 429,
            retry_after,
            ..
        } = &response.outcome
        {
            let until = now + retry_after_wait(retry_after.as_deref(), now);
            self.calls.hold(Some(call.source), until);
        }
        Some(call)
    }

    /// Notes that `call` failed for `reason` at `at`: it is tried again after [`HISTORY_RETRY`],
    /// or its source's `Retry-After` if longer. Offline and Rate limited back the source off for
    /// everyone and fail the Units waiting behind it the same way, since they would fail too.
    pub(crate) fn history_failed(
        &mut self,
        call: &HistoryCall,
        reason: FetchFailure,
        at: DateTime<Utc>,
    ) {
        tracing::warn!(unit_id = %call.unit, price_source_id = %call.source, ?reason, "A Unit's history request failed");
        let hold = self.calls.held_until(Some(call.source));
        let retry_at = hold.map_or(at + HISTORY_RETRY, |until| until.max(at + HISTORY_RETRY));
        let mut failed = vec![(call.unit, call.link.clone(), call.settings.clone())];
        if reason.is_round() {
            if matches!(reason, FetchFailure::Offline | FetchFailure::RateLimited) {
                let until = hold.map_or(at + back_off(call.settings.updates), |until| {
                    until.max(at + back_off(call.settings.updates))
                });
                self.last
                    .back_off_quietly(call.source, call.settings.clone(), until);
            }
            for waiting in self.history_queue.remove(&call.source).unwrap_or_default() {
                failed.push((waiting.unit, waiting.link, waiting.settings));
            }
        }
        for (unit, link, settings) in failed {
            self.history_failures.insert(
                unit,
                HistoryFailure {
                    reason,
                    retry_at,
                    link,
                    settings,
                },
            );
        }
    }

    /// Notes that `call`'s provider answered with nothing for `weeks`, so they aren't asked again
    /// on this link until the Ledger reopens.
    pub(crate) fn history_empty(
        &mut self,
        call: &HistoryCall,
        weeks: impl IntoIterator<Item = NaiveDate>,
    ) {
        let entry = self
            .empty_weeks
            .entry(call.unit)
            .or_insert_with(|| (call.link.clone(), HashSet::new()));
        if entry.0 != call.link {
            *entry = (call.link.clone(), HashSet::new());
        }
        entry.1.extend(weeks);
    }

    /// The weeks `unit`'s provider answered with nothing on the link it has now.
    pub(crate) fn empty_weeks(&self, unit: &UnitRow) -> HashSet<NaiveDate> {
        self.empty_weeks
            .get(&unit.id)
            .filter(|(link, _)| *link == unit.link)
            .map(|(_, weeks)| weeks.clone())
            .unwrap_or_default()
    }

    /// Whether `unit`'s history request is queued or out.
    pub fn history_fetching(&self, unit: UnitId) -> bool {
        self.history_out.values().any(|call| call.unit == unit)
    }

    /// Whether `unit` has a history request waiting, queued or out.
    pub(crate) fn history_wanted(&self, unit: UnitId) -> bool {
        self.history_fetching(unit)
            || self
                .history_queue
                .values()
                .any(|queue| queue.iter().any(|call| call.unit == unit))
    }

    /// Why `unit`'s last history request on `source` failed and when it is tried again, while it
    /// still holds: the Unit is active and neither its link nor its source's settings changed.
    pub fn history_failure(
        &self,
        unit: &UnitRow,
        source: &PriceSource<'_>,
    ) -> Option<(FetchFailure, DateTime<Utc>)> {
        let failure = self.history_failures.get(&unit.id)?;
        (unit.is_active
            && source.id == unit.link.source_id
            && failure.link == unit.link
            && failure.settings == SourceSettings::of(source))
        .then_some((failure.reason, failure.retry_at))
    }

    /// The Units whose failed history request is due to be tried again at `now`.
    pub(crate) fn history_retries(&self, now: DateTime<Utc>) -> Vec<UnitId> {
        let mut due: Vec<UnitId> = self
            .history_failures
            .iter()
            .filter(|(_, failure)| failure.retry_at <= now)
            .map(|(unit, _)| *unit)
            .collect();
        due.sort();
        due
    }

    /// Forgets `unit`'s failed history request: it was tried again, or no longer needs trying.
    pub(crate) fn clear_history_failure(&mut self, unit: UnitId) {
        self.history_failures.remove(&unit);
    }

    /// Whether `unit` failed and is still backing off at `now`.
    pub(crate) fn unit_backing_off(
        &self,
        unit: &UnitRow,
        source: &PriceSource<'_>,
        now: DateTime<Utc>,
    ) -> bool {
        self.last.unit_backing_off(unit, source, now)
    }

    /// Whether this Client fetched `unit` on another link than it has now: editing its source,
    /// symbol or quote makes it due straight away.
    pub(crate) fn relinked(&self, unit: &UnitRow) -> bool {
        self.last.relinked(unit)
    }

    /// Notes how `unit`'s fetch on `link` went at `at`, backing it off on a failure.
    pub(crate) fn record_unit(
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
    pub(crate) fn record_source(
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
