use lib_core::UnitId;

use crate::protocols::UnitRepository;
use crate::{PriceLink, WeeklyPrice};

/// Write a Unit's backfilled weeks (ADR-0043's backfill amendment): each folded Weekly Price into
/// a week the Unit still has none for, so a week set by hand, fetched or synced meanwhile is never
/// overwritten. The weeks carry their real `written_at`. One Unit's weeks are one input, so the
/// Client commits them once.
///
/// Writes nothing for a Unit deleted, deactivated or relinked since its history was asked for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordPriceHistory {
    pub unit: UnitId,
    /// The link the history was asked on.
    pub link: PriceLink,
    pub weeks: Vec<WeeklyPrice>,
}

impl RecordPriceHistory {
    /// Returns the weeks written.
    pub fn run(self, units: &mut dyn UnitRepository) -> usize {
        let unit_id = self.unit;
        let mut unit = match units.unit(unit_id) {
            Some(unit) if unit.is_active && unit.link == self.link => unit,
            _ => {
                tracing::info!(
                    %unit_id,
                    "Skipped a Unit's history for a Unit deleted, deactivated or relinked since"
                );
                return 0;
            }
        };
        let mut written = 0;
        let mut priced: std::collections::HashSet<_> =
            unit.prices.iter().map(|price| price.week).collect();
        for week in self.weeks {
            if !priced.insert(week.week) {
                continue;
            }
            unit.prices.push(week);
            written += 1;
        }
        if written > 0 {
            unit.prices.sort_by_key(|price| price.week);
            units.upsert_unit(unit);
        }
        tracing::info!(%unit_id, weeks = written, "Recorded a Unit's backfilled Weekly Prices");
        written
    }
}

#[cfg(test)]
mod tests {
    use chrono::{DateTime, NaiveDate, Utc};
    use lib_core::{Money, UnitKind};

    use super::*;
    use crate::fake::InMemoryUnits;
    use crate::{FRANKFURTER, Precision, UnitRow};

    fn day(d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 10, d).expect("a real date")
    }

    fn at() -> DateTime<Utc> {
        day(21).and_hms_opt(9, 0, 0).expect("a real time").and_utc()
    }

    fn week(monday: NaiveDate, close: &str) -> WeeklyPrice {
        let close: Money = close.parse().expect("a decimal");
        WeeklyPrice {
            week: monday,
            open: Some(close.clone()),
            high: Some(close.clone()),
            low: Some(close.clone()),
            close,
            written_at: at(),
        }
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

    #[test]
    fn only_the_weeks_still_empty_are_written_in_week_order() {
        let mut unit = eur();
        unit.prices = vec![week(day(12), "9.99")];
        let mut units = InMemoryUnits {
            units: vec![unit.clone()],
        };

        let written = RecordPriceHistory {
            unit: unit.id,
            link: unit.link.clone(),
            weeks: vec![week(day(12), "1.61"), week(day(5), "1.60")],
        }
        .run(&mut units);

        assert_eq!(written, 1);
        assert_eq!(
            units.unit(unit.id).expect("stored").prices,
            [week(day(5), "1.60"), week(day(12), "9.99")],
            "the week set meanwhile keeps its price"
        );
    }

    #[test]
    fn a_unit_deactivated_or_relinked_since_is_skipped() {
        let mut unit = eur();
        let asked = unit.link.clone();
        unit.link.symbol = "EURO".into();
        let mut units = InMemoryUnits {
            units: vec![unit.clone()],
        };
        let record = |units: &mut InMemoryUnits, link: PriceLink| {
            RecordPriceHistory {
                unit: unit.id,
                link,
                weeks: vec![week(day(5), "1.60")],
            }
            .run(units)
        };

        assert_eq!(record(&mut units, asked), 0, "relinked");
        units.units[0].is_active = false;
        assert_eq!(record(&mut units, unit.link.clone()), 0, "deactivated");
        units.units.clear();
        assert_eq!(record(&mut units, unit.link.clone()), 0, "deleted");
    }
}
