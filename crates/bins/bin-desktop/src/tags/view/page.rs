//! The Tags Settings page's View: the `lib-tags` page Presenter's [`PageView`], which the generic
//! `ModulePage` hosts (ADR-0056). It names the page's Router, the domains it repaints on, and how
//! it draws the view model (`render.rs`); the keymap, the take, send and handle path and the
//! Dialogs' landing are the generic page's.

mod render;

use gpui::{AnyElement, App, Context, Window};
use lib_database::Domain;
use lib_tags::presenter::{TagsPagePresenter, TagsPageViewModel};

use crate::settings::SettingsPage;
use crate::settings::snapshot::PageSnapshot;
use crate::shell::view::module_page::{PageDraw, PageView, SettingsPageView};
use crate::shell::view::statusline::PageStatus;
use crate::tags::router::PageRouter;
use crate::tags::{SettingsTagsSnapshot, TagsPage, TagsRoutes};

impl PageView for TagsPagePresenter {
    type Setup = TagsRoutes;
    type Widgets = ();
    type Before = ();

    /// A Tags write, a remove or merge, a rollback; and a Transaction, since usage drives the
    /// likely duplicates and the merge counts.
    const REPAINTS: &'static [Domain] = &[Domain::Tags, Domain::Transactions];

    fn with_router<R>(
        presenter: &mut Self,
        setup: &TagsRoutes,
        cx: &mut Context<'_, TagsPage>,
        handle: impl FnOnce(&mut Self, &mut Self::Router<'_>) -> R,
    ) -> R {
        handle(presenter, &mut PageRouter { routes: setup, cx })
    }

    fn render(
        view: &TagsPageViewModel,
        page: PageDraw<'_, Self>,
        _window: &mut Window,
        cx: &mut Context<'_, TagsPage>,
    ) -> AnyElement {
        render::page(view, page.clicks, page.focused, cx)
    }

    /// The Presenter's legend, rebound keys included, and its count of Tags and likely duplicates.
    fn status(view: &TagsPageViewModel) -> PageStatus {
        PageStatus {
            hints: view.hints.clone(),
            right: view.status.clone(),
        }
    }

    fn heading_meta(view: &TagsPageViewModel, cx: &App) -> Option<AnyElement> {
        Some(render::scope_note(&view.scope, cx))
    }
}

impl SettingsPageView for TagsPagePresenter {
    const PAGE: SettingsPage = SettingsPage::Tags;
}

impl PageSnapshot for SettingsTagsSnapshot {
    type Presenter = TagsPagePresenter;

    /// The Tags A–Z, their ids and the selected row's name.
    fn read(page: &TagsPage, _cx: &App) -> Self {
        let view = page.view_model();
        SettingsTagsSnapshot {
            selected_tag: view
                .rows
                .iter()
                .find(|row| Some(row.id) == view.selected)
                .map(|row| row.name.clone()),
            tag_ids: view.rows.iter().map(|row| row.id).collect(),
            tag_names: view.rows.into_iter().map(|row| row.name).collect(),
        }
    }
}
