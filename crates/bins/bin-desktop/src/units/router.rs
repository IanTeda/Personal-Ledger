//! `impl UnitsRouter` for the Desktop: where the Units page's Presenter asks to go. A Dialog opens
//! in today's Dialog host, a Toast and the status line reach the Shell as [`ViewEvent`]s, all
//! through the page's gpui context, which is why the Router is built for each `handle` call.

use gpui::Context;
use lib_toast::ToastKind;
use lib_units::presenter::UnitsOutcome;
use lib_units::{UnitsDialog as Which, UnitsRouter};

use super::ports::DesktopPorts;
use super::view::dialog::UnitsDialog;
use super::view::page::UnitsPage;
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

    fn raise_toast(&mut self, kind: ToastKind, text: String) {
        self.cx.emit(ViewEvent::RaiseToast { kind, text });
    }

    fn set_status(&mut self, text: String) {
        self.cx.emit(ViewEvent::SetStatus(Some(text)));
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
