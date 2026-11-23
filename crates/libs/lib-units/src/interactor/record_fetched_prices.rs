use chrono::{DateTime, NaiveDate, Utc};
use lib_core::{Money, PriceSourceId, UnitId};

use crate::fetching::{FetchFailure, Landed, PriceFetching, PriceResponse, Reading, read};
use crate::protocols::UnitRepository;

/// Write one round of fetched prices: each response's price into its Unit's week of the Clock's
/// day, through [`crate::UnitRow::record_price`] (ADR-0052's fetching amendment). The round is one
/// input, so the Client commits it once.
///
/// A response writes nothing for a Unit deleted, deactivated or relinked since its request went
/// out, for a request this Ledger never sent, and for an answer with no price in it. How each
/// Unit and the source fared is noted in [`PriceFetching`], never in the Ledger: a round failure
/// backs off the source, a Unit failure that Unit alone, and a success clears either.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordFetchedPrices {
    pub(crate) landed: Landed,
}

/// A price written for a Unit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FetchedPrice {
    pub unit: UnitId,
    pub close: Money,
}

/// How a round went, for the Toast a round a person asked for raises.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RoundOutcome {
    /// The source asked; `None` for a response for no round.
    pub source: Option<PriceSourceId>,
    pub written: Vec<FetchedPrice>,
    /// The Units the round failed for, and why, round failures included.
    pub failed: Vec<(UnitId, FetchFailure)>,
    /// The first round failure, which backed the source off.
    pub round_failure: Option<FetchFailure>,
}

impl RecordFetchedPrices {
    /// `today` is the Clock's day, which decides the week; `at` is when the prices are written.
    pub fn run(
        self,
        units: &mut dyn UnitRepository,
        fetching: &mut PriceFetching,
        today: NaiveDate,
        at: DateTime<Utc>,
    ) -> RoundOutcome {
        let source = self.landed.source;
        let mut outcome = RoundOutcome {
            source: source.as_ref().map(|(id, _)| *id),
            ..RoundOutcome::default()
        };
        for response in self.landed.responses {
            for settled in record(response, units, fetching, today, at) {
                match settled {
                    Settled::Written(price) => outcome.written.push(price),
                    Settled::Failed(unit, failure) => {
                        if failure.is_round() && outcome.round_failure.is_none() {
                            outcome.round_failure = Some(failure);
                        }
                        outcome.failed.push((unit, failure));
                    }
                }
            }
        }
        if let Some((id, settings)) = source {
            fetching.record_source(id, &settings, at, outcome.round_failure);
        }
        outcome
    }
}

/// What one response settled for its Unit.
enum Settled {
    Written(FetchedPrice),
    Failed(UnitId, FetchFailure),
}

/// What `response` settled for each Unit it asked for: none for no request, or for a Unit
/// changed since. A Unit missing from a batch reply reads Not found or No price.
fn record(
    response: PriceResponse,
    units: &mut dyn UnitRepository,
    fetching: &mut PriceFetching,
    today: NaiveDate,
    at: DateTime<Utc>,
) -> Vec<Settled> {
    let request = response.request.0;
    let Some(sent) = fetching.take(response.request) else {
        tracing::debug!(request, "Dropped a price response for no request in flight");
        return Vec::new();
    };
    let mut settled = Vec::with_capacity(sent.units.len());
    for call in sent.units {
        let unit_id = call.unit;
        let reading = read(&response.outcome, &call.price_path);
        let mut unit = match units.unit(unit_id) {
            Some(unit) if unit.is_active && unit.link == call.link => unit,
            _ => {
                tracing::info!(
                    %unit_id,
                    "Skipped a fetched price for a Unit deleted, deactivated or relinked since"
                );
                // A round failure still fails the source, whichever Unit it came back for.
                if let Reading::Failed(failure) = reading
                    && failure.is_round()
                {
                    settled.push(Settled::Failed(unit_id, failure));
                }
                continue;
            }
        };
        let close = match reading {
            Reading::Priced(close) => close,
            Reading::Failed(failure) => {
                tracing::warn!(%unit_id, request, ?failure, "A price fetch failed");
                fetching.record_unit(unit_id, call.link, &sent.settings, at, Some(failure));
                settled.push(Settled::Failed(unit_id, failure));
                continue;
            }
        };
        fetching.record_unit(unit_id, call.link, &sent.settings, at, None);
        unit.record_price(close.clone(), today, at);
        units.upsert_unit(unit);
        tracing::info!(%unit_id, "Recorded a fetched price");
        settled.push(Settled::Written(FetchedPrice {
            unit: unit_id,
            close,
        }));
    }
    settled
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use super::*;
    use crate::fake::{FakeTransport, InMemoryPorts};
    use crate::fetching::RequestId;
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
        fn record(&mut self, response: PriceResponse) -> Vec<FetchedPrice> {
            let landed = self
                .fetching
                .land(response, at())
                .expect("a round of one lands");
            RecordFetchedPrices { landed }
                .run(&mut self.ports.units, &mut self.fetching, today(), at())
                .written
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
}
