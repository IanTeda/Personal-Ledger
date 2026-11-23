//! The Settings module's ports over the stores (#1041): [`StoredSettingsReads`] answers the Units
//! Settings › Ledger reads from the Units store, and [`LedgerPreferencesAdapter`] writes the base
//! Unit, the Ledger's one Preference, through that store's write. `lib-units`' `SetBaseUnit` is
//! still the Units page's Set base unit Dialog's rule, so the write reuses its Interactor rather
//! than moving it here: a Unit that is gone or inactive is refused as it is there.

use lib_core::{FinancialYearStart, LedgerName, Owner, UnitId, UnitRef};
use lib_settings::{
    BaseUnitAnswer, HeadlineAmount, LedgerCounts, LedgerFigures, LedgerPreferences, LedgerReads,
    LedgerSummaries,
};
use lib_units::interactor::SetBaseUnit;
use lib_units::{UnitUse, units_by_code};

use super::store::{
    SharedAccountsStore, SharedCategoriesStore, SharedDocumentsStore, SharedInstitutionsStore,
    SharedInventoryStore, SharedLedgerStore, SharedPayeesStore, SharedStores, SharedTagsStore,
    SharedTransactionsStore, SharedUnitsStore, StoredTagReads,
};
use super::units;

/// `lib-settings`' read ports answered by the stores, each borrowed for the one read: the Units
/// the Ledger page offers, and the counts the Settings index's detail pane quotes (#1109), which
/// call the domains' own rules (net worth, likely duplicates, levels deep) and re-implement none.
#[derive(Clone)]
pub(crate) struct StoredSettingsReads {
    pub units: SharedUnitsStore,
    pub ledger: SharedLedgerStore,
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
            ledger: stores.ledger.clone(),
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
    fn figures(&self) -> LedgerFigures {
        LedgerFigures {
            accounts: self.accounts.borrow().accounts().account_count(),
            transactions: self
                .transactions
                .borrow()
                .transactions()
                .transaction_count(),
            units: self.units.borrow().units().unit_count(),
            institutions: self
                .institutions
                .borrow()
                .institutions()
                .institution_count(),
        }
    }

    fn counts(&self) -> LedgerCounts {
        let figures = self.figures();
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
        let likely_duplicate_tags =
            lib_tags::likely_duplicate_count(&lib_tags::duplicate_groups(&tags, &usage));
        let payees = self.payees.borrow().payees().payees();
        let types = self.documents.borrow().documents().document_types();
        let inventory = self.inventory.borrow().inventory().inventory();
        LedgerCounts {
            units: figures.units,
            base_unit: base.map(|unit| unit.code.clone()),
            institutions: figures.institutions,
            accounts: figures.accounts,
            transactions: figures.transactions,
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

impl LedgerReads for StoredSettingsReads {
    /// The active Units, by code.
    fn choices(&self) -> Vec<UnitRef> {
        let units = self.units.borrow().units().units();
        units::unit_refs(
            units_by_code(&units)
                .into_iter()
                .filter(|unit| unit.is_active),
        )
    }

    fn base_unit(&self) -> Option<UnitId> {
        Some(self.units.borrow().base_unit().base_unit())
    }

    fn name(&self) -> LedgerName {
        self.ledger.borrow().ledger().name.clone()
    }

    fn owner(&self) -> Option<Owner> {
        self.ledger.borrow().ledger().owner.clone()
    }

    fn financial_year_start(&self) -> FinancialYearStart {
        self.ledger.borrow().ledger().financial_year_start
    }
}

/// `lib-settings`' write port over the Units store the Units module also holds, for the base
/// Unit, and the Ledger store for the name, Owner and Financial year start. A write joins the
/// pending batch every adapter shares, so the bin's one commit after the input sends it.
pub(crate) struct LedgerPreferencesAdapter {
    units: SharedUnitsStore,
    ledger: SharedLedgerStore,
}

impl LedgerPreferencesAdapter {
    pub fn new(units: SharedUnitsStore, ledger: SharedLedgerStore) -> Self {
        Self { units, ledger }
    }
}

impl LedgerPreferences for LedgerPreferencesAdapter {
    fn set_base_unit(&self, id: UnitId) -> BaseUnitAnswer {
        let mut store = self.units.borrow_mut();
        // The usage `split_mut` lends beside the write ports goes unread: setting the base checks
        // only that the Unit exists and is active.
        let unread = UnitUse::default();
        store.split_write(&unread, |ports| {
            match SetBaseUnit(id).run(&*ports.units, ports.base_unit) {
                Ok(()) => BaseUnitAnswer::Saved,
                Err(_) => BaseUnitAnswer::Refused,
            }
        })
    }

    fn set_name(&self, name: LedgerName) {
        self.ledger.borrow_mut().set_name(name);
    }

    fn set_owner(&self, owner: Option<Owner>) {
        self.ledger.borrow_mut().set_owner(owner);
    }

    fn set_financial_year_start(&self, start: FinancialYearStart) {
        self.ledger.borrow_mut().set_financial_year_start(start);
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
        let mut shared = stores.share().stores;
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

    fn pair(
        units: SharedUnitsStore,
        ledger: SharedLedgerStore,
    ) -> (StoredSettingsReads, LedgerPreferencesAdapter) {
        let mut stores = sample_stores(units.clone());
        stores.ledger = ledger.clone();
        (
            StoredSettingsReads::over(&stores),
            LedgerPreferencesAdapter::new(units, ledger),
        )
    }

    fn sample_ledger() -> SharedLedgerStore {
        Rc::new(RefCell::new(crate::seed::sample_ledger_row()))
    }

    /// The in-memory Units `Stores::fake` holds.
    struct InMemoryFactory;

    impl LedgerFactory for InMemoryFactory {
        type Reads = StoredSettingsReads;
        type Writes = LedgerPreferencesAdapter;

        fn create(&mut self, deactivated: &[UnitId]) -> (Self::Reads, Self::Writes) {
            pair(
                Rc::new(RefCell::new(sample_units(deactivated))),
                sample_ledger(),
            )
        }
    }

    #[test]
    fn the_sample_ledgers_counts_are_the_domains_own() {
        let (stores, _) = crate::Stores::fake(crate::FakeStores::sample(today()));
        let shared = stores.share().stores;
        let counts = StoredSettingsReads::over(&shared).counts();
        let accounts = shared.accounts.borrow().accounts().accounts();
        let tags = shared.tags.borrow().tags().tags();
        let categories = shared.categories.borrow().categories().categories();
        assert_eq!(counts.accounts, accounts.len());
        let transactions = shared.transactions.borrow().transactions().transactions();
        assert!(!transactions.is_empty());
        assert_eq!(counts.transactions, transactions.len());
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
            // Seeding the Units wrote the ledger row with its base Unit, so the Preferences are
            // set on it as the Sample Ledger holds them.
            let row = crate::seed::sample_ledger_row();
            self.writer
                .run(move |pool| async move {
                    sqlx::query("UPDATE ledger SET name = ?, owner = ?, financial_year_start = ?")
                        .bind(row.name.as_str())
                        .bind(row.owner.as_ref().map(Owner::as_str))
                        .bind(row.financial_year_start.month())
                        .execute(&pool)
                        .await?;
                    Ok(())
                })
                .expect("the Ledger row is set");
            let units = UnitsAdapter::load(self.pending.clone()).expect("the Units load");
            let ledger = crate::LedgerAdapter::load(self.pending.clone()).expect("the row loads");
            pair(Rc::new(RefCell::new(units)), Rc::new(RefCell::new(ledger)))
        }
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn the_preferences_read_as_the_sample_holds_them(
        _: SqlitePoolOptions,
        options: SqliteConnectOptions,
    ) {
        contract::the_preferences_read_as_the_sample_holds_them(&mut AdapterFactory::new(options));
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn a_saved_preference_reads_back(_: SqlitePoolOptions, options: SqliteConnectOptions) {
        contract::a_saved_preference_reads_back(&mut AdapterFactory::new(options));
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn no_owner_clears_the_owner(_: SqlitePoolOptions, options: SqliteConnectOptions) {
        contract::no_owner_clears_the_owner(&mut AdapterFactory::new(options));
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn the_three_preferences_land_in_the_file(
        _: SqlitePoolOptions,
        options: SqliteConnectOptions,
    ) {
        let mut factory = AdapterFactory::new(options);
        let (_, writes) = factory.create(&[]);
        writes.set_name(LedgerName::new("Household").expect("a name"));
        writes.set_owner(None);
        writes.set_financial_year_start(FinancialYearStart::APRIL);
        // A commit sends them, so a fresh load sees the row as written.
        crate::Commit::commit(&factory.pending);
        factory
            .writer
            .run(|_| async { Ok(()) })
            .expect("the writer is idle");

        let fresh = crate::LedgerAdapter::load(factory.pending.clone()).expect("the row loads");
        assert_eq!(fresh.row().name.as_str(), "Household");
        assert_eq!(fresh.row().owner, None);
        assert_eq!(fresh.row().financial_year_start, FinancialYearStart::APRIL);
    }

    #[test]
    fn a_write_notes_the_ledger_domain() {
        let (stores, _) = crate::Stores::fake(crate::FakeStores::sample(today()));
        let shared = stores.share();
        let writes = LedgerPreferencesAdapter::new(
            shared.stores.units.clone(),
            shared.stores.ledger.clone(),
        );

        writes.set_owner(None);

        assert_eq!(shared.commit.commit().wrote, [crate::Domain::Ledger]);
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

    #[test]
    fn the_figures_are_the_four_matching_counts() {
        let (stores, _) = crate::Stores::fake(crate::FakeStores::sample(today()));
        let reads = StoredSettingsReads::over(&stores.share().stores);
        let (figures, counts) = (reads.figures(), reads.counts());
        assert!(figures.transactions > 0);
        assert_eq!(figures.accounts, counts.accounts);
        assert_eq!(figures.transactions, counts.transactions);
        assert_eq!(figures.units, counts.units);
        assert_eq!(figures.institutions, counts.institutions);
    }
}
