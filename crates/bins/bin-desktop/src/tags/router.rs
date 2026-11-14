//! `impl TagsRouter` for the Desktop: where the Tags page's Presenter asks to go. A Dialog opens as
//! a `ModuleDialog` in the Dialog host through the generic [`PageRouter`], built for each `handle`
//! call (`PageView::with_router`), which queues a Notice for the input's `send` to drain. The
//! Remove tag and Merge tags Dialogs are the Tag Removal module's (`crate::tag_removal`, #1003),
//! opened through its entry points.

use gpui::{AppContext as _, Entity};
use lib_tags::presenter::{TagFormPresenter, TagsPagePresenter};
use lib_tags::{TagsDialog as Which, TagsRouter};
use lib_transactions::query::TransactionFilters;

use super::{TagsPage, TagsSetup};
use crate::announcer::Announcer;
use crate::shell::view::module_page::ModulePage;
use crate::shell::view::page_router::PageRouter;

impl TagsRouter for PageRouter<'_, '_, TagsPagePresenter> {
    /// Builds `dialog`'s Presenter and hosts it: Add and Edit on the shared ports, Remove and
    /// Merge on the Tag Removal module's port, each handed the Dialog keys. Its confirmed outcome
    /// comes back to the page through [`ModulePage::landing`].
    fn open_dialog(&mut self, dialog: Which) -> bool {
        let setup = self.setup;
        let keys = setup.dialog_keys.get();
        let form = match dialog {
            Which::RemoveTag(id) => {
                let landed = self.landing();
                return crate::tag_removal::router::open_remove(
                    self.cx,
                    &setup.removal,
                    id,
                    keys,
                    landed,
                );
            }
            Which::MergeTags { source, target } => {
                let landed = self.landing();
                return crate::tag_removal::router::open_merge(
                    self.cx,
                    &setup.removal,
                    source,
                    target,
                    keys,
                    landed,
                );
            }
            Which::AddTag => TagFormPresenter::add(setup.ports.clone(), keys),
            Which::EditTag(id) => {
                let Some(form) = TagFormPresenter::edit(setup.ports.clone(), id, keys) else {
                    return false;
                };
                form
            }
        };
        self.open(form);
        true
    }

    /// Shows the Tag's Transactions through the Transactions entry point: the page
    /// filtered to it, from its first row.
    fn open_transactions(&mut self, id: lib_core::TagId) {
        let filters = TransactionFilters::for_tag(self.setup.clock.today(), id);
        crate::transactions::router::show_filtered(self.cx, filters);
    }
}

/// The Tags module's entry point: its Settings page, `presenter` routed over `setup`, which the
/// Settings View hosts as any other page.
pub fn tags_page(
    presenter: TagsPagePresenter,
    setup: TagsSetup,
    announcer: Entity<Announcer>,
    cx: &mut gpui::App,
) -> Entity<TagsPage> {
    cx.new(|cx| ModulePage::new(presenter, setup, &announcer, cx))
}
