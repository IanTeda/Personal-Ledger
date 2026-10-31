//! The Units ports as the TUI holds them: the store `build_shell` is given, with `UnitUsage` and
//! `UnitHoldings` answered by the stores opened beside it (#887), so a delete is checked against
//! what the Ledger file holds. Settings › Ledger and the base Unit guard count rows in the same
//! stores (#922). The TUI's other Views still draw their own fixtures, so no read copy
//! needs refreshing after a write.

use std::cell::RefCell;
use std::rc::Rc;

use lib_core::UnitId;
use lib_units::{
    BaseUnitRepository, Holding, PortsMut, PriceSourceRepository, UnitHoldings, UnitRepository,
    UnitUsage, UnitUse, UnitsPorts,
};

use lib_database::{
    AccountsStore, BillsStore, BudgetsStore, Commit, InstitutionsStore, InventoryStore,
    TransactionsStore, UnitsStore,
};

/// The Units ports the Units View and its Dialogs share.
pub type TuiPorts = Rc<RefCell<LedgerBackedPorts>>;

/// `store`'s Units, Price Sources and base Unit, with usage and holdings read from `usage`, and
/// the commit that ends each Interactor call. The commit is every adapter's, held here while Units
/// is the only module that writes through it.
pub struct LedgerBackedPorts {
    store: Box<dyn UnitsStore>,
    usage: StoredUsage,
    commit: Rc<dyn Commit>,
}

/// What names each Unit, read from the stores of the domains that name one: the Accounts, Bill
/// Plans, Budgets (#952), Locations and Institutions. The Transactions are held for their count
/// alone, which Settings › Ledger and the base Unit guard quote.
pub struct StoredUsage {
    pub accounts: Box<dyn AccountsStore>,
    pub bills: Box<dyn BillsStore>,
    pub budgets: Box<dyn BudgetsStore>,
    pub inventory: Box<dyn InventoryStore>,
    pub institutions: Box<dyn InstitutionsStore>,
    pub transactions: Box<dyn TransactionsStore>,
}

impl UnitUsage for StoredUsage {
    fn unit_usage(&self, id: UnitId) -> UnitUse {
        UnitUse {
            accounts: self.accounts.unit_accounts(id),
            bill_plans: self
                .bills
                .plans()
                .bill_plans()
                .iter()
                .filter(|plan| plan.unit == id)
                .count(),
            budgets: self.budgets.unit_budgets(id),
            locations: self.inventory.unit_locations(id),
            institutions: self
                .institutions
                .institutions()
                .institutions()
                .iter()
                .filter(|institution| institution.usual_unit == Some(id))
                .count(),
        }
    }
}

impl UnitHoldings for StoredUsage {
    fn holdings(&self, id: UnitId) -> Vec<Holding> {
        self.accounts.holdings(id)
    }
}

impl LedgerBackedPorts {
    pub fn shared(
        store: Box<dyn UnitsStore>,
        usage: StoredUsage,
        commit: Rc<dyn Commit>,
    ) -> TuiPorts {
        Rc::new(RefCell::new(Self {
            store,
            usage,
            commit,
        }))
    }

    /// The stores beside the Units, whose rows Settings › Ledger and the base Unit guard count.
    pub fn stores(&self) -> &StoredUsage {
        &self.usage
    }

    /// Ends the Interactor call just made, if any, as one transaction.
    pub fn commit(&mut self) {
        self.commit.commit();
    }

    /// Puts the rows back as stored, after a write that named the Units failed (ADR-0046).
    ///
    /// # Errors
    /// Returns the error reading the rows back; the rows in memory are left as they were.
    pub fn reload(&mut self) -> lib_database::Result<()> {
        self.store.reload()
    }
}

impl UnitsPorts for LedgerBackedPorts {
    fn units(&self) -> &dyn UnitRepository {
        self.store.units()
    }

    fn price_sources(&self) -> &dyn PriceSourceRepository {
        self.store.price_sources()
    }

    fn base_unit(&self) -> &dyn BaseUnitRepository {
        self.store.base_unit()
    }

    fn usage(&self) -> &dyn UnitUsage {
        &self.usage
    }

    fn holdings(&self) -> &dyn UnitHoldings {
        &self.usage
    }

    fn split_mut(&mut self) -> PortsMut<'_> {
        self.store.split_mut(&self.usage)
    }
}
