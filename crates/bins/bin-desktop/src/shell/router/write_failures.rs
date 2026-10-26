//! The failed-write concern of `Root`: listening on the writer's failure channel, each failure one
//! input (`ShellInput::WriteFailed`), and putting each named domain back as stored, with an Error
//! Toast shown at once (ADR-0046). An `impl Root` block and the root Router's rollback; the
//! writer is `lib_database::Writer`.

use std::sync::mpsc;
use std::time::Duration;

use gpui::{App, Context, Entity};
use lib_database::{Domain, WriteFailure};
use lib_shell::ShellInput;
use lib_toast::{Notice, ToastKind};

use super::Root;
use crate::ledger_store::LedgerStore;

/// How often the Shell looks for a failed write. The writer reports on a blocking channel, so the
/// Shell polls it rather than holding a thread on it. Public so the headless tests advance past
/// exactly this.
#[doc(hidden)]
pub const WRITE_FAILURE_POLL: Duration = Duration::from_millis(250);

impl Root {
    /// Polls `failures` for the window's life and sends each one as an input, which rolls it back
    /// and redraws without waiting for a keypress. Its commit is empty, and it isn't a key, so it
    /// leaves the status line be. Stops once the writer has gone, or the window with the Shell.
    /// Called once, by `shell::router::build`.
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
                    .update(cx, |shell, cx| {
                        shell.send_shell(ShellInput::WriteFailed(failure), cx);
                    })
                    .is_err()
                {
                    break;
                }
            }
        })
        .detach();
    }
}

/// Puts each domain `failure` names back as stored, refreshing the Ledger read copy so every view
/// model reading it is rebuilt, and returns an Error Toast for each. If a domain can't be read
/// back either, its rows in memory stand and the Toast says they aren't saved.
pub(super) fn roll_back(
    failure: &WriteFailure,
    units: &crate::units::DesktopPorts,
    ledger: &Entity<LedgerStore>,
    cx: &mut App,
) -> Vec<Notice> {
    let mut done = Vec::new();
    let mut notices = Vec::new();
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
                reloaded: crate::units::ports::reload(units, ledger, cx),
                restored: crate::msg::desktop_toast_write_failed_units,
                unsaved: crate::msg::desktop_toast_write_failed_units_unsaved,
            },
            Domain::Institutions => Reload {
                name: "Institutions",
                reloaded: ledger.update(cx, |store, cx| store.reload_institutions(cx)),
                restored: crate::msg::desktop_toast_write_failed_institutions,
                unsaved: crate::msg::desktop_toast_write_failed_institutions_unsaved,
            },
            Domain::Accounts => Reload {
                name: "Accounts",
                reloaded: ledger.update(cx, |store, cx| store.reload_accounts(cx)),
                restored: crate::msg::desktop_toast_write_failed_accounts,
                unsaved: crate::msg::desktop_toast_write_failed_accounts_unsaved,
            },
            Domain::Payees => Reload {
                name: "Payees",
                reloaded: ledger.update(cx, |store, cx| store.reload_payees(cx)),
                restored: crate::msg::desktop_toast_write_failed_payees,
                unsaved: crate::msg::desktop_toast_write_failed_payees_unsaved,
            },
            Domain::Transactions => Reload {
                name: "Transactions",
                reloaded: ledger.update(cx, |store, cx| store.reload_transactions(cx)),
                restored: crate::msg::desktop_toast_write_failed_transactions,
                unsaved: crate::msg::desktop_toast_write_failed_transactions_unsaved,
            },
            Domain::Bills => Reload {
                name: "Bills",
                reloaded: ledger.update(cx, |store, cx| store.reload_bills(cx)),
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
        notices.push(Notice::Toast(ToastKind::Error, text));
    }
    notices
}

/// One domain's reload and the Toast text for each way it ends.
struct Reload {
    name: &'static str,
    reloaded: lib_database::Result<()>,
    restored: fn() -> String,
    unsaved: fn() -> String,
}
