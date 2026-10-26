//! `impl CategoriesRouter` for the Desktop: where the Categories page's Presenter asks to go. A
//! Dialog opens as an Entity in the Dialog host through the page's gpui context, which is why the
//! Router is built for each `handle` call, and a Notice goes onto the bin's queue
//! (`crate::notices`), which the input's `send` drains.

use chrono::NaiveDate;
use gpui::{Context, Entity};
use lib_categories::{CategoriesDialog as Which, CategoriesOutcome, CategoriesRouter};
use lib_toast::{Notice, Notify};

use super::CategoriesSetup;
use super::view::dialog::CategoriesDialog;
use super::view::page::CategoriesPage;
use crate::ledger_store::LedgerStore;
use crate::notices::Queue;
use crate::shell::view::dialog_entity::host_dialog;

pub(super) struct PageRouter<'a, 'b> {
    pub categories: lib_categories::SharedCategories,
    pub ledger: Entity<LedgerStore>,
    pub today: NaiveDate,
    pub cx: &'a mut Context<'b, CategoriesPage>,
}

impl CategoriesRouter for PageRouter<'_, '_> {
    /// Builds `dialog`'s Presenter on the shared Categories, its usage and budget read from the
    /// Ledger, and hosts its View; its confirmed outcome comes back to the page through
    /// [`CategoriesPage::dialog_landed`].
    fn open_dialog(&mut self, dialog: Which) -> bool {
        let page = self.cx.weak_entity();
        let landed = Box::new(move |outcome, cx: &mut gpui::App| {
            // A page that has gone has nothing to land on.
            let _ = page.update(cx, |page, cx| page.dialog_landed(outcome, cx));
        });
        let Some(dialog) = CategoriesDialog::open(
            dialog,
            &self.categories,
            &self.ledger,
            self.today,
            landed,
            self.cx,
        ) else {
            return false;
        };
        host_dialog(self.cx, |_| dialog);
        true
    }

    /// The page closes no Dialog: a Dialog's own Presenter closes it through the host.
    fn close_dialog(&mut self, _outcome: Option<CategoriesOutcome>) {}
}

impl Notify for PageRouter<'_, '_> {
    /// Queues `notice`, for the input's `send` to drain into the Shell.
    fn notify(&mut self, notice: Notice) {
        Queue.notify(notice);
    }
}

/// The Categories module's entry point: its Settings page, built on `setup`'s store, with the
/// Monthly budget read as of `clock`'s day, which the Settings View hosts as any other page.
pub fn categories_page(
    setup: CategoriesSetup,
    ledger: Entity<LedgerStore>,
    clock: &lib_core::Clock,
    cx: &mut gpui::App,
) -> Entity<CategoriesPage> {
    use gpui::AppContext as _;
    let today = clock.today();
    cx.new(|cx| CategoriesPage::new(setup, ledger, today, cx))
}

/// What a Categories Dialog's Presenter asks: closing, and Notices, which go onto the bin's queue
/// for `Root` to drain once the key or click returns. The page opens Dialogs.
#[derive(Default)]
pub(super) struct DialogRouter {
    /// `Some` once the Presenter closed its Dialog, with what its confirm did.
    pub closed: Option<Option<CategoriesOutcome>>,
}

impl CategoriesRouter for DialogRouter {
    fn open_dialog(&mut self, _dialog: Which) -> bool {
        false
    }

    fn close_dialog(&mut self, outcome: Option<CategoriesOutcome>) {
        self.closed = Some(outcome);
    }
}

impl Notify for DialogRouter {
    fn notify(&mut self, notice: Notice) {
        Queue.notify(notice);
    }
}
