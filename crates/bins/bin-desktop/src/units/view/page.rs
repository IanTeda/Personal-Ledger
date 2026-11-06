//! The Units Settings page's View: the `lib-units` page Presenter in a gpui Entity. It looks up
//! each key the Shell passes on, as chord text, in the Units keymap, turns clicks into events, draws the view model
//! (`render.rs`, `pane.rs`), and announces what each event wrote ([`ports::announce`]). Each key and click is one input (`crate::input::send`), which commits
//! what it wrote. The Settings View hosts it as any other page.
//!
//! A Unit's prices view (`prices.rs`) shows in place of the tables while it is open, on
//! `lib-units`' prices Presenter: it takes the keys in its own keymap, and `Esc` closes it back
//! to the tables with their selection as it was, before Settings would step back to the index.

mod pane;
mod prices;
mod render;

use std::rc::Rc;

use chrono::Utc;
use gpui::{
    AnyElement, App, Context, Entity, EventEmitter, IntoElement as _, Pixels, Point, Subscription,
};

use crate::announcer::Domain;
use lib_units::UnitId;
use lib_units::presenter::{
    UnitPricesEvent, UnitPricesPresenter, UnitPricesViewModel, UnitsPageEvent, UnitsPagePresenter,
    UnitsPageViewModel,
};
use lib_viper::keymap::Keymap;

use lib_shell::Hint;

use crate::units::ports::{self, DesktopPorts};
use crate::units::router::PageRouter;
use crate::{
    announcer::{Announcer, notify_on},
    navigation::movement::Movement,
    settings::{
        SettingsPage,
        pages::{Page, PageFrame, PageRowClicked},
    },
    shell::view::{
        page_clicks::{PageClicks, PresenterPage},
        window_size::WindowSize,
    },
    view::event::ViewEvent,
};

pub use pane::detail_pane;

impl EventEmitter<ViewEvent> for UnitsPage {}
impl EventEmitter<PageRowClicked> for UnitsPage {}

pub struct UnitsPage {
    presenter: UnitsPagePresenter,
    ports: DesktopPorts,
    deletion: lib_unit_deletion::SharedUnitDeletion,
    announcer: Entity<Announcer>,
    keymap: Keymap<UnitsPageEvent>,
    prices_keymap: Keymap<UnitPricesEvent>,
    /// The Unit prices view, while it shows in place of the tables.
    prices: Option<UnitPricesPresenter>,
    /// Where the open row menu is drawn: the pointer that opened it, in window coordinates.
    menu_at: Point<Pixels>,
    _units_changed: Subscription,
    _accounts_changed: Subscription,
}

impl UnitsPage {
    pub fn new(
        ports: DesktopPorts,
        deletion: lib_unit_deletion::SharedUnitDeletion,
        announcer: Entity<Announcer>,
        keymap: Keymap<UnitsPageEvent>,
        prices_keymap: Keymap<UnitPricesEvent>,
        cx: &mut Context<'_, Self>,
    ) -> Self {
        let presenter = UnitsPagePresenter::new(ports.clone(), Rc::new(Utc::now), keymap.clone());
        Self {
            presenter,
            ports,
            deletion,
            _units_changed: notify_on(&announcer, Domain::Units, cx),
            // The pane's ACCOUNTS group reads the Ledger's Accounts.
            _accounts_changed: notify_on(&announcer, Domain::Accounts, cx),
            announcer,
            keymap,
            prices_keymap,
            prices: None,
            menu_at: Point::default(),
        }
    }

    /// A key or click on the page: `event` as one input, committed once it is handled.
    fn send(&mut self, event: UnitsPageEvent, cx: &mut Context<'_, Self>) {
        crate::input::join_or_send(cx, |cx| self.handle(event, cx));
    }

    /// Hands `event` to the Presenter with the Desktop's Router, then announces whatever it
    /// wrote. Inside an input already, as [`Self::send`] or a Dialog's
    /// confirm.
    fn handle(&mut self, event: UnitsPageEvent, cx: &mut Context<'_, Self>) {
        ports::announce(&self.ports, &self.announcer, cx);
        let mut router = PageRouter {
            ports: self.ports.clone(),
            deletion: self.deletion.clone(),
            announcer: self.announcer.clone(),
            cx,
            prices: None,
        };
        self.presenter.handle(event, &mut router);
        let prices = router.prices;
        self.apply_prices(prices);
        ports::announce(&self.ports, &self.announcer, cx);
        cx.notify();
    }

    /// A key or click on the prices view: `event` as one input, as [`Self::send`].
    fn send_prices(&mut self, event: UnitPricesEvent, cx: &mut Context<'_, Self>) {
        crate::input::join_or_send(cx, |cx| self.handle_prices(event, cx));
    }

    fn handle_prices(&mut self, event: UnitPricesEvent, cx: &mut Context<'_, Self>) {
        let Some(presenter) = &mut self.prices else {
            return;
        };
        ports::announce(&self.ports, &self.announcer, cx);
        let mut router = PageRouter {
            ports: self.ports.clone(),
            deletion: self.deletion.clone(),
            announcer: self.announcer.clone(),
            cx,
            prices: None,
        };
        presenter.handle(event, &mut router);
        let prices = router.prices;
        self.apply_prices(prices);
        ports::announce(&self.ports, &self.announcer, cx);
        cx.notify();
    }

    /// Opens the prices view a Presenter asked for (`Some`) or closes it (`None`).
    fn apply_prices(&mut self, prices: Option<Option<UnitId>>) {
        match prices {
            Some(Some(unit)) => {
                self.prices = Some(UnitPricesPresenter::new(
                    self.ports.clone(),
                    Rc::new(Utc::now),
                    self.prices_keymap.clone(),
                    unit,
                ));
            }
            Some(None) => self.prices = None,
            None => {}
        }
    }

    /// The prices view as drawn, while it shows.
    pub fn prices_view_model(&self) -> Option<UnitPricesViewModel> {
        self.prices.as_ref().map(UnitPricesPresenter::view_model)
    }

    /// A confirmed Dialog's outcome, handed back by the Dialog host inside the confirm's input.
    pub(in crate::units) fn dialog_landed(
        &mut self,
        outcome: lib_units::presenter::UnitsOutcome,
        cx: &mut Context<'_, Self>,
    ) {
        self.handle(UnitsPageEvent::DialogClosed(Some(outcome)), cx);
    }

    /// The page as drawn, with usage and holdings as the stores count them now.
    pub fn view_model(&self, _cx: &App) -> UnitsPageViewModel {
        self.presenter.view_model()
    }
}

/// The page's clicks on its tables, its row menu and its detail pane.
impl PresenterPage<UnitsPageEvent> for UnitsPage {
    fn send(&mut self, event: UnitsPageEvent, cx: &mut Context<'_, Self>) {
        UnitsPage::send(self, event, cx);
    }

    /// A click on a row hands the page focus.
    fn row_clicked(&mut self, cx: &mut Context<'_, Self>) {
        cx.emit(PageRowClicked);
    }

    /// The open row menu is drawn where the pointer opened it.
    fn menu_at(&mut self, at: Point<Pixels>, _cx: &mut Context<'_, Self>) {
        self.menu_at = at;
    }
}

/// The prices view's clicks on its weeks.
impl PresenterPage<UnitPricesEvent> for UnitsPage {
    fn send(&mut self, event: UnitPricesEvent, cx: &mut Context<'_, Self>) {
        self.send_prices(event, cx);
    }
}

impl Page for UnitsPage {
    const PAGE: SettingsPage = SettingsPage::Units;
    /// The tables fill the View's width, down to their 610px minimum.
    const FULL_WIDTH: bool = true;

    /// Every chord the Units keymap binds, the arrows and `g g` among them. The Shell has already
    /// taken its own keys.
    fn module_key(&mut self, chord: &str, cx: &mut Context<'_, Self>) -> bool {
        if self.prices.is_some() {
            let Some(event) = self.prices_keymap.event(chord) else {
                return false;
            };
            self.send_prices(event, cx);
            return true;
        }
        let Some(event) = self.keymap.event(chord) else {
            return false;
        };
        self.send(event, cx);
        true
    }

    /// A palette Command, by its keymap `command` name: the event its key sends on the page. An
    /// open prices view closes first, in the same input, so the Command acts on the page the
    /// user then sees.
    fn module_command(&mut self, command: &str, cx: &mut Context<'_, Self>) -> bool {
        let Some(event) = self.keymap.event_for(command) else {
            return false;
        };
        crate::input::join_or_send(cx, |cx| {
            if self.prices.is_some() {
                self.handle_prices(UnitPricesEvent::Back, cx);
            }
            self.handle(event, cx);
        });
        true
    }

    /// A movement the keymap doesn't bind (`enter`) does nothing on the page, and Settings leaves
    /// the page where it is.
    fn apply_movement(&mut self, _movement: Movement, _cx: &mut Context<'_, Self>) -> bool {
        true
    }

    /// `Esc` closes the prices view or an open row menu; otherwise it is left to Settings.
    fn escape(&mut self, cx: &mut Context<'_, Self>) -> bool {
        if self.prices.is_some() {
            self.send_prices(UnitPricesEvent::Back, cx);
            return true;
        }
        if self.presenter.view_model().menu.is_none() {
            return false;
        }
        self.send(UnitsPageEvent::CloseMenu, cx);
        true
    }

    /// The Presenter's legend, rebound keys included, which the default `page_status` and the `?`
    /// cheat sheet both read.
    fn hints(&self, _cx: &App) -> Vec<Hint> {
        if let Some(view) = self.prices_view_model() {
            return view
                .hints
                .into_iter()
                .chain([Hint::new("esc", view.back)])
                .collect();
        }
        self.presenter.view_model().hints
    }

    /// X-Large docks the detail pane. The layout, not an input, so nothing is committed.
    fn set_frame(&mut self, frame: PageFrame, cx: &mut Context<'_, Self>) {
        let docked = frame.window_size == WindowSize::XLarge;
        if docked != self.presenter.view_model().docked {
            self.handle(UnitsPageEvent::SetDocked(docked), cx);
        }
    }

    fn heading_meta(&self, _cx: &App) -> AnyElement {
        self.presenter.view_model().scope.into_any_element()
    }

    /// The tables' detail pane, which gives way while the prices view shows.
    fn side_pane(&self, entity: &Entity<Self>, cx: &App) -> Option<AnyElement> {
        if self.prices.is_some() {
            return None;
        }
        let view = self.view_model(cx);
        let pane = view.pane.as_ref()?;
        Some(detail_pane(pane, &PageClicks::of(entity), cx))
    }
}
