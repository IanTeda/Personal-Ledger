//! The Tags Settings page's View: the `lib-tags` page Presenter in a gpui Entity. It looks up each
//! key the Shell passes on, as chord text, in the Tags keymap, turns clicks into events, draws the
//! view model (`render.rs`), and keeps the Ledger read copy in line after every event
//! ([`ports::refresh`]). Each key and click is one input (`crate::input::send`), which commits
//! what it wrote. The Settings View hosts it as any other page.

mod render;

use gpui::{AnyElement, App, Context, Entity, EventEmitter, Subscription};
use lib_ledger::Domain;
use lib_tags::SharedPorts;
use lib_tags::TagsOutcome;
use lib_tags::presenter::{TagsPageEvent, TagsPagePresenter, TagsPageViewModel};
use lib_viper::keymap::Keymap;

use crate::tags::router::PageRouter;
use crate::tags::{SettingsTagsSnapshot, TagsSetup, ports};
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

impl EventEmitter<ViewEvent> for TagsPage {}
impl EventEmitter<PageRowClicked> for TagsPage {}

pub struct TagsPage {
    presenter: TagsPagePresenter,
    ports: SharedPorts,
    ledger: Entity<LedgerStore>,
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
    pub fn new(setup: TagsSetup, ledger: Entity<LedgerStore>, cx: &mut Context<'_, Self>) -> Self {
        let ports = ports::shared(&setup.store);
        Self {
            presenter: TagsPagePresenter::new(ports.clone(), setup.keymap.clone()),
            ports,
            _tags_changed: notify_on(&ledger, Domain::Tags, cx),
            _transactions_changed: notify_on(&ledger, Domain::Transactions, cx),
            ledger,
            keymap: setup.keymap,
            page_focused: false,
        }
    }

    /// A key or click on the page: `event` as one input, committed once it is handled.
    pub(in crate::tags) fn send(&mut self, event: TagsPageEvent, cx: &mut Context<'_, Self>) {
        crate::input::send(cx, |cx| self.handle(event, cx));
    }

    /// Hands `event` to the Presenter with the Desktop's Router, then brings the Ledger read copy
    /// in line with whatever it wrote. Inside an input already, as [`Self::send`] or a Dialog's
    /// confirm.
    fn handle(&mut self, event: TagsPageEvent, cx: &mut Context<'_, Self>) {
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
    pub(in crate::tags) fn dialog_landed(
        &mut self,
        outcome: TagsOutcome,
        cx: &mut Context<'_, Self>,
    ) {
        self.handle(TagsPageEvent::DialogClosed(Some(outcome)), cx);
    }

    /// The palette's `tags merge`: the Merge Dialog with neither Tag chosen.
    pub fn open_merge_dialog(&mut self, cx: &mut Context<'_, Self>) {
        self.send(TagsPageEvent::OpenMerge, cx);
    }

    pub fn view_model(&self) -> TagsPageViewModel {
        self.presenter.view_model()
    }

    /// A row click: `event`, and focus on the page.
    fn click(&mut self, event: TagsPageEvent, cx: &mut Context<'_, Self>) {
        self.send(event, cx);
        cx.emit(PageRowClicked);
    }

    /// The Tags A–Z, their ids and the selected row's name.
    #[doc(hidden)]
    pub fn snapshot(&self, _cx: &App) -> SettingsTagsSnapshot {
        let view = self.view_model();
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
        self.send(event, cx);
        true
    }

    /// A movement the keymap doesn't bind (`enter`) does nothing on the page, and Settings leaves
    /// the page where it is: there is no hand-off from a Tag.
    fn apply_movement(&mut self, _movement: Movement, _cx: &mut Context<'_, Self>) -> bool {
        true
    }

    /// The Presenter's legend, rebound keys included, which the `?` cheat sheet reads.
    fn hints(&self, _cx: &App) -> Vec<Hint> {
        legend(self.view_model().hints)
    }

    fn page_status(&self, _cx: &App) -> Option<PageStatus> {
        let view = self.view_model();
        Some(PageStatus {
            hints: legend(view.hints),
            right: view.status,
        })
    }

    fn set_page_focused(&mut self, focused: bool) {
        self.page_focused = focused;
    }

    fn heading_meta(&self, cx: &App) -> AnyElement {
        render::scope_note(&self.view_model().scope, cx)
    }

    fn subscribe_row_clicks(
        entity: &Entity<Self>,
        cx: &mut Context<'_, SettingsView>,
    ) -> Option<Subscription> {
        focus_on_row_click(entity, cx)
    }
}

/// The Presenter's hints as the status line and the `?` cheat sheet take them.
fn legend(hints: Vec<lib_tags::presenter::Hint>) -> Vec<Hint> {
    hints
        .into_iter()
        .map(|hint| (hint.chord, hint.action))
        .collect()
}
