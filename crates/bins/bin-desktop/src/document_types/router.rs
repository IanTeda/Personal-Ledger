//! `impl DocumentTypesRouter` for the Desktop: where the Document Types page's Presenter asks to
//! go. A Dialog opens as a `ModuleDialog` in the Dialog host through the page's gpui context, which
//! is why the Router is built for each `handle` call (`PageView::with_router`), and a Notice goes
//! onto the bin's queue (`crate::notices`), which the input's `send` drains.

use gpui::{AppContext as _, Context, Entity};
use lib_documents::presenter::{DefaultNoticePresenter, RemoveTypePresenter, TypeFormPresenter};
use lib_documents::{DocumentTypesDialog as Which, DocumentTypesRouter};
use lib_toast::{Notice, Notify};

use super::{DocumentTypesPage, DocumentTypesRoutes, DocumentTypesSetup};
use crate::announcer::Announcer;
use crate::notices::Queue;
use crate::shell::view::dialog_entity::ModuleDialog;
use crate::shell::view::module_page::ModulePage;

pub(super) struct PageRouter<'a, 'b> {
    pub routes: &'a DocumentTypesRoutes,
    pub cx: &'a mut Context<'b, DocumentTypesPage>,
}

impl DocumentTypesRouter for PageRouter<'_, '_> {
    /// Builds `dialog`'s Presenter on the shared repository, handed the Dialog keys, and hosts it;
    /// its confirmed outcome comes back to the page through [`ModulePage::landing`].
    fn open_dialog(&mut self, dialog: Which) -> bool {
        let routes = self.routes;
        let landed = DocumentTypesPage::landing(self.cx);
        let (documents, keys) = (routes.documents.clone(), routes.dialog_keys.get());
        match dialog {
            Which::Add => {
                ModuleDialog::open(self.cx, TypeFormPresenter::add(documents, keys), landed);
            }
            Which::Edit(id) => {
                let Some(form) = TypeFormPresenter::edit(documents, id, keys) else {
                    return false;
                };
                ModuleDialog::open(self.cx, form, landed);
            }
            Which::Remove(id) => {
                let Some(remove) = RemoveTypePresenter::new(documents, id, keys) else {
                    return false;
                };
                ModuleDialog::open(self.cx, remove, landed);
            }
            Which::DefaultNotice => {
                ModuleDialog::open(self.cx, DefaultNoticePresenter::new(keys), landed);
            }
        }
        true
    }
}

impl Notify for PageRouter<'_, '_> {
    /// Queues `notice`, for the input's `send` to drain into the Shell.
    fn notify(&mut self, notice: Notice) {
        Queue.notify(notice);
    }
}

/// The Document Types module's entry point: its Settings page, built on `setup`'s store, which
/// the Settings View hosts as any other page.
pub fn document_types_page(
    setup: DocumentTypesSetup,
    announcer: Entity<Announcer>,
    cx: &mut gpui::App,
) -> Entity<DocumentTypesPage> {
    let DocumentTypesSetup {
        presenter,
        documents,
        dialog_keys,
    } = setup;
    let routes = DocumentTypesRoutes {
        documents,
        dialog_keys,
    };
    cx.new(|cx| ModulePage::new(presenter, routes, &announcer, cx))
}
