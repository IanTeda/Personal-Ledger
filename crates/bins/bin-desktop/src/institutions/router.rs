//! `impl InstitutionsRouter` for the Desktop: where the Institutions page's Presenter asks to go.
//! The Add, Edit and Delete Dialogs open as a `ModuleDialog` in the Dialog host through the page's
//! gpui context, which is why the Router is built for each `handle` call
//! (`PageView::with_router`), and a Notice goes onto the bin's queue (`crate::notices`), which the
//! input's `send` drains.

use gpui::{AppContext as _, Context, Entity};
use lib_institutions::presenter::{DeleteInstitutionPresenter, InstitutionFormPresenter};
use lib_institutions::{InstitutionsDialog as Which, InstitutionsRouter};
use lib_toast::{Notice, Notify};

use super::{InstitutionsPage, InstitutionsRoutes, InstitutionsSetup};
use crate::announcer::Announcer;
use crate::notices::Queue;
use crate::shell::view::dialog_entity::ModuleDialog;
use crate::shell::view::module_page::ModulePage;

pub(super) struct PageRouter<'a, 'b> {
    pub routes: &'a InstitutionsRoutes,
    pub cx: &'a mut Context<'b, InstitutionsPage>,
}

impl InstitutionsRouter for PageRouter<'_, '_> {
    /// Builds `dialog`'s Presenter on the shared ports, handed the Dialog keys, and hosts it; its
    /// confirmed outcome comes back to the page through [`ModulePage::landing`].
    fn open_dialog(&mut self, dialog: Which) -> bool {
        let routes = self.routes;
        let landed = InstitutionsPage::landing(self.cx);
        let (ports, keys) = (routes.ports.clone(), routes.dialog_keys.clone());
        match dialog {
            Which::AddInstitution => {
                ModuleDialog::open(self.cx, InstitutionFormPresenter::add(ports, keys), landed);
            }
            Which::EditInstitution(id) => {
                let Some(form) = InstitutionFormPresenter::edit(ports, id, keys) else {
                    return false;
                };
                ModuleDialog::open(self.cx, form, landed);
            }
            Which::DeleteInstitution(id) => {
                let Some(delete) = DeleteInstitutionPresenter::new(ports, id, keys) else {
                    return false;
                };
                ModuleDialog::open(self.cx, delete, landed);
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

/// The Institutions module's entry point: its Settings page, built on `setup`'s ports, which the
/// Settings View hosts as any other page.
pub fn institutions_page(
    setup: InstitutionsSetup,
    announcer: Entity<Announcer>,
    cx: &mut gpui::App,
) -> Entity<InstitutionsPage> {
    let InstitutionsSetup {
        presenter,
        ports,
        dialog_keys,
    } = setup;
    let routes = InstitutionsRoutes { ports, dialog_keys };
    cx.new(|cx| ModulePage::new(presenter, routes, &announcer, cx))
}
