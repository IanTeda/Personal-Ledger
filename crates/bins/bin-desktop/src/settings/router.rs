//! The Settings module's Routers for the Desktop: [`FrameRouter`], what the frame's Presenter asks
//! of the Shell, [`DeviceRouter`], what the This device pages' Presenters ask, and
//! [`TracingRouter`], what Tracing's does, and [`LedgerRouter`], what Ledger's does. Each goes
//! through its Entity's gpui context, which is why a Router is built for each `handle` call. A
//! page change reaches the root Router as [`PageChanged`], which feeds the Shell's history; the
//! picker's mode is the View's own (`ViewEvent::EnterMode`); Toasts and the Display output reach
//! the root Router as [`ViewEvent`]s; a colour change is applied once the current update ends; a
//! Notice goes onto the bin's queue (`crate::notices`), which the input's `send` drains.

use gpui::{Context, Entity, EventEmitter, ListState};
use lib_settings::{
    ClearLogsPresenter, ColourChoice, DisplayOutput, LogListChange, LogScroll, SettingsPage,
    SettingsRouter,
};
use lib_toast::{Notice, Notify};

use super::ledger::LedgerPage;
use super::state::SettingsView;
use super::tracing::{TracingPage, apply_change, dialogs::clear_logs::ClearLogsDialog, scroll};
use crate::ledger_store::LedgerStore;
use crate::units::DesktopPorts;
use crate::view::event::{ViewEvent, ViewMode};

/// The page on show changed by the person's hand, for the root Router to record in the Shell's
/// history (`NavInput::SettingsPage`), so Back and Forward step through it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PageChanged(pub SettingsPage);

pub(super) struct FrameRouter<'a, 'b> {
    pub cx: &'a mut Context<'b, SettingsView>,
    /// Whether the frame refused what it was asked (a This Ledger page with no Ledger open), so
    /// the View leaves the page on show as it was.
    pub refused: bool,
}

impl SettingsRouter for FrameRouter<'_, '_> {
    fn page_changed(&mut self, page: SettingsPage) {
        self.cx.emit(PageChanged(page));
    }

    /// The picker reads every key while it is open, as a filter popover does.
    fn picker_opened(&mut self) {
        self.cx.emit(ViewEvent::EnterMode(ViewMode::Filter));
    }

    fn picker_closed(&mut self) {
        self.cx.emit(ViewEvent::ExitMode);
    }
}

impl Notify for FrameRouter<'_, '_> {
    /// Queues `notice`, for the input's `send` to drain into the Shell. The frame's only Notice is
    /// a refusal.
    fn notify(&mut self, notice: Notice) {
        self.refused = true;
        crate::notices::Queue.notify(notice);
    }
}

/// `impl lib_settings::DeviceRouter` for a This device page `P` (General or Display).
pub(super) struct DeviceRouter<'a, 'b, P> {
    pub cx: &'a mut Context<'b, P>,
}

impl<P: EventEmitter<ViewEvent>> lib_settings::DeviceRouter for DeviceRouter<'_, '_, P> {
    fn toasts_changed(&mut self, on: bool) {
        self.cx.emit(ViewEvent::ToastsChanged(on));
    }

    fn display_changed(&mut self, display: DisplayOutput) {
        self.cx.emit(ViewEvent::DisplayChanged(display));
    }

    /// A colour change refreshes every window, which must wait until the Shell's update ends.
    fn colour_changed(&mut self, colour: ColourChoice) {
        self.cx
            .defer(move |cx| crate::theme::colours::apply(colour, cx));
    }
}

impl<P> Notify for DeviceRouter<'_, '_, P> {
    /// Queues `notice`, for the input's `send` to drain into the Shell.
    fn notify(&mut self, notice: Notice) {
        crate::notices::Queue.notify(notice);
    }
}

/// `impl lib_settings::TracingRouter` for the Tracing page: the log box's list and the Dialog it
/// opens. `list` is the page's own, lent beside its Presenter.
pub(super) struct TracingRouter<'a, 'b> {
    pub list: &'a ListState,
    pub cx: &'a mut Context<'b, TracingPage>,
    /// Whether the Presenter changed the log list, so a capture wake redraws only then.
    pub list_changed: bool,
}

impl lib_settings::TracingRouter for TracingRouter<'_, '_> {
    fn scroll(&mut self, to: LogScroll) {
        scroll(self.list, to);
    }

    fn log_changed(&mut self, change: LogListChange) {
        apply_change(self.list, change);
        self.list_changed = true;
    }

    /// Clear logs asks first, since the capture is emptied for good (#502), and clears the status
    /// line as the page's other Dialogs do.
    fn open_clear_logs(&mut self, dialog: ClearLogsPresenter) {
        self.cx.emit(ViewEvent::SetStatus(None));
        let page = self.cx.weak_entity();
        crate::shell::view::dialog_entity::host_dialog(self.cx, |_| {
            ClearLogsDialog::new(dialog, page)
        });
    }
}

impl Notify for TracingRouter<'_, '_> {
    /// Queues `notice`, for the input's `send` to drain into the Shell.
    fn notify(&mut self, notice: Notice) {
        crate::notices::Queue.notify(notice);
    }
}

/// `impl lib_settings::LedgerRouter` for the Ledger page: the Units ports and the Ledger's read
/// copy of them, lent beside its Presenter.
pub(super) struct LedgerRouter<'a, 'b> {
    pub ports: &'a DesktopPorts,
    pub ledger: &'a Entity<LedgerStore>,
    pub cx: &'a mut Context<'b, LedgerPage>,
}

impl lib_settings::LedgerRouter for LedgerRouter<'_, '_> {
    /// Brings the Ledger's read copy in line, announcing `Domain::Units` so every view of the base
    /// Unit follows at once.
    fn base_unit_changed(&mut self) {
        crate::units::ports::refresh(self.ports, self.ledger, self.cx);
    }
}
