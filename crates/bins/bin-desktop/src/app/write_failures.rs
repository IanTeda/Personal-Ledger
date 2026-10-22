//! The failed-write concern of `Shell`: listening on the writer's failure channel and putting each
//! named domain back as stored, with an error Toast (ADR-0046). An `impl Shell` block; the writer
//! is `lib_database::Writer`.

use std::sync::mpsc;
use std::time::Duration;

use gpui::Context;
use lib_database::{Domain, WriteFailure};
use lib_toast::ToastKind;

use super::Shell;

/// How often the Shell looks for a failed write. The writer reports on a blocking channel, so the
/// Shell polls it rather than holding a thread on it. Public so the headless tests advance past
/// exactly this.
#[doc(hidden)]
pub const WRITE_FAILURE_POLL: Duration = Duration::from_millis(250);

impl Shell {
    /// Polls `failures` for the window's life and rolls each one back. Stops once the writer has
    /// gone, or the window with the Shell. Called once, by `build_shell`.
    pub fn listen_for_write_failures(
        &self,
        failures: mpsc::Receiver<WriteFailure>,
        cx: &mut Context<'_, Self>,
    ) {
        cx.spawn(async move |this, cx| {
            loop {
                // The executor's timer, not `Timer::after`, so headless tests can advance it.
                cx.background_executor().timer(WRITE_FAILURE_POLL).await;
                let failure = match failures.try_recv() {
                    Ok(failure) => failure,
                    Err(mpsc::TryRecvError::Empty) => continue,
                    Err(mpsc::TryRecvError::Disconnected) => break,
                };
                if this
                    .update(cx, |shell, cx| shell.roll_back(&failure, cx))
                    .is_err()
                {
                    break;
                }
            }
        })
        .detach();
    }

    /// Puts each domain `failure` names back as stored, refreshing the Ledger read copy so every
    /// view model reading it is rebuilt, and raises an error Toast for each. If a domain can't be
    /// read back either, its rows in memory stand and the Toast says they aren't saved.
    fn roll_back(&mut self, failure: &WriteFailure, cx: &mut Context<'_, Self>) {
        if failure.domains.contains(&Domain::Units) {
            let text = match crate::units::ports::reload(&self.units, &self.ledger, cx) {
                Ok(()) => crate::msg::desktop_toast_write_failed_units(),
                Err(error) => {
                    tracing::error!(%error, "Failed to reload the Units after a failed write");
                    crate::msg::desktop_toast_write_failed_units_unsaved()
                }
            };
            self.raise_toast(ToastKind::Error, text);
        }
        if failure.domains.contains(&Domain::Transactions) {
            let reloaded = self
                .ledger
                .update(cx, |store, cx| store.reload_transactions(cx));
            let text = match reloaded {
                Ok(()) => crate::msg::desktop_toast_write_failed_transactions(),
                Err(error) => {
                    tracing::error!(%error, "Failed to reload the Transactions after a failed write");
                    crate::msg::desktop_toast_write_failed_transactions_unsaved()
                }
            };
            self.raise_toast(ToastKind::Error, text);
        }
        if failure.domains.contains(&Domain::Bills) {
            let reloaded = self.ledger.update(cx, |store, cx| store.reload_bills(cx));
            let text = match reloaded {
                Ok(()) => crate::msg::desktop_toast_write_failed_bills(),
                Err(error) => {
                    tracing::error!(%error, "Failed to reload the Bills after a failed write");
                    crate::msg::desktop_toast_write_failed_bills_unsaved()
                }
            };
            self.raise_toast(ToastKind::Error, text);
        }
        cx.notify();
    }
}
