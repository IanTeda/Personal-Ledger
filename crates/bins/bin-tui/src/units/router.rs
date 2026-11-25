//! `impl UnitsRouter` for the TUI. A Presenter's `handle` can't reach the Units View it runs in, so
//! the Router collects what it asked for (a Dialog to open, the open one to close, Toasts) and the
//! View applies them once `handle` returns. The page Presenter holds the prices view itself. Built for each call,
//! as on the Desktop. A Dialog's Presenter closes through the shared `DialogRouter`, which this
//! Router also implements; the TUI's Dialogs still read their own keys until its redesign (#805).

use lib_toast::{Notice, Notify, ToastKind};
use lib_units::presenter::UnitsOutcome;
use lib_units::{SharedPorts, UnitsDialog as Which, UnitsRouter};
use lib_viper::dialog::DialogRouter;

use super::view::dialog::Dialog;

pub struct Router<'a> {
    ports: SharedPorts,
    /// Where the Dialogs read the day and the time from.
    clock: lib_core::Clock,
    /// The Open Ledger's price fetching, which the price source forms test through.
    fetching: Option<lib_units::SharedFetching>,
    toasts: &'a mut Vec<(ToastKind, String)>,
    /// The Dialog the page asked to open, built on the shared ports.
    pub opened: Option<Dialog>,
    /// Set once the open Dialog asked to close, with its confirmed outcome or `None` on cancel.
    pub closed: Option<Option<UnitsOutcome>>,
}

impl<'a> Router<'a> {
    /// A Router whose Dialogs read the day and the time from `clock`, the Open Ledger's, as the
    /// Units View it runs in does.
    pub fn with_clock(
        ports: SharedPorts,
        toasts: &'a mut Vec<(ToastKind, String)>,
        clock: lib_core::Clock,
    ) -> Self {
        Self {
            ports,
            clock,
            fetching: None,
            toasts,
            opened: None,
            closed: None,
        }
    }

    /// The Router opening price source forms that test through `fetching`.
    pub fn with_fetching(mut self, fetching: Option<lib_units::SharedFetching>) -> Self {
        self.fetching = fetching;
        self
    }
}

impl UnitsRouter for Router<'_> {
    fn open_dialog(&mut self, dialog: Which) -> bool {
        self.opened = Dialog::open(dialog, &self.ports, &self.clock, self.fetching.as_ref());
        self.opened.is_some()
    }
}

impl DialogRouter<UnitsOutcome> for Router<'_> {
    fn close(&mut self, outcome: Option<UnitsOutcome>) {
        self.closed = Some(outcome);
    }
}

impl Notify for Router<'_> {
    /// The TUI's status line has no message slot, so a status reads as an Info Toast, whose echo
    /// lands in the footer.
    fn notify(&mut self, notice: Notice) {
        self.toasts.push(match notice {
            Notice::Toast(kind, text) => (kind, text),
            Notice::Status(text) => (ToastKind::Info, text),
            // The TUI keeps no History.
            Notice::Changed(_) => return,
        });
    }
}
