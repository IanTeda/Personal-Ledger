//! The Accounts Settings page's View: the `lib-accounts` page Presenter's [`PageView`], which the
//! generic `ModulePage` hosts (ADR-0056). It names the page's Router, the domains it repaints on,
//! and how it draws the view model (`render.rs`); the keymap, the take, send and handle path and
//! the Dialogs' landing are the generic page's.

mod render;

use gpui::{AnyElement, App, Context, IntoElement as _, Window};
use lib_accounts::presenter::{AccountsPageEvent, AccountsPagePresenter, AccountsPageViewModel};
use lib_database::Domain;

use crate::accounts::router::PageRouter;
use crate::accounts::{AccountsPage, AccountsRoutes, SettingsAccountsSnapshot};
use crate::settings::SettingsPage;
use crate::settings::snapshot::PageSnapshot;
use crate::shell::view::module_page::{PageDraw, PageView, SettingsPageView};
use crate::shell::view::statusline::PageStatus;
use crate::view::event::Reveal;

impl PageView for AccountsPagePresenter {
    type Setup = AccountsRoutes;
    type Widgets = ();

    /// An Accounts write, and the Units and Institutions whose codes and names the rows show.
    const REPAINTS: &'static [Domain] = &[Domain::Accounts, Domain::Units, Domain::Institutions];

    fn with_router<R>(
        setup: &AccountsRoutes,
        cx: &mut Context<'_, AccountsPage>,
        handle: impl FnOnce(&mut Self::Router<'_>) -> R,
    ) -> R {
        handle(&mut PageRouter { routes: setup, cx })
    }

    fn render(
        view: &AccountsPageViewModel,
        page: PageDraw<'_, Self>,
        _window: &mut Window,
        cx: &mut Context<'_, AccountsPage>,
    ) -> AnyElement {
        render::page(view, page.clicks, page.focused, cx)
    }

    /// The Presenter's legend, rebound keys included, and its count of Accounts.
    fn status(view: &AccountsPageViewModel) -> PageStatus {
        PageStatus {
            hints: view.hints.clone(),
            right: view.status.clone(),
        }
    }

    fn heading_meta(view: &AccountsPageViewModel, _cx: &App) -> Option<AnyElement> {
        Some(view.scope.clone().into_any_element())
    }

    /// Selects an Account another View handed off.
    fn reveal(reveal: Reveal) -> Option<AccountsPageEvent> {
        match reveal {
            Reveal::Account(id) => Some(AccountsPageEvent::Reveal(id)),
            _ => None,
        }
    }
}

impl SettingsPageView for AccountsPagePresenter {
    const PAGE: SettingsPage = SettingsPage::Accounts;
}

impl PageSnapshot for SettingsAccountsSnapshot {
    type Page = AccountsPage;

    fn read(page: &AccountsPage, _cx: &App) -> Self {
        let view = page.view_model();
        let lines = || view.groups.iter().flat_map(|group| &group.lines);
        SettingsAccountsSnapshot {
            selected_account: lines()
                .find(|line| Some(line.id) == view.selected)
                .map(|line| line.name.clone()),
            account_names: lines().map(|line| line.name.clone()).collect(),
        }
    }
}
