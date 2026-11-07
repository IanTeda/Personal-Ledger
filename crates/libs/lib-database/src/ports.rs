//! What a Client holds a Ledger by (#1076): [`LedgerPorts`], built once from [`Stores`], with one
//! `Stored<Domain>Ports` per domain implementing that domain crate's own ports trait, the
//! cross-domain adapters, the module reads, the commit and the failure channel. A Client hands a
//! Presenter the domain crate's shared ports straight from here, with no adapter of its own.
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
//! A domain joining it adds its `Stored<Domain>Ports` in a file of its own under `ports/`, a field
//! here holding it as the domain crate's `SharedPorts`, and a line in [`LedgerPorts::new`] building
//! it over [`SharedStores`]. The domains not yet migrated are built by the Clients from
//! [`LedgerPorts::stores`] (#1083, #1084).

mod tags;

pub use tags::StoredTagsPorts;

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::mpsc;

use crate::adapter::store::{SharedStores, StoredTagReads, StoredTagRemovalReads};
use crate::{Commit, Domain, Result, Stores, TagRemovalAdapter, WriteFailure};

/// A Ledger as a Client holds it: every domain's ports over the shared stores, the cross-domain
/// adapters, the commit every input ends in and the channel a failed commit is reported on.
pub struct LedgerPorts {
    /// Every domain's store, shared: the bottom layer the ports, reads and adapters hold.
    pub stores: SharedStores,
    /// The Tags page's and its Add and Edit Dialogs' ports ([`StoredTagsPorts`]).
    pub tags: lib_tags::SharedPorts,
    /// Tag remove and merge's atomic port over the Tags and Transactions stores
    /// ([`TagRemovalAdapter`]).
    pub tag_removal: lib_tag_removal::SharedTagRemoval,
    /// What the Remove tag and Merge tags Dialogs show when they open.
    pub tag_removal_reads: StoredTagRemovalReads,
    pub commit: Rc<dyn Commit>,
    pub failures: mpsc::Receiver<WriteFailure>,
}

impl LedgerPorts {
    /// Shares out each of `stores` once and builds every domain's ports and the cross-domain
    /// adapters over them.
    pub fn new(stores: Stores) -> Self {
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
        Self {
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

impl From<Stores> for LedgerPorts {
    fn from(stores: Stores) -> Self {
        Self::new(stores)
    }
}

/// The Sample Ledger as a test's [`LedgerPorts`]: in memory, and as a Ledger file.
#[cfg(test)]
pub(crate) mod test_support {
    use std::path::Path;

    use chrono::NaiveDate;

    use crate::{FakeStores, LedgerPorts, Stores, open_storage, seed};

    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 9, 19).expect("a real date")
    }

    /// The Sample Ledger in memory, as the UI tests build the Client on it.
    pub fn fakes() -> LedgerPorts {
        LedgerPorts::new(Stores::fake(FakeStores::sample(today())).0)
    }

    /// The Sample Ledger created as a Ledger file in `dir`, opened as a Client opens it.
    pub fn file(dir: &Path) -> LedgerPorts {
        let path = dir.join("ledger.pldb");
        seed::sample_ledger(&path, today()).expect("the Sample Ledger is created");
        LedgerPorts::new(Stores::from(open_storage(&path).expect("the Ledger opens")))
    }
}
