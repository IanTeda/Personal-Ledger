use chrono::{DateTime, NaiveDate, Utc};

use crate::fetching::FetchedPrice;
use crate::protocols::UnitRepository;

/// Write one round of fetched prices, as [`crate::fetching::PriceFetching::land`] handed them
/// back: each price into its Unit's week of the Clock's day, through
/// [`crate::UnitRow::record_price`] (ADR-0052's fetching amendment). The round is one input, so
/// the Client commits it once.
///
/// Fetching already left out a Unit deleted, deactivated or relinked since its request went out;
/// the same check here is a safety net, so a price never lands on a Unit that changed meanwhile.
/// How each Unit and the source fared is fetching's to note, never the Ledger's.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordFetchedPrices {
    pub(crate) prices: Vec<FetchedPrice>,
    /// The Clock's day, which decides the week.
    pub(crate) today: NaiveDate,
    /// When the prices are written.
    pub(crate) at: DateTime<Utc>,
}

impl RecordFetchedPrices {
    /// Returns how many prices were written.
    pub fn run(self, units: &mut dyn UnitRepository) -> usize {
        let mut written = 0;
        for price in self.prices {
            let unit_id = price.unit;
            let mut unit = match units.unit(unit_id) {
                Some(unit) if unit.is_active && unit.link == price.link => unit,
                _ => {
                    tracing::info!(
                        %unit_id,
                        "Skipped a fetched price for a Unit deleted, deactivated or relinked since"
                    );
                    continue;
                }
            };
            unit.record_price(price.close, self.today, self.at);
            units.upsert_unit(unit);
            tracing::info!(%unit_id, "Recorded a fetched price");
            written += 1;
        }
        written
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use lib_core::{Money, UnitId};

    use super::*;
    use crate::fake::{FakeTransport, InMemoryPorts, InMemoryUnits};
    use crate::fetching::{Landing, PriceFetching, PriceResponse, RequestId};
    use crate::interactor::FetchNow;
    use crate::{FRANKFURTER, Precision, PriceLink, UnitKind, UnitRow, WeeklyPrice};

    const BODY: &str = r#"{"date":"2026-10-21","base":"EUR","quote":"AUD","rate":1.6512}"#;

    fn money(text: &str) -> Money {
        text.parse().expect("a decimal")
    }

    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 10, 21).expect("a real date")
    }

    fn at() -> DateTime<Utc> {
        today().and_hms_opt(9, 0, 0).expect("a real time").and_utc()
    }

    struct Round {
        ports: InMemoryPorts,
        transport: FakeTransport,
        fetching: PriceFetching,
        eur: UnitId,
    }

    /// The sample with EUR on Frankfurter, its price asked for.
    fn sent() -> Round {
        let mut ports = InMemoryPorts::sample(at());
        let unit = UnitRow::new(
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
        let eur = unit.id;
        ports.units.units.push(unit);
        let transport = FakeTransport::default();
        let mut fetching = PriceFetching::new(Rc::new(transport.clone()));
        FetchNow { id: eur }
            .run(&ports, &mut fetching, at())
            .expect("sent");
        Round {
            ports,
            transport,
            fetching,
            eur,
        }
    }

    impl Round {
        /// Lands `response` and writes the prices it hands back, returning them.
        fn record(&mut self, response: PriceResponse) -> Vec<FetchedPrice> {
            let landing = self
                .fetching
                .land(response, at(), today(), &Utc, &self.ports);
            let Landing::Prices { prices, .. } = landing else {
                return Vec::new();
            };
            let written = RecordFetchedPrices {
                prices: prices.clone(),
                today: today(),
                at: at(),
            }
            .run(&mut self.ports.units);
            assert_eq!(
                written,
                prices.len(),
                "every price fetching landed is written"
            );
            prices
        }

        fn prices(&self) -> Vec<WeeklyPrice> {
            self.ports.units.unit(self.eur).expect("eur").prices
        }
    }

    #[test]
    fn an_answer_writes_this_weeks_close_and_lands_the_request() {
        let mut round = sent();
        let response = round.transport.answer_last(200, BODY);

        let written = round.record(response);

        assert_eq!(
            written,
            [FetchedPrice {
                unit: round.eur,
                link: round.ports.units.unit(round.eur).expect("eur").link,
                close: money("1.6512")
            }]
        );
        let week = WeeklyPrice::week_of(today());
        assert_eq!(
            round.prices(),
            [WeeklyPrice {
                week,
                close: money("1.6512"),
                open: None,
                high: None,
                low: None,
                written_at: at(),
            }]
        );
        assert_eq!(round.fetching.in_flight(), 0);
    }

    #[test]
    fn a_second_fetch_overwrites_the_provisional_close_and_widens_high_and_low() {
        let mut round = sent();
        let first = round.transport.answer_last(200, BODY);
        round.record(first);
        FetchNow { id: round.eur }
            .run(&round.ports, &mut round.fetching, at())
            .expect("sent");
        let second = round.transport.answer_last(200, r#"{"rate":1.7}"#);

        round.record(second);

        let prices = round.prices();
        assert_eq!(prices.len(), 1);
        assert_eq!(prices[0].close, money("1.7"));
        assert_eq!(prices[0].high, Some(money("1.7")));
        assert_eq!(prices[0].low, Some(money("1.6512")));
    }

    #[test]
    fn a_unit_deleted_deactivated_or_relinked_since_writes_nothing() {
        let changes: [fn(&mut InMemoryPorts, UnitId); 3] = [
            |ports, id| ports.units.units.retain(|unit| unit.id != id),
            |ports, id| {
                let unit = ports.units.units.iter_mut().find(|unit| unit.id == id);
                unit.expect("eur").is_active = false;
            },
            |ports, id| {
                let unit = ports.units.units.iter_mut().find(|unit| unit.id == id);
                unit.expect("eur").link.symbol = "USD".into();
            },
        ];
        for change in changes {
            let mut round = sent();
            change(&mut round.ports, round.eur);
            let response = round.transport.answer_last(200, BODY);

            assert!(round.record(response).is_empty());
            assert!(
                round
                    .ports
                    .units
                    .unit(round.eur)
                    .is_none_or(|unit| unit.prices.is_empty())
            );
            assert_eq!(round.fetching.in_flight(), 0);
        }
    }

    #[test]
    fn a_failure_an_error_status_no_price_or_an_unknown_request_writes_nothing() {
        let mut round = sent();
        let failed = round.transport.answer(
            RequestId(0),
            crate::TransportOutcome::Failed(crate::fetching::TransportError::Offline),
        );
        assert!(round.record(failed).is_empty());

        for (status, body) in [(404, BODY), (200, r#"{"rates":{}}"#)] {
            FetchNow { id: round.eur }
                .run(&round.ports, &mut round.fetching, at())
                .expect("sent");
            let response = round.transport.answer_last(status, body);
            assert!(round.record(response).is_empty());
        }
        let stray = round.transport.answer(
            RequestId(99),
            crate::TransportOutcome::Answered {
                status: 200,
                body: BODY.into(),
                retry_after: None,
            },
        );
        assert!(round.record(stray).is_empty());
        assert!(round.prices().is_empty());
        assert_eq!(round.fetching.in_flight(), 0);
    }

    #[test]
    fn a_price_for_a_unit_changed_since_it_landed_is_not_written() {
        let round = sent();
        let eur = round.ports.units.unit(round.eur).expect("eur");
        let price = FetchedPrice {
            unit: eur.id,
            link: eur.link.clone(),
            close: money("1.6"),
        };
        let mut relinked = eur.clone();
        relinked.link.symbol = "USD".into();
        let mut inactive = eur.clone();
        inactive.is_active = false;

        for unit in [Some(relinked), Some(inactive), None] {
            let mut units = InMemoryUnits {
                units: unit.into_iter().collect(),
            };
            let written = RecordFetchedPrices {
                prices: vec![price.clone()],
                today: today(),
                at: at(),
            }
            .run(&mut units);

            assert_eq!(written, 0);
            assert!(units.units.iter().all(|unit| unit.prices.is_empty()));
        }
    }
}
