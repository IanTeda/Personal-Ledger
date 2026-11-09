//! The Units Settings page's View: the `lib-units` page Presenter's [`PageView`], which the generic
//! `ModulePage` hosts (ADR-0056). It names the page's Router, the domains it repaints on, and how
//! it draws the view model (`render.rs`, `pane.rs`); the keymap, the take, send and handle path and
//! the Dialogs' landing are the generic page's. Where the pointer opened a row's menu is the View's
//! widget state.
//!
//! A Unit's prices view (`prices.rs`) is a state of the page Presenter, drawn in place of the
//! tables while it shows: the Presenter's keymap is then the view's, and `Esc` closes it back to
//! the tables with their selection as it was, before an open row menu, before Settings would step
//! back to the index.

mod pane;
mod prices;
mod render;

use std::rc::Rc;

use gpui::{AnyElement, App, Context, IntoElement as _, Pixels, Point, Window};
use lib_database::Domain;
use lib_shell::Hint;
use lib_units::presenter::{
    UnitPricesEvent, UnitsPageEvent, UnitsPagePresenter, UnitsPageViewModel,
};

use crate::settings::SettingsPage;
use crate::settings::pages::PageFrame;
use crate::shell::view::module_page::{PageDraw, PageView, SettingsPageView};
use crate::shell::view::page_clicks::PageClicks;
use crate::shell::view::statusline::PageStatus;
use crate::shell::view::window_size::WindowSize;
use crate::units::router::PageRouter;
use crate::units::{UnitsPage, UnitsRoutes};

pub use pane::detail_pane;

/// `clock`'s now, as the Units Presenters read the time.
pub(crate) fn now_of(clock: &lib_core::Clock) -> Rc<dyn Fn() -> chrono::DateTime<chrono::Utc>> {
    let clock = clock.clone();
    Rc::new(move || clock.now())
}

impl PageView for UnitsPagePresenter {
    type Setup = UnitsRoutes;
    /// Where the open row menu is drawn: the pointer that opened it, in window coordinates.
    type Widgets = Point<Pixels>;
    type Before = ();

    /// A Units write or a rollback, and the Accounts the detail pane's ACCOUNTS group reads.
    const REPAINTS: &'static [Domain] = &[Domain::Units, Domain::Accounts];

    fn with_router<R>(
        presenter: &mut Self,
        setup: &UnitsRoutes,
        cx: &mut Context<'_, UnitsPage>,
        handle: impl FnOnce(&mut Self, &mut Self::Router<'_>) -> R,
    ) -> R {
        handle(presenter, &mut PageRouter { routes: setup, cx })
    }

    /// X-Large docks the detail pane, which the Presenter takes only as a change.
    fn framed(frame: PageFrame) -> Option<UnitsPageEvent> {
        Some(UnitsPageEvent::SetDocked(
            frame.window_size == WindowSize::XLarge,
        ))
    }

    fn render(
        view: &UnitsPageViewModel,
        page: PageDraw<'_, Self>,
        _window: &mut Window,
        cx: &mut Context<'_, UnitsPage>,
    ) -> AnyElement {
        if let Some(prices) = &view.prices {
            return prices::prices_view(prices.clone(), page.clicks, cx);
        }
        render::page(view, page.clicks, *page.widgets, cx)
    }

    /// The Presenter's legend, rebound keys included; while the prices view shows, its legend and
    /// the `esc` that closes it, a key the Shell keeps.
    fn status(view: &UnitsPageViewModel) -> PageStatus {
        let hints = match &view.prices {
            Some(prices) => prices
                .hints
                .iter()
                .cloned()
                .chain([Hint::new("esc", prices.back.clone())])
                .collect(),
            None => view.hints.clone(),
        };
        PageStatus {
            hints,
            right: crate::settings::scope_note(SettingsPage::Units),
        }
    }

    fn heading_meta(view: &UnitsPageViewModel, _cx: &App) -> Option<AnyElement> {
        Some(view.scope.clone().into_any_element())
    }

    /// The tables' detail pane, which gives way while the prices view shows.
    fn side_pane(
        view: &UnitsPageViewModel,
        clicks: &PageClicks<UnitsPage>,
        cx: &App,
    ) -> Option<AnyElement> {
        if view.prices.is_some() {
            return None;
        }
        Some(detail_pane(view.pane.as_ref()?, clicks, cx))
    }

    /// `Esc` closes the prices view, then an open row menu; otherwise it is left to Settings.
    fn escape(view: &UnitsPageViewModel) -> Option<UnitsPageEvent> {
        if view.prices.is_some() {
            return Some(UnitsPageEvent::Prices(UnitPricesEvent::Back));
        }
        view.menu.is_some().then_some(UnitsPageEvent::CloseMenu)
    }

    /// The open row menu is drawn where the pointer opened it.
    fn menu_at(widgets: &mut Point<Pixels>, at: Point<Pixels>) {
        *widgets = at;
    }
}

impl SettingsPageView for UnitsPagePresenter {
    const PAGE: SettingsPage = SettingsPage::Units;
    /// The tables fill the View's width, down to their 610px minimum.
    const FULL_WIDTH: bool = true;
}
