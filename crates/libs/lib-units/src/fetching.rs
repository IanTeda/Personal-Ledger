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

use crate::{
    Endpoint, HistoryEndpoint, KeyPlacement, PriceLink, PriceSource, PriceSourceAuth, UnitRow,
    Updates, WeeklyPrice,
};

/// The shortest a failing source or Unit is left alone, whatever its source's Updates.
pub const MIN_BACK_OFF: Duration = Duration::minutes(15);

/// How long a 429 waits when its `Retry-After` is missing, unreadable or already past.
pub const RETRY_AFTER_DEFAULT: Duration = Duration::seconds(60);

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

    /// The rate of `source`, `keyed` when the call carries an API key: CoinGecko takes 5 a minute
    /// keyless and 30 with a Demo key, Yahoo Finance 30, Frankfurter 60 and a Custom source, or
    /// one not yet saved (`None`), 30.
    pub fn of(source: Option<PriceSourceId>, keyed: bool) -> Self {
        let calls = match source {
            Some(id) if id == crate::COINGECKO => {
                if keyed {
                    30
                } else {
                    5
                }
            }
            Some(id) if id == crate::FRANKFURTER => 60,
            Some(id) if id == crate::YAHOO_FINANCE => 30,
            Some(_) | None => 30,
        };
        Self { calls }
    }
}

/// How long a 429's raw `Retry-After` asks to wait from `now`: whole seconds, or an HTTP date.
/// Missing, unreadable or already past, it is [`RETRY_AFTER_DEFAULT`].
pub(crate) fn retry_after_wait(raw: Option<&str>, now: DateTime<Utc>) -> Duration {
    let Some(raw) = raw.map(str::trim) else {
        return RETRY_AFTER_DEFAULT;
    };
    if let Ok(seconds) = raw.parse::<u32>() {
        return Duration::seconds(i64::from(seconds));
    }
    match DateTime::parse_from_rfc2822(raw) {
        Ok(at) if at.with_timezone(&Utc) > now => at.with_timezone(&Utc) - now,
        Ok(_) | Err(_) => RETRY_AFTER_DEFAULT,
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

/// What a response held: a price, or why not.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Reading {
    Priced(Money),
    Failed(FetchFailure),
}

/// The round failure `outcome` is, if it is one: what stops a round whatever Unit it was for.
fn round_failure(outcome: &TransportOutcome) -> Option<FetchFailure> {
    match read(outcome, "") {
        Reading::Failed(failure) if failure.is_round() => Some(failure),
        Reading::Priced(_) | Reading::Failed(_) => None,
    }
}

/// Reads `outcome` onto a price at `price_path`, or onto one of the [`FetchFailure`]s.
pub(crate) fn read(outcome: &TransportOutcome, price_path: &str) -> Reading {
    let failed = Reading::Failed;
    match outcome {
        TransportOutcome::Failed(TransportError::Offline | TransportError::Timeout) => {
            failed(FetchFailure::Offline)
        }
        TransportOutcome::Failed(TransportError::Tls | TransportError::Other) => {
            failed(FetchFailure::SourceError)
        }
        TransportOutcome::Answered { status, body, .. } => match status {
            200..=299 => {
                if serde_json::from_str::<serde_json::Value>(body).is_err() {
                    failed(FetchFailure::SourceError)
                } else {
                    parse_price(body, price_path)
                        .map_or(failed(FetchFailure::NoPrice), Reading::Priced)
                }
            }
            401 | 403 => failed(FetchFailure::KeyRejected),
            429 => failed(FetchFailure::RateLimited),
            // A provider answers an unknown symbol or quote with a 404, or a 400 or 422 for one it
            // can't read.
            400..=499 => failed(FetchFailure::NotFound),
            _ => failed(FetchFailure::SourceError),
        },
    }
}

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

/// Names one request sent, so its response can be matched to what asked for it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct RequestId(pub u64);

/// The HTTP method a request goes by. Every provider answers a price on a GET.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Method {
    Get,
}

/// One request for a price, built in full here so the transport knows nothing about providers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PriceRequest {
    pub method: Method,
    /// Never holds the key, so the request can be printed and traced.
    pub url: String,
    /// Header names and values, in the order sent.
    pub headers: Vec<(String, String)>,
    /// The source's API key, which the transport puts where `placement` says as the request goes
    /// out: the only place it is exposed (ADR-0043).
    pub key: Option<RequestKey>,
}

/// An API key a request carries: a query parameter or a header named `name`.
#[derive(Clone)]
pub struct RequestKey {
    pub placement: KeyPlacement,
    pub name: String,
    pub value: SecretString,
}

/// The key's name and placement, never the key.
impl std::fmt::Debug for RequestKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RequestKey")
            .field("placement", &self.placement)
            .field("name", &self.name)
            .finish_non_exhaustive()
    }
}

/// Equal when the same key goes in the same place: what a test asks of a request it recorded.
impl PartialEq for RequestKey {
    fn eq(&self, other: &Self) -> bool {
        self.placement == other.placement
            && self.name == other.name
            && self.value.expose_secret() == other.value.expose_secret()
    }
}

impl Eq for RequestKey {}

/// Sends a request without waiting: its response comes back later as a [`PriceResponse`], which
/// the Client hands to [`crate::presenter::FetchedPricesPresenter`] through its input door.
pub trait PriceTransport {
    fn send(&self, id: RequestId, request: PriceRequest);
}

/// What the transport made of request `request`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PriceResponse {
    pub request: RequestId,
    pub outcome: TransportOutcome,
}

/// The provider answered with `status` and `body`, whatever the status, or no answer came.
/// `retry_after` is the raw `Retry-After` header, if it sent one, which only a 429 is read for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransportOutcome {
    Answered {
        status: u16,
        body: String,
        retry_after: Option<String>,
    },
    Failed(TransportError),
}

/// Why no answer came.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransportError {
    /// The provider couldn't be reached: no network, a refused connection or a failed lookup.
    Offline,
    /// The provider took longer than the transport's timeout.
    Timeout,
    /// The secure connection couldn't be made.
    Tls,
    /// Anything else: a request the transport couldn't build or a body it couldn't read.
    Other,
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
    /// Whether the status line names its source until it lands: a source or every source asked
    /// for, not a Unit's own price, which says nothing but its outcome.
    named: bool,
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
    pub named: bool,
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
            history_out: HashMap::new(),
            history_queue: HashMap::new(),
            history_failures: HashMap::new(),
            empty_weeks: HashMap::new(),
            quiet_back_off: HashMap::new(),
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

    /// The sources with a named round still out, in no order: what the status line names until
    /// everything lands. A scheduled round is silent, so it isn't among them.
    pub(crate) fn named_sources(&self) -> HashSet<PriceSourceId> {
        self.rounds
            .values()
            .filter(|round| round.named)
            .map(|round| round.source)
            .collect()
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
    /// `Retry-After` at `now`, rounded up: what the status line counts down. `None` while no such
    /// call is held, a queued one only waiting its call rate included.
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

    /// Whether `response` answers a test this Client sent, dropped or not, so it is never read as
    /// a round's.
    pub(crate) fn is_test(&self, response: &PriceResponse) -> bool {
        self.tests.contains_key(&response.request)
    }

    /// Reads a test's `response` at `now`, holding it for its form unless it answers with a
    /// Toast. `None` for a response that is no test's, or a dropped one's.
    pub(crate) fn land_test(
        &mut self,
        response: &PriceResponse,
        now: DateTime<Utc>,
    ) -> Option<TestResult> {
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
            asked,
        };
        if !result.asked.toast {
            self.test_results.insert(response.request, result.clone());
        }
        Some(result)
    }

    /// Whether test `id` is still out.
    pub(crate) fn testing(&self, id: RequestId) -> bool {
        self.tests.contains_key(&id)
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
                named: false,
                units: HashSet::new(),
                pending: HashSet::new(),
                landed: Vec::new(),
            },
        );
        id
    }

    /// Has the status line name `round`'s source until it lands.
    pub(crate) fn name_round(&mut self, round: RoundId) {
        if let Some(round) = self.rounds.get_mut(&round) {
            round.named = true;
        }
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
                named: false,
                responses: vec![response],
            });
        };
        round.pending.remove(&response.request);
        if let Some(failure) = round_failure(&response.outcome) {
            if let TransportOutcome::Answered { retry_after, .. } = &response.outcome
                && failure == FetchFailure::RateLimited
            {
                let until = now + retry_after_wait(retry_after.as_deref(), now);
                tracing::info!(price_source_id = %round.source, %until, "A price source asked to wait");
                holds.insert(Some(round.source), until);
            }
            // The round's calls still queued never go; what is left pending is out already.
            let dropped: HashSet<RequestId> = queue
                .iter()
                .filter(|call| round.pending.contains(&call.id))
                .map(|call| call.id)
                .collect();
            if !dropped.is_empty() {
                tracing::info!(
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
        Some(Landed {
            source: Some((round.source, round.settings)),
            asked: round.asked,
            named: round.named,
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
        tracing::info!(
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

    /// The history requests a person's fetch now is waiting on, as (Unit code, source), in the
    /// order they were sent.
    pub(crate) fn asked_history(&self) -> Vec<(String, PriceSourceId)> {
        let mut asked: Vec<(&RequestId, &HistoryCall)> = self
            .history_out
            .iter()
            .filter(|(_, call)| call.asked)
            .collect();
        asked.sort_by_key(|(id, _)| **id);
        asked
            .into_iter()
            .map(|(_, call)| (call.code.clone(), call.source))
            .collect()
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
    unit.prices
        .iter()
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
                let (request, _) =
                    price_request(endpoint, &one.link.symbol, &one.quote, key.cloned());
                let sent = InFlight {
                    units: vec![call(one)],
                    settings: settings.clone(),
                };
                (request, sent)
            })
            .collect();
    };
    let mut symbols: Vec<&str> = Vec::new();
    for one in wanted {
        if !symbols.contains(&one.link.symbol.as_str()) {
            symbols.push(&one.link.symbol);
        }
    }
    symbols
        .chunks(batch.max_symbols.max(1))
        .map(|chunk| {
            let units: Vec<&Wanted> = wanted
                .iter()
                .filter(|one| chunk.contains(&one.link.symbol.as_str()))
                .collect();
            let mut quotes: Vec<&str> = Vec::new();
            for one in &units {
                if !quotes.contains(&one.quote.as_str()) {
                    quotes.push(&one.quote);
                }
            }
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
        method: Method::Get,
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

/// The price at JSON pointer `path` in `body`: a number, or a string holding one. `None` for a
/// body that isn't JSON or has no price there.
pub(crate) fn parse_price(body: &str, path: &str) -> Option<Money> {
    let json: serde_json::Value = serde_json::from_str(body).ok()?;
    let text = match json.pointer(path)? {
        serde_json::Value::Number(number) => number.to_string(),
        serde_json::Value::String(text) => text.clone(),
        _ => return None,
    };
    text.trim().parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{COINGECKO, FRANKFURTER, RequestHeader, YAHOO_FINANCE, built_in};

    #[test]
    fn frankfurters_request_is_v2_rate_for_the_symbol_in_the_quote() {
        let endpoint = &built_in(FRANKFURTER).expect("a built-in").endpoint;

        let (request, path) = price_request(endpoint, "EUR", "AUD", None);

        assert_eq!(
            request,
            PriceRequest {
                method: Method::Get,
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
                method: Method::Get,
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

    /// Replies captured from each built-in, read at the price path its request fills.
    mod captured {
        use super::*;

        fn priced(source: PriceSourceId, symbol: &str, quote: &str, body: &str) -> Reading {
            let endpoint = &built_in(source).expect("a built-in").endpoint;
            let (_, path) = price_request(endpoint, symbol, quote, None);
            read(
                &TransportOutcome::Answered {
                    status: 200,
                    body: body.into(),
                    retry_after: None,
                },
                &path,
            )
        }

        fn money(text: &str) -> Reading {
            Reading::Priced(text.parse().expect("a decimal"))
        }

        #[test]
        fn coingeckos_simple_price_reply_is_priced() {
            let body = include_str!("../fixtures/prices/coingecko_simple_price.json");
            assert_eq!(priced(COINGECKO, "bitcoin", "AUD", body), money("92568"));
        }

        #[test]
        fn coingeckos_empty_reply_for_an_unknown_coin_is_no_price() {
            assert_eq!(
                priced(COINGECKO, "notacoin", "AUD", "{}"),
                Reading::Failed(FetchFailure::NoPrice)
            );
        }

        #[test]
        fn yahoos_chart_reply_is_priced_at_its_regular_market_price() {
            let body = include_str!("../fixtures/prices/yahoo_chart.json");
            assert_eq!(priced(YAHOO_FINANCE, "BHP.AX", "AUD", body), money("52.52"));
        }

        #[test]
        fn yahoos_not_found_reply_is_not_found() {
            let body = include_str!("../fixtures/prices/yahoo_chart_not_found.json");
            let endpoint = &built_in(YAHOO_FINANCE).expect("a built-in").endpoint;
            let (_, path) = price_request(endpoint, "NOPE.AX", "AUD", None);
            assert_eq!(
                read(
                    &TransportOutcome::Answered {
                        status: 404,
                        body: body.into(),
                        retry_after: None,
                    },
                    &path
                ),
                Reading::Failed(FetchFailure::NotFound)
            );
        }

        #[test]
        fn frankfurters_rate_reply_is_priced() {
            let body = include_str!("../fixtures/prices/frankfurter_rate.json");
            assert_eq!(priced(FRANKFURTER, "EUR", "AUD", body), money("1.6513"));
        }
    }

    #[test]
    fn a_price_is_read_at_its_path_as_a_number_or_a_string() {
        let money = |text: &str| text.parse::<Money>().expect("a decimal");
        assert_eq!(
            parse_price(r#"{"base":"EUR","quote":"AUD","rate":1.6512}"#, "/rate"),
            Some(money("1.6512"))
        );
        assert_eq!(
            parse_price(r#"{"a":{"b":"0.25"}}"#, "/a/b"),
            Some(money("0.25"))
        );
    }

    #[test]
    fn no_price_is_read_from_a_missing_path_a_non_number_or_a_body_that_isnt_json() {
        assert_eq!(parse_price(r#"{"rate":1.5}"#, "/rates/AUD"), None);
        assert_eq!(parse_price(r#"{"rate":true}"#, "/rate"), None);
        assert_eq!(parse_price(r#"{"rate":"n/a"}"#, "/rate"), None);
        assert_eq!(parse_price("<html>", "/rate"), None);
    }

    #[test]
    fn each_transport_outcome_reads_onto_its_reason_at_its_level() {
        let answered = |status: u16, body: &str| TransportOutcome::Answered {
            status,
            body: body.into(),
            retry_after: None,
        };
        let cases = [
            (
                TransportOutcome::Failed(TransportError::Offline),
                FetchFailure::Offline,
            ),
            (
                TransportOutcome::Failed(TransportError::Timeout),
                FetchFailure::Offline,
            ),
            (
                TransportOutcome::Failed(TransportError::Tls),
                FetchFailure::SourceError,
            ),
            (answered(429, ""), FetchFailure::RateLimited),
            (answered(401, ""), FetchFailure::KeyRejected),
            (answered(403, ""), FetchFailure::KeyRejected),
            (answered(503, "<html>"), FetchFailure::SourceError),
            (answered(200, "<html>"), FetchFailure::SourceError),
            (
                answered(404, r#"{"message":"not found"}"#),
                FetchFailure::NotFound,
            ),
            (answered(422, ""), FetchFailure::NotFound),
            (answered(200, r#"{"rates":{}}"#), FetchFailure::NoPrice),
        ];
        for (outcome, reason) in cases {
            assert_eq!(
                read(&outcome, "/rate"),
                Reading::Failed(reason),
                "{outcome:?}"
            );
        }
        assert_eq!(
            read(&answered(200, r#"{"rate":1.5}"#), "/rate"),
            Reading::Priced("1.5".parse().expect("a decimal"))
        );
        let round = [
            FetchFailure::Offline,
            FetchFailure::RateLimited,
            FetchFailure::KeyMissing,
            FetchFailure::KeyRejected,
            FetchFailure::SourceError,
        ];
        assert!(round.iter().all(|reason| reason.is_round()));
        assert!(!FetchFailure::NotFound.is_round());
        assert!(!FetchFailure::NoPrice.is_round());
    }

    mod retry_after {
        use super::*;

        fn now() -> DateTime<Utc> {
            NaiveDate::from_ymd_opt(2015, 10, 21)
                .and_then(|day| day.and_hms_opt(7, 27, 0))
                .expect("a real time")
                .and_utc()
        }

        #[test]
        fn seconds_wait_that_many_seconds() {
            assert_eq!(retry_after_wait(Some("40"), now()), Duration::seconds(40));
            assert_eq!(
                retry_after_wait(Some(" 120 "), now()),
                Duration::seconds(120)
            );
        }

        #[test]
        fn an_http_date_waits_until_it() {
            assert_eq!(
                retry_after_wait(Some("Wed, 21 Oct 2015 07:28:00 GMT"), now()),
                Duration::seconds(60)
            );
            assert_eq!(
                retry_after_wait(Some("Wed, 21 Oct 2015 07:37:30 GMT"), now()),
                Duration::seconds(630)
            );
        }

        #[test]
        fn missing_unreadable_or_past_waits_60_seconds() {
            for raw in [
                None,
                Some("soon"),
                Some("-5"),
                Some("Wed, 21 Oct 2015 07:00:00 GMT"),
            ] {
                assert_eq!(
                    retry_after_wait(raw, now()),
                    Duration::seconds(60),
                    "{raw:?}"
                );
            }
        }
    }

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
