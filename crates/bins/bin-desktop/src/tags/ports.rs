//! The Tags ports as the Desktop holds them (#862, #863): the store `shell::router::build` is
//! given, shared by the Tags module, Tag remove and merge (`crate::tag_removal`) and the
//! Transactions' reads. The writes are committed by the input they were made in
//! (`crate::input::send`), not here.
//!
//! The Tags page writes the repository through `lib-tags`' Interactors; Tag remove and merge
//! rewrite Splits too, so they write through `lib-tag-removal`'s atomic port (#1003). Neither keeps
//! a Ledger read copy of the Tags: a write announces `Domain::Tags` ([`announce`]), and what carries
//! each Tag is counted in the other domains' stores ([`StoredTagReads`]).

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use gpui::{App, Entity};
use lib_core::TagId;
use lib_database::{SharedTagsStore, StoredTagReads};
use lib_tags::fake::{InMemoryPorts, InMemoryTagUsage, InMemoryTags};
use lib_tags::{SharedPorts, Tag, TagRepository, TagUsage, TagUse};

use crate::ledger_store::LedgerStore;

/// The stub Tags in memory, for a test that needs no storage. What carries them is read through
/// [`StoredTagReads`].
pub fn in_memory() -> SharedTagsStore {
    Rc::new(RefCell::new(InMemoryPorts {
        tags: InMemoryTags {
            tags: lib_tags::default_tags(),
        },
        usage: InMemoryTagUsage::default(),
    }))
}

/// The shared store as the module's Presenters hold the ports, borrowed for each call: the store
/// is shared behind a `RefCell` whose guard can't be lent out of it.
///
/// The Tags are the store's; what carries them is counted by `usage` in the other domains' stores
/// (#994).
pub struct StorePorts {
    store: SharedTagsStore,
    usage: StoredTagReads,
}

impl TagRepository for StorePorts {
    fn tags(&self) -> Vec<Tag> {
        self.store.borrow().tags().tags()
    }

    fn upsert_tag(&mut self, tag: Tag) {
        self.store.borrow_mut().tags_mut().upsert_tag(tag);
    }
}

impl TagUsage for StorePorts {
    fn tag_usage(&self, id: TagId) -> TagUse {
        self.usage.tag_usage(id)
    }

    fn transactions_by_tag(&self, ids: &[TagId]) -> HashMap<TagId, usize> {
        self.usage.transactions_by_tag(ids)
    }
}

/// `store` as the Tags module's Presenters share it.
pub fn shared(store: &SharedTagsStore, usage: StoredTagReads) -> SharedPorts {
    Rc::new(RefCell::new(StorePorts {
        store: store.clone(),
        usage,
    }))
}

/// Announces `Domain::Tags`, after a write to the Tags store, so the Tags page, the Transactions
/// and the Shell redraw.
pub fn announce(ledger: &Entity<LedgerStore>, cx: &mut App) {
    ledger.update(cx, |store, cx| store.announce_tags(cx));
}

/// Puts the Tags back as stored, after a write that named them failed, and announces them so
/// every reader redraws (ADR-0046).
///
/// # Errors
/// Returns the error reading them back; the store is left as it was.
pub fn reload(ledger: &Entity<LedgerStore>, cx: &mut App) -> lib_database::Result<()> {
    let store = ledger.read(cx).tags_store().clone();
    store.borrow_mut().reload()?;
    announce(ledger, cx);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn usage_is_counted_from_the_ledgers_transactions() {
        let shared =
            lib_tags::find_by_name(&lib_tags::default_tags(), "shared").expect("a stub Tag");
        let setup = crate::ledger_store::LedgerSetup::stub(today());
        let ports = StorePorts {
            store: in_memory(),
            usage: StoredTagReads {
                transactions: setup.transactions.clone(),
                accounts: setup.accounts.clone(),
                units: setup.units.clone(),
            },
        };

        let used = ports.tag_usage(shared);

        assert!(used.transactions > 0, "the stub's Splits carry it");
        assert_eq!(ports.tag_usage(TagId::seed(999)), TagUse::default());
    }

    fn today() -> chrono::NaiveDate {
        chrono::NaiveDate::from_ymd_opt(2026, 10, 11).expect("a real date")
    }
}
