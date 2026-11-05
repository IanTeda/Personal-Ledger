//! The Category Deletion module's entry point for the Categories page's Router ([`open_delete`]).
//! The Dialog's Presenter closes it through the host's `HostRouter`, which puts its Toast onto the
//! bin's queue (`crate::notices`).

use gpui::{Context, EventEmitter};
use lib_categories::CategoriesOutcome;
use lib_category_deletion::presenter::DeleteCategoryPresenter;
use lib_core::CategoryId;

use super::view::dialog::Landed;
use super::{CategoryDeletionDialog, DeletionStores};
use crate::shell::view::dialog_entity::host_dialog;
use crate::view::event::ViewEvent;

/// Opens the Delete category Dialog on Category `id` over page `P`, whose `landed` runs with what
/// was deleted once a confirm lands, so the page redraws. Its use is read from `stores` as it
/// opens. Returns whether it opened: a Category that has gone has no Dialog.
pub fn open_delete<P: EventEmitter<ViewEvent> + 'static>(
    cx: &mut Context<'_, P>,
    stores: &DeletionStores,
    id: CategoryId,
    landed: impl Fn(&mut P, CategoriesOutcome, &mut Context<'_, P>) + 'static,
) -> bool {
    let Some(presenter) = DeleteCategoryPresenter::new(stores.deletion.clone(), &stores.reads, id)
    else {
        return false;
    };
    let page = cx.weak_entity();
    let landed: Landed = Box::new(move |outcome, cx: &mut gpui::App| {
        // A page that has gone has nothing to land on.
        let _ = page.update(cx, |page, cx| landed(page, outcome, cx));
    });
    host_dialog(cx, |_| CategoryDeletionDialog::new(presenter, landed));
    true
}
