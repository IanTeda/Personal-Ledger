//! The Units Settings page's View: the `lib-units` page Presenter in a gpui Entity. It turns keys
//! into chord text for the Units keymap and clicks into events, draws the view model
//! (`render.rs`, `pane.rs`), and keeps the Ledger read copy in line after every event
//! ([`ports::sync`]). The Settings View hosts it as any other page.

mod pane;
mod render;

use std::rc::Rc;

use chrono::Utc;
use gpui::{
    AnyElement, App, Context, Entity, EventEmitter, IntoElement as _, Keystroke, Pixels, Point,
    Subscription,
};

use crate::navigation::chord::{chord, interned, is_global};
use lib_ledger::Domain;
use lib_units::presenter::{UnitsPageEvent, UnitsPagePresenter, UnitsPageViewModel};
use lib_viper::keymap::Keymap;

use crate::units::ports::{self, DesktopPorts};
use crate::units::router::PageRouter;
use crate::{
    chrome::{statusline::PageStatus, window_size::WindowSize},
    ledger_store::{LedgerStore, notify_on},
    navigation::key_router::Movement,
    settings::{
        SettingsPage,
        page::PageRowClicked,
        pages::{Page, PageFrame, focus_on_row_click},
        state::SettingsView,
    },
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

    /// Sends `event` to the Presenter with the Desktop's Router, then brings the Ledger read copy
    /// in line with whatever it wrote.
    pub(in crate::units) fn send(&mut self, event: UnitsPageEvent, cx: &mut Context<'_, Self>) {
        ports::sync(&self.ports, &self.ledger, cx);
        let mut router = PageRouter {
            ports: self.ports.clone(),
            cx,
        };
        self.presenter.handle(event, &mut router);
        ports::sync(&self.ports, &self.ledger, cx);
        cx.notify();
    }

    /// A confirmed Dialog's outcome, handed back by the Dialog host.
    pub(in crate::units) fn dialog_landed(
        &mut self,
        outcome: lib_units::presenter::UnitsOutcome,
        cx: &mut Context<'_, Self>,
    ) {
        self.send(UnitsPageEvent::DialogClosed(Some(outcome)), cx);
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

    /// Every chord the Units keymap binds, looked up ahead of the router, except the Shell's own
    /// global keys. `g g` stays the router's, since `g` is its jump prefix: it comes back as a
    /// movement.
    fn key_ahead(&mut self, keystroke: &Keystroke, cx: &mut Context<'_, Self>) -> bool {
        if is_global(keystroke) {
            return false;
        }
        let Some(event) = chord(keystroke).and_then(|chord| self.keymap.event(&chord)) else {
            return false;
        };
        self.send(event, cx);
        true
    }

    /// The router's movements the keymap didn't take (`g g`, the arrows) walk both tables.
    fn apply_movement(&mut self, movement: Movement, cx: &mut Context<'_, Self>) -> bool {
        if let Some(step) = movement.step() {
            self.send(UnitsPageEvent::Move(step), cx);
        }
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

    fn page_status(&self, _cx: &App) -> Option<PageStatus> {
        Some(PageStatus {
            hints: self
                .presenter
                .view_model()
                .hints
                .into_iter()
                .map(|hint| (interned(&hint.chord), hint.action))
                .collect(),
            right: SettingsPage::Units.scope_note(),
        })
    }

    /// X-Large docks the detail pane.
    fn set_frame(&mut self, frame: PageFrame, cx: &mut Context<'_, Self>) {
        let docked = frame.window_size == WindowSize::XLarge;
        if docked != self.presenter.view_model().docked {
            self.send(UnitsPageEvent::SetDocked(docked), cx);
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
