//! `impl InventoryRouter` for the Desktop: where the Inventory page's Presenter asks to go. A
//! Dialog opens as an Entity in the Dialog host through the page's gpui context, which is why the
//! Router is built for each `handle` call, and a Notice goes onto the bin's queue
//! (`crate::notices`), which the input's `send` drains.

use gpui::{App, Context, Entity};
use lib_inventory::presenter::{
    AreaBlockedPresenter, AreaFormPresenter, LocationFormPresenter, RemoveAreaPresenter,
    RemoveLocationPresenter,
};
use lib_inventory::{InventoryDialog as Which, InventoryRouter};
use lib_toast::{Notice, Notify};

use super::{InventoryPage, InventoryRoutes, InventorySetup};
use crate::announcer::Announcer;
use crate::notices::Queue;
use crate::shell::view::dialog_entity::ModuleDialog;
use crate::shell::view::module_page::ModulePage;

pub(super) struct PageRouter<'a, 'b> {
    pub routes: &'a InventoryRoutes,
    pub cx: &'a mut Context<'b, InventoryPage>,
}

impl InventoryRouter for PageRouter<'_, '_> {
    /// Builds `dialog`'s Presenter on the shared Inventory, its Units and Documents read through
    /// `StoredInventoryReads`, handed the Dialog keys, and hosts it as a `ModuleDialog`; its
    /// confirmed outcome comes back to the page through [`ModulePage::landing`].
    fn open_dialog(&mut self, dialog: Which) -> bool {
        let landed = InventoryPage::landing(self.cx);
        let routes = self.routes;
        let (inventory, reads, keys) = (&routes.inventory, &routes.reads, routes.dialog_keys.get());
        match dialog {
            Which::AddLocation { unit } => {
                let form = LocationFormPresenter::add(inventory, reads, unit, keys);
                ModuleDialog::open(self.cx, form, landed);
            }
            Which::EditLocation(id) => {
                let Some(form) = LocationFormPresenter::edit(inventory, reads, id, keys) else {
                    return false;
                };
                ModuleDialog::open(self.cx, form, landed);
            }
            Which::RemoveLocation(id) => {
                let Some(remove) = RemoveLocationPresenter::new(inventory, reads, id, keys) else {
                    return false;
                };
                ModuleDialog::open(self.cx, remove, landed);
            }
            Which::AddArea(location) => {
                let Some(form) = AreaFormPresenter::add(inventory, location, keys) else {
                    return false;
                };
                ModuleDialog::open(self.cx, form, landed);
            }
            Which::EditArea(id) => {
                let Some(form) = AreaFormPresenter::edit(inventory, id, keys) else {
                    return false;
                };
                ModuleDialog::open(self.cx, form, landed);
            }
            Which::RemoveArea(id) => {
                let Some(remove) = RemoveAreaPresenter::new(inventory, id, keys) else {
                    return false;
                };
                ModuleDialog::open(self.cx, remove, landed);
            }
            Which::AreaBlocked(id) => {
                let Some(blocked) = AreaBlockedPresenter::new(inventory, id, keys) else {
                    return false;
                };
                ModuleDialog::open(self.cx, blocked, landed);
            }
        }
        true
    }
}

impl Notify for PageRouter<'_, '_> {
    /// Queues `notice`, for the input's `send` to drain into the Shell.
    fn notify(&mut self, notice: Notice) {
        Queue.notify(notice);
    }
}

/// The Inventory module's entry point: its Settings page, built on `setup`'s store, which the
/// Settings View hosts as any other page.
pub fn inventory_page(
    setup: InventorySetup,
    announcer: Entity<Announcer>,
    cx: &mut App,
) -> Entity<InventoryPage> {
    use gpui::AppContext as _;
    let InventorySetup {
        presenter,
        inventory,
        reads,
        dialog_keys,
    } = setup;
    let routes = InventoryRoutes {
        inventory,
        reads,
        dialog_keys,
    };
    cx.new(|cx| ModulePage::new(presenter, routes, &announcer, cx))
}
