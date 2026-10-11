//! The Categories destination's Entities (ADR-0032). [`CategoriesStore`] reads the tree through
//! [`LedgerStore`], which holds the only copy; [`CategoriesView`] owns the Settings tree's cursor, its
//! selected id and which nodes are expanded. `Shell` opens the dialogs and moves the Budgets and
//! Transactions with a delete, since those stores are `Shell`'s to coordinate.

use gpui::{App, Context, Entity, EventEmitter, Subscription};
use lib_categories::{
    Category, CategoryError, change_category_type, delete_category, edit_category, insert_category,
    move_category, prepare_delete,
};
use lib_core::CategoryTypes;
use lib_ledger::{Domain, LedgerHatch};

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

    /// Adds a Category and returns its id; the caller decides what to select.
    pub fn insert(
        &mut self,
        cx: &mut App,
        name: String,
        parent_id: Option<u32>,
        category_type: CategoryTypes,
    ) -> Result<u32, CategoryError> {
        self.try_write(cx, |ledger| {
            insert_category(ledger.categories, name, parent_id, category_type)
        })
    }

    pub fn edit(&mut self, cx: &mut App, id: u32, name: String) -> Result<(), CategoryError> {
        self.try_write(cx, |ledger| edit_category(ledger.categories, id, name))
    }

    pub fn move_to(
        &mut self,
        cx: &mut App,
        id: u32,
        parent_id: Option<u32>,
    ) -> Result<(), CategoryError> {
        self.try_write(cx, |ledger| move_category(ledger.categories, id, parent_id))
    }

    /// Changes `id`'s type and its descendants'.
    pub fn change_type(
        &mut self,
        cx: &mut App,
        id: u32,
        category_type: CategoryTypes,
    ) -> Result<(), CategoryError> {
        self.try_write(cx, |ledger| {
            change_category_type(ledger.categories, id, category_type)
        })
    }

    /// The first half of a delete: checks leaf `id` and returns the Uncategorised Category its
    /// Splits move to, creating it if needed. See [`prepare_delete`].
    pub fn prepare_delete(&mut self, cx: &mut App, id: u32) -> Result<u32, CategoryError> {
        self.try_write(cx, |ledger| prepare_delete(ledger.categories, id))
    }

    /// Drops leaf Category `id`. The caller has already re-pointed its Splits and dropped its
    /// budget, so this only removes the row.
    pub fn delete(&mut self, cx: &mut App, id: u32) -> Result<(), CategoryError> {
        self.try_write(cx, |ledger| delete_category(ledger.categories, id))
    }

    #[expect(
        deprecated,
        reason = "Categories writes move onto Ledger commands in #639"
    )]
    fn try_write<T>(
        &self,
        cx: &mut App,
        change: impl FnOnce(LedgerHatch<'_>) -> Result<T, CategoryError>,
    ) -> Result<T, CategoryError> {
        self.ledger.update(cx, |ledger, cx| {
            ledger.try_write(cx, &[Domain::Categories], change)
        })
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

    use super::*;

    /// A `CategoriesStore` over the seeded tree, with every event it emits collected.
    fn seeded_store(
        cx: &mut TestAppContext,
    ) -> (Entity<CategoriesStore>, Rc<RefCell<Vec<CategoriesEvent>>>) {
        let ledger = crate::ledger_store::test_ledger(
            cx,
            lib_ledger::LedgerParts {
                categories: lib_categories::default_categories(),
                ..lib_ledger::LedgerParts::default()
            },
        );
        let store = cx.new(|cx| CategoriesStore::new(ledger, cx));
        let events = Rc::new(RefCell::new(Vec::new()));
        let sink = events.clone();
        cx.update(|cx| {
            cx.subscribe(&store, move |_, event: &CategoriesEvent, _| {
                sink.borrow_mut().push(*event)
            })
            .detach();
        });
        (store, events)
    }

    #[gpui::test]
    fn a_landed_write_emits_changed_and_a_refused_one_does_not(cx: &mut TestAppContext) {
        let (store, events) = seeded_store(cx);

        store.update(cx, |store, cx| {
            // Salary (11) is Income, so an Expense child under it is refused.
            assert!(
                store
                    .insert(cx, "Bonus".into(), Some(11), CategoryTypes::Expense)
                    .is_err()
            );
        });
        cx.run_until_parked();
        assert!(events.borrow().is_empty());

        store.update(cx, |store, cx| {
            store
                .insert(cx, "Snacks".into(), Some(6), CategoryTypes::Expense)
                .expect("a leaf under an Expense parent is accepted");
        });
        cx.run_until_parked();
        assert_eq!(*events.borrow(), vec![CategoriesEvent::Changed]);
    }
}
