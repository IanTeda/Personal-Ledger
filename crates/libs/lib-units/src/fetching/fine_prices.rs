//! The Unit prices view's Daily and Hourly Price requests: the ones queued, out or waiting for
//! their source's round, the series fetched this session and why a series last failed.

use std::collections::HashMap;

use chrono::{DateTime, NaiveDate, Utc};
use lib_core::PriceSourceId;

use super::{
    FINE_FRESH, FetchFailure, FineKey, FinePrice, HistoryReading, PriceRequest, PriceResponse,
    RequestId, SourceSettings, back_off, fine_prices, read_history,
};
use crate::{HistoryEndpoint, PriceLink};

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
    pub(super) fn source(&self) -> PriceSourceId {
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

/// Every finer prices request, and the series fetched or failed.
#[derive(Default)]
pub(super) struct FinePrices {
    /// The Unit prices view's Daily and Hourly Price requests queued or out, by request.
    fine_out: HashMap<RequestId, FineCall>,
    /// Daily and Hourly Price requests waiting for their source's round to land, oldest first.
    fine_parked: Vec<FineCall>,
    /// The Daily and Hourly Prices fetched this session, never written to the Ledger.
    fine_cache: HashMap<FineKey, FineEntry>,
    /// Why a series' last request failed, until it is asked again: shown in place and nowhere
    /// else, so never a Unit's or source's Last fetch.
    fine_failed: HashMap<FineKey, (PriceLink, FetchFailure)>,
}

impl FinePrices {
    /// Forgets the series' last failure and any series the Unit fetched on another link, as a new
    /// ask for `call` does.
    pub(super) fn forget(&mut self, call: &FineCall) {
        let key = call.key;
        self.fine_failed.remove(&key);
        self.fine_cache
            .retain(|cached, entry| cached.unit != key.unit || entry.link == call.link);
    }

    /// Holds `call` until its source's round lands.
    pub(super) fn park(&mut self, call: FineCall) {
        self.fine_parked.push(call);
    }

    /// Remembers `call` as request `id`, out until it lands.
    pub(super) fn sent(&mut self, id: RequestId, call: FineCall) {
        self.fine_out.insert(id, call);
    }

    /// The finer prices requests that waited for `source`'s round, now it has landed.
    pub(super) fn release_parked(&mut self, source: PriceSourceId) -> Vec<FineCall> {
        let (going, staying) = std::mem::take(&mut self.fine_parked)
            .into_iter()
            .partition(|call| call.source() == source);
        self.fine_parked = staying;
        going
    }

    /// Notes that a series couldn't be asked for `reason` without sending.
    pub(super) fn refused(&mut self, key: FineKey, link: PriceLink, reason: FetchFailure) {
        self.fine_failed.insert(key, (link, reason));
    }

    /// Whether the series `key` is queued, out or waiting for a round.
    pub(super) fn asked(&self, key: FineKey) -> bool {
        self.fine_out.values().any(|call| call.key == key)
            || self.fine_parked.iter().any(|call| call.key == key)
    }

    /// Whether `source` has a finer prices request waiting or out.
    pub(super) fn in_flight(&self, source: PriceSourceId) -> bool {
        self.fine_out.values().any(|call| call.source() == source)
            || self.fine_parked.iter().any(|call| call.source() == source)
    }

    /// Whether `response` answers a finer prices request.
    pub(super) fn is_fine(&self, response: &PriceResponse) -> bool {
        self.fine_out.contains_key(&response.request)
    }

    /// The source of the request `id`, if it is a finer prices one out.
    pub(super) fn source_of(&self, id: RequestId) -> Option<PriceSourceId> {
        self.fine_out.get(&id).map(FineCall::source)
    }

    /// The request out for the series `key` and its source.
    pub(super) fn out_call(&self, key: FineKey) -> Option<(RequestId, PriceSourceId)> {
        self.fine_out
            .iter()
            .find(|(_, call)| call.key == key)
            .map(|(id, call)| (*id, call.source()))
    }

    /// Reads a finer prices `response` at `now` into the cache, its days dated by the provider and
    /// its hours in `zone`, or notes its failure for the view alone. `hold` is its source's
    /// `Retry-After` hold from a 429. Returns until when an Offline or Rate limited failure backs
    /// the source off, as a history request's does, for the caller to write.
    pub(super) fn land<Tz: chrono::TimeZone>(
        &mut self,
        response: &PriceResponse,
        now: DateTime<Utc>,
        today: NaiveDate,
        zone: &Tz,
        hold: Option<DateTime<Utc>>,
    ) -> Option<(SourceSettings, DateTime<Utc>)> {
        let call = self.fine_out.remove(&response.request)?;
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
                None
            }
            HistoryReading::Failed(reason) => {
                tracing::warn!(unit_id = %call.key.unit, price_source_id = %call.source(), ?reason, "A finer prices request failed");
                let backed = matches!(reason, FetchFailure::Offline | FetchFailure::RateLimited)
                    .then(|| {
                        let backed = now + back_off(call.settings.updates);
                        (
                            call.settings.clone(),
                            hold.map_or(backed, |until| until.max(backed)),
                        )
                    });
                self.fine_failed.insert(call.key, (call.link, reason));
                backed
            }
        }
    }

    /// Whether the series `key` is cached on `link` and still fresh at `now`: a past window for the
    /// session, one holding today for [`FINE_FRESH`].
    pub(super) fn fresh(&self, key: FineKey, link: &PriceLink, now: DateTime<Utc>) -> bool {
        self.fine_cache.get(&key).is_some_and(|entry| {
            entry.link == *link && (!entry.current || now - entry.fetched_at < FINE_FRESH)
        })
    }

    /// What the view has of the series `key` on `link` at `now`. `queued` is whether the series'
    /// call still waits in the call queue and `hold` its source's `Retry-After` hold.
    pub(super) fn state(
        &self,
        key: FineKey,
        link: &PriceLink,
        now: DateTime<Utc>,
        queued: bool,
        hold: Option<DateTime<Utc>>,
    ) -> FineState<'_> {
        if self.asked(key) {
            let waiting = if queued { hold } else { None }.map_or(0, |until| {
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
}
