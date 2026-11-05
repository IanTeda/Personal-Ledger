//! `impl SettingsRouter` for the Desktop: what the Settings frame's Presenter asks of the Shell. It
//! goes through the Settings View's gpui context, which is why the Router is built for each
//! `handle` call. A page change reaches the root Router as [`PageChanged`], which feeds the
//! Shell's history; the picker's mode is the View's own (`ViewEvent::EnterMode`); a Notice goes
//! onto the bin's queue (`crate::notices`), which the input's `send` drains.

use gpui::Context;
use lib_settings::{SettingsPage, SettingsRouter};
use lib_toast::{Notice, Notify};

use super::state::SettingsView;
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
