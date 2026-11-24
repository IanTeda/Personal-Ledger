//! The call queue every request waits its turn in: paced by its source's [`CallRate`] and any
//! `Retry-After` the source answered with (ADR-0043's batching and throttling amendment), and
//! sent through the transport when the queue is next released. It hands out the request ids, so
//! nothing else counts them, and knows nothing of what a call is for.

use std::collections::{HashMap, HashSet, VecDeque};
use std::rc::Rc;

use chrono::{DateTime, Duration, Utc};
use lib_core::PriceSourceId;

use super::{PriceRequest, PriceTransport, RequestId};

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

/// Which window and `Retry-After` a call counts against: its source, `None` for a test of a
/// source not saved yet.
pub(super) type Lane = Option<PriceSourceId>;

/// A call waiting its turn: it goes out once its lane's window has room and no `Retry-After`
/// holds it, a call a person asked for ahead of a scheduled one.
struct Queued {
    id: RequestId,
    lane: Lane,
    rate: CallRate,
    asked: bool,
    request: PriceRequest,
}

/// The calls, round and test alike, waiting their turn, with each lane's recent calls and holds.
pub(super) struct CallQueue {
    transport: Rc<dyn PriceTransport>,
    next: u64,
    /// Oldest first.
    queue: Vec<Queued>,
    /// When each lane's calls in the last [`CallRate::WINDOW`] went out, oldest first.
    windows: HashMap<Lane, VecDeque<DateTime<Utc>>>,
    /// Until when each lane waits out a `Retry-After`.
    holds: HashMap<Lane, DateTime<Utc>>,
    /// The `Retry-After` hold a *Waiting for…* Toast last announced, so each wait raises one
    /// whichever of the page or the pump notices it first.
    announced_wait: Option<(PriceSourceId, DateTime<Utc>)>,
}

impl CallQueue {
    pub(super) fn new(transport: Rc<dyn PriceTransport>) -> Self {
        Self {
            transport,
            next: 0,
            queue: Vec::new(),
            windows: HashMap::new(),
            holds: HashMap::new(),
            announced_wait: None,
        }
    }

    /// Queues `request` on `lane`, `asked` when a person asked for it, handing back its id. It
    /// goes out on a later [`Self::release`].
    pub(super) fn enqueue(&mut self, lane: Lane, asked: bool, request: PriceRequest) -> RequestId {
        let id = RequestId(self.next);
        self.next += 1;
        let rate = CallRate::of(lane, request.key.is_some());
        self.queue.push(Queued {
            id,
            lane,
            rate,
            asked,
            request,
        });
        id
    }

    /// Sends every queued call its lane allows at `now`: the asked-for first, then the scheduled,
    /// oldest first within each, while the lane's window has room and no `Retry-After` holds it.
    /// A lane that can't send one call sends none after it, so no call overtakes another on its
    /// lane but an asked-for one. Returns the ids that went out, in the order sent.
    pub(super) fn release(&mut self, now: DateTime<Utc>) -> Vec<RequestId> {
        self.holds.retain(|_, until| now < *until);
        if self.queue.is_empty() {
            return Vec::new();
        }
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
        // Released on every poll, so a queue that sends nothing is left as it is, and unlogged.
        if going.is_empty() {
            return Vec::new();
        }
        let mut queue: Vec<Option<Queued>> = std::mem::take(&mut self.queue)
            .into_iter()
            .map(Some)
            .collect();
        let mut sent = Vec::with_capacity(going.len());
        for index in going {
            let Some(call) = queue[index].take() else {
                continue;
            };
            sent.push(call.id);
            self.transport.send(call.id, call.request);
        }
        self.queue = queue.into_iter().flatten().collect();
        if !self.queue.is_empty() {
            tracing::debug!(queued = self.queue.len(), "Price requests wait their turn");
        }
        sent
    }

    /// The source a call a person asked for waits on, and the whole seconds left of its
    /// `Retry-After` at `now`, rounded up, passing over every call `skip` names. `None` while no
    /// such call is held, a queued one only waiting its call rate included.
    pub(super) fn waiting(
        &self,
        now: DateTime<Utc>,
        skip: impl Fn(RequestId) -> bool,
    ) -> Option<(PriceSourceId, i64)> {
        self.queue
            .iter()
            .filter(|call| call.asked && !skip(call.id))
            .find_map(|call| {
                let source = call.lane?;
                let until = self.holds.get(&call.lane)?;
                let left = (*until - now).num_milliseconds();
                (left > 0).then_some((source, (left + 999) / 1000))
            })
    }

    /// [`Self::waiting`] the first time it is asked of a hold, `None` after: a wait is announced
    /// once as it begins, never counted down. `skip` is as in [`Self::waiting`].
    pub(super) fn wait_begun(
        &mut self,
        now: DateTime<Utc>,
        skip: impl Fn(RequestId) -> bool,
    ) -> Option<(PriceSourceId, i64)> {
        let (source, seconds) = self.waiting(now, skip)?;
        let until = self.held_until(Some(source))?;
        if self.announced_wait == Some((source, until)) {
            return None;
        }
        self.announced_wait = Some((source, until));
        Some((source, seconds))
    }

    /// Whether call `id` is still waiting its turn.
    pub(super) fn queued(&self, id: RequestId) -> bool {
        self.queue.iter().any(|call| call.id == id)
    }

    /// Takes every call in `ids` still waiting out of the queue, so it never goes, handing back
    /// the ids it removed.
    pub(super) fn remove(&mut self, ids: &HashSet<RequestId>) -> HashSet<RequestId> {
        let removed: HashSet<RequestId> = self
            .queue
            .iter()
            .filter(|call| ids.contains(&call.id))
            .map(|call| call.id)
            .collect();
        if !removed.is_empty() {
            self.queue.retain(|call| !removed.contains(&call.id));
        }
        removed
    }

    /// Holds `lane`'s calls until `until`, as a `Retry-After` asked.
    pub(super) fn hold(&mut self, lane: Lane, until: DateTime<Utc>) {
        self.holds.insert(lane, until);
    }

    /// Until when `lane` was last held, though the hold may have passed.
    pub(super) fn held_until(&self, lane: Lane) -> Option<DateTime<Utc>> {
        self.holds.get(&lane).copied()
    }
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;

    use super::*;
    use crate::fake::FakeTransport;
    use crate::fetching::price_request;
    use crate::{COINGECKO, FRANKFURTER, PriceSource, YAHOO_FINANCE, built_in};

    fn at(seconds: i64) -> DateTime<Utc> {
        NaiveDate::from_ymd_opt(2026, 10, 21)
            .and_then(|day| day.and_hms_opt(9, 0, 0))
            .expect("a real time")
            .and_utc()
            + Duration::seconds(seconds)
    }

    struct Client {
        transport: FakeTransport,
        calls: CallQueue,
    }

    fn client() -> Client {
        let transport = FakeTransport::default();
        Client {
            calls: CallQueue::new(Rc::new(transport.clone())),
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
        /// Queues a call to `source` for `symbol`, `asked` or scheduled, and releases at `now`.
        fn round(&mut self, id: PriceSourceId, symbol: &str, asked: bool, now: DateTime<Utc>) {
            let source = source(id);
            let endpoint = source.endpoint().expect("fetching");
            let (request, _) = price_request(endpoint, symbol, "AUD", None);
            self.calls.enqueue(Some(id), asked, request);
            self.calls.release(now);
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

        client.calls.release(at(59));
        assert_eq!(client.sent(), 60, "never more than 60 in any 60 seconds");

        client.calls.release(at(60));
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

        client.calls.release(at(60));
        assert_eq!(client.sent(), 6, "a's slot is free, b to e's aren't yet");
        client.calls.release(at(90));
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

        client.calls.release(at(60));

        let urls = client.transport.urls();
        assert_eq!(urls.len(), 7);
        assert!(urls[5].contains("ids=asked&"), "{urls:?}");
        assert!(urls[6].contains("ids=scheduled&"), "{urls:?}");
    }

    #[test]
    fn a_held_asked_wait_is_begun_once_per_hold() {
        let mut client = client();
        let built = source(COINGECKO);
        let (request, _) = price_request(built.endpoint().expect("fetching"), "a", "AUD", None);
        client.calls.enqueue(Some(COINGECKO), true, request);
        client.calls.hold(Some(COINGECKO), at(30));

        assert_eq!(
            client.calls.wait_begun(at(0), |_| false),
            Some((COINGECKO, 30))
        );
        assert_eq!(
            client.calls.wait_begun(at(1), |_| false),
            None,
            "the same hold is announced once"
        );

        client.calls.hold(Some(COINGECKO), at(60));
        assert_eq!(
            client.calls.wait_begun(at(1), |_| false),
            Some((COINGECKO, 59)),
            "a new hold is announced"
        );
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
