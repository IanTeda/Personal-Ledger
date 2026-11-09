//! `impl TagsRouter` for the Desktop: where the Tags page's Presenter asks to go. A Dialog opens as
//! a `ModuleDialog` in the Dialog host through the page's gpui context, which is why the Router is
//! built for each `handle` call (`PageView::with_router`), and a Notice goes onto the bin's queue
//! (`crate::notices`), which the input's `send` drains. The Remove tag and Merge tags Dialogs are
//! the Tag Removal module's (`crate::tag_removal`, #1003), opened through its entry points.

use gpui::{AppContext as _, Context, Entity};
use lib_tags::presenter::TagFormPresenter;
use lib_tags::{TagsDialog as Which, TagsRouter};
use lib_toast::{Notice, Notify};
use lib_transactions::query::TransactionFilters;

use super::{TagsPage, TagsRoutes, TagsSetup};
use crate::announcer::Announcer;
use crate::notices::Queue;
use crate::shell::view::dialog_entity::ModuleDialog;
use crate::shell::view::module_page::ModulePage;

pub(super) struct PageRouter<'a, 'b> {
    pub routes: &'a TagsRoutes,
    pub cx: &'a mut Context<'b, TagsPage>,
}

impl TagsRouter for PageRouter<'_, '_> {
    /// Builds `dialog`'s Presenter and hosts it: Add and Edit on the shared ports, Remove and
    /// Merge on the Tag Removal module's port, each handed the Dialog keys. Its confirmed outcome
    /// comes back to the page through [`ModulePage::landing`].
    fn open_dialog(&mut self, dialog: Which) -> bool {
        let routes = self.routes;
        let landed = TagsPage::landing(self.cx);
        let keys = routes.dialog_keys.get();
        let form = match dialog {
            Which::RemoveTag(id) => {
                return crate::tag_removal::router::open_remove(
                    self.cx,
                    &routes.removal,
                    id,
                    keys,
                    landed,
                );
            }
            Which::MergeTags { source, target } => {
                return crate::tag_removal::router::open_merge(
                    self.cx,
                    &routes.removal,
                    source,
                    target,
                    keys,
                    landed,
                );
            }
            Which::AddTag => TagFormPresenter::add(routes.ports.clone(), keys),
            Which::EditTag(id) => {
                let Some(form) = TagFormPresenter::edit(routes.ports.clone(), id, keys) else {
                    return false;
                };
                form
            }
        };
        ModuleDialog::open(self.cx, form, landed);
        true
    }

    /// Shows the Tag's Transactions through the Transactions entry point: the page
    /// filtered to it, from its first row.
    fn open_transactions(&mut self, id: lib_core::TagId) {
        let filters = TransactionFilters::for_tag(self.routes.clock.today(), id);
        crate::transactions::router::show_filtered(self.cx, filters);
    }
}

impl Notify for PageRouter<'_, '_> {
    /// Queues `notice`, for the input's `send` to drain into the Shell.
    fn notify(&mut self, notice: Notice) {
        Queue.notify(notice);
    }
}

/// The Tags module's entry point: its Settings page, built on `setup`'s ports and dated by
/// `clock`, which the Settings View hosts as any other page.
pub fn tags_page(
    setup: TagsSetup,
    announcer: Entity<Announcer>,
    clock: lib_core::Clock,
    cx: &mut gpui::App,
) -> Entity<TagsPage> {
    let TagsSetup {
        presenter,
        ports,
        removal,
        dialog_keys,
    } = setup;
    let routes = TagsRoutes {
        ports,
        removal,
        dialog_keys,
        clock,
    };
    cx.new(|cx| ModulePage::new(presenter, routes, &announcer, cx))
}
