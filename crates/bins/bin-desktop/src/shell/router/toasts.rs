//! The Toast concern of `Root`: showing Notices on the Shell's Presenter, the clock and the history
//! modal. An `impl Root` block, not the Toast stack (the Presenter's, as `lib_toast`'s model) or
//! its drawing (`shell::view::toast`).

use chrono::Local;
use gpui::{App, AppContext as _, Context};
use lib_toast::{Notice, ToastKind};

use super::Root;
use lib_shell::ToastHistoryPresenter;

use crate::shell::view::dialog_entity::DialogHandle;
use crate::shell::view::toast::history::ToastHistoryDialog;

impl Root {
    /// Opens the session Toast history as a modal, which pauses the Toast timers.
    pub(super) fn open_toast_history(&mut self, cx: &mut Context<'_, Self>) {
        self.presenter.close_palette();
        let presenter = ToastHistoryPresenter::open(self.presenter.toasts().history());
        let dialog = cx.new(|_| ToastHistoryDialog::new(presenter));
        self.host_dialog(DialogHandle::new(dialog), cx);
    }

    /// Shows every Notice the Routers queued since the last drain (`crate::notices`) on the Shell's
    /// Presenter, stamped with the local time for the Toast history. Each input's `send` runs it
    /// once the input is handled and committed (`crate::input`). An open Toast history card reads
    /// them too, since it hides the Toast stack.
    pub(crate) fn drain_notices(&mut self, cx: &mut App) {
        let mut shown = false;
        for notice in crate::notices::drain() {
            self.presenter.show(notice, Local::now());
            shown = true;
        }
        if !shown {
            return;
        }
        if let Some(dialog) = self
            .hosted_dialog()
            .and_then(|dialog| dialog.downcast::<ToastHistoryDialog>())
        {
            let history = self.presenter.toasts().history();
            dialog.update(cx, |dialog, cx| dialog.refresh(history, cx));
        }
    }

    /// Says a Ledger file start-up or `:open` was told to open couldn't be, as an Error Toast naming
    /// it and why (#977). The Shell keeps whatever Ledger was open, or none.
    pub fn ledger_open_failed(&mut self, failure: &crate::OpenFailure) {
        self.raise_toast(
            ToastKind::Error,
            lib_locale::msg::toast_ledger_open_failed(
                &super::explorer::file_name(&failure.path),
                &failure.reason,
            ),
        );
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
        self.hosted_dialog()
            .is_some_and(|dialog| dialog.downcast::<ToastHistoryDialog>().is_some())
    }
}
