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
        let mut done = Vec::new();
        for &domain in &failure.domains {
            if done.contains(&domain) {
                continue;
            }
            done.push(domain);
            // A new module's domain adds one arm: how it reloads, and its two Messages.
            let Reload {
                name,
                reloaded,
                restored,
                unsaved,
            } = match domain {
                Domain::Units => Reload {
                    name: "Units",
                    reloaded: crate::units::ports::reload(&self.units, &self.ledger, cx),
                    restored: crate::msg::desktop_toast_write_failed_units,
                    unsaved: crate::msg::desktop_toast_write_failed_units_unsaved,
                },
                Domain::Transactions => Reload {
                    name: "Transactions",
                    reloaded: self
                        .ledger
                        .update(cx, |store, cx| store.reload_transactions(cx)),
                    restored: crate::msg::desktop_toast_write_failed_transactions,
                    unsaved: crate::msg::desktop_toast_write_failed_transactions_unsaved,
                },
                Domain::Bills => Reload {
                    name: "Bills",
                    reloaded: self.ledger.update(cx, |store, cx| store.reload_bills(cx)),
                    restored: crate::msg::desktop_toast_write_failed_bills,
                    unsaved: crate::msg::desktop_toast_write_failed_bills_unsaved,
                },
            };
            let text = match reloaded {
                Ok(()) => restored(),
                Err(error) => {
                    tracing::error!(%error, domain = name, "Failed to reload after a failed write");
                    unsaved()
                }
            };
            self.raise_toast(ToastKind::Error, text);
        }
        cx.notify();
    }
}

/// One domain's reload and the Toast text for each way it ends.
struct Reload {
    name: &'static str,
    reloaded: lib_database::Result<()>,
    restored: fn() -> String,
    unsaved: fn() -> String,
}
