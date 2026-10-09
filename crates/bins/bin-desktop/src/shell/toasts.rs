//! The Toast concern of `Shell`: raising, the clock, pausing and the history modal. An `impl Shell`
//! block, not the Toast model (`lib_toast`) or its drawing (`chrome::toast`).

use std::time::{Duration, Instant};

use chrono::Local;
use gpui::{Context, Timer};
use lib_toast::ToastKind;

use super::Shell;
use crate::{
    chrome::dialog_host::{OpenDialog, ToastHistoryDialog},
    navigation::nav::InputMode,
};

impl Shell {
    /// Opens the session Toast history as a modal, which pauses the Toast timers.
    pub(super) fn open_toast_history(&mut self) {
        self.chrome.palette = None;
        self.open_dialog(OpenDialog::ToastHistory(ToastHistoryDialog));
    }

    /// Sets the Client-scoped Toasts Preference (ADR-0027), held in memory like the Colour Theme
    /// Preferences. The Desktop can always draw a Toast, so only `toasts_on` ever changes.
    pub fn set_toasts_on(&mut self, on: bool) {
        self.chrome.toasts.set_display(lib_toast::Display {
            toasts_on: on,
            ..self.chrome.toasts.display()
        });
    }

    /// Raises a Toast whose Message the caller has already resolved to text.
    pub fn raise_toast(&mut self, kind: ToastKind, text: impl Into<String>) {
        self.chrome.toasts.raise(kind, text, Local::now());
    }

    /// Starts the Toast clock for the window's life: every [`crate::chrome::toast::TICK`] it advances
    /// the model by the real time elapsed, paused while the pointer is over the stack or a modal
    /// surface is open, and redraws only when a Toast has gone.
    pub fn start_toast_clock(&self, cx: &mut Context<'_, Self>) {
        cx.spawn(async move |this, cx| {
            let mut last = Instant::now();
            loop {
                Timer::after(crate::chrome::toast::TICK).await;
                let now = Instant::now();
                let elapsed = now - last;
                last = now;
                // Stops once the window, and with it the Shell, has gone.
                if this
                    .update(cx, |shell, cx| {
                        if shell.advance_toasts(elapsed) {
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

    /// One clock tick; `true` when the stack changed and needs a redraw.
    pub(super) fn advance_toasts(&mut self, elapsed: Duration) -> bool {
        let before = (
            self.chrome.toasts.visible().len(),
            self.chrome.toasts.more_count(),
            self.chrome.toasts.echo().is_some(),
        );
        if before.0 == 0 {
            // A dismissed stack never reports the pointer leaving it.
            self.chrome.toasts_hovered = false;
            return false;
        }
        if self.chrome.toasts_hovered || self.modal_open() {
            self.chrome.toasts.pause();
        } else {
            self.chrome.toasts.resume();
        }
        self.chrome.toasts.advance(elapsed);
        before
            != (
                self.chrome.toasts.visible().len(),
                self.chrome.toasts.more_count(),
                self.chrome.toasts.echo().is_some(),
            )
    }

    /// Whether a modal surface is open -- the palette, the file explorer, a dialog, the filter
    /// popover or the help overlay -- which pauses the Toast timers.
    pub(super) fn modal_open(&self) -> bool {
        self.chrome.palette.is_some()
            || self.file_explorer.is_some()
            || matches!(
                self.nav.mode(),
                InputMode::Command | InputMode::Dialog | InputMode::Filter | InputMode::Help
            )
    }

    pub(super) fn toast_history_open(&self) -> bool {
        matches!(self.chrome.dialog, Some(OpenDialog::ToastHistory(_)))
    }
}
