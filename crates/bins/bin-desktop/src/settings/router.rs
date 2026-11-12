//! The Settings module's Routers for the Desktop: [`FrameRouter`], what the frame's Presenter asks
//! of the Shell, [`DeviceRouter`], what the This device pages' Presenters ask, and
//! [`TracingRouter`], what Tracing's does, and [`LedgerRouter`], what Ledger's does. Each goes
//! through its Entity's gpui context, which is why a Router is built for each `handle` call. A
//! page change reaches the root Router as [`PageChanged`], which feeds the Shell's history; the
//! picker's mode is the View's own (`ViewEvent::EnterMode`); Toasts and the Display output reach
//! the root Router as [`ViewEvent`]s; a colour change is applied once the current update ends; a
//! Notice goes onto the bin's queue (`crate::notices`), which the input's `send` drains.

use gpui::{Context, EventEmitter, ScrollHandle};
use lib_settings::{
    ClearLogsPresenter, ColourChoice, DisplayOutput, LogListChange, LogScroll, SettingsPage,
    SettingsRouter,
};
use lib_toast::{Notice, Notify};

use super::pages::SettingsPages;
use super::state::SettingsView;
use super::tracing::{ListEffects, LogListEffect, TracingPage};
use crate::view::event::{ViewEvent, ViewMode};

/// The page on show changed by the person's hand, for the root Router to record in the Shell's
/// history (`NavInput::SettingsPage`), so Back and Forward step through it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PageChanged(pub SettingsPage);

/// The frame's Router, lent the View's pages and scrolls beside its context, so what the frame
/// reports of the keyboard and the selected page is applied as it is reported.
pub(super) struct FrameRouter<'a, 'b> {
    pub cx: &'a mut Context<'b, SettingsView>,
    pub pages: &'a SettingsPages,
    /// The page body's scroll.
    pub scroll_handle: &'a ScrollHandle,
    /// The index page's scroll.
    pub index_scroll: &'a ScrollHandle,
}

impl FrameRouter<'_, '_> {
    /// Tells `page` whether it has the keyboard. A This Ledger page with no Ledger open isn't
    /// built, so has nothing to tell.
    fn set_focused(&mut self, page: SettingsPage, focused: bool) {
        if let Some(page) = self.pages.get(page) {
            page.set_focused(focused, self.cx);
        }
    }
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

    fn page_selected(&mut self, _page: SettingsPage) {
        self.scroll_handle.set_offset(gpui::Point::default());
    }

    fn keyboard_left(&mut self, page: SettingsPage) {
        self.set_focused(page, false);
    }

    fn keyboard_entered(&mut self, page: SettingsPage) {
        self.set_focused(page, true);
    }

    fn index_revealed(&mut self, page: SettingsPage) {
        self.index_scroll
            .scroll_to_item(super::index_page_child(page));
    }
}

impl Notify for FrameRouter<'_, '_> {
    /// Queues `notice`, for the input's `send` to drain into the Shell.
    fn notify(&mut self, notice: Notice) {
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

    fn test_toasts(&mut self) {
        self.cx.emit(ViewEvent::TestToasts);
    }

    fn pick_data_folder(&mut self) {
        self.cx.emit(ViewEvent::PickDataFolder);
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
/// opens. The list is the page's widget state, which the Router never holds: it queues what the
/// Presenter asks of it in the page's `list_effects`, for its `PageView::after` to apply.
pub(super) struct TracingRouter<'a, 'b> {
    pub cx: &'a mut Context<'b, TracingPage>,
    pub list_effects: &'a ListEffects,
}

impl lib_settings::TracingRouter for TracingRouter<'_, '_> {
    fn scroll(&mut self, to: LogScroll) {
        self.list_effects
            .borrow_mut()
            .push(LogListEffect::Scroll(to));
    }

    fn log_changed(&mut self, change: LogListChange) {
        self.list_effects
            .borrow_mut()
            .push(LogListEffect::Changed(change));
    }

    /// Clear logs asks first, since the capture is emptied for good (#502), and clears the status
    /// line as the page's other Dialogs do.
    fn open_clear_logs(&mut self, dialog: ClearLogsPresenter) {
        self.cx.emit(ViewEvent::SetStatus(None));
        let landed = TracingPage::landing(self.cx);
        crate::shell::view::dialog_entity::ModuleDialog::open(self.cx, dialog, landed);
    }
}

impl Notify for TracingRouter<'_, '_> {
    /// Queues `notice`, for the input's `send` to drain into the Shell.
    fn notify(&mut self, notice: Notice) {
        crate::notices::Queue.notify(notice);
    }
}

/// `impl lib_settings::LedgerRouter` for the Ledger page. It does nothing: the input the base
/// Unit is changed in announces `Domain::Units` from its commit, so every view of the base Unit
/// follows at once (#1081).
pub(super) struct LedgerRouter;

impl lib_settings::LedgerRouter for LedgerRouter {
    fn base_unit_changed(&mut self) {}
}
