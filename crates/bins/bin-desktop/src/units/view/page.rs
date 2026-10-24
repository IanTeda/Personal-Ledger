//! The Units Settings page's View: the `lib-units` page Presenter in a gpui Entity. It looks up
//! each key the Shell passes on, as chord text, in the Units keymap, turns clicks into events, draws the view model
//! (`render.rs`, `pane.rs`), and keeps the Ledger read copy in line after every event
//! ([`ports::refresh`]). Each key and click is one input (`crate::input::send`), which commits
//! what it wrote. The Settings View hosts it as any other page.

mod pane;
mod render;

use std::rc::Rc;

use chrono::Utc;
use gpui::{
    AnyElement, App, Context, Entity, EventEmitter, IntoElement as _, Pixels, Point, Subscription,
};

use lib_ledger::Domain;
use lib_units::presenter::{UnitsPageEvent, UnitsPagePresenter, UnitsPageViewModel};
use lib_viper::keymap::Keymap;

use crate::units::ports::{self, DesktopPorts};
use crate::units::router::PageRouter;
use crate::{
    ledger_store::{LedgerStore, notify_on},
    navigation::movement::Movement,
    settings::{
        SettingsPage,
        page::PageRowClicked,
        pages::{Page, PageFrame, focus_on_row_click},
        state::SettingsView,
    },
    shell::view::{statusline::Hint, window_size::WindowSize},
    view::event::ViewEvent,
};

pub use pane::detail_pane;

impl EventEmitter<ViewEvent> for UnitsPage {}
impl EventEmitter<PageRowClicked> for UnitsPage {}

pub struct UnitsPage {
    presenter: UnitsPagePresenter,
    ports: DesktopPorts,
    ledger: Entity<LedgerStore>,
    keymap: Keymap<UnitsPageEvent>,
    /// Where the open row menu is drawn: the pointer that opened it, in window coordinates.
    menu_at: Point<Pixels>,
    _units_changed: Subscription,
    _accounts_changed: Subscription,
}

impl UnitsPage {
    pub fn new(
        ports: DesktopPorts,
        ledger: Entity<LedgerStore>,
        keymap: Keymap<UnitsPageEvent>,
        cx: &mut Context<'_, Self>,
    ) -> Self {
        let presenter = UnitsPagePresenter::new(ports.clone(), Rc::new(Utc::now), keymap.clone());
        Self {
            presenter,
            ports,
            _units_changed: notify_on(&ledger, Domain::Units, cx),
            // The pane's ACCOUNTS group reads the Ledger's Accounts.
            _accounts_changed: notify_on(&ledger, Domain::Accounts, cx),
            ledger,
            keymap,
            menu_at: Point::default(),
        }
    }

    /// A key or click on the page: `event` as one input, committed once it is handled.
    pub(in crate::units) fn send(&mut self, event: UnitsPageEvent, cx: &mut Context<'_, Self>) {
        crate::input::send(cx, |cx| self.handle(event, cx));
    }

    /// Hands `event` to the Presenter with the Desktop's Router, then brings the Ledger read copy
    /// in line with whatever it wrote. Inside an input already, as [`Self::send`] or a Dialog's
    /// confirm.
    fn handle(&mut self, event: UnitsPageEvent, cx: &mut Context<'_, Self>) {
        ports::refresh(&self.ports, &self.ledger, cx);
        let mut router = PageRouter {
            ports: self.ports.clone(),
            ledger: self.ledger.clone(),
            cx,
        };
        self.presenter.handle(event, &mut router);
        ports::refresh(&self.ports, &self.ledger, cx);
        cx.notify();
    }

    /// A confirmed Dialog's outcome, handed back by the Dialog host inside the confirm's input.
    pub(in crate::units) fn dialog_landed(
        &mut self,
        outcome: lib_units::presenter::UnitsOutcome,
        cx: &mut Context<'_, Self>,
    ) {
        self.handle(UnitsPageEvent::DialogClosed(Some(outcome)), cx);
    }

    /// The page as drawn, with usage and holdings as the Ledger has them now.
    pub fn view_model(&self, cx: &App) -> UnitsPageViewModel {
        ports::copy_usage(&self.ports, self.ledger.read(cx).ledger());
        self.presenter.view_model()
    }

    /// A row click: `event`, and focus on the page.
    fn click(&mut self, event: UnitsPageEvent, cx: &mut Context<'_, Self>) {
        self.send(event, cx);
        cx.emit(PageRowClicked);
    }

    /// A right-click on a row opens its menu at the pointer.
    fn open_menu(
        &mut self,
        row: lib_units::presenter::UnitsRow,
        at: Point<Pixels>,
        cx: &mut Context<'_, Self>,
    ) {
        self.menu_at = at;
        self.click(UnitsPageEvent::OpenMenu(row), cx);
    }
}

impl Page for UnitsPage {
    const PAGE: SettingsPage = SettingsPage::Units;

    /// Every chord the Units keymap binds, the arrows and `g g` among them. The Shell has already
    /// taken its own keys.
    fn module_key(&mut self, chord: &str, cx: &mut Context<'_, Self>) -> bool {
        let Some(event) = self.keymap.event(chord) else {
            return false;
        };
        self.send(event, cx);
        true
    }

    /// A movement the keymap doesn't bind (`enter`) does nothing on the page, and Settings leaves
    /// the page where it is.
    fn apply_movement(&mut self, _movement: Movement, _cx: &mut Context<'_, Self>) -> bool {
        true
    }

    /// `Esc` closes an open row menu; otherwise it is left to Settings.
    fn escape(&mut self, cx: &mut Context<'_, Self>) -> bool {
        if self.presenter.view_model().menu.is_none() {
            return false;
        }
        self.send(UnitsPageEvent::CloseMenu, cx);
        true
    }

    /// The Presenter's legend, rebound keys included, which the default `page_status` and the `?`
    /// cheat sheet both read.
    fn hints(&self, _cx: &App) -> Vec<Hint> {
        self.presenter
            .view_model()
            .hints
            .into_iter()
            .map(|hint| (hint.chord, hint.action))
            .collect()
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

    fn side_pane(&self, entity: &Entity<Self>, cx: &App) -> Option<AnyElement> {
        let view = self.view_model(cx);
        let pane = view.pane.as_ref()?;
        Some(detail_pane(pane, entity, cx))
    }

    fn subscribe_row_clicks(
        entity: &Entity<Self>,
        cx: &mut Context<'_, SettingsView>,
    ) -> Option<Subscription> {
        focus_on_row_click(entity, cx)
    }
}
