//! `lib-units`' ports answered over the stores: the Units, Price Sources and base Unit from the
//! Units store, and what names each Unit and the Accounts held in it from the Accounts,
//! Transactions, Bills, Budgets, Inventory and Institutions stores through [`StoredUnitReads`]
//! (#993, #1084), which also answers when a Unit's Accounts were first opened, its history's
//! floor (#1394). The Price Source API keys are opened from their sealed rows with the device's
//! API key secret ([`StoredPriceSourceKeys`]).

use lib_core::{PriceSourceId, UnitId};
use lib_units::{
    BaseUnitRepository, KeyState, PortsMut, PriceSourceKeys, PriceSourceRecord,
    PriceSourceRepository, UnitHoldings, UnitOpenings, UnitRepository, UnitRow, UnitUsage,
    UnitsPorts,
};
use secrecy::SecretString;

use crate::adapter::store::{SharedPriceSourceKeysStore, SharedUnitsStore, StoredUnitReads};
use crate::{ApiKeySecret, SealedKey};

/// The shared Units store with the usage its writes are lent beside, borrowed for each call: the
/// store's guard can't be lent out of its `RefCell`, so each of the three repositories borrows it
/// for the one read or write.
#[derive(Clone)]
struct Rows {
    store: SharedUnitsStore,
    usage: StoredUnitReads,
}

impl Rows {
    /// Runs `write` on the store's write ports, which reach its recorder, so the commit names the
    /// Units in what the input wrote.
    fn write(&self, write: impl FnOnce(PortsMut<'_>)) {
        self.store.borrow_mut().split_write(&self.usage, write);
    }
}

/// The Units store's Units.
struct Units(Rows);

impl UnitRepository for Units {
    fn units(&self) -> Vec<UnitRow> {
        self.0.store.borrow().units().units()
    }

    fn unit_count(&self) -> usize {
        self.0.store.borrow().units().unit_count()
    }

    fn unit(&self, id: UnitId) -> Option<UnitRow> {
        self.0.store.borrow().units().unit(id)
    }

    fn upsert_unit(&mut self, unit: UnitRow) {
        self.0.write(|ports| ports.units.upsert_unit(unit));
    }

    fn delete_unit(&mut self, id: UnitId) {
        self.0.write(|ports| ports.units.delete_unit(id));
    }
}

/// The Units store's stored Price Sources.
struct PriceSources(Rows);

impl PriceSourceRepository for PriceSources {
    fn price_source_records(&self) -> Vec<PriceSourceRecord> {
        self.0.store.borrow().price_sources().price_source_records()
    }

    fn upsert_price_source(&mut self, record: PriceSourceRecord) {
        self.0
            .write(|ports| ports.price_sources.upsert_price_source(record));
    }

    fn delete_price_source(&mut self, id: PriceSourceId) {
        self.0
            .write(|ports| ports.price_sources.delete_price_source(id));
    }
}

/// The Units store's base Unit.
struct BaseUnit(Rows);

impl BaseUnitRepository for BaseUnit {
    fn base_unit(&self) -> UnitId {
        self.0.store.borrow().base_unit().base_unit()
    }

    fn set_base_unit(&mut self, id: UnitId) {
        self.0.write(|ports| ports.base_unit.set_base_unit(id));
    }
}

/// `lib-units`' API keys over the sealed rows, opened with the device's secret. One secret seals
/// every row of a Ledger: a device writes only while its secret's fingerprint matches the rows', or
/// there are none, and generates a secret for the first key if it has none (ADR-0043).
pub(crate) struct StoredPriceSourceKeys {
    rows: SharedPriceSourceKeysStore,
    secret: ApiKeySecret,
}

impl StoredPriceSourceKeys {
    pub(crate) fn new(rows: SharedPriceSourceKeysStore, secret: ApiKeySecret) -> Self {
        Self { rows, secret }
    }

    /// Whether this device's secret sealed every row: true too with no rows.
    fn sealed_here(&self) -> Option<bool> {
        let rows = self.rows.borrow().sealed();
        if rows.is_empty() {
            return Some(true);
        }
        let sealer = self.secret.sealer()?;
        Some(rows.iter().all(|row| row.key_check == sealer.check()))
    }

    /// `source`'s sealed row, if it has one.
    fn sealed_row(&self, source: PriceSourceId) -> Option<SealedKey> {
        self.rows
            .borrow()
            .sealed()
            .into_iter()
            .find(|row| row.source == source)
    }

    /// `row`'s key, opened with this device's secret: `None` when the secret can't open it.
    fn open(&self, row: &SealedKey) -> Option<SecretString> {
        self.secret.sealer()?.open(row)
    }
}

impl PriceSourceKeys for StoredPriceSourceKeys {
    fn key(&self, source: PriceSourceId) -> Option<SecretString> {
        self.open(&self.sealed_row(source)?)
    }

    fn key_state(&self, source: PriceSourceId) -> KeyState {
        match self.sealed_row(source) {
            None => KeyState::NotSet,
            Some(row) if self.open(&row).is_some() => KeyState::Saved,
            Some(_) => KeyState::Unreadable,
        }
    }

    fn can_write(&self) -> bool {
        self.sealed_here().unwrap_or(false)
    }

    fn store_key(&mut self, source: PriceSourceId, key: SecretString) -> lib_units::Result<()> {
        if !self.sealed_here().unwrap_or(false) {
            return Err(lib_units::Error::KeysLocked);
        }
        let sealer = match self.secret.sealer() {
            Some(sealer) => sealer,
            None => self.secret.generate().map_err(|error| {
                tracing::error!(%error, "This device's API key secret couldn't be kept");
                lib_units::Error::KeyNotSaved
            })?,
        };
        let sealed = sealer.seal(source, &key).map_err(|error| {
            tracing::error!(%error, price_source_id = %source, "An API key couldn't be sealed");
            lib_units::Error::KeyNotSaved
        })?;
        self.rows.borrow_mut().seal(sealed);
        Ok(())
    }

    fn remove_key(&mut self, source: PriceSourceId) {
        let saved = self
            .rows
            .borrow()
            .sealed()
            .iter()
            .any(|row| row.source == source);
        if saved {
            self.rows.borrow_mut().delete(source);
        }
    }
}

/// Settings › Data & backup's view of the keys: the secret's fingerprint, and the way out when the
/// secret is lost, deleting every row, which syncs as deletes (ADR-0043).
impl lib_settings::ApiKeys for StoredPriceSourceKeys {
    fn fingerprint(&self) -> Option<String> {
        self.secret.fingerprint()
    }

    fn remove_all(&self) {
        let sources: Vec<PriceSourceId> = self
            .rows
            .borrow()
            .sealed()
            .iter()
            .map(|row| row.source())
            .collect();
        tracing::debug!(keys = sources.len(), "Removing every Price Source API key");
        let mut rows = self.rows.borrow_mut();
        for source in sources {
            rows.delete(source);
        }
    }
}

/// [`lib_units::UnitsPorts`] over the shared Units store, and the usage and holdings read from the
/// stores of the domains that name a Unit. A write is lent the same reads as its usage, so no write
/// passes a stand-in usage to reach the store.
pub(crate) struct StoredUnitsPorts {
    units: Units,
    price_sources: PriceSources,
    base_unit: BaseUnit,
    reads: StoredUnitReads,
    keys: StoredPriceSourceKeys,
}

impl StoredUnitsPorts {
    pub fn new(
        units: SharedUnitsStore,
        reads: StoredUnitReads,
        keys: SharedPriceSourceKeysStore,
        secret: ApiKeySecret,
    ) -> Self {
        let rows = Rows {
            store: units,
            usage: reads.clone(),
        };
        Self {
            units: Units(rows.clone()),
            price_sources: PriceSources(rows.clone()),
            base_unit: BaseUnit(rows),
            reads,
            keys: StoredPriceSourceKeys::new(keys, secret),
        }
    }
}

impl UnitsPorts for StoredUnitsPorts {
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
        &self.reads
    }

    fn holdings(&self) -> &dyn UnitHoldings {
        &self.reads
    }

    fn openings(&self) -> &dyn UnitOpenings {
        &self.reads
    }

    fn split_mut(&mut self) -> PortsMut<'_> {
        PortsMut {
            units: &mut self.units,
            price_sources: &mut self.price_sources,
            base_unit: &mut self.base_unit,
            usage: &self.reads,
        }
    }

    fn keys(&self) -> &dyn PriceSourceKeys {
        &self.keys
    }

    fn keys_mut(&mut self) -> &mut dyn PriceSourceKeys {
        &mut self.keys
    }
}

#[cfg(test)]
mod tests {
    use lib_core::UnitId;
    use lib_unit_deletion::interactor::DeleteUnit;

    use crate::ports::test_support::{fakes, file};
    use crate::{Domain, LedgerPorts};

    use super::StoredUnitsPorts;
    use crate::adapter::store::StoredUnitReads;

    /// `lib-units`' usage check over a [`LedgerPorts`]: the Units ports with the reads over the
    /// other stores, a booking upserted into the Transactions store and the commit the Client's.
    struct LedgerUsageFactory {
        ledger: Option<LedgerPorts>,
        make: Box<dyn Fn() -> LedgerPorts>,
    }

    impl LedgerUsageFactory {
        fn ledger(&self) -> &LedgerPorts {
            self.ledger.as_ref().expect("create ran first")
        }
    }

    impl lib_units::contract::UsageFactory for LedgerUsageFactory {
        type Ports = StoredUnitsPorts;

        fn create(&mut self) -> StoredUnitsPorts {
            let ledger = (self.make)();
            let stores = &ledger.stores;
            let ports = StoredUnitsPorts::new(
                stores.units.clone(),
                StoredUnitReads {
                    accounts: stores.accounts.clone(),
                    transactions: stores.transactions.clone(),
                    bills: stores.bills.clone(),
                    budgets: stores.budgets.clone(),
                    inventory: stores.inventory.clone(),
                    institutions: stores.institutions.clone(),
                },
                stores.price_source_keys.clone(),
                crate::ports::test_support::secret(),
            );
            self.ledger = Some(ledger);
            ports
        }

        fn book(&mut self, _ports: &mut StoredUnitsPorts, account: &str) {
            let stores = &self.ledger().stores;
            let account = stores
                .accounts
                .borrow()
                .accounts()
                .accounts()
                .into_iter()
                .find(|row| row.name == account)
                .expect("the Account is stored")
                .id;
            let mut booked = stores
                .transactions
                .borrow()
                .all_transactions()
                .into_iter()
                .next()
                .expect("a sample Transaction");
            booked.id = lib_core::TransactionId::new();
            booked.account_id = account;
            stores.transactions.borrow_mut().upsert_transaction(booked);
        }

        fn commit(&mut self, _ports: &mut StoredUnitsPorts) {
            let committed = self.ledger().commit.commit();
            assert_eq!(committed.wrote, [Domain::Transactions]);
        }
    }

    #[test]
    fn usage_counts_the_accounts_and_transactions_in_a_unit_in_memory() {
        lib_units::contract::usage_counts_the_accounts_and_transactions_in_a_unit(
            &mut LedgerUsageFactory {
                ledger: None,
                make: Box::new(fakes),
            },
        );
    }

    #[test]
    fn usage_counts_the_accounts_and_transactions_in_a_unit_on_sqlite() {
        let dir = std::rc::Rc::new(tempfile::tempdir().expect("a temporary directory"));
        lib_units::contract::usage_counts_the_accounts_and_transactions_in_a_unit(
            &mut LedgerUsageFactory {
                ledger: None,
                make: Box::new(move || file(dir.path())),
            },
        );
    }

    #[test]
    fn each_units_write_reports_the_units_written_on_either() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        for ledger in [fakes(), file(dir.path())] {
            let LedgerPorts { units, commit, .. } = ledger;
            let mut aud = units
                .borrow()
                .units()
                .unit(UnitId::SEED_AUD)
                .expect("the sample's AUD");
            aud.name.push_str(" renamed");

            units.borrow_mut().split_mut().units.upsert_unit(aud);
            assert_eq!(commit.commit().wrote, [Domain::Units], "a Unit");

            units
                .borrow_mut()
                .split_mut()
                .base_unit
                .set_base_unit(UnitId::SEED_BTC);
            assert_eq!(commit.commit().wrote, [Domain::Units], "the base Unit");
        }
    }

    #[test]
    fn deleting_a_custom_source_deletes_its_key_in_the_same_commit_on_either() {
        use lib_units::interactor::{DeletePriceSource, KeyChange, SavePriceSourceKey};
        use lib_units::{
            CustomSource, Endpoint, KeyPlacement, KeyState, PriceSourceAuth, PriceSourceRecord,
            Updates,
        };

        let dir = tempfile::tempdir().expect("a temporary directory");
        for ledger in [fakes(), file(dir.path())] {
            let id = lib_core::PriceSourceId::new();
            let units = ledger.units();
            units
                .borrow_mut()
                .split_mut()
                .price_sources
                .upsert_price_source(PriceSourceRecord::Custom(CustomSource {
                    id,
                    name: "Rates".into(),
                    endpoint: Endpoint {
                        address: "https://rates.example".into(),
                        request: "/{symbol}".into(),
                        price_path: "/price".into(),
                        auth: PriceSourceAuth::ApiKey {
                            placement: KeyPlacement::Param,
                            name: "apikey".into(),
                            optional: false,
                        },
                        headers: Vec::new().into(),
                        batch: None,
                        history: None,
                    },
                    updates: Updates::Daily,
                }));
            units
                .borrow_mut()
                .keys_mut()
                .store_key(id, "k3y".into())
                .expect("the key is sealed");
            let _ = ledger.commit().commit();
            ledger.reload(Domain::Units).expect("the Units reload");
            assert_eq!(units.borrow().keys().key_state(id), KeyState::Saved);

            {
                let mut ports = units.borrow_mut();
                let split = ports.split_mut();
                DeletePriceSource(id)
                    .run(split.price_sources, split.units)
                    .expect("a Custom source deletes");
                SavePriceSourceKey {
                    source: id,
                    change: KeyChange::Remove,
                }
                .run(ports.keys_mut())
                .expect("a removal is never refused");
            }
            assert_eq!(ledger.commit().commit().wrote, [Domain::Units]);
            ledger.reload(Domain::Units).expect("the Units reload");

            let ports = units.borrow();
            assert!(ports.source_catalogue().source(id).is_none());
            assert_eq!(ports.keys().key_state(id), KeyState::NotSet);
        }
    }

    #[test]
    fn a_units_history_floor_is_its_earliest_opened_account() {
        let ledger = fakes();
        let units = ledger.units.borrow();
        let openings = units.openings();

        assert_eq!(
            openings.earliest_opened(UnitId::SEED_AUD),
            chrono::NaiveDate::from_ymd_opt(2019, 3, 1),
            "the earliest of the sample's aud Accounts"
        );
        assert_eq!(openings.earliest_opened(UnitId::new()), None, "no Account");
    }

    #[test]
    fn the_usage_is_counted_in_the_other_stores() {
        let ledger = fakes();

        let usage = ledger.units.borrow().usage().unit_usage(UnitId::SEED_AUD);

        assert!(usage.accounts > 0, "the sample's Accounts count in AUD");
        assert!(ledger.commit.commit().is_empty(), "a read writes nothing");
    }

    #[test]
    fn a_unit_delete_reports_the_units_written_on_either() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        for ledger in [fakes(), file(dir.path())] {
            let LedgerPorts {
                units,
                unit_deletion,
                commit,
                ..
            } = ledger;
            let mut spare = units
                .borrow()
                .units()
                .unit(UnitId::SEED_AUD)
                .expect("the sample's AUD");
            spare.id = UnitId::new();
            spare.code = "zzz".into();
            spare.link.quote = spare.id;
            let vas = spare.id;
            units.borrow_mut().split_mut().units.upsert_unit(spare);
            let _ = commit.commit();
            let usage = units.borrow().usage().unit_usage(vas);

            DeleteUnit {
                id: vas,
                new_base: None,
                replacements: Vec::new(),
            }
            .run(&mut *unit_deletion.borrow_mut(), &usage)
            .expect("an unused Unit");

            assert_eq!(commit.commit().wrote, [Domain::Units]);
            assert!(units.borrow().units().unit(vas).is_none());
        }
    }
}
