//! The Account Deletion module's entry point for other modules' pages ([`open_delete`]). The
//! Delete Dialog's Presenter closes it through the host's `HostRouter`, which puts a Notice onto
//! the bin's queue (`crate::notices`).

use gpui::{Context, Entity, EventEmitter};
use lib_account_deletion::AccountDeleted;
use lib_account_deletion::presenter::DeleteAccountPresenter;
use lib_core::AccountId;

use super::{DeleteAccountDialog, DeletionStores, LedgerReads};
use crate::ledger_store::LedgerStore;
use crate::shell::view::dialog_entity::host_dialog;
use crate::view::event::ViewEvent;

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
