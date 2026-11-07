//! The Categories Settings page's View: the `lib-categories` page Presenter in a gpui Entity. It
//! looks up each key the Shell passes on, as chord text, in the Categories keymap, turns clicks
//! into events, draws the view model (`render.rs`). Each key and click is one input (`crate::input::send`), which
//! commits what it wrote. The Settings View hosts it as any other page.

mod render;

use gpui::{AnyElement, App, Context, Entity, EventEmitter, IntoElement as _, Subscription};
use lib_categories::presenter::{
    CategoriesPageEvent, CategoriesPagePresenter, CategoriesPageViewModel,
};
use lib_categories::{CategoriesOutcome, SharedCategories};
use lib_database::Domain;
use lib_viper::keymap::Keymap;

use lib_shell::Hint;

use crate::categories::router::PageRouter;
use crate::categories::{CategoriesSetup, SettingsCategoriesSnapshot};
use crate::category_deletion::DeletionStores;
use crate::settings::snapshot::PageSnapshot;
use crate::{
    announcer::{Announcer, notify_on},
    navigation::movement::Movement,
    settings::{
        SettingsPage,
        pages::{Page, PageRowClicked},
    },
    shell::view::page_clicks::PresenterPage,
    view::event::ViewEvent,
};

impl EventEmitter<ViewEvent> for CategoriesPage {}
impl EventEmitter<PageRowClicked> for CategoriesPage {}

pub struct CategoriesPage {
    presenter: CategoriesPagePresenter,
    categories: SharedCategories,
    /// Category delete's port and reads, which the Delete category Dialog opens on.
    deletion: DeletionStores,
    /// The ports the page's Dialogs read and write beyond the Categories.
    save: lib_database::StoredCategoriesPorts,
    /// The Budgets store, which the snapshot reads.
    budgets: lib_database::SharedBudgetsStore,
    keymap: Keymap<CategoriesPageEvent>,
    /// Whether the page, not Settings' index, has keyboard focus, told by the Settings View as it
    /// draws the page.
    page_focused: bool,
    /// Repaints the page when the tree changes: a Categories write, a reassign, a rollback.
    _categories_changed: Subscription,
}

impl CategoriesPage {
    pub fn new(
        setup: CategoriesSetup,
        announcer: Entity<Announcer>,
        cx: &mut Context<'_, Self>,
    ) -> Self {
        let categories = setup.categories;
        Self {
            presenter: CategoriesPagePresenter::new(categories.clone(), setup.keymap.clone()),
            categories,
            deletion: setup.deletion,
            save: setup.save,
            budgets: setup.budgets,
            _categories_changed: notify_on(&announcer, Domain::Categories, cx),
            keymap: setup.keymap,
            page_focused: false,
        }
    }

    /// The one dispatch a keymap event takes into the page, from a key ([`Self::module_key`]) or a palette Command ([`Self::module_command`]): sent as one input, unless the page doesn't take it now.
    fn take(&mut self, event: CategoriesPageEvent, cx: &mut Context<'_, Self>) -> bool {
        if !self.presenter.takes(&event) {
            return false;
        }
        self.send(event, cx);
        true
    }

    /// A key or click on the page: `event` as one input, committed once it is handled.
    fn send(&mut self, event: CategoriesPageEvent, cx: &mut Context<'_, Self>) {
        crate::input::join_or_send(cx, |cx| self.handle(event, cx));
    }

    /// Hands `event` to the Presenter with the Desktop's Router. Inside an input already, as
    /// [`Self::send`] or a Dialog's confirm.
    fn handle(&mut self, event: CategoriesPageEvent, cx: &mut Context<'_, Self>) {
        let mut router = PageRouter {
            categories: self.categories.clone(),
            deletion: self.deletion.clone(),
            save: self.save.clone(),
            cx,
        };
        self.presenter.handle(event, &mut router);
        cx.notify();
    }

    /// A confirmed Dialog's outcome, handed back by the Dialog host inside the confirm's input,
    /// whose commit names what the confirm wrote: a delete's Splits, Bill Plans, Payee defaults
    /// and Budget Limits as well as the Categories.
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
}

/// The page's clicks on its rows and buttons.
impl PresenterPage<CategoriesPageEvent> for CategoriesPage {
    fn send(&mut self, event: CategoriesPageEvent, cx: &mut Context<'_, Self>) {
        CategoriesPage::send(self, event, cx);
    }

    /// A click on a row hands the page focus.
    fn row_clicked(&mut self, cx: &mut Context<'_, Self>) {
        cx.emit(PageRowClicked);
    }
}

impl PageSnapshot for SettingsCategoriesSnapshot {
    type Page = CategoriesPage;

    /// The Categories in the store's tree order, and the selected row's name.
    fn read(page: &CategoriesPage, _cx: &App) -> Self {
        let categories = page.categories.borrow().categories();
        let selected = page.view_model().selected;
        let selected_budget_months = selected
            .and_then(|id| {
                let budget = page
                    .budgets
                    .borrow()
                    .budgets()
                    .budget(crate::budgets::PERSONAL_SPENDING_ID)?;
                let records = budget.limits.get(&id)?;
                Some(
                    records
                        .iter()
                        .map(|record| (record.month.year, record.month.month))
                        .collect(),
                )
            })
            .unwrap_or_default();
        SettingsCategoriesSnapshot {
            selected_budget_months,
            selected_category: categories
                .iter()
                .find(|category| Some(category.id) == selected)
                .map(|category| category.name.clone()),
            category_types: categories
                .iter()
                .map(|c| (c.name.clone(), c.category_type.clone()))
                .collect(),
            category_ids: categories.iter().map(|c| c.id).collect(),
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
        self.take(event, cx)
    }

    /// A palette Command, by its keymap `command` name: the event its key sends, if the page
    /// takes it now, as [`Self::module_key`] asks.
    fn module_command(&mut self, command: &str, cx: &mut Context<'_, Self>) -> bool {
        let Some(event) = self.keymap.event_for(command) else {
            return false;
        };
        self.take(event, cx)
    }

    /// A movement the keymap doesn't bind (`enter`) does nothing on the page, and Settings leaves
    /// the page where it is: there is no hand-off from a Category.
    fn apply_movement(&mut self, _movement: Movement, _cx: &mut Context<'_, Self>) -> bool {
        true
    }

    /// The Presenter's legend, rebound keys included, which the `?` cheat sheet reads.
    fn hints(&self, _cx: &App) -> Vec<Hint> {
        self.view_model().hints
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
