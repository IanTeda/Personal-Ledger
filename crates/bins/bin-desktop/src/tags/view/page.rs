//! The Tags Settings page's View: the `lib-tags` page Presenter in a gpui Entity. It looks up each
//! key the Shell passes on, as chord text, in the Tags keymap, turns clicks into events, draws the
//! view model (`render.rs`), and announces `Domain::Tags` after an event or Dialog that wrote the
//! Tags ([`ports::announce`]). Each key and click is one input (`crate::input::send`), which
//! commits what it wrote. The Settings View hosts it as any other page.

mod render;

use gpui::{AnyElement, App, Context, Entity, EventEmitter, Subscription};
use lib_ledger::Domain;
use lib_tags::presenter::{TagsPageEvent, TagsPagePresenter, TagsPageViewModel};
use lib_tags::{SharedPorts, TagChange, TagsOutcome};
use lib_viper::keymap::Keymap;

use lib_shell::Hint;

use crate::settings::snapshot::PageSnapshot;
use crate::tag_removal::RemovalStores;
use crate::tags::router::PageRouter;
use crate::tags::{SettingsTagsSnapshot, TagsSetup, ports};
use crate::{
    ledger_store::{LedgerStore, notify_on},
    navigation::movement::Movement,
    settings::{SettingsPage, page::PageRowClicked, pages::Page},
    shell::view::page_clicks::PresenterPage,
    view::event::ViewEvent,
};

impl EventEmitter<ViewEvent> for TagsPage {}
impl EventEmitter<PageRowClicked> for TagsPage {}

pub struct TagsPage {
    presenter: TagsPagePresenter,
    ports: SharedPorts,
    /// Tag remove and merge's port and reads, which their Dialogs open on.
    removal: RemovalStores,
    /// What hears of a Tags write: the Tags page itself, the Transactions and the Shell.
    ledger: Entity<LedgerStore>,
    clock: lib_core::Clock,
    keymap: Keymap<TagsPageEvent>,
    /// Whether the page, not Settings' index, has keyboard focus, told by the Settings View as it
    /// draws the page.
    page_focused: bool,
    /// Repaints the page when the Tags change: a Tags write, a remove or merge, a rollback.
    _tags_changed: Subscription,
    /// Usage drives the likely duplicates and the merge counts, so a Transaction repaints too.
    _transactions_changed: Subscription,
}

impl TagsPage {
    pub fn new(
        setup: TagsSetup,
        ledger: Entity<LedgerStore>,
        clock: lib_core::Clock,
        cx: &mut Context<'_, Self>,
    ) -> Self {
        let ports = ports::shared(&setup.store, setup.usage.clone());
        Self {
            presenter: TagsPagePresenter::new(ports.clone(), setup.keymap.clone()),
            ports,
            removal: setup.removal,
            _tags_changed: notify_on(&ledger, Domain::Tags, cx),
            _transactions_changed: notify_on(&ledger, Domain::Transactions, cx),
            ledger,
            clock,
            keymap: setup.keymap,
            page_focused: false,
        }
    }

    /// The one dispatch a keymap event takes into the page, from a key ([`Self::module_key`]) or a palette Command ([`Self::module_command`]): sent as one input, unless the page doesn't take it now.
    fn take(&mut self, event: TagsPageEvent, cx: &mut Context<'_, Self>) -> bool {
        if !self.presenter.takes(&event) {
            return false;
        }
        self.send(event, cx);
        true
    }

    /// A key or click on the page: `event` as one input, committed once it is handled.
    fn send(&mut self, event: TagsPageEvent, cx: &mut Context<'_, Self>) {
        crate::input::send(cx, |cx| self.handle(event, cx));
    }

    /// Hands `event` to the Presenter with the Desktop's Router, announcing the Tags when it wrote
    /// them. Inside an input already, as [`Self::send`] or a Dialog's confirm.
    fn handle(&mut self, event: TagsPageEvent, cx: &mut Context<'_, Self>) {
        let before = self.ports.borrow().tags();
        let mut router = PageRouter {
            ports: self.ports.clone(),
            removal: self.removal.clone(),
            clock: self.clock.clone(),
            cx,
        };
        self.presenter.handle(event, &mut router);
        if self.ports.borrow().tags() != before {
            ports::announce(&self.ledger, cx);
        }
        cx.notify();
    }

    /// A confirmed Dialog's outcome, handed back by the Dialog host inside the confirm's input.
    /// Every confirm wrote the Tags, so they are announced; a remove or merge rewrote Splits too,
    /// so the Ledger's Transactions read copy is brought in line for the Views still reading it.
    pub(crate) fn dialog_landed(&mut self, outcome: TagsOutcome, cx: &mut Context<'_, Self>) {
        if matches!(outcome.change, TagChange::Removed | TagChange::Merged) {
            self.ledger
                .update(cx, |store, cx| store.refresh_transactions(cx));
        }
        ports::announce(&self.ledger, cx);
        self.handle(TagsPageEvent::DialogClosed(Some(outcome)), cx);
    }

    pub fn view_model(&self) -> TagsPageViewModel {
        self.presenter.view_model()
    }
}

/// The page's clicks on its rows and buttons.
impl PresenterPage<TagsPageEvent> for TagsPage {
    fn send(&mut self, event: TagsPageEvent, cx: &mut Context<'_, Self>) {
        TagsPage::send(self, event, cx);
    }

    /// A click on a row hands the page focus.
    fn row_clicked(&mut self, cx: &mut Context<'_, Self>) {
        cx.emit(PageRowClicked);
    }
}

impl PageSnapshot for SettingsTagsSnapshot {
    type Page = TagsPage;

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

impl Page for TagsPage {
    const PAGE: SettingsPage = SettingsPage::Tags;

    /// Every chord the Tags keymap binds, the arrows and `g g` among them. The Shell has already
    /// taken its own keys.
    fn module_key(&mut self, chord: &str, cx: &mut Context<'_, Self>) -> bool {
        let Some(event) = self.keymap.event(chord) else {
            return false;
        };
        self.take(event, cx)
    }

    /// A palette Command, by its keymap `command` name: the event its key sends.
    fn module_command(&mut self, command: &str, cx: &mut Context<'_, Self>) -> bool {
        let Some(event) = self.keymap.event_for(command) else {
            return false;
        };
        self.take(event, cx)
    }

    /// A movement the keymap doesn't bind (`enter`) does nothing on the page, and Settings leaves
    /// the page where it is: there is no hand-off from a Tag.
    fn apply_movement(&mut self, _movement: Movement, _cx: &mut Context<'_, Self>) -> bool {
        true
    }

    /// The Presenter's legend, rebound keys included, which the `?` cheat sheet reads.
    fn hints(&self, _cx: &App) -> Vec<Hint> {
        self.view_model().hints
    }

    fn status_note(&self, _cx: &App) -> String {
        self.view_model().status
    }

    fn set_page_focused(&mut self, focused: bool, _cx: &mut Context<'_, Self>) {
        self.page_focused = focused;
    }

    fn heading_meta(&self, cx: &App) -> AnyElement {
        render::scope_note(&self.view_model().scope, cx)
    }
}
