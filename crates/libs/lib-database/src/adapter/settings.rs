//! The Settings module's ports over the stores (#1041): [`StoredSettingsReads`] answers the Units
//! Settings › Ledger reads from the Units store, and [`LedgerPreferencesAdapter`] writes the base
//! Unit, the Ledger's one Preference, through that store's write. `lib-units`' `SetBaseUnit` is
//! still the Units page's Set base unit Dialog's rule, so the write reuses its Interactor rather
//! than moving it here: a Unit that is gone or inactive is refused as it is there.

use lib_core::UnitId;
use lib_settings::{BaseUnitAnswer, LedgerPreferences, LedgerUnits, UnitChoice};
use lib_units::interactor::SetBaseUnit;
use lib_units::{UnitUse, units_by_code};

use super::store::SharedUnitsStore;

/// `lib-settings`' read port answered by the Units store, borrowed for the one read.
#[derive(Clone)]
pub struct StoredSettingsReads {
    pub units: SharedUnitsStore,
}

impl LedgerUnits for StoredSettingsReads {
    /// The active Units, by code.
    fn choices(&self) -> Vec<UnitChoice> {
        let units = self.units.borrow().units().units();
        units_by_code(&units)
            .into_iter()
            .filter(|unit| unit.is_active)
            .map(|unit| UnitChoice {
                id: unit.id,
                code: unit.code.clone(),
                name: unit.name.clone(),
            })
            .collect()
    }

    fn base_unit(&self) -> Option<UnitId> {
        Some(self.units.borrow().base_unit().base_unit())
    }
}

/// `lib-settings`' write port over the Units store the Units module also holds. The write joins
/// the pending batch every adapter shares, so the bin's one commit after the input sends it.
pub struct LedgerPreferencesAdapter {
    units: SharedUnitsStore,
}

impl LedgerPreferencesAdapter {
    pub fn new(units: SharedUnitsStore) -> Self {
        Self { units }
    }
}

impl LedgerPreferences for LedgerPreferencesAdapter {
    fn set_base_unit(&self, id: UnitId) -> BaseUnitAnswer {
        let mut store = self.units.borrow_mut();
        // The usage `split_mut` lends beside the write ports goes unread: setting the base checks
        // only that the Unit exists and is active.
        let unread = UnitUse::default();
        let ports = store.split_mut(&unread);
        match SetBaseUnit(id).run(&*ports.units, ports.base_unit) {
            Ok(()) => BaseUnitAnswer::Saved,
            Err(_) => BaseUnitAnswer::Refused,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::rc::Rc;
    use std::sync::mpsc;

    use lib_settings::contract::{self, LedgerFactory};
    use lib_units::fake::InMemoryPorts;
    use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

    use super::*;
    use crate::writer::WriteFailure;
    use crate::{PendingWrites, UnitsAdapter, Writer};

    fn stub_units(deactivated: &[UnitId]) -> InMemoryPorts {
        let mut ports = InMemoryPorts::stub(chrono::Utc::now());
        for unit in &mut ports.units.units {
            if deactivated.contains(&unit.id) {
                unit.is_active = false;
            }
        }
        ports
    }

    fn pair(units: SharedUnitsStore) -> (StoredSettingsReads, LedgerPreferencesAdapter) {
        (
            StoredSettingsReads {
                units: units.clone(),
            },
            LedgerPreferencesAdapter::new(units),
        )
    }

    /// The in-memory Units `Stores::fake` holds.
    struct InMemoryFactory;

    impl LedgerFactory for InMemoryFactory {
        type Reads = StoredSettingsReads;
        type Writes = LedgerPreferencesAdapter;

        fn create(&mut self, deactivated: &[UnitId]) -> (Self::Reads, Self::Writes) {
            pair(Rc::new(RefCell::new(stub_units(deactivated))))
        }
    }

    #[test]
    fn the_in_memory_stores_keep_the_ledger_contract() {
        contract::run_all_ledger(&mut InMemoryFactory);
    }

    /// The Units adapter on one test database.
    struct AdapterFactory {
        writer: Writer,
        pending: PendingWrites,
        _failures: mpsc::Receiver<WriteFailure>,
    }

    impl AdapterFactory {
        fn new(options: SqliteConnectOptions) -> Self {
            let (writer, failures) = Writer::spawn(options).expect("the test database opens");
            Self {
                pending: PendingWrites::new(writer.clone()),
                writer,
                _failures: failures,
            }
        }
    }

    impl LedgerFactory for AdapterFactory {
        type Reads = StoredSettingsReads;
        type Writes = LedgerPreferencesAdapter;

        fn create(&mut self, deactivated: &[UnitId]) -> (Self::Reads, Self::Writes) {
            UnitsAdapter::seed_from(&self.writer, stub_units(deactivated))
                .expect("the Units are written");
            let units = UnitsAdapter::load(self.pending.clone()).expect("the Units load");
            pair(Rc::new(RefCell::new(units)))
        }
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn the_choices_are_the_active_units_by_code(
        _: SqlitePoolOptions,
        options: SqliteConnectOptions,
    ) {
        contract::the_choices_are_the_active_units_by_code(&mut AdapterFactory::new(options));
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn a_saved_base_unit_reads_back(_: SqlitePoolOptions, options: SqliteConnectOptions) {
        contract::a_saved_base_unit_reads_back(&mut AdapterFactory::new(options));
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn a_unit_that_is_gone_or_inactive_is_refused(
        _: SqlitePoolOptions,
        options: SqliteConnectOptions,
    ) {
        contract::a_unit_that_is_gone_or_inactive_is_refused(&mut AdapterFactory::new(options));
    }
}
