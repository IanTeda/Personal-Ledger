//! `impl InstitutionsRouter` for the Desktop: where the Institutions page's Presenter asks to go.
//! The Add, Edit and Delete Dialogs open as a `ModuleDialog` in the Dialog host through the
//! generic [`PageRouter`], built for each `handle` call (`PageView::with_router`), which queues a
//! Notice for the input's `send` to drain.

use gpui::{AppContext as _, Entity};
use lib_institutions::presenter::{
    DeleteInstitutionPresenter, InstitutionFormPresenter, InstitutionsPagePresenter,
};
use lib_institutions::{InstitutionsDialog as Which, InstitutionsRouter};

use super::{InstitutionsPage, InstitutionsSetup};
use crate::announcer::Announcer;
use crate::shell::view::module_page::ModulePage;
use crate::shell::view::page_router::PageRouter;

impl InstitutionsRouter for PageRouter<'_, '_, InstitutionsPagePresenter> {
    /// Builds `dialog`'s Presenter on the shared ports, handed the Dialog keys, and hosts it; its
    /// confirmed outcome comes back to the page through [`ModulePage::landing`].
    fn open_dialog(&mut self, dialog: Which) -> bool {
        let (ports, keys) = (self.setup.ports.clone(), self.setup.dialog_keys.get());
        match dialog {
            Which::AddInstitution => self.open(InstitutionFormPresenter::add(ports, keys)),
            Which::EditInstitution(id) => {
                let Some(form) = InstitutionFormPresenter::edit(ports, id, keys) else {
                    return false;
                };
                self.open(form);
            }
            Which::DeleteInstitution(id) => {
                let Some(delete) = DeleteInstitutionPresenter::new(ports, id, keys) else {
                    return false;
                };
                self.open(delete);
            }
        }
        true
    }
}

/// The Institutions module's entry point: its Settings page, `presenter` routed over `setup`'s
/// ports, which the Settings View hosts as any other page.
pub fn institutions_page(
    presenter: InstitutionsPagePresenter,
    setup: InstitutionsSetup,
    announcer: Entity<Announcer>,
    cx: &mut gpui::App,
) -> Entity<InstitutionsPage> {
    cx.new(|cx| ModulePage::new(presenter, setup, &announcer, cx))
}
