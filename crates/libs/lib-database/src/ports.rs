//! What a Client holds a Ledger by (#1076): [`LedgerPorts`], built once from [`Stores`], with one
//! `Stored<Domain>Ports` per domain implementing that domain crate's own ports trait, the
//! cross-domain adapters, the module reads, the commit and the failure channel. A Client hands a
//! Presenter the domain crate's shared ports straight from here, with no adapter of its own. The
//! module reads come from here too: the Dashboard's, the Shell rail's, the Inventory's, the
//! Documents' and Settings › Ledger's (#1085). The decision is ADR-0055.
//!
//! There are two layers. The bottom is one shared store per domain ([`SharedStores`]), the
//! adapter or the fake wrapped in its recorder. The top is the `Stored<Domain>Ports`, each holding
//! its own domain's store and the `Stored<Domain>Reads` it needs over other domains' stores.
//!
//! The cycle invariant: `Stored*Reads` and cross-domain adapters hold only bottom-layer stores, no
//! store holds anything, and a `Stored<Domain>Ports` is held only by Clients, never by a read, an
//! adapter or another `Stored<Domain>Ports`. So every `Rc` points down a layer, no `Rc` cycle can
//! be built, and no build order has to guard against one.
//!
//! A domain joining it adds its store (wrapped in its recorder by [`Stores::new`]), its
//! `Stored<Domain>Ports` in a file of its own under `ports/`, a field here holding it as the domain
//! crate's `SharedPorts` with a line in [`LedgerPorts::new`] building it over [`SharedStores`], and
//! a [`Domain`] variant.

mod accounts;
mod bills;
mod budgets;
mod categories;
mod documents;
mod institutions;
mod inventory;
mod payees;
mod tags;
mod transactions;
mod units;

pub use accounts::StoredAccountsPorts;
pub use bills::StoredBillsPorts;
pub use budgets::StoredBudgetsPorts;
pub use categories::StoredCategoriesPorts;
pub use documents::StoredDocumentsPorts;
pub use institutions::StoredInstitutionsPorts;
pub use inventory::StoredInventoryPorts;
pub use payees::StoredPayeesPorts;
pub use tags::StoredTagsPorts;
pub use transactions::StoredTransactionsPorts;
pub use units::StoredUnitsPorts;

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::mpsc;

use lib_core::Clock;

use crate::adapter::store::{
    SharedStores, StoredAccountReads, StoredBillReads, StoredBillUnits, StoredBudgetReads,
    StoredCategoryDeletionReads, StoredDashboardReads, StoredDocumentReads, StoredImportReads,
    StoredInstitutionReads, StoredInventoryReads, StoredPayeeReads, StoredShellReads,
    StoredTagReads, StoredTagRemovalReads, StoredTransactionReads, StoredUnitReads,
};
use crate::{
    AccountDeletionAdapter, BillPaymentsAdapter, CategoryDeletionAdapter, Commit, Domain,
    LedgerPreferencesAdapter, Result, StoredCategoryRows, StoredSettingsReads, Stores,
    TagRemovalAdapter, TransactionImportAdapter, UnitDeletionAdapter, WriteFailure,
};

/// A Ledger as a Client holds it: every domain's ports over the shared stores, the cross-domain
/// adapters, the commit every input ends in and the channel a failed commit is reported on.
pub struct LedgerPorts {
    /// Every domain's store, shared: the bottom layer the ports, reads and adapters hold.
    pub stores: SharedStores,
    /// The Units page's, its Dialogs' and the Ledger settings page's ports ([`StoredUnitsPorts`]).
    pub units: lib_units::SharedPorts,
    /// Unit delete's atomic port over the Units and Institutions stores ([`UnitDeletionAdapter`]).
    pub unit_deletion: lib_unit_deletion::SharedUnitDeletion,
    /// The Institutions page's and its Dialogs' ports ([`StoredInstitutionsPorts`]).
    pub institutions: lib_institutions::SharedPorts,
    /// The Accounts page's and its Dialogs' ports ([`StoredAccountsPorts`]).
    pub accounts: lib_accounts::SharedPorts,
    /// What names an Account and the Units it may count in, over the stores: what the Delete
    /// account Dialog quotes when it opens.
    pub account_reads: StoredAccountReads,
    /// Account delete's atomic port over the Accounts, Transactions and Bills stores
    /// ([`AccountDeletionAdapter`]).
    pub account_deletion: lib_account_deletion::SharedDeletion,
    /// The Payees page's and its Dialogs' ports ([`StoredPayeesPorts`]).
    pub payees: lib_payees::SharedPorts,
    /// The Bills page's and its Dialogs' ports ([`StoredBillsPorts`]).
    pub bills: lib_bills::SharedPorts,
    /// The Tags page's and its Add and Edit Dialogs' ports ([`StoredTagsPorts`]).
    pub tags: lib_tags::SharedPorts,
    /// Tag remove and merge's atomic port over the Tags and Transactions stores
    /// ([`TagRemovalAdapter`]).
    pub tag_removal: lib_tag_removal::SharedTagRemoval,
    /// What the Remove tag and Merge tags Dialogs show when they open.
    pub tag_removal_reads: StoredTagRemovalReads,
    /// The Categories page's repository ([`StoredCategoriesPorts`]).
    pub categories: lib_categories::SharedCategories,
    /// The same ports for the Add and Edit category Dialogs' save: the usage, the Monthly budget
    /// and the handover. A value of its own over the same stores, so a Presenter holding the
    /// repository above never borrows it while the Dialog writes through this.
    pub category_save: StoredCategoriesPorts,
    /// Category delete's atomic port over the Categories, Transactions, Bills, Payees and Budgets
    /// stores ([`CategoryDeletionAdapter`]).
    pub category_deletion: lib_category_deletion::SharedCategoryDeletion,
    /// What the Delete category Dialog shows when it opens.
    pub category_deletion_reads: StoredCategoryDeletionReads,
    /// The Transactions page's ports ([`StoredTransactionsPorts`]).
    pub transactions: lib_transactions::SharedPorts,
    /// The Import's atomic port over the Payees and Transactions stores, which is also its Payees
    /// read ([`TransactionImportAdapter`]).
    pub transaction_import: Rc<RefCell<TransactionImportAdapter>>,
    /// The Accounts and Categories the Import's step offers.
    pub import_reads: Rc<StoredImportReads>,
    /// Pay it directly's atomic port over the Transactions and Bills stores
    /// ([`BillPaymentsAdapter`]).
    pub bill_payments: lib_bill_payments::SharedPayments,
    /// The Budgets page's and its Dialogs' ports ([`StoredBudgetsPorts`]).
    pub budgets: lib_budgets::SharedPorts,
    /// The Inventory page's and its Dialogs' repository ([`StoredInventoryPorts`]).
    pub inventory: lib_inventory::SharedInventory,
    /// The Documents and Document Types pages' repository ([`StoredDocumentsPorts`]).
    pub documents: lib_documents::SharedRepository,
    /// What the Inventory page and its Dialogs read the Units and Documents through.
    pub inventory_reads: StoredInventoryReads,
    /// What the Documents page reads the other domains through.
    pub document_reads: StoredDocumentReads,
    /// The Settings › Ledger page's Units read ([`StoredSettingsReads`]).
    pub ledger_units: Rc<StoredSettingsReads>,
    /// The Settings › Ledger page's base Unit write ([`LedgerPreferencesAdapter`]).
    pub ledger_preferences: Rc<LedgerPreferencesAdapter>,
    /// The Dashboard's Bills and Budgets reads ([`StoredDashboardReads`]).
    pub dashboard_reads: Rc<StoredDashboardReads>,
    /// The Shell's rail-count reads ([`StoredShellReads`]).
    pub shell_reads: StoredShellReads,
    pub commit: Rc<dyn Commit>,
    pub failures: mpsc::Receiver<WriteFailure>,
}

impl LedgerPorts {
    /// Shares out each of `stores` once and builds every domain's ports and the cross-domain
    /// adapters over them. `clock` is what the Categories ports read the day from on each call.
    pub fn new(stores: Stores, clock: Clock) -> Self {
        let Stores {
            units,
            institutions,
            accounts,
            categories,
            tags,
            payees,
            transactions,
            bills,
            budgets,
            inventory,
            documents,
            commit,
            failures,
        } = stores;
        let stores = SharedStores {
            units: Rc::new(RefCell::new(units)),
            institutions: Rc::new(RefCell::new(institutions)),
            accounts: Rc::new(RefCell::new(accounts)),
            categories: Rc::new(RefCell::new(categories)),
            tags: Rc::new(RefCell::new(tags)),
            payees: Rc::new(RefCell::new(payees)),
            transactions: Rc::new(RefCell::new(transactions)),
            bills: Rc::new(RefCell::new(bills)),
            budgets: Rc::new(RefCell::new(budgets)),
            inventory: Rc::new(RefCell::new(inventory)),
            documents: Rc::new(RefCell::new(documents)),
        };
        let category_save = StoredCategoriesPorts::new(
            stores.categories.clone(),
            stores.budgets.clone(),
            StoredCategoryRows {
                transactions: stores.transactions.clone(),
                bills: stores.bills.clone(),
                payees: stores.payees.clone(),
            },
            clock,
        );
        let transaction_import = Rc::new(RefCell::new(TransactionImportAdapter::new(
            stores.payees.clone(),
            stores.transactions.clone(),
        )));
        let budget_reads = Rc::new(StoredBudgetReads {
            categories: stores.categories.clone(),
            accounts: stores.accounts.clone(),
            transactions: stores.transactions.clone(),
            bills: stores.bills.clone(),
            units: stores.units.clone(),
            payees: stores.payees.clone(),
        });
        let account_reads = StoredAccountReads {
            transactions: stores.transactions.clone(),
            budgets: stores.budgets.clone(),
            units: stores.units.clone(),
            institutions: stores.institutions.clone(),
        };
        let dashboard_reads = Rc::new(StoredDashboardReads {
            budgets: stores.budgets.clone(),
            budget_reads: (*budget_reads).clone(),
        });
        let shell_reads = StoredShellReads {
            budgets: stores.budgets.clone(),
            documents: stores.documents.clone(),
            budget_reads: (*budget_reads).clone(),
        };
        Self {
            inventory_reads: StoredInventoryReads {
                units: stores.units.clone(),
                inventory: stores.inventory.clone(),
                documents: stores.documents.clone(),
            },
            document_reads: StoredDocumentReads {
                transactions: stores.transactions.clone(),
                payees: stores.payees.clone(),
                accounts: stores.accounts.clone(),
                institutions: stores.institutions.clone(),
                bills: stores.bills.clone(),
                inventory: stores.inventory.clone(),
            },
            ledger_units: Rc::new(StoredSettingsReads {
                units: stores.units.clone(),
            }),
            ledger_preferences: Rc::new(LedgerPreferencesAdapter::new(stores.units.clone())),
            dashboard_reads,
            shell_reads,
            units: Rc::new(RefCell::new(StoredUnitsPorts::new(
                stores.units.clone(),
                StoredUnitReads {
                    accounts: stores.accounts.clone(),
                    transactions: stores.transactions.clone(),
                    bills: stores.bills.clone(),
                    budgets: stores.budgets.clone(),
                    inventory: stores.inventory.clone(),
                    institutions: stores.institutions.clone(),
                },
            ))),
            unit_deletion: Rc::new(RefCell::new(UnitDeletionAdapter::new(
                stores.units.clone(),
                stores.institutions.clone(),
            ))),
            institutions: Rc::new(RefCell::new(StoredInstitutionsPorts::new(
                stores.institutions.clone(),
                StoredInstitutionReads {
                    accounts: stores.accounts.clone(),
                    units: stores.units.clone(),
                },
            ))),
            accounts: Rc::new(RefCell::new(StoredAccountsPorts::new(
                stores.accounts.clone(),
                account_reads.clone(),
            ))),
            account_reads,
            account_deletion: Rc::new(RefCell::new(AccountDeletionAdapter::new(
                stores.accounts.clone(),
                stores.transactions.clone(),
                stores.bills.clone(),
            ))),
            payees: Rc::new(RefCell::new(StoredPayeesPorts::new(
                stores.payees.clone(),
                StoredPayeeReads {
                    transactions: stores.transactions.clone(),
                    bills: stores.bills.clone(),
                    categories: stores.categories.clone(),
                },
            ))),
            bills: Rc::new(RefCell::new(StoredBillsPorts::new(
                stores.bills.clone(),
                StoredBillReads {
                    payees: stores.payees.clone(),
                    accounts: stores.accounts.clone(),
                    categories: stores.categories.clone(),
                    transactions: stores.transactions.clone(),
                },
                StoredBillUnits {
                    units: stores.units.clone(),
                },
            ))),
            categories: Rc::new(RefCell::new(category_save.clone())),
            category_save,
            category_deletion: Rc::new(RefCell::new(CategoryDeletionAdapter::new(
                stores.categories.clone(),
                stores.transactions.clone(),
                stores.bills.clone(),
                stores.payees.clone(),
                stores.budgets.clone(),
            ))),
            category_deletion_reads: StoredCategoryDeletionReads {
                categories: stores.categories.clone(),
                transactions: stores.transactions.clone(),
                budgets: stores.budgets.clone(),
            },
            transactions: Rc::new(RefCell::new(StoredTransactionsPorts::new(
                stores.transactions.clone(),
                StoredTransactionReads {
                    accounts: stores.accounts.clone(),
                    categories: stores.categories.clone(),
                    payees: stores.payees.clone(),
                    tags: stores.tags.clone(),
                    units: stores.units.clone(),
                },
            ))),
            transaction_import,
            import_reads: Rc::new(StoredImportReads {
                accounts: stores.accounts.clone(),
                categories: stores.categories.clone(),
            }),
            bill_payments: Rc::new(RefCell::new(BillPaymentsAdapter::new(
                stores.transactions.clone(),
                stores.bills.clone(),
            ))),
            budgets: Rc::new(RefCell::new(StoredBudgetsPorts::new(
                stores.budgets.clone(),
                budget_reads,
            ))),
            inventory: Rc::new(RefCell::new(StoredInventoryPorts::new(
                stores.inventory.clone(),
            ))),
            documents: Rc::new(RefCell::new(StoredDocumentsPorts::new(
                stores.documents.clone(),
            ))),
            tags: Rc::new(RefCell::new(StoredTagsPorts::new(
                stores.tags.clone(),
                StoredTagReads {
                    transactions: stores.transactions.clone(),
                    accounts: stores.accounts.clone(),
                    units: stores.units.clone(),
                },
            ))),
            tag_removal: Rc::new(RefCell::new(TagRemovalAdapter::new(
                stores.tags.clone(),
                stores.transactions.clone(),
            ))),
            tag_removal_reads: StoredTagRemovalReads {
                tags: stores.tags.clone(),
                transactions: stores.transactions.clone(),
            },
            stores,
            commit,
            failures,
        }
    }

    /// Reads `domain`'s rows back as stored and drops its writes not yet committed: after a failed
    /// write, or a commit's cascade (ADR-0046).
    ///
    /// # Errors
    /// Returns the error reading the rows back; memory is left as it was.
    pub fn reload(&self, domain: Domain) -> Result<()> {
        self.stores.reload(domain)
    }
}

/// The Sample Ledger as a test's [`LedgerPorts`]: in memory, and as a Ledger file.
#[cfg(test)]
pub(crate) mod test_support {
    use std::path::Path;

    use chrono::NaiveDate;
    use lib_core::Clock;

    use crate::{FakeStores, LedgerPorts, Stores, open_storage, seed};

    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 9, 19).expect("a real date")
    }

    /// The Sample Ledger in memory, as the UI tests build the Client on it.
    pub fn fakes() -> LedgerPorts {
        LedgerPorts::new(
            Stores::fake(FakeStores::sample(today())).0,
            Clock::fixed(today()),
        )
    }

    /// The Sample Ledger created as a Ledger file in `dir`, opened as a Client opens it.
    pub fn file(dir: &Path) -> LedgerPorts {
        let path = dir.join("ledger.pldb");
        seed::sample_ledger(&path, today()).expect("the Sample Ledger is created");
        LedgerPorts::new(
            Stores::from(open_storage(&path).expect("the Ledger opens")),
            Clock::fixed(today()),
        )
    }
}
