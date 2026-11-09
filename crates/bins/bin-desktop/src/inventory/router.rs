//! `impl InventoryRouter` for the Desktop: where the Inventory page's Presenter asks to go. A
//! Dialog opens as an Entity in the Dialog host through the page's gpui context, which is why the
//! Router is built for each `handle` call, and a Notice goes onto the bin's queue
//! (`crate::notices`), which the input's `send` drains.

use gpui::{App, Context, Entity};
use lib_database::StoredInventoryReads;
use lib_inventory::presenter::{
    AreaBlockedPresenter, AreaFormPresenter, LocationFormPresenter, RemoveAreaPresenter,
    RemoveLocationPresenter,
};
use lib_inventory::{InventoryDialog as Which, InventoryOutcome, InventoryRouter, SharedInventory};
use lib_toast::{Notice, Notify};

use super::InventorySetup;
use super::view::page::InventoryPage;
use crate::announcer::Announcer;
use crate::notices::Queue;
use crate::shell::view::dialog_entity::ModuleDialog;

pub(super) struct PageRouter<'a, 'b> {
    pub inventory: SharedInventory,
    pub reads: StoredInventoryReads,
    /// The Dialog keys the Dialogs' status-line hints are built from.
    pub dialog_keys: crate::dialog_keys::LiveDialogKeys,
    pub cx: &'a mut Context<'b, InventoryPage>,
}

impl InventoryRouter for PageRouter<'_, '_> {
    /// Builds `dialog`'s Presenter on the shared Inventory, its Units and Documents read through
    /// [`StoredInventoryReads`], handed the Dialog keys, and hosts it as a `ModuleDialog`; its
    /// confirmed outcome comes back to the page through [`InventoryPage::dialog_landed`].
    fn open_dialog(&mut self, dialog: Which) -> bool {
        let page = self.cx.weak_entity();
        let landed = move |outcome: InventoryOutcome, cx: &mut App| {
            // A page that has gone has nothing to land on.
            let _ = page.update(cx, |page, cx| page.dialog_landed(outcome, cx));
        };
        let (inventory, reads, keys) = (&self.inventory, &self.reads, self.dialog_keys.get());
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
    cx.new(|cx| InventoryPage::new(setup, announcer, cx))
}
