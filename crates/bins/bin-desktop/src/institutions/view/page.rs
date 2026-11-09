//! The Institutions Settings page's View: the `lib-institutions` page Presenter's [`PageView`],
//! which the generic `ModulePage` hosts (ADR-0056). It names the page's Router, the domains it
//! repaints on, and how it draws the view model (`render.rs`); the keymap, the take, send and
//! handle path and the Dialogs' landing are the generic page's. Where the pointer opened a row's
//! menu is the View's widget state.

mod render;

use gpui::{AnyElement, App, Context, IntoElement as _, Pixels, Point, Window};
use lib_database::Domain;
use lib_institutions::presenter::{
    InstitutionsPageEvent, InstitutionsPagePresenter, InstitutionsPageViewModel,
};

use crate::institutions::router::PageRouter;
use crate::institutions::{InstitutionsPage, InstitutionsRoutes, SettingsInstitutionsSnapshot};
use crate::settings::SettingsPage;
use crate::settings::snapshot::PageSnapshot;
use crate::shell::view::module_page::{PageDraw, PageView, SettingsPageView};
use crate::shell::view::statusline::PageStatus;

impl PageView for InstitutionsPagePresenter {
    type Setup = InstitutionsRoutes;
    /// Where the open row menu is drawn: the pointer that opened it, in window coordinates.
    type Widgets = Point<Pixels>;
    type Before = ();

    /// An Institutions write or a rollback.
    const REPAINTS: &'static [Domain] = &[Domain::Institutions];

    fn with_router<R>(
        presenter: &mut Self,
        setup: &InstitutionsRoutes,
        cx: &mut Context<'_, InstitutionsPage>,
        handle: impl FnOnce(&mut Self, &mut Self::Router<'_>) -> R,
    ) -> R {
        handle(presenter, &mut PageRouter { routes: setup, cx })
    }

    fn render(
        view: &InstitutionsPageViewModel,
        page: PageDraw<'_, Self>,
        _window: &mut Window,
        cx: &mut Context<'_, InstitutionsPage>,
    ) -> AnyElement {
        render::page(view, page.clicks, page.focused, *page.widgets, cx)
    }

    /// The Presenter's legend, rebound keys included, and its count of Institutions.
    fn status(view: &InstitutionsPageViewModel) -> PageStatus {
        PageStatus {
            hints: view.hints.clone(),
            right: view.scope.clone(),
        }
    }

    fn heading_meta(view: &InstitutionsPageViewModel, _cx: &App) -> Option<AnyElement> {
        Some(view.scope.clone().into_any_element())
    }

    /// `Esc` closes an open row menu; otherwise it is left to Settings.
    fn escape(view: &InstitutionsPageViewModel) -> Option<InstitutionsPageEvent> {
        view.menu
            .is_some()
            .then_some(InstitutionsPageEvent::CloseMenu)
    }

    /// The open row menu is drawn where the pointer opened it.
    fn menu_at(widgets: &mut Point<Pixels>, at: Point<Pixels>) {
        *widgets = at;
    }
}

impl SettingsPageView for InstitutionsPagePresenter {
    const PAGE: SettingsPage = SettingsPage::Institutions;
}

impl PageSnapshot for SettingsInstitutionsSnapshot {
    type Presenter = InstitutionsPagePresenter;

    fn read(page: &InstitutionsPage, _cx: &App) -> Self {
        let view = page.view_model();
        SettingsInstitutionsSnapshot {
            menu: view.menu.as_ref().map(|menu| {
                menu.entries
                    .iter()
                    .map(|entry| entry.action.slug())
                    .collect()
            }),
            selected_institution: view
                .rows
                .iter()
                .find(|row| Some(row.id) == view.selected)
                .map(|row| row.name.clone()),
            institutions: view
                .rows
                .into_iter()
                .map(|row| (row.name, row.products))
                .collect(),
        }
    }
}
