//! The Account Deletion module's entry point for other modules' pages ([`open_delete`]), and
//! `impl AccountDeletionRouter` for the Delete Dialog. Its Router records a close for the Dialog
//! to read, and puts a Notice onto the bin's queue (`crate::notices`).

use gpui::{Context, Entity, EventEmitter};
use lib_account_deletion::presenter::DeleteAccountPresenter;
use lib_account_deletion::{AccountDeleted, AccountDeletionRouter};
use lib_core::AccountId;
use lib_toast::{Notice, Notify};

use super::{DeleteAccountDialog, DeletionStores, LedgerReads};
use crate::ledger_store::LedgerStore;
use crate::shell::view::dialog_entity::host_dialog;
use crate::view::event::ViewEvent;

/// Where the Delete Dialog's Presenter asks to go: closing, which the Dialog reads, and Notices,
/// which `Root` drains once the key or click returns.
#[derive(Default)]
pub(super) struct DialogRouter {
    /// `Some` once the Presenter closed the Dialog, with what its confirm removed.
    pub closed: Option<Option<AccountDeleted>>,
}

impl AccountDeletionRouter for DialogRouter {
    fn close_dialog(&mut self, outcome: Option<AccountDeleted>) {
        self.closed = Some(outcome);
    }
}

impl Notify for DialogRouter {
    fn notify(&mut self, notice: Notice) {
        crate::notices::Queue.notify(notice);
    }
}

/// Opens the Delete account Dialog on Account `id` over page `P`, whose `landed` runs with what
/// was removed once a confirm lands, so the page redraws. Its figures are read from `ledger` as it
/// opens. Returns whether it opened: an Account that has gone has no Dialog.
pub fn open_delete<P: EventEmitter<ViewEvent> + 'static>(
    cx: &mut Context<'_, P>,
    stores: &DeletionStores,
    ledger: &Entity<LedgerStore>,
    id: AccountId,
    landed: impl Fn(&mut P, AccountDeleted, &mut Context<'_, P>) + 'static,
) -> bool {
    let Some(presenter) =
        DeleteAccountPresenter::new(stores.deletion.clone(), &LedgerReads(ledger.read(cx)), id)
    else {
        return false;
    };
    let page = cx.weak_entity();
    let landed = Box::new(move |outcome, cx: &mut gpui::App| {
        // A page that has gone has nothing to land on.
        let _ = page.update(cx, |page, cx| landed(page, outcome, cx));
    });
    let dialog = DeleteAccountDialog::new(presenter, &stores.bills, ledger, landed);
    host_dialog(cx, |_| dialog);
    true
}
