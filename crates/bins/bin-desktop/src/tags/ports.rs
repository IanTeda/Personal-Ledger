//! The Tags ports as the Desktop holds them (#862, #863): the store `shell::router::build` is
//! given, shared by the Tags module and the Ledger's forwarding shim and read copy. The writes are
//! committed by the input they were made in (`crate::input::send`), not here.
//!
//! The Tags page writes the repository through `lib-tags`' Interactors. Tag remove and merge
//! rewrite Splits, so they are the reassign port, answered from the Ledger in [`LedgerPorts`] by
//! running `lib_ledger::RemoveTag` and `MergeTags` through `LedgerStore::apply`, whose forwarding
//! shims carry the writes to the Transactions and Tags stores in the one commit (Q6, until
//! `lib-reassign`, #813). [`forward_deletes`] is the Tags shim.
//!
//! An in-memory store's usage is counted from the Ledger's Transactions (`LedgerStore` recounts it
//! after each write), since a port can't reach the `LedgerStore` Entity without a gpui context. The
//! stored Desktop's usage is the adapter's, read from the file.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use gpui::{App, Entity};
use lib_core::TagId;
use lib_database::{SharedTagsStore, TagsStore};
use lib_ledger::{Ledger, LedgerChange, MergeTags, RemoveTag};
use lib_tags::fake::{InMemoryPorts, InMemoryTagUsage, InMemoryTags};
use lib_tags::{
    SharedPorts, Tag, TagError, TagMerged, TagReassign, TagRemoved, TagRepository, TagUsage, TagUse,
};

use crate::ledger_store::LedgerStore;

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

/// The shared store as the module's Presenters hold the ports, borrowed for each call: the store
/// is shared behind a `RefCell` whose guard can't be lent out of it.
pub struct StorePorts(SharedTagsStore);

impl TagRepository for StorePorts {
    fn tags(&self) -> Vec<Tag> {
        self.0.borrow().tags().tags()
    }

    fn upsert_tag(&mut self, tag: Tag) {
        self.0.borrow_mut().tags_mut().upsert_tag(tag);
    }
}

impl TagUsage for StorePorts {
    fn tag_usage(&self, id: TagId) -> TagUse {
        self.0.borrow().usage().tag_usage(id)
    }

    fn transactions_by_tag(&self, ids: &[TagId]) -> HashMap<TagId, usize> {
        self.0.borrow().usage().transactions_by_tag(ids)
    }
}

/// `store` as the Tags module's Presenters share it.
pub fn shared(store: &SharedTagsStore) -> SharedPorts {
    Rc::new(RefCell::new(StorePorts(store.clone())))
}

/// The forwarding shim: sends each Tag `changes` delete to `store`, returning whether it wrote
/// anything. Run after the Transactions, since the Splits carrying the Tag must be untagged or
/// retagged before it goes (their foreign key). Writer carried: Tag remove and merge, until
/// `lib-reassign` (#813); the Tags page writes the rows itself.
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

/// Brings the Ledger read copy of the Tags in line with the store, announcing `Domain::Tags` when
/// it changed. Run before and after every Tags event and Dialog confirm, so the Transactions and
/// the filters see a write at once, and a remove runs on the rows the page shows.
pub fn refresh(ledger: &Entity<LedgerStore>, cx: &mut App) {
    ledger.update(cx, |store, cx| store.refresh_tags(cx));
}

/// The Tags reassign port the Desktop answers from its Ledger, built for one call around its gpui
/// context: Tag remove and merge.
pub struct LedgerPorts<'a> {
    pub ledger: &'a Entity<LedgerStore>,
    pub cx: &'a mut App,
}

fn name_of(ledger: &Ledger, id: TagId) -> Result<String, TagError> {
    lib_tags::get(ledger.tags(), id)
        .map(|tag| tag.name.clone())
        .ok_or(TagError::NotFound)
}

impl TagReassign for LedgerPorts<'_> {
    /// Today's `LedgerCommand`, on the read copy brought in line with the store first. The name is
    /// read before the write takes it.
    fn remove_tag(&mut self, id: TagId) -> Result<TagRemoved, TagError> {
        self.ledger.update(self.cx, |store, cx| {
            store.refresh_tags(cx);
            let name = name_of(store.ledger(), id)?;
            let untagged = store.apply(cx, RemoveTag(id))?;
            Ok(TagRemoved { name, untagged })
        })
    }

    fn merge_tags(&mut self, source: TagId, target: TagId) -> Result<TagMerged, TagError> {
        self.ledger.update(self.cx, |store, cx| {
            store.refresh_tags(cx);
            let source_name = name_of(store.ledger(), source)?;
            let target_name = name_of(store.ledger(), target)?;
            let retagged = store.apply(cx, MergeTags { source, target })?;
            Ok(TagMerged {
                source_name,
                target_name,
                retagged,
            })
        })
    }
}

#[cfg(test)]
mod tests {
    use gpui::{AppContext as _, TestAppContext};
    use lib_core::CategoryId;

    use super::*;

    #[test]
    fn tag_deletes_reach_the_store() {
        let ledger = Ledger::stub();
        let store = in_memory(&ledger);
        let gone = TagId::seed(7);
        let changes = [
            LedgerChange::DeleteTag(gone),
            LedgerChange::DeleteCategory(CategoryId::seed(8)),
        ];

        assert!(forward_deletes(&mut *store.borrow_mut(), &changes));

        assert!(store.borrow().tags().tag(gone).is_none());
        assert!(!forward_deletes(
            &mut InMemoryPorts::default(),
            &changes[1..]
        ));
    }

    #[test]
    fn an_in_memory_store_counts_the_ledgers_transactions() {
        let ledger = Ledger::stub();
        let shared = lib_tags::find_by_name(ledger.tags(), "shared").expect("a stub Tag");

        let used = StorePorts(in_memory(&ledger)).tag_usage(shared);

        assert!(used.transactions > 0, "the stub's Splits carry it");
        assert_eq!(used, ledger.tag_uses()[&shared]);
        assert_eq!(
            StorePorts(in_memory(&ledger)).tag_usage(TagId::seed(999)),
            TagUse::default()
        );
    }

    fn today() -> chrono::NaiveDate {
        chrono::NaiveDate::from_ymd_opt(2026, 10, 11).expect("a real date")
    }

    #[gpui::test]
    fn the_reassign_port_untags_and_removes_through_the_ledger(cx: &mut TestAppContext) {
        let ledger = cx.new(|_| LedgerStore::new(Ledger::stub_on(today())));
        let shared = lib_tags::find_by_name(
            ledger
                .read_with(cx, |store, _| store.ledger().tags().to_vec())
                .as_slice(),
            "shared",
        )
        .expect("seeded");

        let removed = cx.update(|cx| {
            LedgerPorts {
                ledger: &ledger,
                cx,
            }
            .remove_tag(shared)
        });

        let removed = removed.expect("a stub Tag");
        assert_eq!(removed.name, "shared");
        assert!(removed.untagged > 0, "the stub's Splits carry it");
        ledger.read_with(cx, |store, _| {
            assert!(lib_tags::get(store.ledger().tags(), shared).is_none());
        });
        let gone = cx.update(|cx| {
            LedgerPorts {
                ledger: &ledger,
                cx,
            }
            .remove_tag(shared)
        });
        assert_eq!(gone, Err(TagError::NotFound));
    }

    #[gpui::test]
    fn the_reassign_port_retags_a_merge_and_names_both_tags(cx: &mut TestAppContext) {
        let ledger = cx.new(|_| LedgerStore::new(Ledger::stub_on(today())));
        let tags = ledger.read_with(cx, |store, _| store.ledger().tags().to_vec());
        let source = lib_tags::find_by_name(&tags, "Work Trip").expect("seeded");
        let target = lib_tags::find_by_name(&tags, "work-trip").expect("seeded");

        let merged = cx.update(|cx| {
            LedgerPorts {
                ledger: &ledger,
                cx,
            }
            .merge_tags(source, target)
        });

        let merged = merged.expect("two stub Tags");
        assert_eq!(
            (merged.source_name.as_str(), merged.target_name.as_str()),
            ("Work Trip", "work-trip")
        );
        ledger.read_with(cx, |store, _| {
            assert!(lib_tags::get(store.ledger().tags(), source).is_none());
        });
        let itself = cx.update(|cx| {
            LedgerPorts {
                ledger: &ledger,
                cx,
            }
            .merge_tags(target, target)
        });
        assert_eq!(itself, Err(TagError::MergeIntoItself));
    }
}
