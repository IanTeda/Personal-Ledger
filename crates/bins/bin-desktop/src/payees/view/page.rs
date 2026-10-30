//! The Payees Settings page's View: the `lib-payees` page Presenter in a gpui Entity. It looks up
//! each key the Shell passes on, as chord text, in the Payees keymap, turns clicks into events,
//! draws the view model (`render.rs`), and keeps the Ledger read copy in line after every event
//! ([`ports::refresh`]). Each key and click is one input (`crate::input::send`), which commits
//! what it wrote. The Settings View hosts it as any other page.

mod render;

use gpui::{AnyElement, App, Context, Entity, EventEmitter, Subscription};
use lib_ledger::Domain;
use lib_payees::PayeesOutcome;
use lib_payees::presenter::{PayeesPageEvent, PayeesPagePresenter, PayeesPageViewModel};
use lib_viper::keymap::Keymap;

use crate::payees::SettingsPayeesSnapshot;
use crate::payees::ports::{self, DesktopPorts};
use crate::payees::router::PageRouter;
use crate::{
    ledger_store::{LedgerStore, notify_on},
    navigation::movement::Movement,
    settings::{SettingsPage, page::PageRowClicked, pages::Page},
    shell::view::statusline::Hint,
    view::event::{Reveal, ViewEvent},
};

impl EventEmitter<ViewEvent> for PayeesPage {}
impl EventEmitter<PageRowClicked> for PayeesPage {}

pub struct PayeesPage {
    presenter: PayeesPagePresenter,
    ports: DesktopPorts,
    ledger: Entity<LedgerStore>,
    clock: lib_core::Clock,
    keymap: Keymap<PayeesPageEvent>,
    /// Whether the page, not Settings' index, has keyboard focus, told by the Settings View as it
    /// draws the page.
    page_focused: bool,
    /// Repaints the page when its rows change: a Payees write, Import's new Payees, a rollback.
    _payees_changed: Subscription,
    /// The Default category labels are the Ledger's Categories'.
    _categories_changed: Subscription,
}

impl PayeesPage {
    pub fn new(
        ports: DesktopPorts,
        ledger: Entity<LedgerStore>,
        clock: lib_core::Clock,
        keymap: Keymap<PayeesPageEvent>,
        cx: &mut Context<'_, Self>,
    ) -> Self {
        ports::copy_from_ledger(&ports, ledger.read(cx).ledger());
        Self {
            presenter: PayeesPagePresenter::new(ports.clone(), keymap.clone()),
            ports,
            _payees_changed: notify_on(&ledger, Domain::Payees, cx),
            _categories_changed: notify_on(&ledger, Domain::Categories, cx),
            ledger,
            clock,
            keymap,
            page_focused: false,
        }
    }

    /// A key or click on the page: `event` as one input, committed once it is handled.
    pub(in crate::payees) fn send(&mut self, event: PayeesPageEvent, cx: &mut Context<'_, Self>) {
        crate::input::send(cx, |cx| self.handle(event, cx));
    }

    /// Hands `event` to the Presenter with the Desktop's Router, then brings the Ledger read copy
    /// in line with whatever it wrote. Inside an input already, as [`Self::send`] or a Dialog's
    /// confirm.
    fn handle(&mut self, event: PayeesPageEvent, cx: &mut Context<'_, Self>) {
        ports::refresh(&self.ports, &self.ledger, cx);
        let mut router = PageRouter {
            ports: self.ports.clone(),
            ledger: self.ledger.clone(),
            clock: self.clock.clone(),
            cx,
        };
        self.presenter.handle(event, &mut router);
        ports::refresh(&self.ports, &self.ledger, cx);
        cx.notify();
    }

    /// A confirmed Dialog's outcome, handed back by the Dialog host inside the confirm's input.
    pub(in crate::payees) fn dialog_landed(
        &mut self,
        outcome: PayeesOutcome,
        cx: &mut Context<'_, Self>,
    ) {
        self.handle(PayeesPageEvent::DialogClosed(Some(outcome)), cx);
    }

    /// The page as drawn, with the Categories as the Ledger has them now.
    pub fn view_model(&self, cx: &App) -> PayeesPageViewModel {
        ports::copy_from_ledger(&self.ports, self.ledger.read(cx).ledger());
        self.presenter.view_model()
    }

    /// A row click: `event`, and focus on the page.
    fn click(&mut self, event: PayeesPageEvent, cx: &mut Context<'_, Self>) {
        self.send(event, cx);
        cx.emit(PageRowClicked);
    }

    #[doc(hidden)]
    pub fn snapshot(&self, cx: &App) -> SettingsPayeesSnapshot {
        let view = self.view_model(cx);
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

impl Page for PayeesPage {
    const PAGE: SettingsPage = SettingsPage::Payees;

    /// Every chord the Payees keymap binds, the arrows and `g g` among them. The Shell has already
    /// taken its own keys.
    fn module_key(&mut self, chord: &str, cx: &mut Context<'_, Self>) -> bool {
        let Some(event) = self.keymap.event(chord) else {
            return false;
        };
        self.send(event, cx);
        true
    }

    /// A movement the keymap doesn't bind (`enter`) does nothing on the page, and Settings leaves
    /// the page where it is: there is no hand-off from a Payee.
    fn apply_movement(&mut self, _movement: Movement, _cx: &mut Context<'_, Self>) -> bool {
        true
    }

    /// A Documents Payee link (the root Router's `Reveal::Payee`) selects its Payee.
    fn reveal(&mut self, reveal: Reveal, cx: &mut Context<'_, Self>) {
        if let Reveal::Payee(id) = reveal {
            self.send(PayeesPageEvent::Reveal(id), cx);
        }
    }

    /// The Presenter's legend, rebound keys included, which the `?` cheat sheet reads.
    fn hints(&self, cx: &App) -> Vec<Hint> {
        legend(self.view_model(cx).hints)
    }

    fn status_note(&self, cx: &App) -> String {
        self.view_model(cx).status
    }

    fn set_page_focused(&mut self, focused: bool, _cx: &mut Context<'_, Self>) {
        self.page_focused = focused;
    }

    fn heading_meta(&self, cx: &App) -> AnyElement {
        render::scope_note(&self.view_model(cx).scope, cx)
    }
}

/// The Presenter's hints as the status line and the `?` cheat sheet take them.
fn legend(hints: Vec<lib_payees::presenter::Hint>) -> Vec<Hint> {
    hints
        .into_iter()
        .map(|hint| (hint.chord, hint.action))
        .collect()
}
