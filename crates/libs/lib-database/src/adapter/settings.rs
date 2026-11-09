//! The Settings module's ports over the stores (#1041): [`StoredSettingsReads`] answers the Units
//! Settings › Ledger reads from the Units store, and [`LedgerPreferencesAdapter`] writes the base
//! Unit, the Ledger's one Preference, through that store's write. `lib-units`' `SetBaseUnit` is
//! still the Units page's Set base unit Dialog's rule, so the write reuses its Interactor rather
//! than moving it here: a Unit that is gone or inactive is refused as it is there.

use lib_core::UnitId;
use lib_settings::{
    BaseUnitAnswer, HeadlineAmount, LedgerCounts, LedgerPreferences, LedgerSummaries, LedgerUnits,
    UnitChoice,
};
use lib_units::interactor::SetBaseUnit;
use lib_units::{UnitUse, units_by_code};

use super::store::{
    SharedAccountsStore, SharedCategoriesStore, SharedDocumentsStore, SharedInstitutionsStore,
    SharedInventoryStore, SharedPayeesStore, SharedStores, SharedTagsStore,
    SharedTransactionsStore, SharedUnitsStore, StoredTagReads,
};

/// `lib-settings`' read ports answered by the stores, each borrowed for the one read: the Units
/// the Ledger page offers, and the counts the Settings index's detail pane quotes (#1109), which
/// call the domains' own rules (net worth, likely duplicates, levels deep) and re-implement none.
#[derive(Clone)]
pub struct StoredSettingsReads {
    pub units: SharedUnitsStore,
    pub institutions: SharedInstitutionsStore,
    pub accounts: SharedAccountsStore,
    pub categories: SharedCategoriesStore,
    pub tags: SharedTagsStore,
    pub payees: SharedPayeesStore,
    pub transactions: SharedTransactionsStore,
    pub inventory: SharedInventoryStore,
    pub documents: SharedDocumentsStore,
}

impl StoredSettingsReads {
    /// The reads over the shared stores a Client holds.
    pub fn over(stores: &SharedStores) -> Self {
        Self {
            units: stores.units.clone(),
            institutions: stores.institutions.clone(),
            accounts: stores.accounts.clone(),
            categories: stores.categories.clone(),
            tags: stores.tags.clone(),
            payees: stores.payees.clone(),
            transactions: stores.transactions.clone(),
            inventory: stores.inventory.clone(),
            documents: stores.documents.clone(),
        }
    }
}

impl LedgerSummaries for StoredSettingsReads {
    fn counts(&self) -> LedgerCounts {
        let (units, base_id) = {
            let store = self.units.borrow();
            (store.units().units(), store.base_unit().base_unit())
        };
        let base = units.iter().find(|unit| unit.id == base_id);
        let accounts = self.accounts.borrow().accounts().accounts();
        let net_worth = base.map(|unit| {
            let net = lib_accounts::net_worth(&accounts, Some(unit.id));
            HeadlineAmount {
                amount: net.base_net,
                places: unit.precision.places(),
                code: unit.code.clone(),
            }
        });
        let categories = self.categories.borrow().categories().categories();
        let tags = self.tags.borrow().tags().tags();
        let usage = StoredTagReads {
            transactions: self.transactions.clone(),
            accounts: self.accounts.clone(),
            units: self.units.clone(),
        };
        let likely_duplicate_tags = lib_tags::duplicate_groups(&tags, &usage)
            .iter()
            .map(|group| group.duplicates.len())
            .sum();
        let payees = self.payees.borrow().payees().payees();
        let types = self.documents.borrow().documents().document_types();
        let inventory = self.inventory.borrow().inventory().inventory();
        LedgerCounts {
            units: units.len(),
            base_unit: base.map(|unit| unit.code.clone()),
            institutions: self
                .institutions
                .borrow()
                .institutions()
                .institutions()
                .len(),
            accounts: accounts.len(),
            net_worth,
            categories: categories.len(),
            category_levels: lib_categories::levels_deep(&categories),
            tags: lib_tags::active_count(&tags),
            likely_duplicate_tags,
            payees: lib_payees::active_count(&payees),
            payees_without_default_category: lib_payees::without_default_category_count(&payees),
            document_types: types.len(),
            files_typed: lib_documents::types::typed_files(&types) as usize,
            files: lib_documents::types::all_files(&types) as usize,
            locations: inventory.location_count(),
            areas: inventory.area_count(),
            items: inventory.item_count(),
        }
    }
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

    fn today() -> chrono::NaiveDate {
        chrono::NaiveDate::from_ymd_opt(2026, 9, 19).expect("a real date")
    }

    /// The Sample Ledger's stores in memory, with the Units store swapped for `units`.
    fn sample_stores(units: SharedUnitsStore) -> SharedStores {
        let (stores, _) = crate::Stores::fake(crate::FakeStores::sample(today()));
        let mut shared = crate::LedgerPorts::new(stores, lib_core::Clock::fixed(today())).stores;
        shared.units = units;
        shared
    }

    fn sample_units(deactivated: &[UnitId]) -> InMemoryPorts {
        let mut ports = InMemoryPorts::sample(chrono::Utc::now());
        for unit in &mut ports.units.units {
            if deactivated.contains(&unit.id) {
                unit.is_active = false;
            }
        }
        ports
    }

    fn pair(units: SharedUnitsStore) -> (StoredSettingsReads, LedgerPreferencesAdapter) {
        (
            StoredSettingsReads::over(&sample_stores(units.clone())),
            LedgerPreferencesAdapter::new(units),
        )
    }

    /// The in-memory Units `Stores::fake` holds.
    struct InMemoryFactory;

    impl LedgerFactory for InMemoryFactory {
        type Reads = StoredSettingsReads;
        type Writes = LedgerPreferencesAdapter;

        fn create(&mut self, deactivated: &[UnitId]) -> (Self::Reads, Self::Writes) {
            pair(Rc::new(RefCell::new(sample_units(deactivated))))
        }
    }

    #[test]
    fn the_sample_ledgers_counts_are_the_domains_own() {
        let (stores, _) = crate::Stores::fake(crate::FakeStores::sample(today()));
        let shared = crate::LedgerPorts::new(stores, lib_core::Clock::fixed(today())).stores;
        let counts = StoredSettingsReads::over(&shared).counts();
        let accounts = shared.accounts.borrow().accounts().accounts();
        let tags = shared.tags.borrow().tags().tags();
        let categories = shared.categories.borrow().categories().categories();
        assert_eq!(counts.accounts, accounts.len());
        assert_eq!(counts.tags, lib_tags::active_count(&tags));
        assert_eq!(counts.categories, categories.len());
        assert_eq!(
            counts.category_levels,
            lib_categories::levels_deep(&categories)
        );
        assert_eq!(counts.base_unit.as_deref(), Some("aud"));
        let worth = counts.net_worth.expect("the base Unit is known");
        assert_eq!(worth.code, "aud");
        assert_eq!(
            worth.amount,
            lib_accounts::net_worth(&accounts, Some(UnitId::SEED_AUD)).base_net
        );
        assert!(counts.units > 0 && counts.institutions > 0 && counts.payees > 0);
        assert!(counts.locations > 0 && counts.items > 0 && counts.document_types > 0);
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
            UnitsAdapter::seed_from(&self.writer, sample_units(deactivated))
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
