//! The failed-write concern of `Root`: listening on the writer's failure channel, each failure one
//! input (`ShellInput::WriteFailed`), and putting each named domain back as stored, with an Error
//! Toast shown at once (ADR-0046). A commit whose deletes cascade into another domain's rows
//! reloads that domain the same way, without the Toast unless the reload fails (ADR-0046's
//! 2026-11-02 amendment). An `impl Root` block and the root Router's rollback; the writer is
//! `lib_database::Writer`.

use std::sync::mpsc;
use std::time::Duration;

use gpui::{App, Context, Entity, Task};
use lib_database::{Domain, WriteFailure};
use lib_shell::ShellInput;
use lib_toast::{Notice, Notify, ToastKind};

use super::{OpenLedger, Root};
use crate::announcer::{Announcer, Domain as Announced};

/// How often the Shell looks for a failed write. The writer reports on a blocking channel, so the
/// Shell polls it rather than holding a thread on it. Public so the headless tests advance past
/// exactly this.
#[doc(hidden)]
pub const WRITE_FAILURE_POLL: Duration = Duration::from_millis(250);

impl Root {
    /// Polls `failures` for as long as the returned task is held and sends each one as an input,
    /// which rolls it back and redraws without waiting for a keypress. Its commit is empty, and it
    /// isn't a key, so it leaves the status line be. Stops once the writer has gone, the window
    /// with the Shell, or the Open Ledger holding the task.
    pub(super) fn listen_for_write_failures(
        failures: std::rc::Rc<mpsc::Receiver<WriteFailure>>,
        cx: &mut Context<'_, Self>,
    ) -> Task<()> {
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
    }

    /// Reads back each domain in `cascades`, which the input's commit changed in the file by
    /// cascade, so memory drops the rows the file did: the Document Links to a deleted record.
    /// Each reload goes through the writer, behind the commit. One that fails leaves memory as it
    /// was, with an Error Toast that the rows aren't saved; called from `input::send` before the
    /// Notices drain.
    pub(crate) fn reload_cascades(&mut self, cascades: &[Domain], cx: &mut Context<'_, Self>) {
        let Some(open) = &self.open else {
            return;
        };
        for &domain in cascades {
            let reload = reload(domain, open, cx);
            if let Err(error) = reload.reloaded {
                tracing::error!(%error, domain = reload.name, "Failed to reload after a cascade");
                crate::notices::Queue.notify(Notice::Toast(ToastKind::Error, (reload.unsaved)()));
            }
        }
    }
}

/// Puts each domain `failure` names back as stored, reloading its adapter and announcing it so
/// every view model reading it is rebuilt, and returns an Error Toast for each. If a domain can't
/// be read back either, its rows in memory stand and the Toast says they aren't saved.
pub(super) fn roll_back(failure: &WriteFailure, open: &OpenLedger, cx: &mut App) -> Vec<Notice> {
    let mut done = Vec::new();
    let mut notices = Vec::new();
    for &domain in &failure.domains {
        if done.contains(&domain) {
            continue;
        }
        done.push(domain);
        let Reload {
            name,
            reloaded,
            restored,
            unsaved,
        } = reload(domain, open, cx);
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

/// Reads `domain` back as stored through the adapter the Open Ledger holds, then announces it so
/// every view model reading it is rebuilt, with the Messages for how it ended. A new module's
/// domain adds one arm: how it reloads, and its two Messages.
fn reload(domain: Domain, open: &OpenLedger, cx: &mut App) -> Reload {
    let stores = &open.stores;
    let announcer = &open.announcer;
    match domain {
        Domain::Units => Reload {
            name: "Units",
            reloaded: crate::units::ports::reload(&open.units, announcer, cx),
            restored: crate::msg::desktop_toast_write_failed_units,
            unsaved: crate::msg::desktop_toast_write_failed_units_unsaved,
        },
        Domain::Institutions => Reload {
            name: "Institutions",
            reloaded: crate::institutions::ports::reload(&stores.institutions, announcer, cx),
            restored: crate::msg::desktop_toast_write_failed_institutions,
            unsaved: crate::msg::desktop_toast_write_failed_institutions_unsaved,
        },
        Domain::Accounts => Reload {
            name: "Accounts",
            reloaded: crate::accounts::ports::reload(&stores.accounts, announcer, cx),
            restored: crate::msg::desktop_toast_write_failed_accounts,
            unsaved: crate::msg::desktop_toast_write_failed_accounts_unsaved,
        },
        Domain::Categories => Reload {
            name: "Categories",
            reloaded: crate::categories::ports::reload(&stores.category_stores(), announcer, cx),
            restored: crate::msg::desktop_toast_write_failed_categories,
            unsaved: crate::msg::desktop_toast_write_failed_categories_unsaved,
        },
        Domain::Tags => Reload {
            name: "Tags",
            reloaded: crate::tags::ports::reload(&stores.tags, announcer, cx),
            restored: crate::msg::desktop_toast_write_failed_tags,
            unsaved: crate::msg::desktop_toast_write_failed_tags_unsaved,
        },
        Domain::Payees => Reload {
            name: "Payees",
            reloaded: crate::payees::ports::reload(&stores.payees, announcer, cx),
            restored: crate::msg::desktop_toast_write_failed_payees,
            unsaved: crate::msg::desktop_toast_write_failed_payees_unsaved,
        },
        Domain::Transactions => Reload {
            name: "Transactions",
            reloaded: reload_and_announce(
                || stores.transactions.borrow_mut().reload(),
                &[Announced::Transactions],
                announcer,
                cx,
            ),
            restored: crate::msg::desktop_toast_write_failed_transactions,
            unsaved: crate::msg::desktop_toast_write_failed_transactions_unsaved,
        },
        Domain::Bills => Reload {
            name: "Bills",
            reloaded: reload_and_announce(
                || stores.bills.borrow_mut().reload(),
                &[Announced::Bills],
                announcer,
                cx,
            ),
            restored: crate::msg::desktop_toast_write_failed_bills,
            unsaved: crate::msg::desktop_toast_write_failed_bills_unsaved,
        },
        Domain::Budgets => Reload {
            name: "Budgets",
            reloaded: reload_and_announce(
                || stores.budgets.borrow_mut().reload(),
                &[Announced::Budgets],
                announcer,
                cx,
            ),
            restored: crate::msg::desktop_toast_write_failed_budgets,
            unsaved: crate::msg::desktop_toast_write_failed_budgets_unsaved,
        },
        Domain::Inventory => Reload {
            name: "Inventory",
            reloaded: reload_and_announce(
                || stores.inventory.borrow_mut().reload(),
                &[Announced::Inventory],
                announcer,
                cx,
            ),
            restored: crate::msg::desktop_toast_write_failed_inventory,
            unsaved: crate::msg::desktop_toast_write_failed_inventory_unsaved,
        },
        Domain::Documents => Reload {
            name: "Documents",
            reloaded: reload_and_announce(
                || stores.documents.borrow_mut().reload(),
                &[Announced::Documents, Announced::DocumentTypes],
                announcer,
                cx,
            ),
            restored: crate::msg::desktop_toast_write_failed_documents,
            unsaved: crate::msg::desktop_toast_write_failed_documents_unsaved,
        },
    }
}

/// Runs `reload`, and once it has read the rows back announces each of `what`; a reload that
/// fails announces nothing, since memory is as it was.
fn reload_and_announce(
    reload: impl FnOnce() -> lib_database::Result<()>,
    what: &[Announced],
    announcer: &Entity<Announcer>,
    cx: &mut App,
) -> lib_database::Result<()> {
    reload()?;
    announcer.update(cx, |announcer, cx| {
        for &domain in what {
            announcer.announce(domain, cx);
        }
    });
    Ok(())
}

/// One domain's reload and the Toast text for each way it ends.
struct Reload {
    name: &'static str,
    reloaded: lib_database::Result<()>,
    restored: fn() -> String,
    unsaved: fn() -> String,
}
