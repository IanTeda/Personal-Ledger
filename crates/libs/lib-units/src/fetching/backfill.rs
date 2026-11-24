//! The history requests (ADR-0043's backfill amendment): each source's one out and the rest
//! queued behind it, each Unit's failed request until it is tried again, and the weeks a provider
//! answered with nothing.

use std::collections::{HashMap, HashSet};

use chrono::{DateTime, Duration, NaiveDate, Utc};
use lib_core::{PriceSourceId, UnitId};

use super::{FetchFailure, PriceRequest, PriceResponse, RequestId, SourceSettings, back_off};
use crate::{HistoryEndpoint, PriceLink, PriceSource, UnitRow};

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

/// Every history request queued or out, and every failure and empty week kept for the next ask.
#[derive(Default)]
pub(super) struct Backfill {
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
}

impl Backfill {
    /// Queues `call` behind its source's history requests, oldest gap first. Returns false,
    /// queuing nothing, for a Unit whose history is already queued or out.
    pub(super) fn queue(&mut self, call: HistoryCall) -> bool {
        if self.wanted(call.unit) {
            return false;
        }
        self.history_failures.remove(&call.unit);
        let queue = self.history_queue.entry(call.source).or_default();
        queue.push(call);
        queue.sort_by_key(|call| call.weeks.first().copied());
        true
    }

    /// `source`'s next waiting history request, with its request taken for the call queue, unless
    /// one is already in flight: one at a time per source.
    pub(super) fn next(&mut self, source: PriceSourceId) -> Option<(HistoryCall, PriceRequest)> {
        if self.history_out.values().any(|call| call.source == source) {
            return None;
        }
        let queue = self.history_queue.get_mut(&source)?;
        if queue.is_empty() {
            self.history_queue.remove(&source);
            return None;
        }
        let mut call = queue.remove(0);
        let request = call.request.take()?;
        Some((call, request))
    }

    /// Remembers `call` as request `id`, out until it lands.
    pub(super) fn sent(&mut self, id: RequestId, call: HistoryCall) {
        self.history_out.insert(id, call);
    }

    /// Whether `response` answers a history request.
    pub(super) fn is_history(&self, response: &PriceResponse) -> bool {
        self.history_out.contains_key(&response.request)
    }

    /// What history request `id` was for, forgetting it.
    pub(super) fn land(&mut self, id: RequestId) -> Option<HistoryCall> {
        self.history_out.remove(&id)
    }

    /// Notes that `call` failed for `reason` at `at`: it is tried again after [`HISTORY_RETRY`],
    /// or its source's `Retry-After` `hold` if longer. Offline and Rate limited fail the Units
    /// waiting behind it the same way, since they would fail too. Returns until when they back the
    /// source off for everyone, for the caller to write.
    pub(super) fn history_failed(
        &mut self,
        call: &HistoryCall,
        reason: FetchFailure,
        at: DateTime<Utc>,
        hold: Option<DateTime<Utc>>,
    ) -> Option<DateTime<Utc>> {
        tracing::warn!(unit_id = %call.unit, price_source_id = %call.source, ?reason, "A Unit's history request failed");
        let retry_at = hold.map_or(at + HISTORY_RETRY, |until| until.max(at + HISTORY_RETRY));
        let mut failed = vec![(call.unit, call.link.clone(), call.settings.clone())];
        let mut backed = None;
        if reason.is_round() {
            if matches!(reason, FetchFailure::Offline | FetchFailure::RateLimited) {
                backed = Some(hold.map_or(at + back_off(call.settings.updates), |until| {
                    until.max(at + back_off(call.settings.updates))
                }));
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
        backed
    }

    /// Notes that `call`'s provider answered with nothing for `weeks`, so they aren't asked again
    /// on this link until the Ledger reopens.
    pub(super) fn history_empty(
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
    pub(super) fn empty_weeks(&self, unit: &UnitRow) -> HashSet<NaiveDate> {
        self.empty_weeks
            .get(&unit.id)
            .filter(|(link, _)| *link == unit.link)
            .map(|(_, weeks)| weeks.clone())
            .unwrap_or_default()
    }

    /// Whether `unit`'s history request is queued or out.
    pub(super) fn fetching(&self, unit: UnitId) -> bool {
        self.history_out.values().any(|call| call.unit == unit)
    }

    /// Whether `unit` has a history request waiting, queued or out.
    pub(super) fn wanted(&self, unit: UnitId) -> bool {
        self.fetching(unit)
            || self
                .history_queue
                .values()
                .any(|queue| queue.iter().any(|call| call.unit == unit))
    }

    /// Why `unit`'s last history request on `source` failed and when it is tried again, while it
    /// still holds: the Unit is active and neither its link nor its source's settings changed.
    pub(super) fn failure(
        &self,
        unit: &UnitRow,
        source: &PriceSource<'_>,
    ) -> Option<(FetchFailure, DateTime<Utc>)> {
        let failure = self.history_failures.get(&unit.id)?;
        (unit.is_active
            && source.id == unit.link.source_id
            && failure.link == unit.link
            && failure.settings.matches(source))
        .then_some((failure.reason, failure.retry_at))
    }

    /// The Units whose failed history request is due to be tried again at `now`.
    pub(super) fn retries(&self, now: DateTime<Utc>) -> Vec<UnitId> {
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
    pub(super) fn clear_failure(&mut self, unit: UnitId) {
        self.history_failures.remove(&unit);
    }
}
