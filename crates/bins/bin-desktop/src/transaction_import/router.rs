//! `impl ImportRouter` for the Import step: where its Presenter asks to go, through the generic
//! [`PageRouter`]'s gpui context, which is why the Router is built for each `handle` call
//! (`PageView::with_router`). A commit lands on the Transactions list's first row, inside the same
//! input; the Router queues a Notice for the input's `send` to drain.

use gpui::{AppContext as _, Entity};
use lib_core::Clock;
use lib_transaction_import::presenter::ImportStepPresenter;
use lib_transaction_import::{ImportCommitted, ImportRouter};

use super::{ImportPage, ImportStores};
use crate::announcer::Announcer;
use crate::shell::view::module_page::ModulePage;
use crate::shell::view::page_router::PageRouter;

impl ImportRouter for PageRouter<'_, '_, ImportStepPresenter> {
    /// The step has closed itself; a commit puts the Transactions list back on its first row.
    fn close_import(&mut self, committed: Option<ImportCommitted>) {
        if committed.is_some() {
            crate::transactions::router::first_row(self.cx);
        }
    }
}

/// The Transaction Import module's entry point: the step, closed, over `stores`, under `config`'s
/// keys. `clock` dates the statement as the step opens.
pub fn import_page(
    stores: ImportStores,
    announcer: &Entity<Announcer>,
    config: &lib_config::KeyBindingConfig,
    clock: Clock,
    cx: &mut gpui::App,
) -> Entity<ImportPage> {
    let presenter = ImportStepPresenter::new(move || stores.ports(&clock), config);
    cx.new(|cx| ModulePage::new(presenter, (), announcer, cx))
}
