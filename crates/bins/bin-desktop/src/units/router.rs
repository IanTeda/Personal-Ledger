//! `impl UnitsRouter` for the Desktop: where the Units page's and the Unit prices view's Presenters
//! ask to go. A Dialog opens as an Entity in the Dialog host through the page's gpui context, which
//! is why the Router is built for each `handle` call; the prices view to open or close is collected
//! for the page to apply once `handle` returns; and a Notice goes onto the bin's queue
//! (`crate::notices`), which the input's `send` drains.

use gpui::Context;
use lib_toast::{Notice, Notify};
use lib_units::presenter::UnitsOutcome;
use lib_units::{UnitId, UnitsDialog as Which, UnitsRouter};

use super::ports::DesktopPorts;
use super::view::dialog::UnitsDialog;
use super::view::page::UnitsPage;
use crate::ledger_store::LedgerStore;
use crate::notices::Queue;
use crate::shell::view::dialog_entity::host_dialog;
use crate::unit_deletion::DeleteUnitDialog;

pub(super) struct PageRouter<'a, 'b> {
    pub ports: DesktopPorts,
    pub deletion: lib_unit_deletion::SharedUnitDeletion,
    pub ledger: gpui::Entity<LedgerStore>,
    pub cx: &'a mut Context<'b, UnitsPage>,
    /// Set once a Presenter asked to show a Unit's prices view (`Some`) or close it (`None`).
    pub prices: Option<Option<UnitId>>,
}

impl UnitsRouter for PageRouter<'_, '_> {
    /// Builds `dialog`'s Presenter on the shared ports and hosts its View; its confirmed outcome
    /// comes back to the page through [`UnitsPage::dialog_landed`]. Delete unit is the Unit
    /// Deletion module's Dialog, which writes the Institutions that name the Unit too.
    fn open_dialog(&mut self, dialog: Which) -> bool {
        let page = self.cx.weak_entity();
        let landed = Box::new(move |outcome, cx: &mut gpui::App| {
            // A page that has gone has nothing to land on.
            let _ = page.update(cx, |page, cx| page.dialog_landed(outcome, cx));
        });
        if let Which::DeleteUnit(id) = dialog {
            let Some(dialog) = DeleteUnitDialog::open(
                id,
                &self.ports,
                &self.deletion,
                &self.ledger,
                landed,
                self.cx,
            ) else {
                return false;
            };
            host_dialog(self.cx, |_| dialog);
            return true;
        }
        let Some(dialog) = UnitsDialog::open(dialog, &self.ports, &self.ledger, landed) else {
            return false;
        };
        host_dialog(self.cx, |_| dialog);
        true
    }

    /// The page closes no Dialog: a Dialog's own Presenter closes it through the host.
    fn close_dialog(&mut self, _outcome: Option<UnitsOutcome>) {}

    fn open_prices(&mut self, unit: UnitId) {
        self.prices = Some(Some(unit));
    }

    fn close_prices(&mut self) {
        self.prices = Some(None);
    }
}

impl Notify for PageRouter<'_, '_> {
    /// Queues `notice`, for the input's `send` to drain into the Shell.
    fn notify(&mut self, notice: Notice) {
        Queue.notify(notice);
    }
}

/// The Units module's entry point: its Settings page, built on `ports`, which the Settings View
/// hosts as any other page.
pub fn units_page(
    ports: DesktopPorts,
    deletion: lib_unit_deletion::SharedUnitDeletion,
    ledger: gpui::Entity<LedgerStore>,
    keymap: lib_viper::keymap::Keymap<lib_units::presenter::UnitsPageEvent>,
    prices_keymap: lib_viper::keymap::Keymap<lib_units::presenter::UnitPricesEvent>,
    cx: &mut gpui::App,
) -> gpui::Entity<UnitsPage> {
    use gpui::AppContext as _;
    cx.new(|cx| UnitsPage::new(ports, deletion, ledger, keymap, prices_keymap, cx))
}
