//! `lib-units`' ports answered over the stores: the Units, Price Sources and base Unit from the
//! Units store, and what names each Unit and the Accounts held in it from the Accounts,
//! Transactions, Bills, Budgets, Inventory and Institutions stores through [`StoredUnitReads`]
//! (#993, #1084).

use lib_core::{PriceSourceId, UnitId};
use lib_units::{
    BaseUnitRepository, PortsMut, PriceSourceRecord, PriceSourceRepository, UnitHoldings,
    UnitRepository, UnitRow, UnitUsage, UnitsPorts,
};

use crate::adapter::store::{SharedUnitsStore, StoredUnitReads};

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
        write(self.store.borrow_mut().split_mut(&self.usage));
    }
}

/// The Units store's Units.
struct Units(Rows);

impl UnitRepository for Units {
    fn units(&self) -> Vec<UnitRow> {
        self.0.store.borrow().units().units()
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

/// [`lib_units::UnitsPorts`] over the shared Units store, and the usage and holdings read from the
/// stores of the domains that name a Unit. A write is lent the same reads as its usage, so no write
/// passes a stand-in usage to reach the store.
pub struct StoredUnitsPorts {
    units: Units,
    price_sources: PriceSources,
    base_unit: BaseUnit,
    reads: StoredUnitReads,
}

impl StoredUnitsPorts {
    pub fn new(units: SharedUnitsStore, reads: StoredUnitReads) -> Self {
        let rows = Rows {
            store: units,
            usage: reads.clone(),
        };
        Self {
            units: Units(rows.clone()),
            price_sources: PriceSources(rows.clone()),
            base_unit: BaseUnit(rows),
            reads,
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

    fn split_mut(&mut self) -> PortsMut<'_> {
        PortsMut {
            units: &mut self.units,
            price_sources: &mut self.price_sources,
            base_unit: &mut self.base_unit,
            usage: &self.reads,
        }
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
                .expect("the sample's aud");
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
    fn the_usage_is_counted_in_the_other_stores() {
        let ledger = fakes();

        let usage = ledger.units.borrow().usage().unit_usage(UnitId::SEED_AUD);

        assert!(usage.accounts > 0, "the sample's Accounts count in aud");
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
                .expect("the sample's aud");
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
