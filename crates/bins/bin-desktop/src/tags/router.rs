//! `impl TagsRouter` for the Desktop: where the Tags page's Presenter asks to go. A Dialog opens as
//! an Entity in the Dialog host through the page's gpui context, which is why the Router is built
//! for each `handle` call, and a Notice goes onto the bin's queue (`crate::notices`), which the
//! input's `send` drains. The Remove tag and Merge tags Dialogs are the Tag Removal module's
//! (`crate::tag_removal`, #1003), opened through its entry points.

use gpui::{Context, Entity};
use lib_tags::{SharedPorts, TagsDialog as Which, TagsOutcome, TagsRouter};
use lib_toast::{Notice, Notify};
use lib_transactions::query::TransactionFilters;

use super::TagsSetup;
use super::view::dialog::TagsDialog;
use super::view::page::TagsPage;
use crate::ledger_store::LedgerStore;
use crate::notices::Queue;
use crate::shell::view::dialog_entity::host_dialog;
use crate::tag_removal::RemovalStores;

pub(super) struct PageRouter<'a, 'b> {
    pub ports: SharedPorts,
    pub removal: RemovalStores,
    pub clock: lib_core::Clock,
    pub cx: &'a mut Context<'b, TagsPage>,
}

impl TagsRouter for PageRouter<'_, '_> {
    /// Builds `dialog`'s Presenter and hosts its View: Add and Edit on the shared ports, Remove
    /// and Merge on the Tag Removal module's port. Its confirmed outcome comes back to the page
    /// through [`TagsPage::dialog_landed`].
    fn open_dialog(&mut self, dialog: Which) -> bool {
        let landed = |page: &mut TagsPage, outcome: TagsOutcome, cx: &mut Context<'_, TagsPage>| {
            page.dialog_landed(outcome, cx);
        };
        let page = self.cx.weak_entity();
        let form_landed = Box::new(move |outcome, cx: &mut gpui::App| {
            // A page that has gone has nothing to land on.
            let _ = page.update(cx, |page, cx| page.dialog_landed(outcome, cx));
        });
        let dialog = match dialog {
            Which::RemoveTag(id) => {
                return crate::tag_removal::router::open_remove(self.cx, &self.removal, id, landed);
            }
            Which::MergeTags { source, target } => {
                return crate::tag_removal::router::open_merge(
                    self.cx,
                    &self.removal,
                    source,
                    target,
                    landed,
                );
            }
            Which::AddTag => TagsDialog::add(&self.ports, form_landed),
            Which::EditTag(id) => {
                let Some(dialog) = TagsDialog::edit(&self.ports, id, form_landed) else {
                    return false;
                };
                dialog
            }
        };
        host_dialog(self.cx, |_| dialog);
        true
    }

    /// Shows the Tag's Transactions through the Transactions entry point: the page
    /// filtered to it, from its first row.
    fn open_transactions(&mut self, id: lib_core::TagId) {
        let filters = TransactionFilters::for_tag(self.clock.today(), id);
        crate::transactions::router::show_filtered(self.cx, filters);
    }
}

impl Notify for PageRouter<'_, '_> {
    /// Queues `notice`, for the input's `send` to drain into the Shell.
    fn notify(&mut self, notice: Notice) {
        Queue.notify(notice);
    }
}

/// The Tags module's entry point: its Settings page, built on `setup`'s store, which the Settings
/// View hosts as any other page.
pub fn tags_page(
    setup: TagsSetup,
    ledger: Entity<LedgerStore>,
    clock: lib_core::Clock,
    cx: &mut gpui::App,
) -> Entity<TagsPage> {
    use gpui::AppContext as _;
    cx.new(|cx| TagsPage::new(setup, ledger, clock, cx))
}
