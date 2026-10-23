//! The Toast concern of `Shell`: raising, the clock, pausing and the history modal. An `impl Shell`
//! block, not the Toast model (`lib_toast`) or its drawing (`chrome::toast`).

use std::time::{Duration, Instant};

use chrono::Local;
use gpui::{Context, Timer};
use lib_toast::{Notice, ToastKind};

use super::Shell;
use crate::{
    chrome::dialog_host::{OpenDialog, ToastHistoryDialog},
    navigation::nav::InputMode,
    settings::display::DisplayPreferences,
};

impl Shell {
    /// Opens the session Toast history as a modal, which pauses the Toast timers.
    pub(super) fn open_toast_history(&mut self) {
        self.chrome.palette = None;
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

    /// Shows every Notice the module Routers queued since the last drain (`crate::notices`): a
    /// Toast on the stack, a message on the status line. Runs after a page's `handle`, which
    /// asks for it, and after each key and Dialog click, which a Dialog's Presenter handles.
    /// Returns whether any came, so a key that changed nothing else still redraws.
    pub(super) fn drain_notices(&mut self) -> bool {
        let notices = crate::notices::drain();
        let any = !notices.is_empty();
        for notice in notices {
            match notice {
                Notice::Toast(kind, text) => self.raise_toast(kind, text),
                Notice::Status(text) => self.chrome.status_message = Some(text),
            }
        }
        any
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
    /// popover -- which pauses the Toast timers.
    pub(super) fn modal_open(&self) -> bool {
        self.chrome.palette.is_some()
            || self.file_explorer.is_some()
            || matches!(
                self.nav.mode(),
                InputMode::Command | InputMode::Dialog | InputMode::Filter
            )
    }

    pub(super) fn toast_history_open(&self) -> bool {
        matches!(self.chrome.dialog, Some(OpenDialog::ToastHistory(_)))
    }
}
