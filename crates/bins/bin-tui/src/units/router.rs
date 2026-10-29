//! `impl UnitsRouter` for the TUI. A Presenter's `handle` can't reach the Units View it runs in, so
//! the Router collects what it asked for (a Dialog to open, the open one to close, the prices view
//! to open or close, Toasts) and the View applies them once `handle` returns. Built for each call,
//! as on the Desktop.

use lib_toast::{Notice, Notify, ToastKind};
use lib_units::presenter::UnitsOutcome;
use lib_units::{SharedPorts, UnitId, UnitsDialog as Which, UnitsRouter};

use super::view::dialog::Dialog;

pub struct Router<'a> {
    ports: SharedPorts,
    toasts: &'a mut Vec<(ToastKind, String)>,
    /// The Dialog the page asked to open, built on the shared ports.
    pub opened: Option<Dialog>,
    /// Set once the open Dialog asked to close, with its confirmed outcome or `None` on cancel.
    pub closed: Option<Option<UnitsOutcome>>,
    /// Set once a Presenter asked to show a Unit's prices view (`Some`) or close it (`None`).
    pub prices: Option<Option<UnitId>>,
}

impl<'a> Router<'a> {
    pub fn new(ports: SharedPorts, toasts: &'a mut Vec<(ToastKind, String)>) -> Self {
        Self {
            ports,
            toasts,
            opened: None,
            closed: None,
            prices: None,
        }
    }
}

impl UnitsRouter for Router<'_> {
    fn open_dialog(&mut self, dialog: Which) -> bool {
        self.opened = Dialog::open(dialog, &self.ports);
        self.opened.is_some()
    }

    fn close_dialog(&mut self, outcome: Option<UnitsOutcome>) {
        self.closed = Some(outcome);
    }

    fn open_prices(&mut self, unit: UnitId) {
        self.prices = Some(Some(unit));
    }

    fn close_prices(&mut self) {
        self.prices = Some(None);
    }
}

impl Notify for Router<'_> {
    /// The TUI's status line has no message slot, so a status reads as an Info Toast, whose echo
    /// lands in the footer.
    fn notify(&mut self, notice: Notice) {
        self.toasts.push(match notice {
            Notice::Toast(kind, text) => (kind, text),
            Notice::Status(text) => (ToastKind::Info, text),
        });
    }
}
