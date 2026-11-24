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
use std::collections::{HashMap, HashSet, VecDeque};
use std::rc::Rc;

use chrono::{DateTime, Duration, NaiveDate, Utc};
use lib_core::{Money, PriceSourceId, UnitId};
use secrecy::{ExposeSecret as _, SecretString};

mod fine;
mod history;
mod reading;
mod request;
mod transport;

pub use fine::{FINE_FRESH, Grain};
pub(crate) use fine::{FineKey, FinePrice, fine_prices, window};
pub(crate) use history::{
    HistoryReading, floor, fold, gaps, history_request, read_history, wanted_weeks,
};
pub use reading::RETRY_AFTER_DEFAULT;
use reading::round_failure;
pub(crate) use reading::{Reading, read, retry_after_wait};
pub(crate) use request::{Wanted, fill, is_due, key_missing, price_request, request, round_calls};
pub use transport::{
    Method, PriceRequest, PriceResponse, PriceTransport, RequestId, RequestKey, TransportError,
    TransportOutcome,
};

use crate::{Endpoint, HistoryEndpoint, PriceLink, PriceSource, UnitRow, Updates};

/// The shortest a failing source or Unit is left alone, whatever its source's Updates.
pub const MIN_BACK_OFF: Duration = Duration::minutes(15);

/// How many calls this Client sends one source in any [`CallRate::WINDOW`], declared in code
/// from what each provider allows (ADR-0043). A burst goes out while the window has room; only
/// the calls past it wait. Counted per Client, with no coordination across devices.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CallRate {
    pub calls: usize,
}

impl CallRate {
    /// The sliding window the calls are counted over.
    pub const WINDOW: Duration = Duration::seconds(60);

    /// A rate of `calls` a minute, as a built-in source declares it.
    pub const fn per_minute(calls: usize) -> Self {
        Self { calls }
    }

    /// The rate of `source`, `keyed` when the call carries an API key: its built-in's
    /// `keyed_call_rate` where it has one, else its `call_rate`. A Custom source, or one not yet
    /// saved (`None`), is 30.
    pub fn of(source: Option<PriceSourceId>, keyed: bool) -> Self {
        let Some(built_in) = source.and_then(crate::built_in) else {
            return Self::per_minute(30);
        };
        match built_in.keyed_call_rate {
            Some(rate) if keyed => rate,
            _ => built_in.call_rate,
        }
    }
}

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

/// How a fetch this Client tried went.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FetchOutcome {
    Fetched,
    /// Failed for `reason`, and left alone until `retry_at`.
    Failed {
        reason: FetchFailure,
        retry_at: DateTime<Utc>,
    },
}

/// What this Client last saw of a source's round or a Unit's fetch: held in memory for the open
/// Ledger only, never written to it or synced.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LastFetch {
    pub at: DateTime<Utc>,
    pub outcome: FetchOutcome,
}

impl LastFetch {
    /// The reason it failed, `None` once fetched.
    pub fn failure(&self) -> Option<FetchFailure> {
        match self.outcome {
            FetchOutcome::Fetched => None,
            FetchOutcome::Failed { reason, .. } => Some(reason),
        }
    }

    /// Whether it failed and is still backing off at `now`.
    fn backing_off(&self, now: DateTime<Utc>) -> bool {
        matches!(self.outcome, FetchOutcome::Failed { retry_at, .. } if now < retry_at)
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

/// A Unit's last fetch, with the link and source settings it was for.
#[derive(Debug, Clone)]
struct UnitRecord {
    last: LastFetch,
    link: PriceLink,
    settings: SourceSettings,
}

/// A source's last round, with the settings it was for.
#[derive(Debug, Clone)]
struct SourceRecord {
    last: LastFetch,
    settings: SourceSettings,
}

/// How long a failure on a source that updates every `updates` is left alone: its interval, never
/// under [`MIN_BACK_OFF`].
pub(crate) fn back_off(updates: Updates) -> Duration {
    updates
        .interval()
        .map_or(MIN_BACK_OFF, |interval| interval.max(MIN_BACK_OFF))
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

/// What a request was for: every Unit it asks the price of, one, or several in a batch, and the
/// source settings it went out on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct InFlight {
    pub units: Vec<UnitCall>,
    pub settings: SourceSettings,
}

/// One Unit a request asks for: the Unit and its link as they were when it was asked, so a
/// response for a Unit relinked since writes nothing, and the price path filled for that link.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct UnitCall {
    pub unit: UnitId,
    pub link: PriceLink,
    pub price_path: String,
}

/// Which window and `Retry-After` a call counts against: its source, `None` for a test of a
/// source not saved yet.
type Lane = Option<PriceSourceId>;

/// A call waiting its turn: it goes out once its lane's window has room and no `Retry-After`
/// holds it, a call a person asked for ahead of a scheduled one.
struct Queued {
    id: RequestId,
    lane: Lane,
    rate: CallRate,
    asked: bool,
    request: PriceRequest,
}

/// Names one round: the requests sent to one source together, written in one commit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct RoundId(u64);

/// One source's round in flight: the requests still out and the responses held until the last
/// lands, so the round writes once.
struct Round {
    source: PriceSourceId,
    settings: SourceSettings,
    /// Whether a person asked for it (fetch now), so its outcome is told; a scheduled round is
    /// silent.
    asked: bool,
    /// Every Unit sent into the round, until it lands.
    units: HashSet<UnitId>,
    /// Its calls queued or out: a round is in flight from its first call joining the queue.
    pending: HashSet<RequestId>,
    landed: Vec<PriceResponse>,
}

/// A round whose every response has come back, ready to be written in one commit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Landed {
    /// The source and its settings when the round went out; `None` for a response for no round.
    pub source: Option<(PriceSourceId, SourceSettings)>,
    pub asked: bool,
    pub responses: Vec<PriceResponse>,
}

/// The fetching state one Client keeps per open Ledger, beside its ports, and drops when the
/// Ledger closes: never persisted (ADR-0062). The requests and rounds in flight, and each source's
/// and Unit's last fetch with its back-off, so closing the Ledger clears every failure.
pub struct PriceFetching {
    transport: Rc<dyn PriceTransport>,
    next: u64,
    /// Every round's call queued or out, by request.
    in_flight: HashMap<RequestId, InFlight>,
    /// The calls, round and test alike, waiting their turn, oldest first.
    queue: Vec<Queued>,
    /// When each lane's calls in the last [`CallRate::WINDOW`] went out, oldest first.
    windows: HashMap<Lane, VecDeque<DateTime<Utc>>>,
    /// Until when each lane waits out a `Retry-After`.
    holds: HashMap<Lane, DateTime<Utc>>,
    next_round: u64,
    rounds: HashMap<RoundId, Round>,
    units: HashMap<UnitId, UnitRecord>,
    sources: HashMap<PriceSourceId, SourceRecord>,
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
    /// Sources a history request found offline or rate limited, backed off for everyone until
    /// then without touching their Last fetch.
    quiet_back_off: HashMap<PriceSourceId, (SourceSettings, DateTime<Utc>)>,
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
            transport,
            next: 0,
            in_flight: HashMap::new(),
            queue: Vec::new(),
            windows: HashMap::new(),
            holds: HashMap::new(),
            next_round: 0,
            rounds: HashMap::new(),
            units: HashMap::new(),
            sources: HashMap::new(),
            tests: HashMap::new(),
            test_results: HashMap::new(),
            dropped: HashSet::new(),
            test_histories: HashMap::new(),
            history_out: HashMap::new(),
            history_queue: HashMap::new(),
            history_failures: HashMap::new(),
            empty_weeks: HashMap::new(),
            quiet_back_off: HashMap::new(),
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
        self.in_flight.len()
    }

    /// Whether request `id` is still queued or out.
    pub(crate) fn in_flight_request(&self, id: RequestId) -> bool {
        self.in_flight.contains_key(&id)
    }

    /// Whether a request for `unit` is queued or out.
    pub fn fetching(&self, unit: UnitId) -> bool {
        self.in_flight
            .values()
            .any(|sent| sent.units.iter().any(|call| call.unit == unit))
    }

    /// Whether `source` has a round queued or out, which a scheduled check leaves alone.
    pub(crate) fn source_in_flight(&self, source: PriceSourceId) -> bool {
        self.rounds.values().any(|round| round.source == source)
    }

    /// Whether `unit` is in a round a person asked for that is still out, which its row shows as
    /// *Fetching…*.
    pub fn unit_asked_for(&self, unit: UnitId) -> bool {
        self.rounds
            .values()
            .any(|round| round.asked && round.units.contains(&unit))
    }

    /// Whether `source` has a round a person asked for still out, which its row shows as
    /// *Fetching…*.
    pub fn source_asked_for(&self, source: PriceSourceId) -> bool {
        self.rounds
            .values()
            .any(|round| round.asked && round.source == source)
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
        let id = self.next_request();
        let lane = asked.source;
        self.tests.insert(id, asked);
        self.enqueue(id, lane, true, request);
        self.release(now);
        id
    }

    fn next_request(&mut self) -> RequestId {
        let id = RequestId(self.next);
        self.next += 1;
        id
    }

    fn enqueue(&mut self, id: RequestId, lane: Lane, asked: bool, request: PriceRequest) {
        let rate = CallRate::of(lane, request.key.is_some());
        self.queue.push(Queued {
            id,
            lane,
            rate,
            asked,
            request,
        });
    }

    /// Sends every queued call its lane allows at `now`: the asked-for first, then the scheduled,
    /// oldest first within each, while the lane's window has room and no `Retry-After` holds it.
    /// A lane that can't send one call sends none after it, so no call overtakes another on its
    /// lane but an asked-for one. Returns how many went out.
    pub(crate) fn release(&mut self, now: DateTime<Utc>) -> usize {
        self.holds.retain(|_, until| now < *until);
        let mut order: Vec<usize> = (0..self.queue.len()).collect();
        order.sort_by_key(|&index| !self.queue[index].asked);
        let mut blocked: HashSet<Lane> = HashSet::new();
        let mut going: Vec<usize> = Vec::new();
        for index in order {
            let call = &self.queue[index];
            if blocked.contains(&call.lane) {
                continue;
            }
            let window = self.windows.entry(call.lane).or_default();
            while window
                .front()
                .is_some_and(|sent| now - *sent >= CallRate::WINDOW)
            {
                window.pop_front();
            }
            if self.holds.contains_key(&call.lane) || window.len() >= call.rate.calls {
                blocked.insert(call.lane);
                continue;
            }
            window.push_back(now);
            going.push(index);
        }
        let mut queue: Vec<Option<Queued>> = std::mem::take(&mut self.queue)
            .into_iter()
            .map(Some)
            .collect();
        let sent = going.len();
        for index in going {
            let Some(call) = queue[index].take() else {
                continue;
            };
            if let Some(test) = self.tests.get_mut(&call.id) {
                // A test times its answer from when it went out, not from when it queued.
                test.sent_at = now;
            }
            self.transport.send(call.id, call.request);
        }
        self.queue = queue.into_iter().flatten().collect();
        if !self.queue.is_empty() {
            tracing::debug!(queued = self.queue.len(), "Price requests wait their turn");
        }
        sent
    }

    /// The source a call a person asked for waits on, and the whole seconds left of its
    /// `Retry-After` at `now`, rounded up. `None` while no such call is held, a queued one only
    /// waiting its call rate included.
    pub(crate) fn waiting(&self, now: DateTime<Utc>) -> Option<(PriceSourceId, i64)> {
        self.queue
            .iter()
            .filter(|call| call.asked && !self.tests.contains_key(&call.id))
            .find_map(|call| {
                let source = call.lane?;
                let until = self.holds.get(&call.lane)?;
                let left = (*until - now).num_milliseconds();
                (left > 0).then_some((source, (left + 999) / 1000))
            })
    }

    /// [`Self::waiting`] the first time it is asked of a hold, `None` after: a wait is announced
    /// once as it begins, never counted down.
    pub(crate) fn wait_begun(&mut self, now: DateTime<Utc>) -> Option<(PriceSourceId, i64)> {
        let (source, seconds) = self.waiting(now)?;
        let until = *self.holds.get(&Some(source))?;
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
        let id = self.next_request();
        self.test_histories.insert(id, (test, history));
        self.enqueue(id, asked.source, true, request);
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
        let id = RoundId(self.next_round);
        self.next_round += 1;
        self.rounds.insert(
            id,
            Round {
                source: source.id,
                settings: SourceSettings::of(source),
                asked,
                units: HashSet::new(),
                pending: HashSet::new(),
                landed: Vec::new(),
            },
        );
        id
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
        let id = self.next_request();
        let mut lane = None;
        let mut asked = false;
        if let Some(round) = self.rounds.get_mut(&round) {
            round.units.extend(sent.units.iter().map(|call| call.unit));
            round.pending.insert(id);
            lane = Some(round.source);
            asked = round.asked;
        }
        self.in_flight.insert(id, sent);
        self.enqueue(id, lane, asked, request);
        self.release(now);
        id
    }

    /// Holds `response` in its round, handing back the whole round once its last response lands.
    /// A round failure stops the round: its calls still queued never go out, and it lands once the
    /// calls already out have. A 429 also holds the source until its `Retry-After`, read at `now`.
    /// A response for no round in flight lands alone, for the writer to drop.
    pub(crate) fn land(&mut self, response: PriceResponse, now: DateTime<Utc>) -> Option<Landed> {
        let Self {
            rounds,
            queue,
            in_flight,
            holds,
            ..
        } = self;
        let Some((&id, round)) = rounds
            .iter_mut()
            .find(|(_, round)| round.pending.contains(&response.request))
        else {
            return Some(Landed {
                source: None,
                asked: false,
                responses: vec![response],
            });
        };
        round.pending.remove(&response.request);
        if let Some(failure) = round_failure(&response.outcome) {
            if let TransportOutcome::Answered { retry_after, .. } = &response.outcome
                && failure == FetchFailure::RateLimited
            {
                let until = now + retry_after_wait(retry_after.as_deref(), now);
                tracing::warn!(price_source_id = %round.source, %until, "A price source asked to wait");
                holds.insert(Some(round.source), until);
            }
            // The round's calls still queued never go; what is left pending is out already.
            let dropped: HashSet<RequestId> = queue
                .iter()
                .filter(|call| round.pending.contains(&call.id))
                .map(|call| call.id)
                .collect();
            if !dropped.is_empty() {
                tracing::warn!(
                    price_source_id = %round.source,
                    dropped = dropped.len(),
                    ?failure,
                    "Stopped a price round before its queued calls went out"
                );
                queue.retain(|call| !dropped.contains(&call.id));
                for id in &dropped {
                    for call in in_flight
                        .remove(id)
                        .map(|sent| sent.units)
                        .unwrap_or_default()
                    {
                        round.units.remove(&call.unit);
                    }
                }
                round.pending.retain(|id| !dropped.contains(id));
            }
        }
        round.landed.push(response);
        if !round.pending.is_empty() {
            return None;
        }
        let round = self.rounds.remove(&id)?;
        self.release_parked(round.source, now);
        Some(Landed {
            source: Some((round.source, round.settings)),
            asked: round.asked,
            responses: round.landed,
        })
    }

    /// What request `id` was for, forgetting it: `None` for one this Ledger never sent.
    pub(crate) fn take(&mut self, id: RequestId) -> Option<InFlight> {
        self.in_flight.remove(&id)
    }

    /// `unit`'s last fetch on `source` while it still holds: the Unit is active and neither its
    /// link nor its source's settings changed since. Editing either, or deactivating the Unit,
    /// drops it.
    pub fn unit_fetch(&self, unit: &UnitRow, source: &PriceSource<'_>) -> Option<&LastFetch> {
        let record = self.units.get(&unit.id)?;
        (unit.is_active
            && source.id == unit.link.source_id
            && record.link == unit.link
            && record.settings == SourceSettings::of(source))
        .then_some(&record.last)
    }

    /// `source`'s last round while it still holds: its settings haven't changed since.
    pub fn source_fetch(&self, source: &PriceSource<'_>) -> Option<&LastFetch> {
        let record = self.sources.get(&source.id)?;
        (record.settings == SourceSettings::of(source)).then_some(&record.last)
    }

    /// A key was stored or replaced for `source`: its Key missing and Key rejected failures, and
    /// its Units', clear, so the next check tries it again.
    pub fn key_stored(&mut self, source: PriceSourceId) {
        let keyed = |last: &LastFetch| {
            matches!(
                last.failure(),
                Some(FetchFailure::KeyMissing | FetchFailure::KeyRejected)
            )
        };
        if self
            .sources
            .get(&source)
            .is_some_and(|record| keyed(&record.last))
        {
            self.sources.remove(&source);
        }
        self.units
            .retain(|_, record| record.link.source_id != source || !keyed(&record.last));
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
        let id = self.next_request();
        let source = call.source();
        tracing::debug!(price_source_id = %source, unit_id = %call.key.unit, grain = ?call.key.grain, "Asking a price source for finer prices");
        self.fine_out.insert(id, call);
        self.enqueue(id, Some(source), true, request);
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
                self.holds.insert(Some(call.source()), until);
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
                    self.quiet_back_off
                        .insert(call.source(), (call.settings.clone(), until));
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
                .and_then(|(id, call)| {
                    self.queue
                        .iter()
                        .any(|queued| queued.id == *id)
                        .then_some(call)
                })
                .and_then(|call| self.holds.get(&Some(call.source())))
                .map_or(0, |until| {
                    let left = (*until - now).num_milliseconds();
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
        let quiet = self
            .quiet_back_off
            .get(&source.id)
            .is_some_and(|(settings, until)| {
                *settings == SourceSettings::of(source) && now < *until
            });
        quiet
            || self
                .source_fetch(source)
                .is_some_and(|last| last.backing_off(now))
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
        let id = self.next_request();
        tracing::debug!(
            price_source_id = %source,
            unit_id = %call.unit,
            weeks = call.weeks.len(),
            "Asking a price source for a Unit's history"
        );
        let asked = call.asked;
        self.history_out.insert(id, call);
        self.enqueue(id, Some(source), asked, request);
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
            self.holds.insert(Some(call.source), until);
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
        let hold = self.holds.get(&Some(call.source)).copied();
        let retry_at = hold.map_or(at + HISTORY_RETRY, |until| until.max(at + HISTORY_RETRY));
        let mut failed = vec![(call.unit, call.link.clone(), call.settings.clone())];
        if reason.is_round() {
            if matches!(reason, FetchFailure::Offline | FetchFailure::RateLimited) {
                let until = hold.map_or(at + back_off(call.settings.updates), |until| {
                    until.max(at + back_off(call.settings.updates))
                });
                self.quiet_back_off
                    .insert(call.source, (call.settings.clone(), until));
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
        self.unit_fetch(unit, source)
            .is_some_and(|last| last.backing_off(now))
    }

    /// Whether this Client fetched `unit` on another link than it has now: editing its source,
    /// symbol or quote makes it due straight away.
    pub(crate) fn relinked(&self, unit: &UnitRow) -> bool {
        self.units
            .get(&unit.id)
            .is_some_and(|record| record.link != unit.link)
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
        let last = Self::last(settings, at, failure);
        self.units.insert(
            unit,
            UnitRecord {
                last,
                link,
                settings: settings.clone(),
            },
        );
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
        let mut last = Self::last(settings, at, failure);
        // A 429 backs off for the longer of the usual rule and its `Retry-After`.
        if let (
            Some(FetchFailure::RateLimited),
            Some(until),
            FetchOutcome::Failed { retry_at, .. },
        ) = (failure, self.holds.get(&Some(source)), &mut last.outcome)
        {
            *retry_at = (*retry_at).max(*until);
        }
        self.sources.insert(
            source,
            SourceRecord {
                last,
                settings: settings.clone(),
            },
        );
    }

    fn last(
        settings: &SourceSettings,
        at: DateTime<Utc>,
        failure: Option<FetchFailure>,
    ) -> LastFetch {
        LastFetch {
            at,
            outcome: match failure {
                None => FetchOutcome::Fetched,
                Some(reason) => FetchOutcome::Failed {
                    reason,
                    retry_at: at + back_off(settings.updates),
                },
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::FRANKFURTER;

    mod pacing {
        use std::rc::Rc;

        use super::*;
        use crate::fake::FakeTransport;
        use crate::{COINGECKO, FRANKFURTER, YAHOO_FINANCE, built_in};

        fn at(seconds: i64) -> DateTime<Utc> {
            NaiveDate::from_ymd_opt(2026, 10, 21)
                .and_then(|day| day.and_hms_opt(9, 0, 0))
                .expect("a real time")
                .and_utc()
                + Duration::seconds(seconds)
        }

        struct Client {
            transport: FakeTransport,
            fetching: PriceFetching,
        }

        fn client() -> Client {
            let transport = FakeTransport::default();
            Client {
                fetching: PriceFetching::new(Rc::new(transport.clone())),
                transport,
            }
        }

        fn source(id: PriceSourceId) -> PriceSource<'static> {
            let built_in = built_in(id).expect("a built-in");
            PriceSource {
                id,
                name: built_in.name,
                origin: crate::SourceOrigin::BuiltIn(built_in),
                updates: built_in.updates,
            }
        }

        impl Client {
            /// Starts a round on `source` for `symbol`, `asked` or scheduled, at `now`.
            fn round(&mut self, id: PriceSourceId, symbol: &str, asked: bool, now: DateTime<Utc>) {
                let source = source(id);
                let endpoint = source.endpoint().expect("fetching");
                let (request, price_path) = price_request(endpoint, symbol, "AUD", None);
                let round = self.fetching.open_round(&source, asked);
                self.fetching.send(
                    round,
                    request,
                    InFlight {
                        units: vec![UnitCall {
                            unit: UnitId::new(),
                            link: PriceLink {
                                source_id: id,
                                symbol: symbol.into(),
                                quote: UnitId::SEED_AUD,
                            },
                            price_path,
                        }],
                        settings: SourceSettings::of(&source),
                    },
                    now,
                );
            }

            fn sent(&self) -> usize {
                self.transport.requests().len()
            }
        }

        #[test]
        fn each_source_has_its_call_rate_and_coingeckos_rises_with_a_demo_key() {
            let custom = Some(PriceSourceId::new());
            assert_eq!(CallRate::of(Some(COINGECKO), false).calls, 5);
            assert_eq!(CallRate::of(Some(COINGECKO), true).calls, 30);
            assert_eq!(CallRate::of(Some(YAHOO_FINANCE), false).calls, 30);
            assert_eq!(CallRate::of(Some(FRANKFURTER), false).calls, 60);
            assert_eq!(CallRate::of(custom, true).calls, 30);
            assert_eq!(CallRate::of(None, false).calls, 30);
        }

        #[test]
        fn a_burst_within_the_rate_goes_at_once_and_the_rest_wait_for_the_window_to_slide() {
            let mut client = client();
            for index in 0..62 {
                client.round(FRANKFURTER, &format!("C{index:02}"), false, at(0));
            }
            assert_eq!(client.sent(), 60, "Frankfurter takes 60 a minute");

            client.fetching.release(at(59));
            assert_eq!(client.sent(), 60, "never more than 60 in any 60 seconds");

            client.fetching.release(at(60));
            assert_eq!(client.sent(), 62);
        }

        #[test]
        fn the_window_slides_so_only_calls_older_than_a_minute_make_room() {
            let mut client = client();
            client.round(COINGECKO, "a", false, at(0));
            for symbol in ["b", "c", "d", "e"] {
                client.round(COINGECKO, symbol, false, at(30));
            }
            client.round(COINGECKO, "f", false, at(31));
            client.round(COINGECKO, "g", false, at(31));
            assert_eq!(client.sent(), 5);

            client.fetching.release(at(60));
            assert_eq!(client.sent(), 6, "a's slot is free, b to e's aren't yet");
            client.fetching.release(at(90));
            assert_eq!(client.sent(), 7);
        }

        #[test]
        fn an_asked_for_call_goes_ahead_of_a_scheduled_one_queued_before_it() {
            let mut client = client();
            for symbol in ["a", "b", "c", "d", "e"] {
                client.round(COINGECKO, symbol, true, at(0));
            }
            client.round(COINGECKO, "scheduled", false, at(1));
            client.round(COINGECKO, "asked", true, at(2));
            assert_eq!(client.sent(), 5, "the window is full");

            client.fetching.release(at(60));

            let urls = client.transport.urls();
            assert_eq!(urls.len(), 7);
            assert!(urls[5].contains("ids=asked&"), "{urls:?}");
            assert!(urls[6].contains("ids=scheduled&"), "{urls:?}");
        }

        #[test]
        fn each_source_counts_its_own_calls() {
            let mut client = client();
            for symbol in ["a", "b", "c", "d", "e", "f"] {
                client.round(COINGECKO, symbol, true, at(0));
            }
            client.round(FRANKFURTER, "EUR", false, at(0));

            assert_eq!(client.sent(), 6, "five CoinGecko and Frankfurter's one");
        }
    }

    #[test]
    fn a_failure_backs_off_for_its_sources_interval_and_never_under_15_minutes() {
        assert_eq!(back_off(Updates::Every15Minutes), Duration::minutes(15));
        assert_eq!(back_off(Updates::Hourly), Duration::hours(1));
        assert_eq!(back_off(Updates::Daily), Duration::hours(24));
        assert_eq!(back_off(Updates::ManualOnly), MIN_BACK_OFF);
    }

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
