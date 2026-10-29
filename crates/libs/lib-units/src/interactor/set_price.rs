use chrono::{DateTime, NaiveDate, Utc};
use lib_core::{Money, UnitId};

use crate::protocols::UnitRepository;
use crate::{Error, MANUAL_ENTRY, Result, UnitRow, WeeklyPrice};

/// Set a Unit's close for one week by hand, the only way a Unit on Manual entry is priced. The week
/// is the one holding `day`; the current week takes it as its provisional price, an earlier week
/// as final.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetPrice {
    pub id: UnitId,
    pub day: NaiveDate,
    pub close: Money,
}

impl SetPrice {
    /// `today` is the Clock's day, which decides the current week; `at` is when the price is
    /// written. Returns the Unit written.
    pub fn run(
        self,
        units: &mut dyn UnitRepository,
        today: NaiveDate,
        at: DateTime<Utc>,
    ) -> Result<UnitRow> {
        let (id, week) = (self.id, WeeklyPrice::week_of(self.day));
        let outcome = self.apply(units, today, at);
        match &outcome {
            Ok(_) => tracing::info!(unit_id = %id, %week, "Set a Unit's price"),
            Err(refusal) => {
                tracing::info!(unit_id = %id, %week, %refusal, "Refused to set a Unit's price");
            }
        }
        outcome
    }

    fn apply(
        self,
        units: &mut dyn UnitRepository,
        today: NaiveDate,
        at: DateTime<Utc>,
    ) -> Result<UnitRow> {
        let mut unit = units.unit(self.id).ok_or(Error::UnitGone)?;
        if unit.link.source_id != MANUAL_ENTRY {
            return Err(Error::PricedBySource);
        }
        if WeeklyPrice::week_of(self.day) > WeeklyPrice::week_of(today) {
            return Err(Error::FutureWeek);
        }
        unit.set_price(self.close, self.day, today, at);
        units.upsert_unit(unit.clone());
        Ok(unit)
    }
}

#[cfg(test)]
mod tests {
    use chrono::Utc;

    use super::*;
    use crate::fake::InMemoryPorts;

    fn day(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).expect("a real date")
    }

    fn at(y: i32, m: u32, d: u32) -> DateTime<Utc> {
        day(y, m, d)
            .and_hms_opt(9, 0, 0)
            .expect("a real time")
            .and_utc()
    }

    fn money(text: &str) -> Money {
        text.parse().expect("a decimal")
    }

    fn set(id: UnitId, on: NaiveDate, close: &str) -> SetPrice {
        SetPrice {
            id,
            day: on,
            close: money(close),
        }
    }

    /// Thursday 29 October 2026, in the ISO week starting Monday 26 October.
    fn today() -> NaiveDate {
        day(2026, 10, 29)
    }

    #[test]
    fn setting_this_weeks_price_writes_it_provisionally_and_it_becomes_the_last_price() {
        let mut ports = InMemoryPorts::stub(Utc::now());

        set(UnitId::SEED_VAS, today(), "101.20")
            .run(&mut ports.units, today(), at(2026, 10, 29))
            .unwrap();
        let unit = set(UnitId::SEED_VAS, day(2026, 10, 30), "102.00")
            .run(&mut ports.units, day(2026, 10, 30), at(2026, 10, 30))
            .unwrap();

        let stored = ports.units.unit(UnitId::SEED_VAS).unwrap();
        assert_eq!(stored, unit);
        assert_eq!(
            stored.prices,
            vec![WeeklyPrice {
                week: day(2026, 10, 26),
                close: money("102.00"),
                open: None,
                high: Some(money("102.00")),
                low: Some(money("101.20")),
                written_at: at(2026, 10, 30),
            }],
            "the current week is provisional: its close is overwritten, its range widened"
        );
        assert_eq!(stored.last_price().unwrap().close, money("102.00"));
    }

    #[test]
    fn a_past_weeks_price_is_written_as_final_replacing_any_entry_it_had() {
        let mut ports = InMemoryPorts::stub(Utc::now());
        let earlier = day(2026, 9, 9);
        set(UnitId::SEED_VAS, earlier, "95.00")
            .run(&mut ports.units, today(), at(2026, 10, 27))
            .unwrap();

        set(UnitId::SEED_VAS, day(2026, 9, 13), "96.50")
            .run(&mut ports.units, today(), at(2026, 10, 29))
            .unwrap();

        assert_eq!(
            ports.units.unit(UnitId::SEED_VAS).unwrap().prices,
            vec![WeeklyPrice {
                week: day(2026, 9, 7),
                close: money("96.50"),
                open: None,
                high: None,
                low: None,
                written_at: at(2026, 10, 29),
            }]
        );
    }

    #[test]
    fn a_past_week_keeps_the_weeks_in_order_and_the_last_price_the_newest_weeks() {
        let mut ports = InMemoryPorts::stub(Utc::now());
        set(UnitId::SEED_VAS, today(), "101.20")
            .run(&mut ports.units, today(), at(2026, 10, 29))
            .unwrap();

        set(UnitId::SEED_VAS, day(2026, 10, 14), "99.00")
            .run(&mut ports.units, today(), at(2026, 10, 29))
            .unwrap();

        let unit = ports.units.unit(UnitId::SEED_VAS).unwrap();
        let weeks: Vec<_> = unit.prices.iter().map(|price| price.week).collect();
        assert_eq!(weeks, [day(2026, 10, 12), day(2026, 10, 26)]);
        assert_eq!(unit.last_price().unwrap().close, money("101.20"));
    }

    #[test]
    fn a_future_week_a_fetched_unit_and_a_gone_unit_are_refused_and_nothing_is_written() {
        let mut ports = InMemoryPorts::stub(Utc::now());
        let before = ports.clone();

        for (request, refusal) in [
            // Monday 2 November starts next week.
            (
                set(UnitId::SEED_VAS, day(2026, 11, 2), "1"),
                Error::FutureWeek,
            ),
            (set(UnitId::SEED_BTC, today(), "1"), Error::PricedBySource),
            (set(UnitId::new(), today(), "1"), Error::UnitGone),
        ] {
            assert_eq!(
                request.run(&mut ports.units, today(), at(2026, 10, 29)),
                Err(refusal)
            );
        }
        assert_eq!(ports, before);
    }

    #[test]
    fn a_later_day_of_the_current_week_is_not_a_future_week() {
        let mut ports = InMemoryPorts::stub(Utc::now());

        // Sunday 1 November ends the current week.
        let unit = set(UnitId::SEED_VAS, day(2026, 11, 1), "100")
            .run(&mut ports.units, today(), at(2026, 10, 29))
            .unwrap();

        assert_eq!(unit.prices[0].week, day(2026, 10, 26));
    }
}
