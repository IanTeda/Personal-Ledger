//! The Category Deletion module's entry point for the Categories page's Router ([`open_delete`]).
//! The Dialog's Presenter closes it through the host's `HostRouter`, which puts its Toast onto the
//! bin's queue (`crate::notices`).

use gpui::{App, Context, EventEmitter};
use lib_categories::CategoriesOutcome;
use lib_category_deletion::presenter::DeleteCategoryPresenter;
use lib_core::CategoryId;
use lib_viper::dialog::DialogKeys;

use super::DeletionStores;
use crate::shell::view::dialog_entity::ModuleDialog;
use crate::view::event::ViewEvent;

/// Opens the Delete category Dialog on Category `id` over page `P`, whose `landed` runs with what
/// was deleted once a confirm lands, so the page redraws. Its use is read from `stores` as it
/// opens, and its hints from `keys`. Returns whether it opened: a Category that has gone has no
/// Dialog.
pub fn open_delete<P: EventEmitter<ViewEvent> + 'static>(
    cx: &mut Context<'_, P>,
    stores: &DeletionStores,
    id: CategoryId,
    keys: DialogKeys,
    landed: impl Fn(CategoriesOutcome, &mut App) + 'static,
) -> bool {
    let Some(presenter) =
        DeleteCategoryPresenter::new(stores.deletion.clone(), &stores.reads, id, keys)
    else {
        return false;
    };
    ModuleDialog::open(cx, presenter, landed);
    true
}
