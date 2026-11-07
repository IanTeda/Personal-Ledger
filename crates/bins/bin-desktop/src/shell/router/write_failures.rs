//! The failed-write concern of `Root`: listening on the writer's failure channel, each failure one
//! input (`ShellInput::WriteFailed`), and putting each named domain back as stored, with an Error
//! Toast shown at once (ADR-0046). A commit whose deletes cascade into another domain's rows
//! reloads that domain the same way, without the Toast unless the reload fails (ADR-0046's
//! 2026-11-02 amendment). An `impl Root` block and the root Router's rollback; the writer is
//! `lib_database::Writer`.

use std::sync::mpsc;
use std::time::Duration;

use gpui::{App, Context, Task};
use lib_database::{Domain, WriteFailure};
use lib_shell::ShellInput;
use lib_toast::{Notice, Notify, ToastKind};

use super::{OpenLedger, Root};

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
    /// was, with an Error Toast that the rows aren't saved; called from `input::send` before it
    /// announces and the Notices drain.
    pub(crate) fn reload_cascades(&self, cascades: &[Domain]) {
        let Some(open) = &self.open else {
            return;
        };
        for &domain in cascades {
            if let Err(error) = open.stores.shared().reload(domain) {
                tracing::error!(%error, ?domain, "Failed to reload after a cascade");
                let (_, unsaved) = messages(domain);
                crate::notices::Queue.notify(Notice::Toast(ToastKind::Error, unsaved()));
            }
        }
    }
}

/// Puts each domain `failure` names back as stored and returns an Error Toast for each. The input
/// the failure arrives as announces each domain read back, so every view model reading it is
/// rebuilt. If a domain can't be read back either, its rows in memory stand and the Toast says
/// they aren't saved.
pub(super) fn roll_back(failure: &WriteFailure, open: &OpenLedger, cx: &mut App) -> Vec<Notice> {
    let mut done = Vec::new();
    let mut notices = Vec::new();
    for &domain in &failure.domains {
        if done.contains(&domain) {
            continue;
        }
        done.push(domain);
        let (restored, unsaved) = messages(domain);
        let text = match open.stores.shared().reload(domain) {
            Ok(()) => {
                crate::input::reloaded(domain, cx);
                restored()
            }
            Err(error) => {
                tracing::error!(%error, ?domain, "Failed to reload after a failed write");
                unsaved()
            }
        };
        notices.push(Notice::Toast(ToastKind::Error, text));
    }
    notices
}

/// A Toast's text, read from the Catalogue when it is shown.
type Message = fn() -> String;

/// The Toast text for `domain`'s failed write: read back as stored, and left unsaved. A new
/// domain adds one arm.
fn messages(domain: Domain) -> (Message, Message) {
    match domain {
        Domain::Units => (
            crate::msg::desktop_toast_write_failed_units,
            crate::msg::desktop_toast_write_failed_units_unsaved,
        ),
        Domain::Institutions => (
            crate::msg::desktop_toast_write_failed_institutions,
            crate::msg::desktop_toast_write_failed_institutions_unsaved,
        ),
        Domain::Accounts => (
            crate::msg::desktop_toast_write_failed_accounts,
            crate::msg::desktop_toast_write_failed_accounts_unsaved,
        ),
        Domain::Categories => (
            crate::msg::desktop_toast_write_failed_categories,
            crate::msg::desktop_toast_write_failed_categories_unsaved,
        ),
        Domain::Tags => (
            crate::msg::desktop_toast_write_failed_tags,
            crate::msg::desktop_toast_write_failed_tags_unsaved,
        ),
        Domain::Payees => (
            crate::msg::desktop_toast_write_failed_payees,
            crate::msg::desktop_toast_write_failed_payees_unsaved,
        ),
        Domain::Transactions => (
            crate::msg::desktop_toast_write_failed_transactions,
            crate::msg::desktop_toast_write_failed_transactions_unsaved,
        ),
        Domain::Bills => (
            crate::msg::desktop_toast_write_failed_bills,
            crate::msg::desktop_toast_write_failed_bills_unsaved,
        ),
        Domain::Budgets => (
            crate::msg::desktop_toast_write_failed_budgets,
            crate::msg::desktop_toast_write_failed_budgets_unsaved,
        ),
        Domain::Inventory => (
            crate::msg::desktop_toast_write_failed_inventory,
            crate::msg::desktop_toast_write_failed_inventory_unsaved,
        ),
        // One store holds both, so either failing reads both back.
        Domain::Documents | Domain::DocumentTypes => (
            crate::msg::desktop_toast_write_failed_documents,
            crate::msg::desktop_toast_write_failed_documents_unsaved,
        ),
    }
}
