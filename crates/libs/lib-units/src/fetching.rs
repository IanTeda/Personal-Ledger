//! Fetching a Unit's price from its Price Source (ADR-0062): the plain transport port the Client's
//! adapter (`lib-price-feed`) answers, the request it is handed and the response it hands back,
//! and [`PriceFetching`], the per-Client bookkeeping of what is in flight and how each source's and
//! Unit's last fetch went.
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

use crate::{
    Endpoint, KeyPlacement, PriceLink, PriceSource, PriceSourceAuth, UnitRow, Updates, WeeklyPrice,
};

/// The shortest a failing source or Unit is left alone, whatever its source's Updates.
pub const MIN_BACK_OFF: Duration = Duration::minutes(15);

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
fn back_off(updates: Updates) -> Duration {
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
        TransportOutcome::Answered { status, body } => match status {
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
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransportOutcome {
    Answered { status: u16, body: String },
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

/// What a request was for: the Unit and its link as they were when it went out, so a response for
/// a Unit relinked since writes nothing, and the price path filled for that link.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct InFlight {
    pub unit: UnitId,
    pub link: PriceLink,
    pub price_path: String,
    pub settings: SourceSettings,
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
    in_flight: HashMap<RequestId, InFlight>,
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
            next_round: 0,
            rounds: HashMap::new(),
            units: HashMap::new(),
            sources: HashMap::new(),
            tests: HashMap::new(),
            test_results: HashMap::new(),
            dropped: HashSet::new(),
        }
    }

    pub fn shared(transport: Rc<dyn PriceTransport>) -> SharedFetching {
        Rc::new(RefCell::new(Self::new(transport)))
    }

    /// How many requests are out.
    pub fn in_flight(&self) -> usize {
        self.in_flight.len()
    }

    /// Whether request `id` is still out.
    pub(crate) fn in_flight_request(&self, id: RequestId) -> bool {
        self.in_flight.contains_key(&id)
    }

    /// Whether a request for `unit` is out.
    pub fn fetching(&self, unit: UnitId) -> bool {
        self.in_flight.values().any(|sent| sent.unit == unit)
    }

    /// Whether `source` has a round out, which a scheduled check leaves alone.
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

    /// Sends a test's `request`, remembering what it asked until it lands. Never in a round, so
    /// it neither waits for nor holds up a fetch.
    pub(crate) fn send_test(&mut self, request: PriceRequest, asked: TestInFlight) -> RequestId {
        let id = RequestId(self.next);
        self.next += 1;
        self.tests.insert(id, asked);
        self.transport.send(id, request);
        id
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
            TransportOutcome::Answered { status, body } => {
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

    /// Sends `request` for `sent` in `round`, remembering it until its response comes back.
    pub(crate) fn send(
        &mut self,
        round: RoundId,
        request: PriceRequest,
        sent: InFlight,
    ) -> RequestId {
        let id = RequestId(self.next);
        self.next += 1;
        let unit = sent.unit;
        self.in_flight.insert(id, sent);
        if let Some(round) = self.rounds.get_mut(&round) {
            round.units.insert(unit);
            round.pending.insert(id);
        }
        self.transport.send(id, request);
        id
    }

    /// Holds `response` in its round, handing back the whole round once its last response lands.
    /// A response for no round in flight lands alone, for the writer to drop.
    pub(crate) fn land(&mut self, response: PriceResponse) -> Option<Landed> {
        let Some((&id, round)) = self
            .rounds
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

    /// Whether `source` failed a round and is still backing off at `now`.
    pub(crate) fn source_backing_off(&self, source: &PriceSource<'_>, now: DateTime<Utc>) -> bool {
        self.source_fetch(source)
            .is_some_and(|last| last.backing_off(now))
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
        let last = Self::last(settings, at, failure);
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
fn fill(text: &str, symbol: &str, quote: &str) -> String {
    text.replace("{symbol}", symbol)
        .replace("{quote}", quote)
        .replace("{symbol_lower}", &symbol.to_lowercase())
        .replace("{quote_lower}", &quote.to_lowercase())
}

/// The request asking `endpoint` for `symbol`'s price in `quote` (the quote Unit's code), carrying
/// the endpoint's fixed headers and `key` where its authentication places it, and the price path
/// filled the same way. With no key, a source whose key is optional is asked keyless.
pub(crate) fn price_request(
    endpoint: &Endpoint,
    symbol: &str,
    quote: &str,
    key: Option<SecretString>,
) -> (PriceRequest, String) {
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
    let request = PriceRequest {
        method: Method::Get,
        url: format!(
            "{}{}",
            endpoint.address.trim_end_matches('/'),
            fill(&endpoint.request, symbol, quote)
        ),
        headers: endpoint
            .headers
            .iter()
            .map(|header| (header.name.to_string(), header.value.to_string()))
            .collect(),
        key,
    };
    (request, fill(&endpoint.price_path, symbol, quote))
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
                        body: body.into()
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
