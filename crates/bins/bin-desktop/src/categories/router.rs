//! `impl CategoriesRouter` for the Desktop: where the Categories page's Presenter asks to go. A
//! Dialog opens as a `ModuleDialog` in the Dialog host through the generic [`PageRouter`], built
//! for each `handle` call (`PageView::with_router`), which queues a Notice for the input's `send`
//! to drain. The Delete category Dialog is the Category Deletion module's
//! (`crate::category_deletion`, #1005), opened through its entry point.

use std::cell::RefCell;
use std::rc::Rc;

use gpui::{AppContext as _, Entity};
use lib_categories::presenter::{CategoriesPagePresenter, CategoryFormPresenter};
use lib_categories::{CategoriesDialog as Which, CategoriesRouter, SharedSavePorts};

use super::{CategoriesPage, CategoriesSetup};
use crate::announcer::Announcer;
use crate::shell::view::module_page::ModulePage;
use crate::shell::view::page_router::PageRouter;

impl CategoriesRouter for PageRouter<'_, '_, CategoriesPagePresenter> {
    /// Builds `dialog`'s Presenter and hosts it: Add and Edit on the shared Categories and a clone
    /// of the Ledger's save ports, which the form holds from open, Delete on the Category Deletion
    /// module's port. Its confirmed outcome comes back to the page through
    /// [`ModulePage::landing`].
    fn open_dialog(&mut self, dialog: Which) -> bool {
        let setup = self.setup;
        let keys = setup.dialog_keys.get();
        let save: SharedSavePorts = Rc::new(RefCell::new(setup.save.clone()));
        let categories = setup.categories.clone();
        match dialog {
            Which::AddCategory { parent } => {
                self.open(CategoryFormPresenter::add(categories, save, parent, keys));
                true
            }
            Which::EditCategory(id) => {
                let Some(form) = CategoryFormPresenter::edit(categories, save, id, keys) else {
                    return false;
                };
                self.open(form);
                true
            }
            Which::DeleteCategory(id) => {
                let landed = self.landing();
                crate::category_deletion::router::open_delete(
                    self.cx,
                    &setup.deletion,
                    id,
                    keys,
                    landed,
                )
            }
        }
    }
}

/// The Categories module's entry point: its Settings page, `presenter` routed over `setup`, which
/// the Settings View hosts as any other page.
pub fn categories_page(
    presenter: CategoriesPagePresenter,
    setup: CategoriesSetup,
    announcer: Entity<Announcer>,
    cx: &mut gpui::App,
) -> Entity<CategoriesPage> {
    cx.new(|cx| ModulePage::new(presenter, setup, &announcer, cx))
}
