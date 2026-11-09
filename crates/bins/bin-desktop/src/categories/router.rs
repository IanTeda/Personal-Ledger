//! `impl CategoriesRouter` for the Desktop: where the Categories page's Presenter asks to go. A
//! Dialog opens as a `ModuleDialog` in the Dialog host through the page's gpui context, which is why
//! the Router is built for each `handle` call (`PageView::with_router`), and a Notice goes onto the
//! bin's queue (`crate::notices`), which the input's `send` drains. The Delete category Dialog is
//! the Category Deletion module's (`crate::category_deletion`, #1005), opened through its entry
//! point.

use std::cell::RefCell;
use std::rc::Rc;

use gpui::{AppContext as _, Context, Entity};
use lib_categories::presenter::CategoryFormPresenter;
use lib_categories::{CategoriesDialog as Which, CategoriesRouter, SharedSavePorts};
use lib_toast::{Notice, Notify};

use super::{CategoriesPage, CategoriesRoutes, CategoriesSetup};
use crate::announcer::Announcer;
use crate::notices::Queue;
use crate::shell::view::dialog_entity::ModuleDialog;
use crate::shell::view::module_page::ModulePage;

pub(super) struct PageRouter<'a, 'b> {
    pub routes: &'a CategoriesRoutes,
    pub cx: &'a mut Context<'b, CategoriesPage>,
}

impl CategoriesRouter for PageRouter<'_, '_> {
    /// Builds `dialog`'s Presenter and hosts it: Add and Edit on the shared Categories and a clone
    /// of the Ledger's save ports, which the form holds from open, Delete on the Category Deletion
    /// module's port. Its confirmed outcome comes back to the page through
    /// [`ModulePage::landing`].
    fn open_dialog(&mut self, dialog: Which) -> bool {
        let routes = self.routes;
        let landed = CategoriesPage::landing(self.cx);
        let keys = routes.dialog_keys.clone();
        let save: SharedSavePorts = Rc::new(RefCell::new(routes.save.clone()));
        let categories = routes.categories.clone();
        match dialog {
            Which::AddCategory { parent } => {
                let form = CategoryFormPresenter::add(categories, save, parent, keys);
                ModuleDialog::open(self.cx, form, landed);
                true
            }
            Which::EditCategory(id) => {
                let Some(form) = CategoryFormPresenter::edit(categories, save, id, keys) else {
                    return false;
                };
                ModuleDialog::open(self.cx, form, landed);
                true
            }
            Which::DeleteCategory(id) => crate::category_deletion::router::open_delete(
                self.cx,
                &routes.deletion,
                id,
                keys,
                landed,
            ),
        }
    }
}

impl Notify for PageRouter<'_, '_> {
    /// Queues `notice`, for the input's `send` to drain into the Shell.
    fn notify(&mut self, notice: Notice) {
        Queue.notify(notice);
    }
}

/// The Categories module's entry point: its Settings page, built on `setup`'s ports, which the
/// Settings View hosts as any other page.
pub fn categories_page(
    setup: CategoriesSetup,
    announcer: Entity<Announcer>,
    cx: &mut gpui::App,
) -> Entity<CategoriesPage> {
    let CategoriesSetup {
        presenter,
        categories,
        save,
        budgets,
        deletion,
        dialog_keys,
    } = setup;
    let routes = CategoriesRoutes {
        categories,
        save,
        budgets,
        deletion,
        dialog_keys,
    };
    cx.new(|cx| ModulePage::new(presenter, routes, &announcer, cx))
}
