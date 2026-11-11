//! The Categories Settings page's View: the `lib-categories` page Presenter's [`PageView`], which
//! the generic `ModulePage` hosts (ADR-0056). It names the page's Router, the domains it repaints
//! on, and how it draws the view model (`render.rs`); the keymap, the take, send and handle path and
//! the Dialogs' landing are the generic page's.

mod render;

use gpui::{AnyElement, App, Context, IntoElement as _, Window};
use lib_categories::presenter::{CategoriesPagePresenter, CategoriesPageViewModel};
use lib_database::Domain;

use crate::categories::router::PageRouter;
use crate::categories::{CategoriesPage, CategoriesRoutes, SettingsCategoriesSnapshot};
use crate::settings::SettingsPage;
use crate::settings::snapshot::PageSnapshot;
use crate::shell::view::module_page::{PageDraw, PageView, SettingsPageView};
use crate::shell::view::statusline::PageStatus;

impl PageView for CategoriesPagePresenter {
    type Setup = CategoriesRoutes;
    type Widgets = ();
    type Before = ();

    /// A Categories write, a reassign, a rollback.
    const REPAINTS: &'static [Domain] = &[Domain::Categories];

    fn with_router<R>(
        presenter: &mut Self,
        setup: &CategoriesRoutes,
        cx: &mut Context<'_, CategoriesPage>,
        handle: impl FnOnce(&mut Self, &mut Self::Router<'_>) -> R,
    ) -> R {
        handle(presenter, &mut PageRouter { routes: setup, cx })
    }

    fn render(
        view: &CategoriesPageViewModel,
        page: PageDraw<'_, Self>,
        _window: &mut Window,
        cx: &mut Context<'_, CategoriesPage>,
    ) -> AnyElement {
        render::page(view, page.clicks, page.focused, cx)
    }

    /// The Presenter's legend, rebound keys included, and its count of Categories.
    fn status(view: &CategoriesPageViewModel) -> PageStatus {
        PageStatus {
            hints: view.hints.clone(),
            right: view.scope.clone().into(),
        }
    }

    fn heading_meta(view: &CategoriesPageViewModel, _cx: &App) -> Option<AnyElement> {
        Some(view.scope.clone().into_any_element())
    }
}

impl SettingsPageView for CategoriesPagePresenter {
    const PAGE: SettingsPage = SettingsPage::Categories;
}

impl PageSnapshot for SettingsCategoriesSnapshot {
    type Presenter = CategoriesPagePresenter;

    /// The Categories in the store's tree order, and the selected row's name.
    fn read(page: &CategoriesPage, _cx: &App) -> Self {
        let categories = page.setup().categories.borrow().categories();
        let selected = page.view_model().selected;
        let selected_budget_months = selected
            .and_then(|id| {
                let budget = page
                    .setup()
                    .budgets
                    .borrow()
                    .budgets()
                    .budget(crate::budgets::PERSONAL_SPENDING_ID)?;
                let records = budget.limits.get(&id)?;
                Some(
                    records
                        .iter()
                        .map(|record| (record.month.year, record.month.month))
                        .collect(),
                )
            })
            .unwrap_or_default();
        SettingsCategoriesSnapshot {
            selected_budget_months,
            selected_category: categories
                .iter()
                .find(|category| Some(category.id) == selected)
                .map(|category| category.name.clone()),
            category_types: categories
                .iter()
                .map(|c| (c.name.clone(), c.category_type.clone()))
                .collect(),
            category_ids: categories.iter().map(|c| c.id).collect(),
            category_names: categories.into_iter().map(|c| c.name).collect(),
        }
    }
}
