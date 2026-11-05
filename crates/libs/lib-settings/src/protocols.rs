//! What the Settings frame asks of the Shell, through [`SettingsRouter`], which each Client
//! implements and lends to every `handle` call. A refusal goes to the status line through
//! [`Notify`] (ADR-0048).

use lib_toast::Notify;

use crate::page::SettingsPage;

/// What only the Shell does for the Settings frame.
pub trait SettingsRouter: Notify {
    /// The page on show changed by the person's hand (a key, a click, a command), so the Shell
    /// records it in its history (`NavInput::SettingsPage`) for Back and Forward. A restored page
    /// is not announced, since the Shell restored it.
    fn page_changed(&mut self, page: SettingsPage);

    /// The page picker opened, so the Shell's input mode becomes the picker's and every key
    /// reaches the frame until it closes.
    fn picker_opened(&mut self);

    /// The page picker closed, so the Shell's input mode returns to normal.
    fn picker_closed(&mut self);
}
