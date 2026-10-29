//! The contract every stored Units port set keeps (ADR-0046), run against both the in-memory
//! ports here and the `lib-database` adapter, so the fakes the Presenter tests use can be trusted.
//!
//! Each check is a function generic over a [`PortsFactory`]: a consumer runs it from its own test,
//! since an adapter's tests need their own set-up (a fresh database per test). Each drives the
//! Interactors, reloads as a restart would, and expects the same rows back.

use chrono::{NaiveDate, Utc};
use lib_core::UnitId;

use crate::fake::{InMemoryPorts, InMemoryUnitUsage};
use crate::interactor::{
    AddPriceSource, AddUnit, DeactivateUnit, DeletePriceSource, DeleteUnit, EditPriceSource,
    EditUnit, SetBaseUnit,
};
use crate::protocols::PortsMut;
use crate::{
    COINGECKO, Endpoint, Error, KeyPlacement, MANUAL_ENTRY, PriceLink, PriceSourceAuth,
    PriceSourceRecord, UnitKind, UnitRow, UnitUsage, Updates,
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

    /// Ends one Interactor call made on `ports`, sending its changes to storage as an adapter's
    /// owner does after each call.
    fn commit(&mut self, ports: &mut Self::Ports);

    /// Makes every write from now on fail, as a store that refuses it would.
    fn break_writes(&mut self, ports: &mut Self::Ports);

    /// Settles the writes sent so far, then returns `ports` reloaded to the last committed rows,
    /// with the number of failures reported naming the Units domain.
    fn recover(&mut self, ports: Self::Ports) -> (Self::Ports, usize);
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

/// The in-memory ports are their own storage, so a reload hands them back as they are. It keeps
/// a copy as of the last commit, which a failed write falls back to.
#[derive(Debug, Default)]
pub struct InMemoryFactory {
    committed: Option<InMemoryPorts>,
    broken: bool,
    failures: usize,
}

impl PortsFactory for InMemoryFactory {
    type Ports = InMemoryPorts;

    fn create(&mut self, seed: InMemoryPorts) -> InMemoryPorts {
        self.committed = Some(seed.clone());
        seed
    }

    fn reload(&mut self, mut ports: InMemoryPorts) -> InMemoryPorts {
        self.commit(&mut ports);
        ports
    }

    fn commit(&mut self, ports: &mut InMemoryPorts) {
        if self.broken {
            self.failures += 1;
        } else {
            self.committed = Some(ports.clone());
        }
    }

    fn break_writes(&mut self, _ports: &mut InMemoryPorts) {
        self.broken = true;
    }

    fn recover(&mut self, _ports: InMemoryPorts) -> (InMemoryPorts, usize) {
        let committed = self.committed.clone().expect("create ran first");
        (committed, std::mem::take(&mut self.failures))
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
        id: UnitId::new(),
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

/// Deleting a Unit other Units are quoted in is refused with how many, before any write reaches
/// storage, whose foreign key would refuse it: the stub's btc and vas are quoted in aud, the base.
pub fn deleting_a_quoted_unit_is_refused_and_writes_nothing<F: PortsFactory>(factory: &mut F) {
    let usage = InMemoryUnitUsage::default();
    let mut ports = factory.create(InMemoryPorts::stub(Utc::now()));
    let before = snapshot(&mut ports);

    let split = ports.split_mut(&usage);
    let refusal = DeleteUnit {
        id: UnitId::SEED_AUD,
        new_base: Some(UnitId::SEED_BTC),
    }
    .run(split.units, split.base_unit, split.usage)
    .expect_err("btc and vas are quoted in aud");
    assert_eq!(refusal, Error::QuotedBy(2));
    factory.commit(&mut ports);

    assert_eq!(
        snapshot(&mut ports),
        before,
        "the refused delete wrote nothing"
    );
    let mut ports = factory.reload(ports);
    assert_eq!(snapshot(&mut ports), before);
}

/// Adding or editing a Unit quoted in a Unit that isn't on the Ledger is refused before any write
/// reaches storage, whose foreign key would refuse it; a new Unit quoted in itself is added.
pub fn a_gone_quote_unit_is_refused_and_writes_nothing<F: PortsFactory>(factory: &mut F) {
    let usage = InMemoryUnitUsage::default();
    let mut ports = factory.create(InMemoryPorts::stub(Utc::now()));
    let before = snapshot(&mut ports);
    let gone = PriceLink::manual(UnitId::new());

    let split = ports.split_mut(&usage);
    let refusal = AddUnit {
        id: UnitId::new(),
        code: "eth".into(),
        name: "ETH".into(),
        kind: UnitKind::Crypto,
        link: gone.clone(),
    }
    .run(split.units, split.price_sources)
    .expect_err("the quote names no Unit");
    assert_eq!(refusal, Error::QuoteGone);
    factory.commit(&mut ports);

    let split = ports.split_mut(&usage);
    let refusal = EditUnit {
        id: UnitId::SEED_BTC,
        code: "btc".into(),
        name: "BTC".into(),
        kind: UnitKind::Crypto,
        link: gone,
    }
    .run(split.units, split.price_sources)
    .expect_err("the quote names no Unit");
    assert_eq!(refusal, Error::QuoteGone);
    factory.commit(&mut ports);

    assert_eq!(
        snapshot(&mut ports),
        before,
        "the refused saves wrote nothing"
    );
    let mut ports = factory.reload(ports);
    assert_eq!(snapshot(&mut ports), before);

    let id = UnitId::new();
    let split = ports.split_mut(&usage);
    let eth = AddUnit {
        id,
        code: "eth".into(),
        name: "ETH".into(),
        kind: UnitKind::Crypto,
        link: PriceLink::manual(id),
    }
    .run(split.units, split.price_sources)
    .expect("a Unit may quote itself");
    factory.commit(&mut ports);
    let mut ports = factory.reload(ports);
    assert!(snapshot(&mut ports).units.contains(&eth));
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

/// Weekly Prices reload the same, a close-only week and a full one alike; an edit in the same quote
/// keeps them in storage, a quote change clears them there, and a priced Unit deletes with them.
/// There is no Set price Interactor yet, so the prices are written through the Units port.
pub fn weekly_prices_reload_and_a_quote_change_clears_them<F: PortsFactory>(factory: &mut F) {
    let usage = InMemoryUnitUsage::default();
    let mut ports = factory.create(InMemoryPorts::stub(Utc::now()));
    let mut eth = add_unit(
        ports.split_mut(&usage),
        "eth",
        UnitKind::Crypto,
        PriceLink::manual(UnitId::SEED_AUD),
    );
    let day = |d: u32| NaiveDate::from_ymd_opt(2026, 10, d).expect("a real date");
    let at = |d: u32| day(d).and_hms_opt(9, 0, 0).expect("a real time").and_utc();
    eth.record_price("3900.00".parse().expect("a decimal"), day(20), at(20));
    eth.record_price("4000.00".parse().expect("a decimal"), day(27), at(27));
    eth.record_price("4100.25".parse().expect("a decimal"), day(28), at(28));
    ports.split_mut(&usage).units.upsert_unit(eth.clone());
    let before = snapshot(&mut ports);
    let mut ports = factory.reload(ports);
    assert_eq!(snapshot(&mut ports), before);

    let edit = |ports: &mut F::Ports, quote: UnitId| {
        let split = ports.split_mut(&usage);
        EditUnit {
            id: UnitId::SEED_BTC,
            code: "btc".into(),
            name: "Bitcoin".into(),
            kind: UnitKind::Crypto,
            link: PriceLink::manual(quote),
        }
        .run(split.units, split.price_sources)
        .expect("btc is on the stub")
    };
    let kept = edit(&mut ports, UnitId::SEED_AUD);
    assert_eq!(kept.prices.len(), 52, "a source change keeps them");
    let mut ports = factory.reload(ports);
    assert_eq!(unit(&mut ports, UnitId::SEED_BTC).prices, kept.prices);

    edit(&mut ports, eth.id);
    let split = ports.split_mut(&usage);
    DeleteUnit {
        id: eth.id,
        new_base: None,
    }
    .run(split.units, split.base_unit, split.usage)
    .expect_err("btc is quoted in eth now");
    edit(&mut ports, UnitId::SEED_AUD);
    let split = ports.split_mut(&usage);
    DeleteUnit {
        id: eth.id,
        new_base: None,
    }
    .run(split.units, split.base_unit, split.usage)
    .expect("eth is unused");
    let before = snapshot(&mut ports);
    let mut ports = factory.reload(ports);
    assert_eq!(snapshot(&mut ports), before);
    assert!(
        unit(&mut ports, UnitId::SEED_BTC).prices.is_empty(),
        "the quote change cleared them"
    );
}

fn unit(ports: &mut impl StoredPorts, id: UnitId) -> UnitRow {
    let usage = InMemoryUnitUsage::default();
    ports
        .split_mut(&usage)
        .units
        .unit(id)
        .expect("the Unit is stored")
}

/// A write that fails is reported once, naming the Units domain, and the ports go back to the last
/// committed rows rather than keeping the change the store refused.
pub fn a_failed_write_reloads_and_reports<F: PortsFactory>(factory: &mut F) {
    let usage = InMemoryUnitUsage::default();
    let mut ports = factory.create(InMemoryPorts::stub(Utc::now()));
    let stored = snapshot(&mut ports);

    factory.break_writes(&mut ports);
    add_unit(
        ports.split_mut(&usage),
        "eur",
        UnitKind::Currency,
        PriceLink::manual(UnitId::SEED_AUD),
    );
    factory.commit(&mut ports);

    let (mut ports, failures) = factory.recover(ports);
    assert_eq!(failures, 1, "one failure naming the Units domain");
    assert_eq!(snapshot(&mut ports), stored, "the refused write is gone");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn in_memory_written_rows_reload_the_same() {
        written_rows_reload_the_same(&mut InMemoryFactory::default());
    }

    #[test]
    fn in_memory_deleting_the_base_unit_with_a_move_lands_in_order() {
        deleting_the_base_unit_with_a_move_lands_in_order(&mut InMemoryFactory::default());
    }

    #[test]
    fn in_memory_deleting_a_quoted_unit_is_refused_and_writes_nothing() {
        deleting_a_quoted_unit_is_refused_and_writes_nothing(&mut InMemoryFactory::default());
    }

    #[test]
    fn in_memory_a_gone_quote_unit_is_refused_and_writes_nothing() {
        a_gone_quote_unit_is_refused_and_writes_nothing(&mut InMemoryFactory::default());
    }

    #[test]
    fn in_memory_deleting_a_custom_source_moves_its_units_to_manual_entry() {
        deleting_a_custom_source_moves_its_units_to_manual_entry(&mut InMemoryFactory::default());
    }

    #[test]
    fn in_memory_weekly_prices_reload_and_a_quote_change_clears_them() {
        weekly_prices_reload_and_a_quote_change_clears_them(&mut InMemoryFactory::default());
    }

    #[test]
    fn in_memory_a_failed_write_reloads_and_reports() {
        a_failed_write_reloads_and_reports(&mut InMemoryFactory::default());
    }
}
