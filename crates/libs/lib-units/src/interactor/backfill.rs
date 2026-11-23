use chrono::{DateTime, Duration, NaiveDate, Utc};
use lib_core::UnitId;

use crate::fetching::{FetchFailure, HistoryCall, PriceFetching, SourceSettings, key_missing};
use crate::history::{floor, gaps, history_request, wanted_weeks};
use crate::protocols::UnitsPorts;
use crate::{UnitRow, Updates, WeeklyPrice, units_by_code};

/// Which Units a backfill asks history for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BackfillUnits {
    /// Every Unit wanting history, as the Ledger opens: never one on a Manual only source.
    Every,
    /// The Units a person's fetch now just priced, a Manual only source's included, which the
    /// status line names while their history is out.
    Asked(Vec<UnitId>),
    /// The Units whose history request failed and is due again, while last week is still empty;
    /// older gaps wait for the next open.
    Retrying(Vec<UnitId>),
}

/// Ask each Unit's source for the past weeks it has no Weekly Price for (ADR-0043's backfill
/// amendment): from the week holding its floor, the earliest Opened date of its Accounts or else
/// its creation, to last week, within its source's history depth, leaving out the weeks its
/// provider already answered with nothing. Only sends: each Unit's weeks are written in one
/// commit when its response comes back, through [`crate::interactor::RecordPriceHistory`].
///
/// Never backfills a Unit on Manual entry, a deactivated Unit, the base Unit or a Unit on a source
/// with no history request. One request per source is in flight at a time, the Unit with the
/// oldest gap first, each paced by the source's call rate behind the calls a person asked for. A
/// source needing a key this Client doesn't have fails the Unit's history Key missing unsent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Backfill {
    pub units: BackfillUnits,
    /// The Clock's day, whose week is never backfilled.
    pub today: NaiveDate,
    pub now: DateTime<Utc>,
}

impl Backfill {
    /// Returns how many Units' history was queued.
    pub fn run(self, ports: &dyn UnitsPorts, fetching: &mut PriceFetching) -> usize {
        let base = ports.base_unit().base_unit();
        let catalogue = ports.source_catalogue();
        let units = ports.units().units();
        let (only, asked, retrying) = match &self.units {
            BackfillUnits::Every => (None, false, false),
            BackfillUnits::Asked(ids) => (Some(ids), true, false),
            BackfillUnits::Retrying(ids) => (Some(ids), false, true),
        };
        let last_week = WeeklyPrice::week_of(self.today) - Duration::weeks(1);
        let mut calls: Vec<HistoryCall> = Vec::new();
        for unit in units_by_code(&units) {
            if only.is_some_and(|ids| !ids.contains(&unit.id)) {
                continue;
            }
            if retrying {
                // Tried again or not, the failure is spent: one that fails again is noted anew.
                fetching.clear_history_failure(unit.id);
            }
            if !unit.is_active || unit.id == base || fetching.history_wanted(unit.id) {
                continue;
            }
            let Some(source) = catalogue.source(unit.link.source_id) else {
                continue;
            };
            let Some(endpoint) = source.endpoint() else {
                continue;
            };
            let Some(history) = endpoint.history else {
                continue;
            };
            if source.updates == Updates::ManualOnly && !asked && !retrying {
                continue;
            }
            let Some(weeks) = self.gaps(ports, fetching, unit, source.history_depth()) else {
                continue;
            };
            if retrying && weeks.last() != Some(&last_week) {
                continue;
            }
            let Some(quote) = units.iter().find(|quote| quote.id == unit.link.quote) else {
                tracing::warn!(unit_id = %unit.id, "Skipped backfilling a Unit whose quote Unit is gone");
                continue;
            };
            let (Some(&from), Some(&to)) = (weeks.first(), weeks.last()) else {
                continue;
            };
            let settings = SourceSettings::of(&source);
            let key = ports.keys().key(source.id);
            let mut call = HistoryCall {
                unit: unit.id,
                code: unit.code.clone(),
                link: unit.link.clone(),
                source: source.id,
                settings,
                history: history.clone(),
                quote: quote.code.clone(),
                weeks,
                asked,
                request: None,
            };
            if key_missing(endpoint, key.as_ref()) {
                tracing::warn!(price_source_id = %source.id, unit_id = %unit.id, "A Unit's history needs an API key this Client doesn't have");
                fetching.history_failed(&call, FetchFailure::KeyMissing, self.now);
                continue;
            }
            call.request = Some(history_request(
                endpoint,
                history,
                &unit.link.symbol,
                &quote.code,
                (from, to + Duration::days(6)),
                key,
            ));
            calls.push(call);
        }
        // Oldest gap first, across every source's Units.
        calls.sort_by_key(|call| call.weeks.first().copied());
        let mut queued = 0;
        for call in calls {
            if fetching.queue_history(call, self.now) {
                queued += 1;
            }
        }
        if queued == 0 {
            tracing::debug!("No Unit's history is wanted");
        }
        queued
    }

    /// `unit`'s empty weeks, oldest first, on a source whose history reaches `depth` days back;
    /// `None` when it wants none.
    fn gaps(
        &self,
        ports: &dyn UnitsPorts,
        fetching: &PriceFetching,
        unit: &UnitRow,
        depth: Option<u32>,
    ) -> Option<Vec<NaiveDate>> {
        let floor = floor(unit, ports.openings().earliest_opened(unit.id))?;
        let wanted = wanted_weeks(floor, self.today, depth)?;
        let weeks = gaps(unit, wanted, &fetching.empty_weeks(unit));
        (!weeks.is_empty()).then_some(weeks)
    }
}
