//! `impl InventoryRouter` for the Desktop: where the Inventory page's Presenter asks to go. A
//! Dialog opens as an Entity in the Dialog host through the page's gpui context, which is why the
//! Router is built for each `handle` call, and a Notice goes onto the bin's queue
//! (`crate::notices`), which the input's `send` drains.

use gpui::{Context, Entity};
use lib_inventory::{InventoryDialog as Which, InventoryOutcome, InventoryRouter, SharedInventory};
use lib_toast::{Notice, Notify};

use super::InventorySetup;
use super::view::dialog::InventoryDialog;
use super::view::page::InventoryPage;
use crate::ledger_store::LedgerStore;
use crate::notices::Queue;
use crate::shell::view::dialog_entity::host_dialog;

pub(super) struct PageRouter<'a, 'b> {
    pub inventory: SharedInventory,
    pub ledger: Entity<LedgerStore>,
    pub cx: &'a mut Context<'b, InventoryPage>,
}

impl InventoryRouter for PageRouter<'_, '_> {
    /// Builds `dialog`'s Presenter on the shared Inventory, its Units and Documents read from the
    /// Ledger, and hosts its View; its confirmed outcome comes back to the page through
    /// [`InventoryPage::dialog_landed`].
    fn open_dialog(&mut self, dialog: Which) -> bool {
        let page = self.cx.weak_entity();
        let landed = Box::new(move |outcome, cx: &mut gpui::App| {
            // A page that has gone has nothing to land on.
            let _ = page.update(cx, |page, cx| page.dialog_landed(outcome, cx));
        });
        let Some(dialog) =
            InventoryDialog::open(dialog, &self.inventory, &self.ledger, landed, self.cx)
        else {
            return false;
        };
        host_dialog(self.cx, |_| dialog);
        true
    }

    /// The page closes no Dialog: a Dialog's own Presenter closes it through the host.
    fn close_dialog(&mut self, _outcome: Option<InventoryOutcome>) {}
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
    ledger: Entity<LedgerStore>,
    cx: &mut gpui::App,
) -> Entity<InventoryPage> {
    use gpui::AppContext as _;
    cx.new(|cx| InventoryPage::new(setup, ledger, cx))
}

/// What an Inventory Dialog's Presenter asks: closing, and Notices, which go onto the bin's queue
/// for `Root` to drain once the key or click returns. The page opens Dialogs.
#[derive(Default)]
pub(super) struct DialogRouter {
    /// `Some` once the Presenter closed its Dialog, with what its confirm did.
    pub closed: Option<Option<InventoryOutcome>>,
}

impl InventoryRouter for DialogRouter {
    fn open_dialog(&mut self, _dialog: Which) -> bool {
        false
    }

    fn close_dialog(&mut self, outcome: Option<InventoryOutcome>) {
        self.closed = Some(outcome);
    }
}

impl Notify for DialogRouter {
    fn notify(&mut self, notice: Notice) {
        Queue.notify(notice);
    }
}
