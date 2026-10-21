//! Where the Desktop's Units live: the SQLite adapter the app opens at start ([`open_storage`]), or
//! the in-memory ports the UI tests build `Shell` on. [`UnitsStore`] is what `build_shell` takes, so
//! either can stand behind the Units module.

use std::path::Path;
use std::sync::mpsc;

use chrono::Utc;
use lib_database::{UnitsAdapter, WriteFailure, Writer};
use lib_units::fake::InMemoryPorts;
use lib_units::{
    BaseUnitRepository, PortsMut, PriceSourceRepository, UnitRepository, UnitUsage, UnitsPorts,
};

/// The Units, Price Sources and base Unit ports, wherever they are kept. Usage and holdings are
/// other domains', so the Desktop answers them itself (see `ports.rs`).
pub trait UnitsStore {
    fn units(&self) -> &dyn UnitRepository;

    fn price_sources(&self) -> &dyn PriceSourceRepository;

    fn base_unit(&self) -> &dyn BaseUnitRepository;

    /// The three ports, with `usage`, for one Interactor call.
    fn split_mut<'a>(&'a mut self, usage: &'a dyn UnitUsage) -> PortsMut<'a>;

    /// Ends one Interactor call's writes: the adapter sends them to the writer as one transaction.
    fn commit(&mut self);

    /// Puts the rows back as they are stored, after a write failed (ADR-0046).
    ///
    /// # Errors
    /// Returns the error reading them back; the rows in memory are left as they were.
    fn reload(&mut self) -> lib_database::Result<()>;
}

impl UnitsStore for UnitsAdapter {
    fn units(&self) -> &dyn UnitRepository {
        UnitsAdapter::units(self)
    }

    fn price_sources(&self) -> &dyn PriceSourceRepository {
        UnitsAdapter::price_sources(self)
    }

    fn base_unit(&self) -> &dyn BaseUnitRepository {
        UnitsAdapter::base_unit(self)
    }

    fn split_mut<'a>(&'a mut self, usage: &'a dyn UnitUsage) -> PortsMut<'a> {
        UnitsAdapter::split_mut(self, usage)
    }

    fn commit(&mut self) {
        UnitsAdapter::commit(self);
    }

    fn reload(&mut self) -> lib_database::Result<()> {
        UnitsAdapter::reload(self)
    }
}

/// Memory is all there is, so a commit has nowhere to send and a reload nothing to read.
impl UnitsStore for InMemoryPorts {
    fn units(&self) -> &dyn UnitRepository {
        UnitsPorts::units(self)
    }

    fn price_sources(&self) -> &dyn PriceSourceRepository {
        UnitsPorts::price_sources(self)
    }

    fn base_unit(&self) -> &dyn BaseUnitRepository {
        UnitsPorts::base_unit(self)
    }

    fn split_mut<'a>(&'a mut self, usage: &'a dyn UnitUsage) -> PortsMut<'a> {
        PortsMut {
            units: &mut self.units,
            price_sources: &mut self.price_sources,
            base_unit: &mut self.base_unit,
            usage,
        }
    }

    fn commit(&mut self) {}

    fn reload(&mut self) -> lib_database::Result<()> {
        Ok(())
    }
}

/// The Ledger file as start-up opens it: the Units adapter, and the writer every adapter saves
/// through, with its failure reports.
pub struct Storage {
    pub units: UnitsAdapter,
    /// Flushed when the app quits, so the last writes land before the thread dies.
    pub writer: Writer,
    pub failures: mpsc::Receiver<WriteFailure>,
}

/// Opens the Ledger at `path`, creating and migrating it if need be, and loads the Units, blocking.
/// A Ledger with no base Unit is new, so it is seeded first with the stub's Units, Price Sources
/// and base Unit: the stub Accounts, Bill Plans and Budgets still name them (#746's first-launch
/// seed, removed once every domain persists).
///
/// # Errors
/// Returns the error opening, migrating, seeding or reading the file.
#[tracing::instrument(name = "Open the Ledger's storage", level = "debug", err)]
pub fn open_storage(path: &Path) -> lib_database::Result<Storage> {
    let (writer, failures) = Writer::open_ledger(path)?;
    let units = match UnitsAdapter::load(writer.clone()) {
        Err(lib_database::Error::NotFound(_)) => {
            let seed = InMemoryPorts::stub(Utc::now());
            UnitsAdapter::seed_from(&writer, seed)?;
            tracing::info!(path = %path.display(), "Seeded a new Ledger with the stub's Units");
            UnitsAdapter::load(writer.clone())?
        }
        loaded => loaded?,
    };
    Ok(Storage {
        units,
        writer,
        failures,
    })
}
