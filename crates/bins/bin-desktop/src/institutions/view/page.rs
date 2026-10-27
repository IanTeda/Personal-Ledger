//! The Institutions Settings page's View: the `lib-institutions` page Presenter in a gpui Entity.
//! It looks up each key the Shell passes on, as chord text, in the Institutions keymap, turns
//! clicks into events, draws the view model (`render.rs`), and keeps the Ledger read copy in line
//! after every event ([`ports::refresh`]). Each key and click is one input (`crate::input::send`),
//! which commits what it wrote. The Settings View hosts it as any other page.

mod render;

use gpui::{AnyElement, App, Context, Entity, EventEmitter, IntoElement as _, Subscription};
use lib_institutions::InstitutionsOutcome;
use lib_institutions::presenter::{
    InstitutionsPageEvent, InstitutionsPagePresenter, InstitutionsPageViewModel,
};
use lib_ledger::Domain;
use lib_viper::keymap::Keymap;

use crate::institutions::SettingsInstitutionsSnapshot;
use crate::institutions::ports::{self, DesktopPorts};
use crate::institutions::router::PageRouter;
use crate::{
    ledger_store::{LedgerStore, notify_on},
    navigation::movement::Movement,
    settings::{
        SettingsPage,
        page::PageRowClicked,
        pages::{Page, focus_on_row_click},
        state::SettingsView,
    },
    shell::view::statusline::{Hint, PageStatus},
    view::event::ViewEvent,
};

impl EventEmitter<ViewEvent> for InstitutionsPage {}
impl EventEmitter<PageRowClicked> for InstitutionsPage {}

pub struct InstitutionsPage {
    presenter: InstitutionsPagePresenter,
    ports: DesktopPorts,
    ledger: Entity<LedgerStore>,
    keymap: Keymap<InstitutionsPageEvent>,
    /// Whether the page, not Settings' index, has keyboard focus, told by the Settings View as it
    /// draws the page.
    page_focused: bool,
    /// Repaints the page when its rows change: an Institutions write or a rollback.
    _institutions_changed: Subscription,
}

impl InstitutionsPage {
    pub fn new(
        ports: DesktopPorts,
        ledger: Entity<LedgerStore>,
        keymap: Keymap<InstitutionsPageEvent>,
        cx: &mut Context<'_, Self>,
    ) -> Self {
        Self {
            presenter: InstitutionsPagePresenter::new(ports.clone(), keymap.clone()),
            ports,
            _institutions_changed: notify_on(&ledger, Domain::Institutions, cx),
            ledger,
            keymap,
            page_focused: false,
        }
    }

    /// A key or click on the page: `event` as one input, committed once it is handled.
    pub(in crate::institutions) fn send(
        &mut self,
        event: InstitutionsPageEvent,
        cx: &mut Context<'_, Self>,
    ) {
        crate::input::send(cx, |cx| self.handle(event, cx));
    }

    /// Hands `event` to the Presenter with the Desktop's Router, then brings the Ledger read copy
    /// in line with whatever it wrote. Inside an input already, as [`Self::send`] or a Dialog's
    /// confirm.
    fn handle(&mut self, event: InstitutionsPageEvent, cx: &mut Context<'_, Self>) {
        ports::refresh(&self.ledger, cx);
        let mut router = PageRouter {
            ports: self.ports.clone(),
            ledger: self.ledger.clone(),
            cx,
        };
        self.presenter.handle(event, &mut router);
        ports::refresh(&self.ledger, cx);
        cx.notify();
    }

    /// A confirmed Dialog's outcome, handed back by the Dialog host inside the confirm's input.
    pub(in crate::institutions) fn dialog_landed(
        &mut self,
        outcome: InstitutionsOutcome,
        cx: &mut Context<'_, Self>,
    ) {
        self.handle(InstitutionsPageEvent::DialogClosed(Some(outcome)), cx);
    }

    /// The page as drawn.
    pub fn view_model(&self, _cx: &App) -> InstitutionsPageViewModel {
        self.presenter.view_model()
    }

    /// A row click: `event`, and focus on the page.
    fn click(&mut self, event: InstitutionsPageEvent, cx: &mut Context<'_, Self>) {
        self.send(event, cx);
        cx.emit(PageRowClicked);
    }

    #[doc(hidden)]
    pub fn snapshot(&self, cx: &App) -> SettingsInstitutionsSnapshot {
        let view = self.view_model(cx);
        SettingsInstitutionsSnapshot {
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

impl Page for InstitutionsPage {
    const PAGE: SettingsPage = SettingsPage::Institutions;

    /// Every chord the Institutions keymap binds, the arrows and `g g` among them. The Shell has
    /// already taken its own keys.
    fn module_key(&mut self, chord: &str, cx: &mut Context<'_, Self>) -> bool {
        let Some(event) = self.keymap.event(chord) else {
            return false;
        };
        self.send(event, cx);
        true
    }

    /// A movement the keymap doesn't bind (`enter`) does nothing on the page, and Settings leaves
    /// the page where it is: there is no hand-off from an Institution.
    fn apply_movement(&mut self, _movement: Movement, _cx: &mut Context<'_, Self>) -> bool {
        true
    }

    /// The Presenter's legend, rebound keys included, which the `?` cheat sheet reads.
    fn hints(&self, cx: &App) -> Vec<Hint> {
        legend(self.view_model(cx).hints)
    }

    fn page_status(&self, cx: &App) -> Option<PageStatus> {
        let view = self.view_model(cx);
        Some(PageStatus {
            hints: legend(view.hints),
            right: view.scope,
        })
    }

    fn set_page_focused(&mut self, focused: bool) {
        self.page_focused = focused;
    }

    fn heading_meta(&self, cx: &App) -> AnyElement {
        self.view_model(cx).scope.into_any_element()
    }

    fn subscribe_row_clicks(
        entity: &Entity<Self>,
        cx: &mut Context<'_, SettingsView>,
    ) -> Option<Subscription> {
        focus_on_row_click(entity, cx)
    }
}

/// The Presenter's hints as the status line and the `?` cheat sheet take them.
fn legend(hints: Vec<lib_institutions::presenter::Hint>) -> Vec<Hint> {
    hints
        .into_iter()
        .map(|hint| (hint.chord, hint.action))
        .collect()
}
