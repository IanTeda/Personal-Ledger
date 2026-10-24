//! `impl UnitsRouter` for the Desktop: where the Units page's Presenter asks to go. A Dialog opens
//! as an Entity in the Dialog host through the page's gpui context, which is why the Router is built for
//! each `handle` call, and a Notice goes onto the bin's queue (`crate::notices`), which the
//! input's `send` drains.

use gpui::Context;
use lib_toast::{Notice, Notify};
use lib_units::presenter::UnitsOutcome;
use lib_units::{UnitsDialog as Which, UnitsRouter};

use super::ports::DesktopPorts;
use super::view::dialog::UnitsDialog;
use super::view::page::UnitsPage;
use crate::ledger_store::LedgerStore;
use crate::notices::Queue;
use crate::shell::view::dialog_entity::host_dialog;

pub(super) struct PageRouter<'a, 'b> {
    pub ports: DesktopPorts,
    pub ledger: gpui::Entity<LedgerStore>,
    pub cx: &'a mut Context<'b, UnitsPage>,
}

impl UnitsRouter for PageRouter<'_, '_> {
    /// Builds `dialog`'s Presenter on the shared ports and hosts its View; its confirmed outcome
    /// comes back to the page through [`UnitsPage::dialog_landed`].
    fn open_dialog(&mut self, dialog: Which) -> bool {
        let page = self.cx.weak_entity();
        let landed = Box::new(move |outcome, cx: &mut gpui::App| {
            // A page that has gone has nothing to land on.
            let _ = page.update(cx, |page, cx| page.dialog_landed(outcome, cx));
        });
        let Some(dialog) = UnitsDialog::open(dialog, &self.ports, &self.ledger, landed) else {
            return false;
        };
        host_dialog(self.cx, |_| dialog);
        true
    }

    /// The page closes no Dialog: a Dialog's own Presenter closes it through the host.
    fn close_dialog(&mut self, _outcome: Option<UnitsOutcome>) {}
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
    ledger: gpui::Entity<LedgerStore>,
    keymap: lib_viper::keymap::Keymap<lib_units::presenter::UnitsPageEvent>,
    cx: &mut gpui::App,
) -> gpui::Entity<UnitsPage> {
    use gpui::AppContext as _;
    cx.new(|cx| UnitsPage::new(ports, ledger, keymap, cx))
}
