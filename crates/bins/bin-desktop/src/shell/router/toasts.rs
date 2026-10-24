//! The Toast concern of `Root`: showing Notices on the Shell's Presenter, the clock and the history
//! modal. An `impl Root` block, not the Toast stack (the Presenter's, as `lib_toast`'s model) or
//! its drawing (`shell::view::toast`).

use chrono::Local;
use gpui::Context;
use lib_toast::{Notice, ToastKind};

use super::Root;
use crate::{
    settings::display::DisplayPreferences,
    shell::view::dialog_host::{OpenDialog, ToastHistoryDialog},
};

impl Root {
    /// Opens the session Toast history as a modal, which pauses the Toast timers.
    pub(super) fn open_toast_history(&mut self) {
        self.state.palette = None;
        self.open_dialog(OpenDialog::ToastHistory(ToastHistoryDialog));
    }

    /// Sets the Client-scoped Toasts Preference (ADR-0027) from outside Settings (`:toasts`), held
    /// in memory like the Colour Theme Preferences. The Settings View owns it with the other
    /// Display Preferences, so it is told too.
    pub fn set_toasts_on(&mut self, on: bool, cx: &mut gpui::App) {
        let display = DisplayPreferences {
            toasts_on: on,
            ..self.views.settings.read(cx).display(cx)
        };
        self.views
            .settings
            .update(cx, |view, cx| view.set_display(display, cx));
        self.push_display_preferences(display, cx);
    }

    /// Shows every Notice the Routers queued since the last drain (`crate::notices`) on the Shell's
    /// Presenter, stamped with the local time for the Toast history. Each input's `send` runs it
    /// once the input is handled and committed (`crate::input`).
    pub(crate) fn drain_notices(&mut self) {
        for notice in crate::notices::drain() {
            self.presenter.show(notice, Local::now());
        }
    }

    /// Raises a Toast whose Message the caller has already resolved to text.
    pub fn raise_toast(&mut self, kind: ToastKind, text: impl Into<String>) {
        self.presenter
            .show(Notice::Toast(kind, text.into()), Local::now());
    }

    /// Sets the status message from the Shell's own handling (a palette command, an Undo with
    /// nothing built behind it), which the next key clears.
    pub(super) fn set_status(&mut self, text: String) {
        self.presenter.show(Notice::Status(text), Local::now());
    }

    /// Starts the Toast clock for the window's life: every [`crate::shell::view::toast::TICK`] while a
    /// Toast is up, the executor's time to the Presenter, which advances the Toasts unless the
    /// pointer is over them or a modal is open, and a redraw only when the stack changed. A tick
    /// writes nothing, so it's no input through `send`, which would commit and redraw each time. The
    /// executor's timer and clock, so headless tests advance them; an empty stack ticks nothing.
    pub fn start_toast_clock(&self, cx: &mut Context<'_, Self>) {
        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor()
                    .timer(crate::shell::view::toast::TICK)
                    .await;
                let now = cx.background_executor().now();
                // Stops once the window, and with it the Shell, has gone.
                if this
                    .update(cx, |shell, cx| {
                        if shell.presenter.toasts_ticking() && shell.presenter.tick_toasts(now) {
                            cx.notify();
                        }
                    })
                    .is_err()
                {
                    break;
                }
            }
        })
        .detach();
    }

    pub(super) fn toast_history_open(&self) -> bool {
        matches!(self.state.dialog, Some(OpenDialog::ToastHistory(_)))
    }
}
