//! The Account Deletion module's entry point for other modules' pages ([`open_delete`]). The
//! Delete Dialog's Presenter closes it through the host's `HostRouter`, which puts a Notice onto
//! the bin's queue (`crate::notices`).

use gpui::{App, Context, EventEmitter};
use lib_account_deletion::AccountDeleted;
use lib_account_deletion::presenter::DeleteAccountPresenter;
use lib_core::AccountId;
use lib_viper::dialog::DialogKeys;

use super::DeletionStores;
use crate::shell::view::dialog_entity::ModuleDialog;
use crate::view::event::ViewEvent;

/// Opens the Delete account Dialog on Account `id` over page `P`, whose `landed` runs with what
/// was removed once a confirm lands, so the page redraws. Its figures are read from the stores as
/// it opens, and its hints from `keys`. Returns whether it opened: an Account that has gone has no
/// Dialog.
pub fn open_delete<P: EventEmitter<ViewEvent> + 'static>(
    cx: &mut Context<'_, P>,
    stores: &DeletionStores,
    id: AccountId,
    keys: DialogKeys,
    landed: impl Fn(AccountDeleted, &mut App) + 'static,
) -> bool {
    let Some(presenter) =
        DeleteAccountPresenter::new(stores.deletion.clone(), &stores.reads, id, keys)
    else {
        return false;
    };
    ModuleDialog::open(cx, presenter, landed);
    true
}
