//! The Tag Removal module's entry points for the Tags page's Router ([`open_remove`],
//! [`open_merge`]). Each Dialog's Presenter closes it through the host's `HostRouter`, which puts
//! its Toast onto the bin's queue (`crate::notices`).

use gpui::{Context, EventEmitter};
use lib_core::TagId;
use lib_tag_removal::presenter::{MergeTagsPresenter, RemoveTagPresenter};
use lib_tags::TagsOutcome;

use super::view::dialog::{Landed, View};
use super::{RemovalStores, TagRemovalDialog};
use crate::shell::view::dialog_entity::host_dialog;
use crate::view::event::ViewEvent;

/// Opens the Remove tag Dialog on Tag `id` over page `P`, whose `landed` runs with what was
/// removed once a confirm lands, so the page redraws. Its usage is read from `stores` as it opens.
/// Returns whether it opened: a Tag that has gone has no Dialog.
pub fn open_remove<P: EventEmitter<ViewEvent> + 'static>(
    cx: &mut Context<'_, P>,
    stores: &RemovalStores,
    id: TagId,
    landed: impl Fn(&mut P, TagsOutcome, &mut Context<'_, P>) + 'static,
) -> bool {
    let Some(presenter) = RemoveTagPresenter::new(stores.removal.clone(), &stores.reads, id) else {
        return false;
    };
    host(cx, View::Remove(presenter), landed);
    true
}

/// Opens the Merge tags Dialog over page `P` with `source` and `target` chosen where given, whose
/// `landed` runs with the target once a confirm lands. Its options are read from `stores` as it
/// opens. It always opens: a Tag that has gone is only left unchosen.
pub fn open_merge<P: EventEmitter<ViewEvent> + 'static>(
    cx: &mut Context<'_, P>,
    stores: &RemovalStores,
    source: Option<TagId>,
    target: Option<TagId>,
    landed: impl Fn(&mut P, TagsOutcome, &mut Context<'_, P>) + 'static,
) -> bool {
    let presenter = MergeTagsPresenter::new(stores.removal.clone(), &stores.reads, source, target);
    host(cx, View::Merge(presenter, 0, 0), landed);
    true
}

/// Hosts `view` over page `P`, handing a confirmed outcome to `landed`.
fn host<P: EventEmitter<ViewEvent> + 'static>(
    cx: &mut Context<'_, P>,
    view: View,
    landed: impl Fn(&mut P, TagsOutcome, &mut Context<'_, P>) + 'static,
) {
    let page = cx.weak_entity();
    let landed: Landed = Box::new(move |outcome, cx: &mut gpui::App| {
        // A page that has gone has nothing to land on.
        let _ = page.update(cx, |page, cx| landed(page, outcome, cx));
    });
    host_dialog(cx, |_| TagRemovalDialog::new(view, landed));
}
