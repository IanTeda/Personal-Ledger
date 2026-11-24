//! The rounds in flight: the requests sent to one source together, held until the last lands so
//! the round writes in one commit, and what each of their requests asked for.

use std::collections::{HashMap, HashSet};

use lib_core::{PriceSourceId, UnitId};

use super::{PriceResponse, RequestId, SourceSettings};
use crate::PriceLink;

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

/// Every round in flight, and every round's request queued or out.
#[derive(Default)]
pub(super) struct Rounds {
    next_round: u64,
    rounds: HashMap<RoundId, Round>,
    /// Every round's call queued or out, by request.
    in_flight: HashMap<RequestId, InFlight>,
}

impl Rounds {
    /// How many of the rounds' requests are queued or out.
    pub(super) fn in_flight(&self) -> usize {
        self.in_flight.len()
    }

    /// Whether request `id` is still queued or out.
    pub(super) fn in_flight_request(&self, id: RequestId) -> bool {
        self.in_flight.contains_key(&id)
    }

    /// Whether a request for `unit` is queued or out.
    pub(super) fn fetching(&self, unit: UnitId) -> bool {
        self.in_flight
            .values()
            .any(|sent| sent.units.iter().any(|call| call.unit == unit))
    }

    /// Whether `source` has a round queued or out.
    pub(super) fn source_in_flight(&self, source: PriceSourceId) -> bool {
        self.rounds.values().any(|round| round.source == source)
    }

    /// Whether `unit` is in a round a person asked for that is still out.
    pub(super) fn unit_asked_for(&self, unit: UnitId) -> bool {
        self.rounds
            .values()
            .any(|round| round.asked && round.units.contains(&unit))
    }

    /// Whether `source` has a round a person asked for still out.
    pub(super) fn source_asked_for(&self, source: PriceSourceId) -> bool {
        self.rounds
            .values()
            .any(|round| round.asked && round.source == source)
    }

    /// Starts a round on `source` with `settings`, `asked` when a person asked for it.
    pub(super) fn open(
        &mut self,
        source: PriceSourceId,
        settings: SourceSettings,
        asked: bool,
    ) -> RoundId {
        let id = RoundId(self.next_round);
        self.next_round += 1;
        self.rounds.insert(
            id,
            Round {
                source,
                settings,
                asked,
                units: HashSet::new(),
                pending: HashSet::new(),
                landed: Vec::new(),
            },
        );
        id
    }

    /// `round`'s source and whether a person asked for it; `None` for a round not in flight.
    pub(super) fn source_of(&self, round: RoundId) -> Option<(PriceSourceId, bool)> {
        self.rounds
            .get(&round)
            .map(|round| (round.source, round.asked))
    }

    /// Notes request `id`, asking for `sent`, as one of `round`'s, remembering it until its
    /// response comes back.
    pub(super) fn sent(&mut self, round: RoundId, id: RequestId, sent: InFlight) {
        if let Some(round) = self.rounds.get_mut(&round) {
            round.units.extend(sent.units.iter().map(|call| call.unit));
            round.pending.insert(id);
        }
        self.in_flight.insert(id, sent);
    }

    /// The round request `request` belongs to and its source; `None` for a response for no round.
    pub(super) fn round_of(&self, request: RequestId) -> Option<(RoundId, PriceSourceId)> {
        self.rounds
            .iter()
            .find(|(_, round)| round.pending.contains(&request))
            .map(|(&id, round)| (id, round.source))
    }

    /// Holds `response` in `round`. When it `failed` the round, hands back the round's requests
    /// still pending, so those still queued can be stopped and [`Self::forget`] them; else none.
    pub(super) fn land(
        &mut self,
        round: RoundId,
        response: PriceResponse,
        failed: bool,
    ) -> HashSet<RequestId> {
        let Some(round) = self.rounds.get_mut(&round) else {
            return HashSet::new();
        };
        round.pending.remove(&response.request);
        round.landed.push(response);
        if failed {
            round.pending.clone()
        } else {
            HashSet::new()
        }
    }

    /// Forgets `round`'s requests `ids`, which never went out, and the Units they asked for.
    pub(super) fn forget(&mut self, round: RoundId, ids: &HashSet<RequestId>) {
        let Some(round) = self.rounds.get_mut(&round) else {
            return;
        };
        for id in ids {
            for call in self
                .in_flight
                .remove(id)
                .map(|sent| sent.units)
                .unwrap_or_default()
            {
                round.units.remove(&call.unit);
            }
        }
        round.pending.retain(|id| !ids.contains(id));
    }

    /// Hands back `round` whole, forgetting it, once nothing of it is pending; `None` while a
    /// request is still out.
    pub(super) fn finish(&mut self, round: RoundId) -> Option<Landed> {
        if !self.rounds.get(&round)?.pending.is_empty() {
            return None;
        }
        let round = self.rounds.remove(&round)?;
        Some(Landed {
            source: Some((round.source, round.settings)),
            asked: round.asked,
            responses: round.landed,
        })
    }

    /// What request `id` was for, forgetting it.
    pub(super) fn take(&mut self, id: RequestId) -> Option<InFlight> {
        self.in_flight.remove(&id)
    }
}
