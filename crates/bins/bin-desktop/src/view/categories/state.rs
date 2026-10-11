//! The Categories destination's Entities (ADR-0032). [`CategoriesStore`] reads the tree through
//! [`LedgerStore`], which holds the only copy; [`CategoriesView`] owns the Settings tree's cursor, its
//! selected id and which nodes are expanded. Writes are Ledger commands (`lib_ledger`), run through
//! `LedgerStore::apply`; `Shell` opens the dialogs.

use gpui::{App, Context, Entity, EventEmitter, Subscription};
use lib_categories::Category;
use lib_ledger::Domain;

use crate::ledger_store::{LedgerChanged, LedgerStore};

/// Emitted by [`CategoriesStore`] after a write lands, so `Shell` can refresh the Views that read
/// the tree (ADR-0032's View events).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CategoriesEvent {
    Changed,
}

/// The shared Categories tree. Budgets, Bills, Transactions, Documents, Payees and Settings all
/// read it from here, so a change is seen everywhere on the next render.
pub struct CategoriesStore {
    ledger: Entity<LedgerStore>,
    _ledger_changed: Subscription,
}

impl EventEmitter<CategoriesEvent> for CategoriesStore {}

impl CategoriesStore {
    /// A store over `ledger`'s Categories.
    pub fn new(ledger: Entity<LedgerStore>, cx: &mut Context<'_, Self>) -> Self {
        let _ledger_changed = cx.subscribe(&ledger, |_, _, event: &LedgerChanged, cx| {
            if event.what.contains(&Domain::Categories) {
                cx.emit(CategoriesEvent::Changed);
            }
        });
        Self {
            ledger,
            _ledger_changed,
        }
    }

    pub fn categories<'a>(&self, cx: &'a App) -> &'a [Category] {
        self.ledger.read(cx).ledger().categories()
    }
}

/// The Settings Categories tree's own state. `selected` is a position in the depth-first rows;
/// `selected_id` is the Category the cursor is on; `expanded` lists the parents shown open.
pub struct CategoriesView {
    selected: usize,
    selected_id: Option<u32>,
    expanded: Vec<u32>,
}

impl Default for CategoriesView {
    fn default() -> Self {
        Self {
            selected: 0,
            selected_id: None,
            // Housing, Utilities, Food expanded by default
            expanded: vec![1, 3, 6],
        }
    }
}

impl CategoriesView {
    pub fn new() -> Self {
        Self::default()
    }

    /// The stored row position. Clamp it before use: removing Categories can leave it past the end.
    pub fn selected(&self) -> usize {
        self.selected
    }

    /// The Category the cursor is on. It may name a Category that no longer exists, so read it
    /// through the Shell's selection rule.
    pub fn selected_id(&self) -> Option<u32> {
        self.selected_id
    }

    pub fn expanded(&self) -> &[u32] {
        &self.expanded
    }

    pub fn set_selected(&mut self, selected: usize, cx: &mut Context<'_, Self>) {
        self.selected = selected;
        cx.notify();
    }

    pub fn set_selected_id(&mut self, id: Option<u32>, cx: &mut Context<'_, Self>) {
        self.selected_id = id;
        cx.notify();
    }

    /// Edits the expanded list in place, for the fold and disclosure rules to run on.
    pub fn update_expanded(
        &mut self,
        cx: &mut Context<'_, Self>,
        edit: impl FnOnce(&mut Vec<u32>),
    ) {
        edit(&mut self.expanded);
        cx.notify();
    }
}

#[cfg(test)]
mod tests {
    use std::{cell::RefCell, rc::Rc};

    use gpui::{AppContext, Entity, TestAppContext};
    use lib_core::CategoryTypes;
    use lib_ledger::AddCategory;

    use super::*;

    /// A `CategoriesStore` over the seeded tree, with every event it emits collected.
    fn seeded_store(
        cx: &mut TestAppContext,
    ) -> (
        Entity<LedgerStore>,
        Entity<CategoriesStore>,
        Rc<RefCell<Vec<CategoriesEvent>>>,
    ) {
        let ledger = crate::ledger_store::test_ledger(
            cx,
            lib_ledger::LedgerParts {
                categories: lib_categories::default_categories(),
                ..lib_ledger::LedgerParts::default()
            },
        );
        let store = cx.new(|cx| CategoriesStore::new(ledger.clone(), cx));
        let events = Rc::new(RefCell::new(Vec::new()));
        let sink = events.clone();
        cx.update(|cx| {
            cx.subscribe(&store, move |_, event: &CategoriesEvent, _| {
                sink.borrow_mut().push(*event)
            })
            .detach();
        });
        (ledger, store, events)
    }

    fn add(ledger: &Entity<LedgerStore>, cx: &mut TestAppContext, parent: u32, name: &str) -> bool {
        ledger.update(cx, |store, cx| {
            store
                .apply(
                    cx,
                    AddCategory {
                        name: name.into(),
                        parent_id: Some(parent),
                        category_type: CategoryTypes::Expense,
                        monthly: None,
                    },
                )
                .is_ok()
        })
    }

    #[gpui::test]
    fn a_landed_write_emits_changed_and_a_refused_one_does_not(cx: &mut TestAppContext) {
        let (ledger, _store, events) = seeded_store(cx);

        // Salary (11) is Income, so an Expense child under it is refused.
        assert!(!add(&ledger, cx, 11, "Bonus"));
        cx.run_until_parked();
        assert!(events.borrow().is_empty());

        assert!(add(&ledger, cx, 6, "Snacks"));
        cx.run_until_parked();
        assert_eq!(*events.borrow(), vec![CategoriesEvent::Changed]);
    }
}
