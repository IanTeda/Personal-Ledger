//! `impl CategoriesRouter` for the Desktop: where the Categories page's Presenter asks to go. A
//! Dialog opens as an Entity in the Dialog host through the page's gpui context, which is why the
//! Router is built for each `handle` call, and a Notice goes onto the bin's queue
//! (`crate::notices`), which the input's `send` drains. The Delete category Dialog is the Category
//! Deletion module's (`crate::category_deletion`, #1005), opened through its entry point.

use chrono::NaiveDate;
use gpui::{Context, Entity};
use lib_categories::{CategoriesDialog as Which, CategoriesOutcome, CategoriesRouter};
use lib_core::Clock;
use lib_toast::{Notice, Notify};

use super::CategoriesSetup;
use super::view::dialog::{CategoriesDialog, PortsOn};
use super::view::page::CategoriesPage;
use crate::announcer::Announcer;
use crate::category_deletion::DeletionStores;
use crate::notices::Queue;
use crate::shell::view::dialog_entity::host_dialog;

pub(super) struct PageRouter<'a, 'b> {
    pub categories: lib_categories::SharedCategories,
    pub deletion: DeletionStores,
    pub stores: crate::categories::ports::CategoryStores,
    pub announcer: Entity<Announcer>,
    pub today: NaiveDate,
    pub clock: Clock,
    pub cx: &'a mut Context<'b, CategoriesPage>,
}

impl CategoriesRouter for PageRouter<'_, '_> {
    /// Builds `dialog`'s Presenter and hosts its View: Add and Edit on the shared Categories, their
    /// usage and budget read from the stores, Delete on the Category Deletion module's port. Its
    /// confirmed outcome comes back to the page through [`CategoriesPage::dialog_landed`].
    fn open_dialog(&mut self, dialog: Which) -> bool {
        if let Which::DeleteCategory(id) = dialog {
            let landed = |page: &mut CategoriesPage,
                          outcome: CategoriesOutcome,
                          cx: &mut Context<'_, CategoriesPage>| {
                page.dialog_landed(outcome, cx);
            };
            return crate::category_deletion::router::open_delete(
                self.cx,
                &self.deletion,
                id,
                landed,
            );
        }
        let page = self.cx.weak_entity();
        let landed = Box::new(move |outcome, cx: &mut gpui::App| {
            // A page that has gone has nothing to land on.
            let _ = page.update(cx, |page, cx| page.dialog_landed(outcome, cx));
        });
        let Some(dialog) = CategoriesDialog::open(
            dialog,
            &self.categories,
            PortsOn {
                stores: self.stores.clone(),
                announcer: self.announcer.clone(),
                today: self.today,
                clock: self.clock.clone(),
            },
            landed,
            self.cx,
        ) else {
            return false;
        };
        host_dialog(self.cx, |_| dialog);
        true
    }
}

impl Notify for PageRouter<'_, '_> {
    /// Queues `notice`, for the input's `send` to drain into the Shell.
    fn notify(&mut self, notice: Notice) {
        Queue.notify(notice);
    }
}

/// The Categories module's entry point: its Settings page, built on `setup`'s store, with the
/// Monthly budget read as of `clock`'s day, which the Settings View hosts as any other page.
pub fn categories_page(
    setup: CategoriesSetup,
    announcer: Entity<Announcer>,
    clock: &Clock,
    cx: &mut gpui::App,
) -> Entity<CategoriesPage> {
    use gpui::AppContext as _;
    let today = clock.today();
    cx.new(|cx| CategoriesPage::new(setup, announcer, today, clock.clone(), cx))
}
