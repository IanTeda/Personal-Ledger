//! Each Unit's and source's Last fetch, and the quiet back-off a history or finer prices request
//! leaves on a source: held in memory for the open Ledger only, never written to it or synced.

use std::collections::HashMap;

use chrono::{DateTime, Duration, Utc};
use lib_core::{PriceSourceId, UnitId};

use super::{FetchFailure, SourceSettings};
use crate::{PriceLink, PriceSource, UnitRow, Updates};

/// The shortest a failing source or Unit is left alone, whatever its source's Updates.
pub const MIN_BACK_OFF: Duration = Duration::minutes(15);

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

    fn of(settings: &SourceSettings, at: DateTime<Utc>, failure: Option<FetchFailure>) -> Self {
        Self {
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

/// Every Unit's and source's Last fetch, and the sources backed off quietly.
#[derive(Default)]
pub(super) struct LastFetches {
    units: HashMap<UnitId, UnitRecord>,
    sources: HashMap<PriceSourceId, SourceRecord>,
    /// Sources a history request found offline or rate limited, backed off for everyone until
    /// then without touching their Last fetch.
    quiet_back_off: HashMap<PriceSourceId, (SourceSettings, DateTime<Utc>)>,
}

impl LastFetches {
    /// `unit`'s last fetch on `source` while it still holds: the Unit is active and neither its
    /// link nor its source's settings changed since.
    pub(super) fn unit_fetch(
        &self,
        unit: &UnitRow,
        source: &PriceSource<'_>,
    ) -> Option<&LastFetch> {
        let record = self.units.get(&unit.id)?;
        (unit.is_active
            && source.id == unit.link.source_id
            && record.link == unit.link
            && record.settings == SourceSettings::of(source))
        .then_some(&record.last)
    }

    /// `source`'s last round while it still holds: its settings haven't changed since.
    pub(super) fn source_fetch(&self, source: &PriceSource<'_>) -> Option<&LastFetch> {
        let record = self.sources.get(&source.id)?;
        (record.settings == SourceSettings::of(source)).then_some(&record.last)
    }

    /// Clears `source`'s Key missing and Key rejected failures, and its Units'.
    pub(super) fn key_stored(&mut self, source: PriceSourceId) {
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

    /// Whether `source` failed a round, or is backed off quietly, and is still backing off at
    /// `now`.
    pub(super) fn source_backing_off(&self, source: &PriceSource<'_>, now: DateTime<Utc>) -> bool {
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

    /// Whether `unit` failed and is still backing off at `now`.
    pub(super) fn unit_backing_off(
        &self,
        unit: &UnitRow,
        source: &PriceSource<'_>,
        now: DateTime<Utc>,
    ) -> bool {
        self.unit_fetch(unit, source)
            .is_some_and(|last| last.backing_off(now))
    }

    /// Whether `unit` was fetched on another link than it has now.
    pub(super) fn relinked(&self, unit: &UnitRow) -> bool {
        self.units
            .get(&unit.id)
            .is_some_and(|record| record.link != unit.link)
    }

    /// Backs `source` off until `until` on `settings` without touching its Last fetch.
    pub(super) fn back_off_quietly(
        &mut self,
        source: PriceSourceId,
        settings: SourceSettings,
        until: DateTime<Utc>,
    ) {
        self.quiet_back_off.insert(source, (settings, until));
    }

    /// Notes how `unit`'s fetch on `link` went at `at`, backing it off on a failure.
    pub(super) fn record_unit(
        &mut self,
        unit: UnitId,
        link: PriceLink,
        settings: &SourceSettings,
        at: DateTime<Utc>,
        failure: Option<FetchFailure>,
    ) {
        self.units.insert(
            unit,
            UnitRecord {
                last: LastFetch::of(settings, at, failure),
                link,
                settings: settings.clone(),
            },
        );
    }

    /// Notes how `source`'s round went at `at`: failed on a round failure, which backs it off, or
    /// fetched, which clears the last. `hold` is the source's `Retry-After` hold, if any.
    pub(super) fn record_source(
        &mut self,
        source: PriceSourceId,
        settings: &SourceSettings,
        at: DateTime<Utc>,
        failure: Option<FetchFailure>,
        hold: Option<DateTime<Utc>>,
    ) {
        let mut last = LastFetch::of(settings, at, failure);
        // A 429 backs off for the longer of the usual rule and its `Retry-After`.
        if let (
            Some(FetchFailure::RateLimited),
            Some(until),
            FetchOutcome::Failed { retry_at, .. },
        ) = (failure, hold, &mut last.outcome)
        {
            *retry_at = (*retry_at).max(until);
        }
        self.sources.insert(
            source,
            SourceRecord {
                last,
                settings: settings.clone(),
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_failure_backs_off_for_its_sources_interval_and_never_under_15_minutes() {
        assert_eq!(back_off(Updates::Every15Minutes), Duration::minutes(15));
        assert_eq!(back_off(Updates::Hourly), Duration::hours(1));
        assert_eq!(back_off(Updates::Daily), Duration::hours(24));
        assert_eq!(back_off(Updates::ManualOnly), MIN_BACK_OFF);
    }
}
