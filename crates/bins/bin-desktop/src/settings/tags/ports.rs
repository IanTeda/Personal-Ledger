//! The Tags ports as the Desktop holds them (#862): the store `shell::router::build` is given,
//! shared by the Ledger's forwarding shim and read copy and the Tags page and its Dialogs, which
//! read each Tag's usage from it. The writes are committed by the input they were made in
//! (`crate::input::send`), not here.
//!
//! The Tags page still writes through `lib_ledger`'s Tag commands (#863), and Tag delete and merge,
//! which rewrite Splits, stay Ledger commands until `lib-reassign` (#813): [`forward`] and
//! [`forward_deletes`] are the Tags shim's two halves, carrying their changes to the store.
//!
//! An in-memory store's usage is counted from the Ledger's Transactions (`LedgerStore` recounts it
//! after each write), since a port can't reach the `LedgerStore` Entity without a gpui context. The
//! stored Desktop's usage is the adapter's, read from the file.

use std::cell::RefCell;
use std::rc::Rc;

use lib_core::TagId;
use lib_database::{SharedTagsStore, TagsStore};
use lib_ledger::{Ledger, LedgerChange};
use lib_tags::fake::{InMemoryPorts, InMemoryTagUsage, InMemoryTags};
use lib_tags::{TagUsage, TagUse};

/// `ledger`'s own Tags in memory, their usage counted from its Transactions, for a test that
/// needs no storage.
pub fn in_memory(ledger: &Ledger) -> SharedTagsStore {
    Rc::new(RefCell::new(InMemoryPorts {
        tags: InMemoryTags {
            tags: ledger.tags().to_vec(),
        },
        usage: InMemoryTagUsage {
            uses: ledger.tag_uses(),
        },
    }))
}

/// The shared store as the page and its Dialogs read usage, borrowed for each read: the store is
/// shared behind a `RefCell` whose guard can't be lent out of it.
pub struct StoreUsage(SharedTagsStore);

impl TagUsage for StoreUsage {
    fn tag_usage(&self, id: TagId) -> TagUse {
        self.0.borrow().usage().tag_usage(id)
    }
}

/// `store`'s usage figures, for the page and its Dialogs.
pub fn usage(store: &SharedTagsStore) -> StoreUsage {
    StoreUsage(store.clone())
}

/// The forwarding shim's first half (#862): sends each Tag row `changes` upserts to `store`, in the
/// order recorded, returning whether it wrote anything. Run before the Transactions, since an
/// Import commit can add a Tag and book Splits carrying it in one command, and the Tags page's
/// Add and Edit write through the Ledger until it writes the port itself (#863).
pub fn forward(store: &mut dyn TagsStore, changes: &[LedgerChange]) -> bool {
    let port = store.tags_mut();
    let mut wrote = false;
    for change in changes {
        if let LedgerChange::UpsertTag(tag) = change {
            port.upsert_tag(tag.clone());
            wrote = true;
        }
    }
    wrote
}

/// The forwarding shim's second half: sends each Tag `changes` delete to `store`, returning whether
/// it wrote anything. Run after the Transactions, since the Splits carrying the Tag must be untagged
/// or retagged before it goes (their foreign key). Writer carried: Tag delete and merge, until
/// `lib-reassign` (#813).
pub fn forward_deletes(store: &mut dyn TagsStore, changes: &[LedgerChange]) -> bool {
    let mut wrote = false;
    for change in changes {
        if let LedgerChange::DeleteTag(id) = change {
            store.delete_tag(*id);
            wrote = true;
        }
    }
    wrote
}

#[cfg(test)]
mod tests {
    use lib_categories::Category;
    use lib_core::CategoryId;

    use super::*;

    #[test]
    fn tag_upserts_and_deletes_reach_the_store_in_their_own_halves() {
        let ledger = Ledger::stub();
        let store = in_memory(&ledger);
        let mut shared = ledger.tags()[1].clone();
        shared.name = "split".into();
        let gone = TagId::seed(7);
        let changes = [
            LedgerChange::UpsertTag(shared.clone()),
            LedgerChange::DeleteTag(gone),
            LedgerChange::DeleteCategory(CategoryId::seed(8)),
            LedgerChange::UpsertCategory(Category {
                id: CategoryId::seed(8),
                name: "Other".into(),
                parent: None,
                category_type: lib_core::CategoryTypes::Expense,
            }),
        ];

        assert!(forward(&mut *store.borrow_mut(), &changes));
        assert!(store.borrow().tags().tag(gone).is_some(), "not yet");
        assert!(forward_deletes(&mut *store.borrow_mut(), &changes));

        let store = store.borrow();
        assert_eq!(store.tags().tag(shared.id), Some(shared));
        assert!(store.tags().tag(gone).is_none());
        assert!(!forward(&mut InMemoryPorts::default(), &changes[2..3]));
        assert!(!forward_deletes(
            &mut InMemoryPorts::default(),
            &changes[..1]
        ));
    }

    #[test]
    fn an_in_memory_store_counts_the_ledgers_transactions() {
        let ledger = Ledger::stub();
        let shared = lib_tags::find_by_name(ledger.tags(), "shared").expect("a stub Tag");

        let used = usage(&in_memory(&ledger)).tag_usage(shared);

        assert!(used.transactions > 0, "the stub's Splits carry it");
        assert_eq!(used, ledger.tag_uses()[&shared]);
        assert_eq!(
            usage(&in_memory(&ledger)).tag_usage(TagId::seed(999)),
            TagUse::default()
        );
    }
}
