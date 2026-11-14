//! `impl DocumentTypesRouter` for the Desktop: where the Document Types page's Presenter asks to
//! go. A Dialog opens as a `ModuleDialog` in the Dialog host through the generic [`PageRouter`],
//! built for each `handle` call (`PageView::with_router`), which queues a Notice for the input's
//! `send` to drain.

use gpui::{AppContext as _, Entity};
use lib_documents::presenter::{
    DefaultNoticePresenter, DocumentTypesPagePresenter, RemoveTypePresenter, TypeFormPresenter,
};
use lib_documents::{DocumentTypesDialog as Which, DocumentTypesRouter};

use super::{DocumentTypesPage, DocumentTypesSetup};
use crate::announcer::Announcer;
use crate::shell::view::module_page::ModulePage;
use crate::shell::view::page_router::PageRouter;

impl DocumentTypesRouter for PageRouter<'_, '_, DocumentTypesPagePresenter> {
    /// Builds `dialog`'s Presenter on the shared repository, handed the Dialog keys, and hosts it;
    /// its confirmed outcome comes back to the page through [`ModulePage::landing`].
    fn open_dialog(&mut self, dialog: Which) -> bool {
        let (documents, keys) = (self.setup.documents.clone(), self.setup.dialog_keys.get());
        match dialog {
            Which::Add => self.open(TypeFormPresenter::add(documents, keys)),
            Which::Edit(id) => {
                let Some(form) = TypeFormPresenter::edit(documents, id, keys) else {
                    return false;
                };
                self.open(form);
            }
            Which::Remove(id) => {
                let Some(remove) = RemoveTypePresenter::new(documents, id, keys) else {
                    return false;
                };
                self.open(remove);
            }
            Which::DefaultNotice => self.open(DefaultNoticePresenter::new(keys)),
        }
        true
    }
}

/// The Document Types module's entry point: its Settings page, `presenter` routed over `setup`'s
/// store, which the Settings View hosts as any other page.
pub fn document_types_page(
    presenter: DocumentTypesPagePresenter,
    setup: DocumentTypesSetup,
    announcer: Entity<Announcer>,
    cx: &mut gpui::App,
) -> Entity<DocumentTypesPage> {
    cx.new(|cx| ModulePage::new(presenter, setup, &announcer, cx))
}
