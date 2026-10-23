//! `impl UnitsRouter` for the Desktop: where the Units page's Presenter asks to go. A Dialog opens
//! in today's Dialog host through the page's gpui context, which is why the Router is built for
//! each `handle` call, and a Notice goes onto the bin's queue (`crate::notices`).

use gpui::Context;
use lib_toast::{Notice, Notify};
use lib_units::presenter::UnitsOutcome;
use lib_units::{UnitsDialog as Which, UnitsRouter};

use super::ports::DesktopPorts;
use super::view::dialog::UnitsDialog;
use super::view::page::UnitsPage;
use crate::notices::Queue;
use crate::{chrome::dialog_host::open_dialog, view::event::ViewEvent};

pub(super) struct PageRouter<'a, 'b> {
    pub ports: DesktopPorts,
    pub cx: &'a mut Context<'b, UnitsPage>,
}

impl UnitsRouter for PageRouter<'_, '_> {
    /// Builds `dialog`'s Presenter on the shared ports and hosts its View; its confirmed outcome
    /// comes back to the page through [`UnitsPage::dialog_landed`].
    fn open_dialog(&mut self, dialog: Which) -> bool {
        let Some(dialog) = UnitsDialog::open(dialog, &self.ports) else {
            return false;
        };
        open_dialog(dialog, UnitsPage::dialog_landed, self.cx);
        true
    }

    /// The page closes no Dialog: a Dialog's own Presenter closes it through the host.
    fn close_dialog(&mut self, _outcome: Option<UnitsOutcome>) {}
}

impl Notify for PageRouter<'_, '_> {
    /// Queues `notice` and asks `Shell` to drain the queue.
    fn notify(&mut self, notice: Notice) {
        Queue.notify(notice);
        self.cx.emit(ViewEvent::NoticesQueued);
    }
}

/// The Units module's entry point: its Settings page, built on `ports`, which the Settings View
/// hosts as any other page.
pub fn units_page(
    ports: DesktopPorts,
    ledger: gpui::Entity<crate::ledger_store::LedgerStore>,
    keymap: lib_viper::keymap::Keymap<lib_units::presenter::UnitsPageEvent>,
    cx: &mut gpui::App,
) -> gpui::Entity<UnitsPage> {
    use gpui::AppContext as _;
    cx.new(|cx| UnitsPage::new(ports, ledger, keymap, cx))
}
