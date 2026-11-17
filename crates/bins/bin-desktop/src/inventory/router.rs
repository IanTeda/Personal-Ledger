//! `impl InventoryRouter` for the Desktop: where the Inventory page's Presenter asks to go. A
//! Dialog opens as an Entity in the Dialog host through the generic [`PageRouter`], built for each
//! `handle` call (`PageView::with_router`), which queues a Notice for the input's `send` to drain.

use gpui::{App, AppContext as _, Entity};
use lib_inventory::presenter::{
    AreaBlockedPresenter, AreaFormPresenter, InventoryPagePresenter, LocationFormPresenter,
    RemoveAreaPresenter, RemoveLocationPresenter,
};
use lib_inventory::{InventoryDialog as Which, InventoryRouter};

use super::{InventoryPage, InventorySetup};
use crate::announcer::Announcer;
use crate::shell::view::module_page::ModulePage;
use crate::shell::view::page_router::PageRouter;

impl InventoryRouter for PageRouter<'_, '_, InventoryPagePresenter> {
    /// Builds `dialog`'s Presenter on the shared Inventory, its Units and Documents read through
    /// `lib_inventory::SharedReads`, handed the Dialog keys, and hosts it as a `ModuleDialog`; its
    /// confirmed outcome comes back to the page through [`ModulePage::landing`].
    fn open_dialog(&mut self, dialog: Which) -> bool {
        let setup = self.setup;
        let (inventory, reads, keys) = (&setup.inventory, &*setup.reads, setup.dialog_keys.get());
        match dialog {
            Which::AddLocation { unit } => {
                self.open(LocationFormPresenter::add(inventory, reads, unit, keys));
            }
            Which::EditLocation(id) => {
                let Some(form) = LocationFormPresenter::edit(inventory, reads, id, keys) else {
                    return false;
                };
                self.open(form);
            }
            Which::RemoveLocation(id) => {
                let Some(remove) = RemoveLocationPresenter::new(inventory, reads, id, keys) else {
                    return false;
                };
                self.open(remove);
            }
            Which::AddArea(location) => {
                let Some(form) = AreaFormPresenter::add(inventory, location, keys) else {
                    return false;
                };
                self.open(form);
            }
            Which::EditArea(id) => {
                let Some(form) = AreaFormPresenter::edit(inventory, id, keys) else {
                    return false;
                };
                self.open(form);
            }
            Which::RemoveArea(id) => {
                let Some(remove) = RemoveAreaPresenter::new(inventory, id, keys) else {
                    return false;
                };
                self.open(remove);
            }
            Which::AreaBlocked(id) => {
                let Some(blocked) = AreaBlockedPresenter::new(inventory, id, keys) else {
                    return false;
                };
                self.open(blocked);
            }
        }
        true
    }
}

/// The Inventory module's entry point: its Settings page, `presenter` routed over `setup`'s store,
/// which the Settings View hosts as any other page.
pub fn inventory_page(
    presenter: InventoryPagePresenter,
    setup: InventorySetup,
    announcer: Entity<Announcer>,
    cx: &mut App,
) -> Entity<InventoryPage> {
    cx.new(|cx| ModulePage::new(presenter, setup, &announcer, cx))
}
