//! The Payees Settings page's View: the `lib-payees` page Presenter's [`PageView`], which the
//! generic `ModulePage` hosts (ADR-0056). It names the page's Router, the domains it repaints on,
//! and how it draws the view model (`render.rs`); the keymap, the take, send and handle path and
//! the Dialogs' landing are the generic page's.

mod render;

use gpui::{AnyElement, App, Context, Window};
use lib_database::Domain;
use lib_payees::presenter::{PayeesPageEvent, PayeesPagePresenter, PayeesPageViewModel};

use crate::payees::router::PageRouter;
use crate::payees::{PayeesPage, PayeesRoutes, SettingsPayeesSnapshot};
use crate::settings::SettingsPage;
use crate::settings::snapshot::PageSnapshot;
use crate::shell::view::module_page::{PageDraw, PageView, SettingsPageView};
use crate::shell::view::statusline::PageStatus;
use crate::view::event::Reveal;

impl PageView for PayeesPagePresenter {
    type Setup = PayeesRoutes;
    type Widgets = ();
    type Before = ();

    /// A Payees write, Import's new Payees, a rollback; and the Categories whose labels the Default
    /// category cells show.
    const REPAINTS: &'static [Domain] = &[Domain::Payees, Domain::Categories];

    fn with_router<R>(
        presenter: &mut Self,
        setup: &PayeesRoutes,
        cx: &mut Context<'_, PayeesPage>,
        handle: impl FnOnce(&mut Self, &mut Self::Router<'_>) -> R,
    ) -> R {
        handle(presenter, &mut PageRouter { routes: setup, cx })
    }

    fn render(
        view: &PayeesPageViewModel,
        page: PageDraw<'_, Self>,
        _window: &mut Window,
        cx: &mut Context<'_, PayeesPage>,
    ) -> AnyElement {
        render::page(view, page.clicks, page.focused, cx)
    }

    /// The Presenter's legend, rebound keys included, and its count of Payees.
    fn status(view: &PayeesPageViewModel) -> PageStatus {
        PageStatus {
            hints: view.hints.clone(),
            right: view.status.clone().into(),
        }
    }

    fn heading_meta(view: &PayeesPageViewModel, cx: &App) -> Option<AnyElement> {
        Some(render::scope_note(&view.scope, cx))
    }

    /// A Documents Payee link (the root Router's `Reveal::Payee`) selects its Payee.
    fn reveal(reveal: Reveal) -> Option<PayeesPageEvent> {
        match reveal {
            Reveal::Payee(id) => Some(PayeesPageEvent::Reveal(id)),
            _ => None,
        }
    }
}

impl SettingsPageView for PayeesPagePresenter {
    const PAGE: SettingsPage = SettingsPage::Payees;
}

impl PageSnapshot for SettingsPayeesSnapshot {
    type Presenter = PayeesPagePresenter;

    fn read(page: &PayeesPage, _cx: &App) -> Self {
        let view = page.view_model();
        SettingsPayeesSnapshot {
            selected_payee: view
                .rows
                .iter()
                .find(|row| Some(row.id) == view.selected)
                .map(|row| row.name.clone()),
            payee_rules: view.rows.iter().map(|row| row.rules.clone()).collect(),
            payee_names: view.rows.into_iter().map(|row| row.name).collect(),
        }
    }
}
