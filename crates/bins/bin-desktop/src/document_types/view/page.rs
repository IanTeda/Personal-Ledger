//! The Document Types Settings page's View: the `lib-documents` page Presenter's [`PageView`],
//! which the generic `ModulePage` hosts (ADR-0056). It names the page's Router, the domains it
//! repaints on, and how it draws the view model (`render.rs`); the keymap, the take, send and
//! handle path and the Dialogs' landing are the generic page's.

mod render;

use gpui::{AnyElement, App, Context, IntoElement as _, Window};
use lib_database::Domain;
use lib_documents::presenter::{DocumentTypesPagePresenter, DocumentTypesPageViewModel};

use crate::document_types::router::PageRouter;
use crate::document_types::{
    DocumentTypesPage, DocumentTypesRoutes, SettingsDocumentTypesSnapshot,
};
use crate::settings::SettingsPage;
use crate::settings::snapshot::PageSnapshot;
use crate::shell::view::module_page::{PageDraw, PageView, SettingsPageView};
use crate::shell::view::statusline::PageStatus;

impl PageView for DocumentTypesPagePresenter {
    type Setup = DocumentTypesRoutes;
    type Widgets = ();
    type Before = ();

    /// A Document Type write: the rows read the types and their Filed file counts.
    const REPAINTS: &'static [Domain] = &[Domain::DocumentTypes];

    fn with_router<R>(
        presenter: &mut Self,
        setup: &DocumentTypesRoutes,
        cx: &mut Context<'_, DocumentTypesPage>,
        handle: impl FnOnce(&mut Self, &mut Self::Router<'_>) -> R,
    ) -> R {
        handle(presenter, &mut PageRouter { routes: setup, cx })
    }

    fn render(
        view: &DocumentTypesPageViewModel,
        page: PageDraw<'_, Self>,
        _window: &mut Window,
        cx: &mut Context<'_, DocumentTypesPage>,
    ) -> AnyElement {
        render::page(view, page.clicks, page.focused, cx)
    }

    /// The Presenter's legend, rebound keys included, and its count of types.
    fn status(view: &DocumentTypesPageViewModel) -> PageStatus {
        PageStatus {
            hints: view.hints.clone(),
            right: view.scope.clone().into(),
        }
    }

    fn heading_meta(view: &DocumentTypesPageViewModel, _cx: &App) -> Option<AnyElement> {
        Some(view.scope.clone().into_any_element())
    }
}

impl SettingsPageView for DocumentTypesPagePresenter {
    const PAGE: SettingsPage = SettingsPage::Documents;
}

impl PageSnapshot for SettingsDocumentTypesSnapshot {
    type Presenter = DocumentTypesPagePresenter;

    fn read(page: &DocumentTypesPage, _cx: &App) -> Self {
        let view = page.view_model();
        SettingsDocumentTypesSnapshot {
            selected_document_type: view
                .rows
                .iter()
                .find(|row| Some(row.id) == view.selected)
                .map(|row| row.name.clone()),
            document_type_files: view.rows.iter().map(|row| row.files).collect(),
            document_type_ids: view.rows.iter().map(|row| row.id).collect(),
            document_type_names: view.rows.into_iter().map(|row| row.name).collect(),
        }
    }
}
