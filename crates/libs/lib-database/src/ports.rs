//! What a Client holds a Ledger by (#1076): [`LedgerPorts`], built once from [`Stores`], with one
//! `Stored<Domain>Ports` per domain implementing that domain crate's own ports trait, the
//! cross-domain adapters, the module reads and the commit, handed out with the failure channel it
//! reports a failed write on (#1202). A Client hands a Presenter the domain crate's shared ports
//! straight from here, with no adapter of its own. The module reads come from here too: the
//! Dashboard's, the Shell rail's, the Inventory's, the Documents' and Settings › Ledger's (#1085).
//! The decision is ADR-0055.
//!
//! There are two layers. The bottom is one shared store per domain ([`SharedStores`]), the
//! adapter or the fake wrapped in its recorder. The top is the `Stored<Domain>Ports`, each holding
//! its own domain's store and the `Stored<Domain>Reads` it needs over other domains' stores.
//!
//! The cycle invariant: `Stored*Reads` and cross-domain adapters hold only bottom-layer stores, no
//! store holds anything (the stores a delete cascades into are held weakly, on the `Written` set
//! the stores share, #1276), and a `Stored<Domain>Ports` is held only by Clients, never by a read, an
//! adapter or another `Stored<Domain>Ports`. So every `Rc` points down a layer, no `Rc` cycle can
//! be built, and no build order has to guard against one.
//!
//! A domain joining it adds its store (wrapped in its recorder by [`Stores::new`]), its
//! `Stored<Domain>Ports` in a file of its own under `ports/`, a field here holding it as the domain
//! crate's `SharedPorts` with a line in [`LedgerPorts::new`] building it over [`SharedStores`], a
//! method handing it out, and a [`Domain`] variant.
//!
//! The fields are private and the `Stored*` types `pub(crate)` (#1212), so the compiler holds the
//! seam: a Client names only a feature crate's ports or the bundle it declares, never a type of
//! this crate's behind them, and reaches the raw stores only through [`LedgerPorts::reload`].
//!
//! A Client opening a Ledger file builds on its [`Stores`] with [`LedgerPorts::new`], which shares
//! them out itself, so the Client never holds a domain's store: the Desktop opens a file this way
//! (#1221). [`LedgerPorts::over`] is for a caller keeping its own handle on the stores, which it
//! shared out first: a test reading rows back, or the TUI until its redesign (#805).

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

pub(crate) use accounts::StoredAccountsPorts;
pub(crate) use bills::StoredBillsPorts;
pub(crate) use budgets::StoredBudgetsPorts;
pub(crate) use categories::StoredCategoriesPorts;
pub(crate) use documents::StoredDocumentsPorts;
pub(crate) use institutions::StoredInstitutionsPorts;
pub(crate) use inventory::StoredInventoryPorts;
pub(crate) use payees::StoredPayeesPorts;
pub(crate) use tags::StoredTagsPorts;
pub(crate) use transactions::StoredTransactionsPorts;
pub(crate) use units::StoredUnitsPorts;

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::mpsc;

use lib_core::Clock;

use crate::adapter::store::{
    SharedStores, StoredAccountReads, StoredBillReads, StoredBillUnits, StoredBudgetReads,
    StoredCategoryBudgetMonths, StoredCategoryDeletionReads, StoredDashboardReads,
    StoredDocumentReads, StoredImportReads, StoredInstitutionReads, StoredInventoryReads,
    StoredPayeeReads, StoredShellReads, StoredTagReads, StoredTagRemovalReads,
    StoredTransactionReads, StoredUnitReads,
};
use crate::reversal::{Journal, LedgerReversal};
use crate::{
    AccountDeletionAdapter, BillPaymentsAdapter, CategoryDeletionAdapter, Commit, Domain,
    LedgerPreferencesAdapter, Result, SharedLedger, StoredCategoryRows, StoredSettingsReads,
    Stores, TagRemovalAdapter, TransactionImportAdapter, UnitDeletionAdapter, WriteFailure,
};
use lib_shell::Step;

/// A Ledger as a Client holds it: every domain's ports over the shared stores, the cross-domain
/// adapters and the commit every input ends in. The channel a failed commit is reported on is
/// handed out beside it when it is built, so a Client polls it without taking it out of here.
pub struct LedgerPorts {
    /// Every domain's store, shared: the bottom layer the ports, reads and adapters hold.
    stores: SharedStores,
    /// The Units page's, its Dialogs' and the Ledger settings page's ports ([`StoredUnitsPorts`]).
    units: lib_units::SharedPorts,
    /// Unit delete's atomic port over the Units and Institutions stores ([`UnitDeletionAdapter`]).
    unit_deletion: lib_unit_deletion::SharedUnitDeletion,
    /// The Institutions page's and its Dialogs' ports ([`StoredInstitutionsPorts`]).
    institutions: lib_institutions::SharedPorts,
    /// The Accounts page's and its Dialogs' ports ([`StoredAccountsPorts`]).
    accounts: lib_accounts::SharedPorts,
    /// What names an Account and the Units it may count in, over the stores: what the Delete
    /// account Dialog quotes when it opens.
    account_reads: Rc<StoredAccountReads>,
    /// Account delete's atomic port over the Accounts, Transactions and Bills stores
    /// ([`AccountDeletionAdapter`]).
    account_deletion: lib_account_deletion::SharedDeletion,
    /// The Payees page's and its Dialogs' ports ([`StoredPayeesPorts`]).
    payees: lib_payees::SharedPorts,
    /// The Bills page's and its Dialogs' ports ([`StoredBillsPorts`]).
    bills: lib_bills::SharedPorts,
    /// The Tags page's and its Add and Edit Dialogs' ports ([`StoredTagsPorts`]).
    tags: lib_tags::SharedPorts,
    /// Tag remove and merge's atomic port over the Tags and Transactions stores
    /// ([`TagRemovalAdapter`]).
    tag_removal: lib_tag_removal::SharedTagRemoval,
    /// What the Remove tag and Merge tags Dialogs show when they open.
    tag_removal_reads: Rc<StoredTagRemovalReads>,
    /// The Categories page's repository ([`StoredCategoriesPorts`]).
    categories: lib_categories::SharedCategories,
    /// The same ports for the Add and Edit category Dialogs' save: the usage, the Monthly budget
    /// and the handover. A value of its own over the same stores, so a Presenter holding the
    /// repository above never borrows it while the Dialog writes through this.
    category_save: StoredCategoriesPorts,
    /// Category delete's atomic port over the Categories, Transactions, Bills, Payees and Budgets
    /// stores ([`CategoryDeletionAdapter`]).
    category_deletion: lib_category_deletion::SharedCategoryDeletion,
    /// What the Delete category Dialog shows when it opens.
    category_deletion_reads: Rc<StoredCategoryDeletionReads>,
    /// The months the Categories' Monthly budgets have a record in, over the Budgets store.
    category_budget_months: Rc<StoredCategoryBudgetMonths>,
    /// The Transactions page's ports ([`StoredTransactionsPorts`]).
    transactions: lib_transactions::SharedPorts,
    /// The Import's atomic port over the Payees and Transactions stores, which is also its Payees
    /// read ([`TransactionImportAdapter`]).
    transaction_import: Rc<RefCell<TransactionImportAdapter>>,
    /// The Accounts and Categories the Import's step offers.
    import_reads: Rc<StoredImportReads>,
    /// Pay it directly's atomic port over the Transactions and Bills stores
    /// ([`BillPaymentsAdapter`]).
    bill_payments: lib_bill_payments::SharedPayments,
    /// The Budgets page's and its Dialogs' ports ([`StoredBudgetsPorts`]).
    budgets: lib_budgets::SharedPorts,
    /// The Inventory page's and its Dialogs' repository ([`StoredInventoryPorts`]).
    inventory: lib_inventory::SharedInventory,
    /// The Documents and Document Types pages' repository ([`StoredDocumentsPorts`]).
    documents: lib_documents::SharedRepository,
    /// What the Inventory page and its Dialogs read the Units and Documents through.
    inventory_reads: Rc<StoredInventoryReads>,
    /// What the Documents page reads the other domains through.
    document_reads: Rc<StoredDocumentReads>,
    /// Settings' reads ([`StoredSettingsReads`]): the Ledger page's Units and the X-Large index
    /// pane's counts.
    settings_reads: Rc<StoredSettingsReads>,
    /// The Settings › Ledger page's base Unit write ([`LedgerPreferencesAdapter`]).
    ledger_preferences: Rc<LedgerPreferencesAdapter>,
    /// The Dashboard's Bills and Budgets reads ([`StoredDashboardReads`]).
    dashboard_reads: Rc<StoredDashboardReads>,
    /// The Shell's rail-count reads ([`StoredShellReads`]).
    shell_reads: Rc<StoredShellReads>,
    /// The one commit every input ends with, returning what it wrote ([`Committed`](crate::Committed)).
    commit: Rc<dyn Commit>,
    /// The row images the stores record each input's writes in, for History (#1247).
    journal: Journal,
}

impl LedgerPorts {
    /// Shares out each of `stores` once and builds every domain's ports and the cross-domain
    /// adapters over them. `clock` is what the Categories ports read the day from on each call.
    /// Returns them with the writer's failed writes, for the Client to reload and announce.
    pub fn new(stores: Stores, clock: Clock) -> (Self, mpsc::Receiver<WriteFailure>) {
        Self::over(stores.share(), clock)
    }

    /// Builds every domain's ports and the cross-domain adapters over `shared`, the stores a
    /// caller shared out itself so it keeps a handle on them: a test reading rows back (#1117), or
    /// the TUI until #805. A Client opening a Ledger file builds with [`LedgerPorts::new`] instead.
    /// Returns them with the writer's failed writes, for the Client to reload and announce.
    pub fn over(shared: SharedLedger, clock: Clock) -> (Self, mpsc::Receiver<WriteFailure>) {
        let SharedLedger {
            stores,
            journal,
            commit,
            failures,
        } = shared;
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
        let account_reads = Rc::new(StoredAccountReads {
            transactions: stores.transactions.clone(),
            budgets: stores.budgets.clone(),
            units: stores.units.clone(),
            institutions: stores.institutions.clone(),
        });
        let dashboard_reads = Rc::new(StoredDashboardReads {
            budgets: stores.budgets.clone(),
            budget_reads: (*budget_reads).clone(),
        });
        let shell_reads = Rc::new(StoredShellReads {
            budgets: stores.budgets.clone(),
            documents: stores.documents.clone(),
            budget_reads: (*budget_reads).clone(),
        });
        let ports = Self {
            inventory_reads: Rc::new(StoredInventoryReads {
                units: stores.units.clone(),
                inventory: stores.inventory.clone(),
                documents: stores.documents.clone(),
            }),
            document_reads: Rc::new(StoredDocumentReads {
                transactions: stores.transactions.clone(),
                payees: stores.payees.clone(),
                accounts: stores.accounts.clone(),
                institutions: stores.institutions.clone(),
                bills: stores.bills.clone(),
                inventory: stores.inventory.clone(),
            }),
            settings_reads: Rc::new(StoredSettingsReads::over(&stores)),
            ledger_preferences: Rc::new(LedgerPreferencesAdapter::new(
                stores.units.clone(),
                stores.ledger.clone(),
            )),
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
                (*account_reads).clone(),
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
            category_deletion_reads: Rc::new(StoredCategoryDeletionReads {
                categories: stores.categories.clone(),
                transactions: stores.transactions.clone(),
                budgets: stores.budgets.clone(),
            }),
            category_budget_months: Rc::new(StoredCategoryBudgetMonths {
                budgets: stores.budgets.clone(),
            }),
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
            tag_removal_reads: Rc::new(StoredTagRemovalReads {
                tags: stores.tags.clone(),
                transactions: stores.transactions.clone(),
            }),
            stores,
            journal,
            commit,
        };
        (ports, failures)
    }

    /// The units ports, a clone of the one value every page shares.
    #[must_use]
    pub fn units(&self) -> lib_units::SharedPorts {
        self.units.clone()
    }

    /// Unit delete's atomic port over the Units and Institutions stores, which the Delete unit
    /// Dialog writes through.
    #[must_use]
    pub fn unit_deletion(&self) -> lib_unit_deletion::SharedUnitDeletion {
        self.unit_deletion.clone()
    }

    /// Pay it directly's atomic port over the Transactions and Bills stores, which the Pay Dialog
    /// writes through.
    #[must_use]
    pub fn bill_payments(&self) -> lib_bill_payments::SharedPayments {
        self.bill_payments.clone()
    }

    /// The Dashboard's Bills and Budgets reads, as the trait object its Presenter reads.
    #[must_use]
    pub fn dashboard_reads(&self) -> Rc<dyn lib_dashboard::DashboardReads> {
        self.dashboard_reads.clone()
    }

    /// The institutions ports, a clone of the one value every page shares.
    #[must_use]
    pub fn institutions(&self) -> lib_institutions::SharedPorts {
        self.institutions.clone()
    }

    /// The accounts ports, a clone of the one value every page shares.
    #[must_use]
    pub fn accounts(&self) -> lib_accounts::SharedPorts {
        self.accounts.clone()
    }

    /// The payees ports, a clone of the one value every page shares.
    #[must_use]
    pub fn payees(&self) -> lib_payees::SharedPorts {
        self.payees.clone()
    }

    /// The bills ports, a clone of the one value every page shares.
    #[must_use]
    pub fn bills(&self) -> lib_bills::SharedPorts {
        self.bills.clone()
    }

    /// What Settings › Ledger and the Settings index read and write through, as `lib-settings`'
    /// own bundle of trait objects.
    #[must_use]
    pub fn settings_ledger(&self) -> lib_settings::SettingsLedger {
        lib_settings::SettingsLedger {
            units: self.settings_reads.clone(),
            summaries: self.settings_reads.clone(),
            preferences: self.ledger_preferences.clone(),
        }
    }

    /// The Shell's rail-count reads, as the trait object its Presenter reads.
    #[must_use]
    pub fn shell_rail(&self) -> Rc<dyn lib_shell::ShellRailCounts> {
        self.shell_reads.clone()
    }

    /// What the Delete account Dialog reads and writes through, as `lib-account-deletion`'s own
    /// bundle: the deletion port and the reads it quotes.
    #[must_use]
    pub fn account_deletion(&self) -> lib_account_deletion::DeletionPorts {
        lib_account_deletion::DeletionPorts {
            deletion: self.account_deletion.clone(),
            reads: self.account_reads.clone(),
        }
    }

    /// What the Remove tag and Merge tags Dialogs write and read through, as `lib-tag-removal`'s
    /// own bundle: the removal port and the reads they show when they open.
    #[must_use]
    pub fn tag_removal(&self) -> lib_tag_removal::TagRemovalPorts {
        lib_tag_removal::TagRemovalPorts {
            removal: self.tag_removal.clone(),
            reads: self.tag_removal_reads.clone(),
        }
    }

    /// The Categories page's repository, a clone of the one value the page and its Dialogs share.
    #[must_use]
    pub fn categories(&self) -> lib_categories::SharedCategories {
        self.categories.clone()
    }

    /// The Add and Edit category Dialogs' save ports: the usage, the Monthly budget and the
    /// handover. A new value over the same stores on each call, so a Presenter holding the
    /// repository from [`Self::categories`] never borrows it while a Dialog writes through these.
    #[must_use]
    pub fn category_save(&self) -> lib_categories::SharedSavePorts {
        Rc::new(RefCell::new(self.category_save.clone()))
    }

    /// The months the Categories' Monthly budgets have a record in, over the Budgets store
    /// ([`StoredCategoryBudgetMonths`]).
    #[must_use]
    pub fn category_budget_months(&self) -> lib_categories::SharedBudgetMonths {
        self.category_budget_months.clone()
    }

    /// What the Delete category Dialog reads and writes through, as `lib-category-deletion`'s own
    /// bundle: the deletion port and the reads it shows.
    #[must_use]
    pub fn category_deletion(&self) -> lib_category_deletion::CategoryDeletionPorts {
        lib_category_deletion::CategoryDeletionPorts {
            deletion: self.category_deletion.clone(),
            reads: self.category_deletion_reads.clone(),
        }
    }

    /// What the Import step writes and reads through, as `lib-transaction-import`'s own bundle: the
    /// import port and its Payees read over the one adapter, and the Accounts and Categories reads.
    /// The caller dates `statement` (the seeded one, from the Clock).
    #[must_use]
    pub fn transaction_import(
        &self,
        statement: Rc<dyn lib_transaction_import::StatementSource>,
    ) -> lib_transaction_import::ImportPorts {
        lib_transaction_import::ImportPorts {
            import: self.transaction_import.clone(),
            payees: self.transaction_import.clone(),
            accounts: self.import_reads.clone(),
            categories: self.import_reads.clone(),
            statement,
        }
    }

    /// What the Inventory page and its Dialogs read the Units and Documents through, as
    /// `lib-inventory`'s own read traits.
    #[must_use]
    pub fn inventory_reads(&self) -> lib_inventory::SharedReads {
        self.inventory_reads.clone()
    }

    /// What the Documents page reads the other domains through, as `lib-documents`' own read
    /// traits.
    #[must_use]
    pub fn document_reads(&self) -> lib_documents::SharedReads {
        self.document_reads.clone()
    }

    /// The tags ports, a clone of the one value every page shares.
    #[must_use]
    pub fn tags(&self) -> lib_tags::SharedPorts {
        self.tags.clone()
    }

    /// The transactions ports, a clone of the one value every page shares.
    #[must_use]
    pub fn transactions(&self) -> lib_transactions::SharedPorts {
        self.transactions.clone()
    }

    /// The budgets ports, a clone of the one value every page shares.
    #[must_use]
    pub fn budgets(&self) -> lib_budgets::SharedPorts {
        self.budgets.clone()
    }

    /// The inventory ports, a clone of the one value every page shares.
    #[must_use]
    pub fn inventory(&self) -> lib_inventory::SharedInventory {
        self.inventory.clone()
    }

    /// The documents ports, a clone of the one value every page shares.
    #[must_use]
    pub fn documents(&self) -> lib_documents::SharedRepository {
        self.documents.clone()
    }

    /// The one commit every input ends with, for the Client's input door to hold.
    pub fn commit(&self) -> Rc<dyn Commit> {
        self.commit.clone()
    }

    /// The shared stores, for the tests that write a row the ports don't reach.
    #[cfg(test)]
    pub(crate) fn stores(&self) -> &SharedStores {
        &self.stores
    }

    /// What the input's writes changed, as the row images that undo and redo it, leaving none for
    /// the next input: `None` where it changed no row. The Client takes it once per input, as the
    /// input closes into a History entry.
    pub fn take_reversal(&self) -> Option<LedgerReversal> {
        self.journal.take()
    }

    /// Writes `reversal`'s rows the way `step` goes, recording nothing: Undo puts them back as they
    /// were before it, the newest first, and Redo writes them as it left them again, in the order
    /// first written. The writes wait for the input's commit like any other.
    pub fn step(&self, reversal: &LedgerReversal, step: Step) {
        crate::reversal::apply(reversal, step, &self.stores, &self.journal);
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

    pub(crate) fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 9, 19).expect("a real date")
    }

    /// The Sample Ledger in memory, as the UI tests build the Client on it.
    pub fn fakes() -> LedgerPorts {
        LedgerPorts::new(
            Stores::fake(FakeStores::sample(today())).0,
            Clock::fixed(today()),
        )
        .0
    }

    /// The Sample Ledger created as a Ledger file in `dir`, opened as a Client opens it.
    pub fn file(dir: &Path) -> LedgerPorts {
        let path = dir.join("ledger.pldb");
        seed::sample_ledger(&path, today()).expect("the Sample Ledger is created");
        LedgerPorts::new(
            Stores::from(open_storage(&path).expect("the Ledger opens")),
            Clock::fixed(today()),
        )
        .0
    }
}
