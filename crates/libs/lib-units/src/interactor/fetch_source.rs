use chrono::{DateTime, Utc};
use lib_core::PriceSourceId;

use crate::fetching::{
    FetchFailure, InFlight, PriceFetching, SourceSettings, key_missing, price_request,
};
use crate::protocols::UnitsPorts;
use crate::{Error, Result, units_by_code};

/// Ask one Price Source for the price of every Unit it values straight away (`GLOSSARY.md`'s Price
/// Source, fetch now), as one round a person asked for. Only sends: the prices are written in one
/// commit when the round's last response comes back, through
/// [`crate::interactor::RecordFetchedPrices`].
///
/// Ignores due-ness, back-off and Manual only, so it sends for every active Unit linked to the
/// source but the base Unit, which is never priced. A deactivated Unit and Manual entry are never
/// fetched.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FetchSource {
    pub id: PriceSourceId,
}

impl FetchSource {
    /// Returns how many requests were sent, `0` for a source no active Unit uses. Refused for a
    /// source that is gone or is Manual entry, and while the source has a round in flight, which
    /// sends nothing. A source needing a key this Client doesn't have sends nothing: its round
    /// and its Units fail Key missing at `now`, and the refusal says so.
    pub fn run(
        self,
        ports: &dyn UnitsPorts,
        fetching: &mut PriceFetching,
        now: DateTime<Utc>,
    ) -> Result<usize> {
        let outcome = self.apply(ports, fetching, now);
        match &outcome {
            Ok(sent) => {
                tracing::info!(price_source_id = %self.id, units = sent, "Fetching a Price Source's Units' prices");
            }
            Err(refusal) => {
                tracing::info!(price_source_id = %self.id, %refusal, "Refused to fetch a Price Source's Units' prices");
            }
        }
        outcome
    }

    fn apply(
        self,
        ports: &dyn UnitsPorts,
        fetching: &mut PriceFetching,
        now: DateTime<Utc>,
    ) -> Result<usize> {
        let catalogue = ports.source_catalogue();
        let source = catalogue.source(self.id).ok_or(Error::PriceSourceGone)?;
        let endpoint = source.endpoint().ok_or(Error::NotFetched)?;
        if fetching.source_in_flight(self.id) {
            return Err(Error::AlreadyFetching);
        }
        let base = ports.base_unit().base_unit();
        let units = ports.units().units();
        let settings = SourceSettings::of(&source);
        let wanted: Vec<_> = units_by_code(&units)
            .into_iter()
            .filter(|unit| unit.is_active && unit.id != base && unit.link.source_id == self.id)
            .collect();
        if wanted.is_empty() {
            return Ok(0);
        }
        let key = ports.keys().key(self.id);
        if key_missing(endpoint, key.as_ref()) {
            let failure = Some(FetchFailure::KeyMissing);
            fetching.record_source(self.id, &settings, now, failure);
            for unit in wanted {
                fetching.record_unit(unit.id, unit.link.clone(), &settings, now, failure);
            }
            return Err(Error::KeyMissing);
        }
        let mut requests = Vec::with_capacity(wanted.len());
        for unit in wanted {
            let Some(quote) = units.iter().find(|quote| quote.id == unit.link.quote) else {
                tracing::warn!(unit_id = %unit.id, "Skipped a Unit whose quote Unit is gone");
                continue;
            };
            let (request, price_path) =
                price_request(endpoint, &unit.link.symbol, &quote.code, key.clone());
            let sent = InFlight {
                unit: unit.id,
                link: unit.link.clone(),
                price_path,
                settings: settings.clone(),
            };
            requests.push((request, sent));
        }
        if requests.is_empty() {
            return Ok(0);
        }
        // Asked for, so its outcome is told.
        let round = fetching.open_round(&source, true);
        fetching.name_round(round);
        let count = requests.len();
        for (request, sent) in requests {
            fetching.send(round, request, sent);
        }
        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use chrono::Utc;
    use lib_core::UnitId;

    use super::*;
    use crate::fake::{FakeTransport, InMemoryPorts};
    use crate::{
        BuiltInOverride, COINGECKO, FRANKFURTER, MANUAL_ENTRY, Precision, PriceLink,
        PriceSourceRecord, UnitKind, UnitRow, Updates,
    };

    fn unit(code: &str, source_id: PriceSourceId) -> UnitRow {
        UnitRow::new(
            code.into(),
            code.into(),
            UnitKind::Currency,
            Precision::of_kind(UnitKind::Currency),
            PriceLink {
                source_id,
                symbol: code.into(),
                quote: UnitId::SEED_AUD,
            },
        )
    }

    struct Ask {
        ports: InMemoryPorts,
        transport: FakeTransport,
        fetching: PriceFetching,
    }

    fn on(units: Vec<UnitRow>) -> Ask {
        let mut ports = InMemoryPorts::sample(Utc::now());
        ports.units.units.extend(units);
        let transport = FakeTransport::default();
        let fetching = PriceFetching::new(Rc::new(transport.clone()));
        Ask {
            ports,
            transport,
            fetching,
        }
    }

    impl Ask {
        fn run(&mut self, id: PriceSourceId) -> Result<usize> {
            FetchSource { id }.run(&self.ports, &mut self.fetching, Utc::now())
        }
    }

    #[test]
    fn every_active_unit_on_the_source_is_sent_in_one_round_asked_for() {
        let mut ask = on(vec![
            unit("USD", FRANKFURTER),
            unit("EUR", FRANKFURTER),
            unit("ETH", COINGECKO),
        ]);

        assert_eq!(ask.run(FRANKFURTER), Ok(2));

        assert_eq!(
            ask.transport.urls(),
            [
                "https://api.frankfurter.dev/v2/rate/EUR/AUD",
                "https://api.frankfurter.dev/v2/rate/USD/AUD"
            ]
        );
        assert!(ask.fetching.source_asked_for(FRANKFURTER));
        assert!(!ask.fetching.source_asked_for(COINGECKO));
    }

    #[test]
    fn a_fresh_price_a_failing_unit_and_manual_only_are_all_ignored() {
        let mut eur = unit("EUR", FRANKFURTER);
        eur.record_price(
            "1.65".parse().expect("a decimal"),
            chrono::Local::now().date_naive(),
            Utc::now(),
        );
        let mut ask = on(vec![eur]);
        ask.ports
            .price_sources
            .records
            .push(PriceSourceRecord::BuiltIn(BuiltInOverride {
                id: FRANKFURTER,
                name: "Frankfurter".into(),
                updates: Updates::ManualOnly,
            }));

        assert_eq!(ask.run(FRANKFURTER), Ok(1));
    }

    #[test]
    fn a_deactivated_unit_is_left_out_and_a_source_nothing_uses_sends_nothing() {
        let mut eur = unit("EUR", FRANKFURTER);
        eur.is_active = false;
        let mut ask = on(vec![eur]);

        assert_eq!(ask.run(FRANKFURTER), Ok(0));
        assert!(ask.transport.requests().is_empty());
        assert!(!ask.fetching.source_asked_for(FRANKFURTER));
    }

    #[test]
    fn manual_entry_and_a_gone_source_are_refused() {
        let mut ask = on(vec![]);

        assert_eq!(ask.run(MANUAL_ENTRY), Err(Error::NotFetched));
        assert_eq!(ask.run(PriceSourceId::new()), Err(Error::PriceSourceGone));
    }

    #[test]
    fn a_second_ask_while_the_round_is_out_sends_nothing() {
        let mut ask = on(vec![unit("EUR", FRANKFURTER)]);
        assert_eq!(ask.run(FRANKFURTER), Ok(1));

        assert_eq!(ask.run(FRANKFURTER), Err(Error::AlreadyFetching));

        assert_eq!(ask.transport.requests().len(), 1);
    }
}
