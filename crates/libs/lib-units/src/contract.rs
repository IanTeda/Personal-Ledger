//! The contract every stored Units port set keeps (ADR-0046), run against both the in-memory
//! ports here and the `lib-database` adapter, so the fakes the Presenter tests use can be trusted.
//!
//! Each check is a function generic over a [`PortsFactory`]: a consumer runs it from its own test,
//! since an adapter's tests need their own set-up (a fresh database per test). Each drives the
//! Interactors, reloads as a restart would, and expects the same rows back.

use chrono::Utc;
use lib_core::UnitId;

use crate::fake::{InMemoryPorts, InMemoryUnitUsage};
use crate::interactor::{
    AddPriceSource, AddUnit, DeactivateUnit, DeletePriceSource, DeleteUnit, EditPriceSource,
    EditUnit, SetBaseUnit,
};
use crate::protocols::PortsMut;
use crate::{
    COINGECKO, Endpoint, KeyPlacement, MANUAL_ENTRY, PriceLink, PriceSourceAuth, PriceSourceRecord,
    UnitKind, UnitRow, UnitUsage, Updates,
};

/// A stored set of the Units, Price Sources and base Unit ports, lent out together for one
/// Interactor call. Usage is another domain's, so the caller brings it.
pub trait StoredPorts {
    fn split_mut<'a>(&'a mut self, usage: &'a dyn UnitUsage) -> PortsMut<'a>;
}

/// Builds the ports a contract check runs on.
pub trait PortsFactory {
    type Ports: StoredPorts;

    /// Ports holding `seed`'s Units, Price Source rows and base Unit, as if stored and loaded.
    fn create(&mut self, seed: InMemoryPorts) -> Self::Ports;

    /// Ends the Interactor calls made on `ports`, then loads a fresh set from wherever they were
    /// stored, as a restart would.
    fn reload(&mut self, ports: Self::Ports) -> Self::Ports;
}

impl StoredPorts for InMemoryPorts {
    fn split_mut<'a>(&'a mut self, usage: &'a dyn UnitUsage) -> PortsMut<'a> {
        PortsMut {
            units: &mut self.units,
            price_sources: &mut self.price_sources,
            base_unit: &mut self.base_unit,
            usage,
        }
    }
}

/// The in-memory ports are their own storage, so a reload hands them back as they are.
#[derive(Debug, Default)]
pub struct InMemoryFactory;

impl PortsFactory for InMemoryFactory {
    type Ports = InMemoryPorts;

    fn create(&mut self, seed: InMemoryPorts) -> InMemoryPorts {
        seed
    }

    fn reload(&mut self, ports: InMemoryPorts) -> InMemoryPorts {
        ports
    }
}

/// What a port set holds, in a fixed order so two sets compare whatever order they list in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Snapshot {
    pub units: Vec<UnitRow>,
    pub records: Vec<PriceSourceRecord>,
    pub base_unit: UnitId,
}

pub fn snapshot(ports: &mut impl StoredPorts) -> Snapshot {
    let usage = InMemoryUnitUsage::default();
    let ports = ports.split_mut(&usage);
    let mut units = ports.units.units();
    units.sort_by_key(|unit| unit.id);
    let mut records = ports.price_sources.price_source_records();
    records.sort_by_key(PriceSourceRecord::id);
    Snapshot {
        units,
        records,
        base_unit: ports.base_unit.base_unit(),
    }
}

/// A Custom source whose key travels in a header, so its auth round-trips too.
fn custom_source(name: &str) -> AddPriceSource {
    AddPriceSource {
        name: name.into(),
        endpoint: Endpoint {
            address: "https://prices.example.com".into(),
            request: "/quote/{symbol}?in={quote}".into(),
            price_path: "/price".into(),
            auth: PriceSourceAuth::ApiKey {
                placement: KeyPlacement::Header,
                name: "X-Api-Key".into(),
            },
        },
        updates: Updates::Hourly,
    }
}

fn add_unit(ports: PortsMut<'_>, code: &str, kind: UnitKind, link: PriceLink) -> UnitRow {
    AddUnit {
        code: code.into(),
        name: code.to_uppercase(),
        kind,
        link,
    }
    .run(ports.units, ports.price_sources)
    .expect("the unit's price source exists")
}

/// Writes every kind of row through the Interactors, reloads, and gets the same rows: added,
/// edited and deactivated Units, a moved base, a Custom source and a built-in's override.
pub fn written_rows_reload_the_same<F: PortsFactory>(factory: &mut F) {
    let usage = InMemoryUnitUsage::default();
    let mut ports = factory.create(InMemoryPorts::stub(Utc::now()));

    let source = custom_source("Example prices").run(ports.split_mut(&usage).price_sources);
    let eur = add_unit(
        ports.split_mut(&usage),
        "eur",
        UnitKind::Currency,
        PriceLink {
            source_id: source.id,
            symbol: "EUR".into(),
            quote: UnitId::SEED_AUD,
        },
    );
    let split = ports.split_mut(&usage);
    EditUnit {
        id: UnitId::SEED_VAS,
        code: "vas".into(),
        name: "Vanguard Australian Shares".into(),
        kind: UnitKind::Equity,
        link: PriceLink::manual(eur.id),
    }
    .run(split.units, split.price_sources)
    .expect("vas is on the stub");
    let split = ports.split_mut(&usage);
    DeactivateUnit(UnitId::SEED_BTC)
        .run(split.units, split.base_unit)
        .expect("btc is not the base");
    let split = ports.split_mut(&usage);
    SetBaseUnit(eur.id)
        .run(split.units, split.base_unit)
        .expect("eur is active");
    EditPriceSource {
        id: COINGECKO,
        name: "Coins".into(),
        updates: Updates::Daily,
        endpoint: None,
    }
    .run(ports.split_mut(&usage).price_sources)
    .expect("CoinGecko is a built-in");

    let before = snapshot(&mut ports);
    let mut ports = factory.reload(ports);
    assert_eq!(snapshot(&mut ports), before);
    assert_eq!(before.base_unit, eur.id);
    assert_eq!(
        before.records.len(),
        2,
        "the override and the Custom source"
    );
}

/// Deleting the base Unit moves the base first, so storage that checks the base names a Unit
/// accepts the delete.
pub fn deleting_the_base_unit_with_a_move_lands_in_order<F: PortsFactory>(factory: &mut F) {
    let usage = InMemoryUnitUsage::default();
    let mut ports = factory.create(InMemoryPorts::stub(Utc::now()));
    // A base nothing quotes, so the delete only has the base to move.
    let usd = add_unit(
        ports.split_mut(&usage),
        "usd",
        UnitKind::Currency,
        PriceLink::manual(UnitId::SEED_AUD),
    );
    let split = ports.split_mut(&usage);
    SetBaseUnit(usd.id)
        .run(split.units, split.base_unit)
        .expect("usd is active");
    let mut ports = factory.reload(ports);

    let split = ports.split_mut(&usage);
    DeleteUnit {
        id: usd.id,
        new_base: Some(UnitId::SEED_AUD),
    }
    .run(split.units, split.base_unit, split.usage)
    .expect("usd is unused and aud is active");
    let before = snapshot(&mut ports);
    let mut ports = factory.reload(ports);

    let after = snapshot(&mut ports);
    assert_eq!(after, before);
    assert_eq!(after.base_unit, UnitId::SEED_AUD);
    assert!(after.units.iter().all(|unit| unit.id != usd.id));
}

/// Deleting a Custom source moves its Units to Manual entry, and they stay there after a reload.
pub fn deleting_a_custom_source_moves_its_units_to_manual_entry<F: PortsFactory>(factory: &mut F) {
    let usage = InMemoryUnitUsage::default();
    let mut ports = factory.create(InMemoryPorts::stub(Utc::now()));
    let source = custom_source("Doomed prices").run(ports.split_mut(&usage).price_sources);
    let gold = add_unit(
        ports.split_mut(&usage),
        "xau",
        UnitKind::Commodity,
        PriceLink {
            source_id: source.id,
            symbol: "gold".into(),
            quote: UnitId::SEED_AUD,
        },
    );
    let mut ports = factory.reload(ports);

    let split = ports.split_mut(&usage);
    let deleted = DeletePriceSource(source.id)
        .run(split.price_sources, split.units)
        .expect("the source is Custom");
    assert_eq!(deleted.moved, 1);
    let before = snapshot(&mut ports);
    let mut ports = factory.reload(ports);

    let after = snapshot(&mut ports);
    assert_eq!(after, before);
    assert!(after.records.is_empty());
    let gold = after
        .units
        .iter()
        .find(|unit| unit.id == gold.id)
        .expect("the Unit stays");
    assert_eq!(gold.link.source_id, MANUAL_ENTRY);
    assert_eq!(gold.link.symbol, "gold", "a moved Unit keeps its symbol");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn in_memory_written_rows_reload_the_same() {
        written_rows_reload_the_same(&mut InMemoryFactory);
    }

    #[test]
    fn in_memory_deleting_the_base_unit_with_a_move_lands_in_order() {
        deleting_the_base_unit_with_a_move_lands_in_order(&mut InMemoryFactory);
    }

    #[test]
    fn in_memory_deleting_a_custom_source_moves_its_units_to_manual_entry() {
        deleting_a_custom_source_moves_its_units_to_manual_entry(&mut InMemoryFactory);
    }
}
