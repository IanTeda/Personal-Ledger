//! In-memory ports and the stub seed, for tests and UI-first work. Each holds its rows in a plain
//! field so a test can set up or read state directly. [`RecordingRouter`] is a Router that only
//! records what it was asked.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use chrono::{DateTime, Utc};
use lib_core::{PriceSourceId, UnitId};
use lib_toast::{Notice, Notify};

use crate::protocols::{PortsMut, SharedPorts, UnitsDialog, UnitsOutcome, UnitsPorts, UnitsRouter};
use crate::{
    BaseUnitRepository, Holding, PriceSourceRecord, PriceSourceRepository, UnitHoldings,
    UnitRepository, UnitRow, UnitUsage, UnitUse, default_units,
};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InMemoryUnits {
    pub units: Vec<UnitRow>,
}

impl UnitRepository for InMemoryUnits {
    fn units(&self) -> Vec<UnitRow> {
        self.units.clone()
    }

    fn upsert_unit(&mut self, unit: UnitRow) {
        match self.units.iter_mut().find(|row| row.id == unit.id) {
            Some(row) => *row = unit,
            None => self.units.push(unit),
        }
    }

    fn delete_unit(&mut self, id: UnitId) {
        self.units.retain(|row| row.id != id);
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InMemoryPriceSources {
    pub records: Vec<PriceSourceRecord>,
}

impl PriceSourceRepository for InMemoryPriceSources {
    fn price_source_records(&self) -> Vec<PriceSourceRecord> {
        self.records.clone()
    }

    fn upsert_price_source(&mut self, record: PriceSourceRecord) {
        match self.records.iter_mut().find(|row| row.id() == record.id()) {
            Some(row) => *row = record,
            None => self.records.push(record),
        }
    }

    fn delete_price_source(&mut self, id: PriceSourceId) {
        self.records.retain(|row| row.id() != id);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InMemoryBaseUnit(pub UnitId);

impl BaseUnitRepository for InMemoryBaseUnit {
    fn base_unit(&self) -> UnitId {
        self.0
    }

    fn set_base_unit(&mut self, id: UnitId) {
        self.0 = id;
    }
}

/// Usage counts a test sets per Unit; a Unit not listed is unused.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InMemoryUnitUsage {
    pub uses: HashMap<UnitId, UnitUse>,
}

impl UnitUsage for InMemoryUnitUsage {
    fn unit_usage(&self, id: UnitId) -> UnitUse {
        self.uses.get(&id).copied().unwrap_or_default()
    }
}

/// Holdings a test sets per Unit; a Unit not listed is held by no Account.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InMemoryHoldings {
    pub by_unit: HashMap<UnitId, Vec<Holding>>,
}

impl UnitHoldings for InMemoryHoldings {
    fn holdings(&self, id: UnitId) -> Vec<Holding> {
        self.by_unit.get(&id).cloned().unwrap_or_default()
    }
}

/// Every Units port in memory, together, so a test can hand an Interactor the ones it takes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InMemoryPorts {
    pub units: InMemoryUnits,
    pub price_sources: InMemoryPriceSources,
    pub base_unit: InMemoryBaseUnit,
    pub usage: InMemoryUnitUsage,
    pub holdings: InMemoryHoldings,
}

impl InMemoryPorts {
    /// The stub seed: the stub Ledger's Units (see [`default_units`]) with aud as the base, no
    /// stored Price Source rows, and nothing using any Unit. Ids match `lib-ledger`'s stub, so its
    /// Accounts, Bill Plans and Budgets still resolve their Units.
    pub fn stub(now: DateTime<Utc>) -> Self {
        Self {
            units: InMemoryUnits {
                units: default_units(now),
            },
            price_sources: InMemoryPriceSources::default(),
            base_unit: InMemoryBaseUnit(UnitId::SEED_AUD),
            usage: InMemoryUnitUsage::default(),
            holdings: InMemoryHoldings::default(),
        }
    }
}

impl InMemoryPorts {
    /// These ports as the Presenters share them, with the handle a test keeps to read them back.
    pub fn shared(self) -> (Rc<RefCell<Self>>, SharedPorts) {
        let ports = Rc::new(RefCell::new(self));
        let shared: SharedPorts = ports.clone();
        (ports, shared)
    }
}

impl UnitsPorts for InMemoryPorts {
    fn units(&self) -> &dyn UnitRepository {
        &self.units
    }

    fn price_sources(&self) -> &dyn PriceSourceRepository {
        &self.price_sources
    }

    fn base_unit(&self) -> &dyn BaseUnitRepository {
        &self.base_unit
    }

    fn usage(&self) -> &dyn UnitUsage {
        &self.usage
    }

    fn holdings(&self) -> &dyn UnitHoldings {
        &self.holdings
    }

    fn split_mut(&mut self) -> PortsMut<'_> {
        PortsMut {
            units: &mut self.units,
            price_sources: &mut self.price_sources,
            base_unit: &mut self.base_unit,
            usage: &self.usage,
        }
    }
}

/// One request a Presenter made of its Router.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RouterCall {
    OpenDialog(UnitsDialog),
    CloseDialog(Option<UnitsOutcome>),
    OpenPrices(UnitId),
    ClosePrices,
    Notice(Notice),
}

/// A Router that does nothing but record each call, in order.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RecordingRouter {
    pub calls: Vec<RouterCall>,
}

impl RecordingRouter {
    /// The calls made since the last `take`.
    pub fn take(&mut self) -> Vec<RouterCall> {
        std::mem::take(&mut self.calls)
    }
}

impl UnitsRouter for RecordingRouter {
    /// Always opens: the recording Router builds no Presenter that could fail.
    fn open_dialog(&mut self, dialog: UnitsDialog) -> bool {
        self.calls.push(RouterCall::OpenDialog(dialog));
        true
    }

    fn close_dialog(&mut self, outcome: Option<UnitsOutcome>) {
        self.calls.push(RouterCall::CloseDialog(outcome));
    }

    fn open_prices(&mut self, unit: UnitId) {
        self.calls.push(RouterCall::OpenPrices(unit));
    }

    fn close_prices(&mut self) {
        self.calls.push(RouterCall::ClosePrices);
    }
}

impl Notify for RecordingRouter {
    fn notify(&mut self, notice: Notice) {
        self.calls.push(RouterCall::Notice(notice));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{BuiltInOverride, COINGECKO, Precision, PriceLink, UnitKind, Updates};

    #[test]
    fn the_stub_seed_holds_aud_btc_and_vas_with_aud_as_base() {
        let ports = InMemoryPorts::stub(Utc::now());
        let codes: Vec<_> = ports
            .units
            .units()
            .into_iter()
            .map(|unit| unit.code)
            .collect();
        assert_eq!(codes, vec!["aud", "btc", "vas"]);
        assert_eq!(ports.base_unit.base_unit(), UnitId::SEED_AUD);
        assert!(ports.price_sources.price_source_records().is_empty());
        assert!(!ports.usage.unit_usage(UnitId::SEED_AUD).is_used());
    }

    #[test]
    fn upserting_a_unit_replaces_the_row_with_its_id_or_appends_it() {
        let mut units = InMemoryPorts::stub(Utc::now()).units;
        let mut btc = units.unit(UnitId::SEED_BTC).unwrap();
        btc.name = "Bitcoin core".into();
        units.upsert_unit(btc.clone());
        let eth = UnitRow::new(
            "eth".into(),
            "Ethereum".into(),
            UnitKind::Crypto,
            Precision::of_kind(UnitKind::Crypto),
            PriceLink::manual(UnitId::SEED_AUD),
        );
        units.upsert_unit(eth.clone());

        assert_eq!(units.units().len(), 4);
        assert_eq!(units.unit(UnitId::SEED_BTC), Some(btc));
        assert_eq!(units.unit(eth.id), Some(eth));

        units.delete_unit(UnitId::SEED_BTC);
        assert_eq!(units.unit(UnitId::SEED_BTC), None);
    }

    #[test]
    fn upserting_a_price_source_replaces_the_row_with_its_id_or_appends_it() {
        let mut sources = InMemoryPriceSources::default();
        let record = |name: &str| {
            PriceSourceRecord::BuiltIn(BuiltInOverride {
                id: COINGECKO,
                name: name.into(),
                updates: Updates::Daily,
            })
        };
        sources.upsert_price_source(record("Coins"));
        sources.upsert_price_source(record("Gecko"));
        assert_eq!(sources.price_source_records(), vec![record("Gecko")]);

        sources.delete_price_source(COINGECKO);
        assert!(sources.price_source_records().is_empty());
    }
}
