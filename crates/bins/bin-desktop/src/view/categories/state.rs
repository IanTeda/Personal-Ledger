//! The Categories destination's Entities (ADR-0032). [`CategoriesStore`] owns the tree, read
//! through `lib_categories`' service; [`CategoriesView`] owns the Settings tree's cursor, its
//! selected id and which nodes are expanded. `Shell` opens the dialogs and moves the Budgets and
//! Transactions with a delete, since those stores are `Shell`'s to coordinate.

use gpui::{Context, EventEmitter};
use lib_categories::{Category, CategoryError, CategoryService};
use lib_core::CategoryTypes;

/// Emitted by [`CategoriesStore`] after a write lands, so `Shell` can refresh the Views that read
/// the tree (ADR-0032's View events).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CategoriesEvent {
    Changed,
}

/// The shared Categories tree. Budgets, Bills, Transactions, Documents, Payees and Settings all
/// read it from here, so a change is seen everywhere on the next render.
pub struct CategoriesStore {
    service: CategoryService,
}

impl EventEmitter<CategoriesEvent> for CategoriesStore {}

impl CategoriesStore {
    /// A store over `categories`, which the Desktop seeds from `lib_categories::default_categories`.
    pub fn new(categories: Vec<Category>) -> Self {
        Self {
            service: CategoryService::new(categories),
        }
    }

    pub fn categories(&self) -> &[Category] {
        self.service.categories()
    }

    /// Adds a Category and returns its id; the caller decides what to select.
    pub fn insert(
        &mut self,
        cx: &mut Context<'_, Self>,
        name: String,
        parent_id: Option<u32>,
        category_type: CategoryTypes,
    ) -> Result<u32, CategoryError> {
        let id = self.service.insert(name, parent_id, category_type)?;
        cx.emit(CategoriesEvent::Changed);
        Ok(id)
    }

    pub fn edit(
        &mut self,
        cx: &mut Context<'_, Self>,
        id: u32,
        name: String,
    ) -> Result<(), CategoryError> {
        self.service.edit(id, name)?;
        cx.emit(CategoriesEvent::Changed);
        Ok(())
    }

    pub fn move_to(
        &mut self,
        cx: &mut Context<'_, Self>,
        id: u32,
        parent_id: Option<u32>,
    ) -> Result<(), CategoryError> {
        self.service.move_to(id, parent_id)?;
        cx.emit(CategoriesEvent::Changed);
        Ok(())
    }

    /// Changes `id`'s type and its descendants'.
    pub fn change_type(
        &mut self,
        cx: &mut Context<'_, Self>,
        id: u32,
        category_type: CategoryTypes,
    ) -> Result<(), CategoryError> {
        self.service.change_type(id, category_type)?;
        cx.emit(CategoriesEvent::Changed);
        Ok(())
    }

    /// The first half of a delete: checks leaf `id` and returns the Uncategorised Category its
    /// Splits move to, creating it if needed. See [`CategoryService::prepare_delete`].
    pub fn prepare_delete(
        &mut self,
        cx: &mut Context<'_, Self>,
        id: u32,
    ) -> Result<u32, CategoryError> {
        let uncategorised = self.service.prepare_delete(id)?;
        cx.emit(CategoriesEvent::Changed);
        Ok(uncategorised)
    }

    /// Drops leaf Category `id`. The caller has already re-pointed its Splits and dropped its
    /// budget, so this only removes the row.
    pub fn delete(&mut self, cx: &mut Context<'_, Self>, id: u32) -> Result<(), CategoryError> {
        self.service.delete(id)?;
        cx.emit(CategoriesEvent::Changed);
        Ok(())
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
        let store = cx.new(|_| CategoriesStore::new(lib_categories::default_categories()));
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
