//! The Categories Settings page's View: the `lib-categories` page Presenter in a gpui Entity. It
//! looks up each key the Shell passes on, as chord text, in the Categories keymap, turns clicks
//! into events, draws the view model (`render.rs`), and keeps the Ledger read copy in line after
//! every event ([`ports::refresh`]). Each key and click is one input (`crate::input::send`), which
//! commits what it wrote. The Settings View hosts it as any other page.

mod render;

use chrono::NaiveDate;
use gpui::{AnyElement, App, Context, Entity, EventEmitter, IntoElement as _, Subscription};
use lib_categories::presenter::{
    CategoriesPageEvent, CategoriesPagePresenter, CategoriesPageViewModel,
};
use lib_categories::{CategoriesOutcome, SharedCategories};
use lib_ledger::Domain;
use lib_viper::keymap::Keymap;

use crate::categories::ports;
use crate::categories::router::PageRouter;
use crate::categories::{CategoriesSetup, SettingsCategoriesSnapshot};
use crate::{
    ledger_store::{LedgerStore, notify_on},
    navigation::movement::Movement,
    settings::{SettingsPage, page::PageRowClicked, pages::Page},
    shell::view::statusline::Hint,
    view::event::ViewEvent,
};

impl EventEmitter<ViewEvent> for CategoriesPage {}
impl EventEmitter<PageRowClicked> for CategoriesPage {}

pub struct CategoriesPage {
    presenter: CategoriesPagePresenter,
    categories: SharedCategories,
    ledger: Entity<LedgerStore>,
    keymap: Keymap<CategoriesPageEvent>,
    /// "Today" for the Monthly budget a Dialog shows, read once when the Shell is built.
    today: NaiveDate,
    /// Whether the page, not Settings' index, has keyboard focus, told by the Settings View as it
    /// draws the page.
    page_focused: bool,
    /// Repaints the page when the tree changes: a Categories write, a reassign, a rollback.
    _categories_changed: Subscription,
}

impl CategoriesPage {
    pub fn new(
        setup: CategoriesSetup,
        ledger: Entity<LedgerStore>,
        today: NaiveDate,
        cx: &mut Context<'_, Self>,
    ) -> Self {
        let categories = ports::shared(&setup.store);
        Self {
            presenter: CategoriesPagePresenter::new(categories.clone(), setup.keymap.clone()),
            categories,
            _categories_changed: notify_on(&ledger, Domain::Categories, cx),
            ledger,
            keymap: setup.keymap,
            today,
            page_focused: false,
        }
    }

    /// A key or click on the page: `event` as one input, committed once it is handled.
    pub(in crate::categories) fn send(
        &mut self,
        event: CategoriesPageEvent,
        cx: &mut Context<'_, Self>,
    ) {
        crate::input::send(cx, |cx| self.handle(event, cx));
    }

    /// Hands `event` to the Presenter with the Desktop's Router, then brings the Ledger read copy
    /// in line with whatever it wrote. Inside an input already, as [`Self::send`] or a Dialog's
    /// confirm.
    fn handle(&mut self, event: CategoriesPageEvent, cx: &mut Context<'_, Self>) {
        ports::refresh(&self.ledger, cx);
        let mut router = PageRouter {
            categories: self.categories.clone(),
            ledger: self.ledger.clone(),
            today: self.today,
            cx,
        };
        self.presenter.handle(event, &mut router);
        ports::refresh(&self.ledger, cx);
        cx.notify();
    }

    /// A confirmed Dialog's outcome, handed back by the Dialog host inside the confirm's input.
    pub(in crate::categories) fn dialog_landed(
        &mut self,
        outcome: CategoriesOutcome,
        cx: &mut Context<'_, Self>,
    ) {
        self.handle(CategoriesPageEvent::DialogClosed(Some(outcome)), cx);
    }

    pub fn view_model(&self) -> CategoriesPageViewModel {
        self.presenter.view_model()
    }

    /// A row click: `event`, and focus on the page.
    fn click(&mut self, event: CategoriesPageEvent, cx: &mut Context<'_, Self>) {
        self.send(event, cx);
        cx.emit(PageRowClicked);
    }

    /// The Categories in the store's tree order, and the selected row's name.
    #[doc(hidden)]
    pub fn snapshot(&self, _cx: &App) -> SettingsCategoriesSnapshot {
        let categories = self.categories.borrow().categories();
        let selected = self.view_model().selected;
        SettingsCategoriesSnapshot {
            selected_category: categories
                .iter()
                .find(|category| Some(category.id) == selected)
                .map(|category| category.name.clone()),
            category_types: categories
                .iter()
                .map(|c| (c.name.clone(), c.category_type.clone()))
                .collect(),
            category_names: categories.into_iter().map(|c| c.name).collect(),
        }
    }
}

impl Page for CategoriesPage {
    const PAGE: SettingsPage = SettingsPage::Categories;

    /// Every chord the Categories keymap binds, the arrows and `g g` among them, except `left`
    /// with nothing to fold or climb to, which is the Settings index's. The Shell has already
    /// taken its own keys.
    fn module_key(&mut self, chord: &str, cx: &mut Context<'_, Self>) -> bool {
        let Some(event) = self.keymap.event(chord) else {
            return false;
        };
        if !self.presenter.takes(&event) {
            return false;
        }
        self.send(event, cx);
        true
    }

    /// A movement the keymap doesn't bind (`enter`) does nothing on the page, and Settings leaves
    /// the page where it is: there is no hand-off from a Category.
    fn apply_movement(&mut self, _movement: Movement, _cx: &mut Context<'_, Self>) -> bool {
        true
    }

    /// The Presenter's legend, rebound keys included, which the `?` cheat sheet reads.
    fn hints(&self, _cx: &App) -> Vec<Hint> {
        legend(self.view_model().hints)
    }

    fn status_note(&self, _cx: &App) -> String {
        self.view_model().scope
    }

    fn set_page_focused(&mut self, focused: bool, _cx: &mut Context<'_, Self>) {
        self.page_focused = focused;
    }

    fn heading_meta(&self, _cx: &App) -> AnyElement {
        self.view_model().scope.into_any_element()
    }
}

/// The Presenter's hints as the status line and the `?` cheat sheet take them.
fn legend(hints: Vec<lib_categories::presenter::Hint>) -> Vec<Hint> {
    hints
        .into_iter()
        .map(|hint| (hint.chord, hint.action))
        .collect()
}
