//! A recording Router, for the Presenters' tests and a Client's UI-first work.

use lib_toast::{Notice, Notify};

use crate::page::SettingsPage;
use crate::protocols::SettingsRouter;

/// One thing the frame asked of the Shell, in the order asked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Routed {
    PageChanged(SettingsPage),
    PickerOpened,
    PickerClosed,
    Notice(Notice),
}

/// A [`SettingsRouter`] that records every call.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RecordingRouter {
    pub routed: Vec<Routed>,
}

impl RecordingRouter {
    /// The pages announced, in order.
    pub fn pages_changed(&self) -> Vec<SettingsPage> {
        self.routed
            .iter()
            .filter_map(|routed| match routed {
                Routed::PageChanged(page) => Some(*page),
                _ => None,
            })
            .collect()
    }

    /// The status-line messages sent, in order.
    pub fn statuses(&self) -> Vec<String> {
        self.routed
            .iter()
            .filter_map(|routed| match routed {
                Routed::Notice(Notice::Status(text)) => Some(text.clone()),
                _ => None,
            })
            .collect()
    }
}

impl Notify for RecordingRouter {
    fn notify(&mut self, notice: Notice) {
        self.routed.push(Routed::Notice(notice));
    }
}

impl SettingsRouter for RecordingRouter {
    fn page_changed(&mut self, page: SettingsPage) {
        self.routed.push(Routed::PageChanged(page));
    }

    fn picker_opened(&mut self) {
        self.routed.push(Routed::PickerOpened);
    }

    fn picker_closed(&mut self) {
        self.routed.push(Routed::PickerClosed);
    }
}
