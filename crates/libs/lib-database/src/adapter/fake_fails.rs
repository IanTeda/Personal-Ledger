//! The in-memory stores' fail switches (#1118): a test arranges, on [`FakeStores`](crate::FakeStores),
//! which domains' writes and reloads fail, as SQLite refusing them would, rather than building a
//! failing store of its own per domain.
//!
//! Each fake is wrapped once in a [`Switched`] as [`Stores::fake`](crate::Stores::fake) builds, and
//! a domain whose writes fail keeps a stored snapshot, the rows as they are on disk: writes never
//! reach it, so a reload puts memory back to it. [`FailingCommit`] reports a commit that wrote a
//! failing domain on the failure channel, as the writer would.

use std::collections::HashMap;
use std::sync::mpsc;

use lib_accounts::AccountRepository;
use lib_bills::{BillPlanRepository, BillScheduleRepository};
use lib_budgets::BudgetRepository;
use lib_categories::CategoryRepository;
use lib_core::{AccountId, CategoryId, PayeeId, TagId, UnitId};
use lib_documents::DocumentRepository;
use lib_institutions::InstitutionRepository;
use lib_inventory::InventoryRepository;
use lib_payees::PayeeRepository;
use lib_tags::TagRepository;
use lib_transactions::{Transaction, TransactionRepository};
use lib_units::{BaseUnitRepository, PortsMut, PriceSourceRepository, UnitRepository, UnitUsage};

use super::store::{
    AccountsStore, BillsStore, BudgetsStore, CategoriesStore, DocumentsStore, InstitutionsStore,
    InventoryStore, PayeesStore, TagsStore, TransactionsStore, UnitsStore,
};
use crate::{Commit, Committed, Domain, Result, TaggedSplit, WriteFailure, Written};

/// What a test arranges to fail on the in-memory stores. Nothing fails by default.
#[derive(Debug, Clone, Default)]
pub struct FakeFails {
    /// The domains whose every write fails: the commit after one reports them on the failure
    /// channel, and a reload puts the stored snapshot back. `Domain::Documents` covers the
    /// Document Types too, since they are kept in the Documents store.
    pub writes: Vec<Domain>,
    /// The domains whose reload, after a failed write, fails too.
    pub reloads: Vec<Domain>,
    /// Whether the Transactions store's count of the Splits naming a Payee fails, as a Ledger file
    /// that can't be read would.
    pub payee_usage_fails: bool,
}

impl FakeFails {
    /// A fake's switches for `domain`, keeping a snapshot of `live` if its writes fail.
    pub(crate) fn switch<T: Clone>(&self, domain: Domain, live: T) -> Switched<T> {
        Switched {
            stored: self.writes.contains(&domain).then(|| live.clone()),
            live,
            reload_fails: self.reloads.contains(&domain),
            payee_usage_fails: false,
            inbox_reads: std::cell::Cell::new(0),
        }
    }
}

/// An in-memory store with its switches: `stored` is the snapshot a reload puts back, `None` when
/// writes succeed, since memory is then all there is.
pub(crate) struct Switched<T> {
    live: T,
    stored: Option<T>,
    reload_fails: bool,
    payee_usage_fails: bool,
    /// How many times the rail asked for the Inbox count, kept on the Documents store (#1120).
    inbox_reads: std::cell::Cell<usize>,
}

impl<T> Switched<T> {
    /// This Transactions store with its Payee usage count failing as `fails` says.
    pub(crate) fn payee_usage_fails(mut self, fails: bool) -> Self {
        self.payee_usage_fails = fails;
        self
    }
}

impl<T: Clone> Switched<T> {
    fn reload_snapshot(&mut self) -> Result<()> {
        if self.reload_fails {
            return Err(crate::Error::Generic("the test's reload fails".into()));
        }
        if let Some(stored) = &self.stored {
            self.live = stored.clone();
        }
        Ok(())
    }
}

/// The commit over the switched fakes: a commit that wrote a domain whose writes fail reports it
/// on the failure channel, as the writer would when SQLite refuses them.
pub(crate) struct FailingCommit {
    pub(crate) written: Written,
    pub(crate) fails: Vec<Domain>,
    pub(crate) report: mpsc::Sender<WriteFailure>,
}

impl Commit for FailingCommit {
    fn commit(&self) -> Committed {
        let wrote = self.written.take();
        let failed: Vec<Domain> = self
            .fails
            .iter()
            .copied()
            .filter(|domain| {
                wrote.contains(domain)
                    || (*domain == Domain::Documents && wrote.contains(&Domain::DocumentTypes))
            })
            .collect();
        if !failed.is_empty() {
            // The receiver is gone only once the Client is, when no one is left to tell.
            let _ = self.report.send(WriteFailure {
                domains: failed,
                error: crate::Error::Generic("the test's write fails".into()),
            });
        }
        Committed {
            wrote,
            cascaded: Vec::new(),
        }
    }
}

impl<T: UnitsStore + Clone> UnitsStore for Switched<T> {
    fn units(&self) -> &dyn UnitRepository {
        self.live.units()
    }

    fn price_sources(&self) -> &dyn PriceSourceRepository {
        self.live.price_sources()
    }

    fn base_unit(&self) -> &dyn BaseUnitRepository {
        self.live.base_unit()
    }

    fn split_mut<'a>(&'a mut self, usage: &'a dyn UnitUsage) -> PortsMut<'a> {
        self.live.split_mut(usage)
    }

    fn unit_code(&self, id: UnitId) -> Option<String> {
        self.live.unit_code(id)
    }

    fn reload(&mut self) -> Result<()> {
        self.reload_snapshot()
    }
}

impl<T: InstitutionsStore + Clone> InstitutionsStore for Switched<T> {
    fn institutions(&self) -> &dyn InstitutionRepository {
        self.live.institutions()
    }

    fn institutions_mut(&mut self) -> &mut dyn InstitutionRepository {
        self.live.institutions_mut()
    }

    fn reload(&mut self) -> Result<()> {
        self.reload_snapshot()
    }
}

impl<T: AccountsStore + Clone> AccountsStore for Switched<T> {
    fn accounts(&self) -> &dyn AccountRepository {
        self.live.accounts()
    }

    fn accounts_mut(&mut self) -> &mut dyn AccountRepository {
        self.live.accounts_mut()
    }

    fn reload(&mut self) -> Result<()> {
        self.reload_snapshot()
    }
}

impl<T: CategoriesStore + Clone> CategoriesStore for Switched<T> {
    fn categories(&self) -> &dyn CategoryRepository {
        self.live.categories()
    }

    fn categories_mut(&mut self) -> &mut dyn CategoryRepository {
        self.live.categories_mut()
    }

    fn delete_category(&mut self, id: CategoryId) {
        self.live.delete_category(id);
    }

    fn reload(&mut self) -> Result<()> {
        self.reload_snapshot()
    }
}

impl<T: TagsStore + Clone> TagsStore for Switched<T> {
    fn tags(&self) -> &dyn TagRepository {
        self.live.tags()
    }

    fn tags_mut(&mut self) -> &mut dyn TagRepository {
        self.live.tags_mut()
    }

    fn delete_tag(&mut self, id: TagId) {
        self.live.delete_tag(id);
    }

    fn reload(&mut self) -> Result<()> {
        self.reload_snapshot()
    }
}

impl<T: InventoryStore + Clone> InventoryStore for Switched<T> {
    fn inventory(&self) -> &dyn InventoryRepository {
        self.live.inventory()
    }

    fn inventory_mut(&mut self) -> &mut dyn InventoryRepository {
        self.live.inventory_mut()
    }

    fn reload(&mut self) -> Result<()> {
        self.reload_snapshot()
    }
}

impl<T: TransactionsStore + Clone> TransactionsStore for Switched<T> {
    fn transactions(&self) -> &dyn TransactionRepository {
        self.live.transactions()
    }

    fn transactions_mut(&mut self) -> &mut dyn TransactionRepository {
        self.live.transactions_mut()
    }

    fn all_transactions(&self) -> Vec<Transaction> {
        self.live.all_transactions()
    }

    fn upsert_transaction(&mut self, transaction: Transaction) {
        self.live.upsert_transaction(transaction);
    }

    fn category_splits(&self, id: CategoryId) -> usize {
        self.live.category_splits(id)
    }

    fn account_transaction_counts(&self, accounts: &[AccountId]) -> HashMap<AccountId, usize> {
        self.live.account_transaction_counts(accounts)
    }

    fn payee_splits(&self, id: PayeeId) -> Option<usize> {
        if self.payee_usage_fails {
            return None;
        }
        self.live.payee_splits(id)
    }

    fn tagged_splits(&self, id: TagId) -> Vec<TaggedSplit> {
        self.live.tagged_splits(id)
    }

    fn tag_transaction_counts(&self) -> HashMap<TagId, usize> {
        self.live.tag_transaction_counts()
    }

    fn reload(&mut self) -> Result<()> {
        self.reload_snapshot()
    }
}

impl<T: BillsStore + Clone> BillsStore for Switched<T> {
    fn plans(&self) -> &dyn BillPlanRepository {
        self.live.plans()
    }

    fn plans_mut(&mut self) -> &mut dyn BillPlanRepository {
        self.live.plans_mut()
    }

    fn entries(&self) -> &dyn BillScheduleRepository {
        self.live.entries()
    }

    fn entries_mut(&mut self) -> &mut dyn BillScheduleRepository {
        self.live.entries_mut()
    }

    fn payee_bill_plans(&self, id: PayeeId) -> Option<usize> {
        self.live.payee_bill_plans(id)
    }

    fn reload(&mut self) -> Result<()> {
        self.reload_snapshot()
    }
}

impl<T: PayeesStore + Clone> PayeesStore for Switched<T> {
    fn payees(&self) -> &dyn PayeeRepository {
        self.live.payees()
    }

    fn payees_mut(&mut self) -> &mut dyn PayeeRepository {
        self.live.payees_mut()
    }

    fn payee_name(&self, id: PayeeId) -> Option<String> {
        self.live.payee_name(id)
    }

    fn reload(&mut self) -> Result<()> {
        self.reload_snapshot()
    }
}

impl<T: BudgetsStore + Clone> BudgetsStore for Switched<T> {
    fn budgets(&self) -> &dyn BudgetRepository {
        self.live.budgets()
    }

    fn budgets_mut(&mut self) -> &mut dyn BudgetRepository {
        self.live.budgets_mut()
    }

    fn account_budgets(&self, id: AccountId) -> usize {
        self.live.account_budgets(id)
    }

    fn unit_budgets(&self, unit: UnitId) -> usize {
        self.live.unit_budgets(unit)
    }

    fn category_budgets(&self, id: CategoryId) -> usize {
        self.live.category_budgets(id)
    }

    fn reload(&mut self) -> Result<()> {
        self.reload_snapshot()
    }
}

impl<T: DocumentsStore + Clone> DocumentsStore for Switched<T> {
    fn documents(&self) -> &dyn DocumentRepository {
        self.live.documents()
    }

    fn documents_mut(&mut self) -> &mut dyn DocumentRepository {
        self.live.documents_mut()
    }

    fn inbox_count(&self) -> usize {
        self.inbox_reads.set(self.inbox_reads.get() + 1);
        lib_documents::inbox_count(&self.live.documents().documents())
    }

    fn inbox_reads(&self) -> usize {
        self.inbox_reads.get()
    }

    fn reload(&mut self) -> Result<()> {
        self.reload_snapshot()
    }
}
