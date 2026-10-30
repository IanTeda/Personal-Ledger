//! The Settings pages as one trait (ADR-0037's amendment, "Settings pages forward through a trait
//! with default methods"). A page implements [`Page`] for what it handles and takes the default for
//! the rest; [`PageHandle`] is the same surface over its `Entity`, so [`SettingsPages::get`] hands
//! the Settings View the open page as one `dyn` handle and `get` is the only match on the page.

use gpui::{
    AnyElement, AnyView, App, AppContext as _, Context, Entity, EventEmitter, IntoElement,
    Keystroke, ListAlignment, ListState, Render, Subscription,
};

use crate::{
    accounts::{AccountsPage, AccountsSetup, accounts_page},
    categories::{CategoriesPage, CategoriesSetup, categories_page},
    institutions::{InstitutionsPage, InstitutionsSetup, institutions_page},
    inventory::{InventoryPage, InventorySetup, inventory_page},
    ledger_store::LedgerStore,
    navigation::movement::Movement,
    payees::{PayeesPage, PayeesSetup, payees_page},
    settings::{
        SettingsPage,
        about::AboutPage,
        data_backup::DataBackupPage,
        display::DisplayPage,
        documents::DocumentsPage,
        general::GeneralPage,
        hints::settings_plain_page_hints,
        page::PageRowClicked,
        state::SettingsView,
        sync_server::SyncServerPage,
        tracing::{
            TracingPage,
            log::{LogView, TracingLevel},
        },
    },
    shell::view::{
        statusline::{Hint, PageStatus, owned_hints},
        window_size::WindowSize,
    },
    tags::{TagsPage, TagsSetup, tags_page},
    units::{UnitsPage, UnitsSetup, units_page},
    view::event::{Reveal, ViewEvent},
};

/// What `shell::router::build` hands the Settings pages that are modules' Views: the Units,
/// Institutions, Accounts, Categories and Payees modules' ports and keymaps.
pub struct SettingsSetup {
    pub units: UnitsSetup,
    pub institutions: InstitutionsSetup,
    pub payees: PayeesSetup,
    pub accounts: AccountsSetup,
    pub categories: CategoriesSetup,
    pub tags: TagsSetup,
    pub inventory: InventorySetup,
}

/// Rows measured beyond the Tracing log box's visible edge, so scrolling doesn't pop rows in.
const LOG_LIST_OVERDRAW: gpui::Pixels = gpui::px(200.0);

/// What the Settings View tells the open page about the space it is drawn in, before its keys and
/// its draw.
#[derive(Debug, Clone, Copy)]
pub struct PageFrame {
    /// The Window Size the View was last drawn at.
    pub window_size: WindowSize,
    /// The page body's width, which sets the Display page's Colour Theme grid columns.
    pub body_width: f32,
}

/// One Settings page's View: what it draws and the keys it takes. Its identity (order, id, label,
/// scope note) stays on [`SettingsPage`], which the page names through [`Self::PAGE`]. Every item
/// but `PAGE` has a default, so a page writes only what it handles. Every page emits
/// [`PageRowClicked`], which the Settings View follows to hand the page focus; a page without rows
/// never emits it.
pub trait Page: Render + EventEmitter<ViewEvent> + EventEmitter<PageRowClicked> + Sized {
    /// The page's place in the index; the defaults read its scope note.
    const PAGE: SettingsPage;

    /// Whether the page fills the body's height and scrolls inside itself (Tracing's log box)
    /// rather than the body scrolling the whole page.
    const FILLS_PAGE: bool = false;

    /// Whether the page draws past [`super::CONTENT_MAX_WIDTH`] to the View's width (Units'
    /// tables, down to their 610px minimum).
    const FULL_WIDTH: bool = false;

    /// A key the page's module keymap takes, by its `chord`, ahead of every key of Settings' own
    /// (step 4 of `crate::input`'s route). Only a page that is a module's View has one.
    fn module_key(&mut self, _chord: &str, _cx: &mut Context<'_, Self>) -> bool {
        false
    }

    /// A key the page claims ahead of the Shell's fallback and Settings' focus keys.
    fn key_ahead(&mut self, _keystroke: &Keystroke, _cx: &mut Context<'_, Self>) -> bool {
        false
    }

    /// A key no other step took.
    fn handle_key(&mut self, _keystroke: &Keystroke, _cx: &mut Context<'_, Self>) -> bool {
        false
    }

    /// Whether the page took the movement. A page without a selection leaves Settings to scroll.
    fn apply_movement(&mut self, _movement: Movement, _cx: &mut Context<'_, Self>) -> bool {
        false
    }

    /// Closes what the page has open; `false` leaves `Esc` to Settings, which steps back to the
    /// index.
    fn escape(&mut self, _cx: &mut Context<'_, Self>) -> bool {
        false
    }

    /// `/` is inert on a page that does not open a Search.
    fn start_search(&mut self, _cx: &mut Context<'_, Self>) -> bool {
        false
    }

    fn search_key(&mut self, _keystroke: &Keystroke, _cx: &mut Context<'_, Self>) -> bool {
        false
    }

    /// Shows a record another View handed off.
    fn reveal(&mut self, _reveal: Reveal, _cx: &mut Context<'_, Self>) {}

    /// Told whether the page, not the index rail, holds keyboard focus, as the View draws it and
    /// as focus moves in or out.
    fn set_page_focused(&mut self, _focused: bool, _cx: &mut Context<'_, Self>) {}

    fn set_frame(&mut self, _frame: PageFrame, _cx: &mut Context<'_, Self>) {}

    /// The heading's right-aligned meta.
    fn heading_meta(&self, _cx: &App) -> AnyElement {
        Self::PAGE.scope_note().into_any_element()
    }

    /// The page's keys as `(key, action)`, for the legend and the `?` cheat sheet. The keys are
    /// owned, so a page on a module Presenter returns its Presenter's hints here, rebound keys
    /// included, and the cheat sheet lists what its status line shows.
    fn hints(&self, _cx: &App) -> Vec<Hint> {
        owned_hints(settings_plain_page_hints())
    }

    /// The status line while the page has focus.
    fn page_status(&self, cx: &App) -> Option<PageStatus> {
        Some(PageStatus {
            hints: self.hints(cx),
            right: self.status_note(cx),
        })
    }

    /// The status line's right-aligned note while the page has focus.
    fn status_note(&self, _cx: &App) -> String {
        Self::PAGE.scope_note()
    }

    /// A pane drawn at the View's right edge, beside the page.
    fn side_pane(&self, _entity: &Entity<Self>, _cx: &App) -> Option<AnyElement> {
        None
    }
}

/// [`Page`] over a page's `Entity`: what the Settings View calls, without knowing which page.
pub trait PageHandle {
    fn module_key(&self, chord: &str, cx: &mut App) -> bool;
    fn key_ahead(&self, keystroke: &Keystroke, cx: &mut App) -> bool;
    fn handle_key(&self, keystroke: &Keystroke, cx: &mut App) -> bool;
    fn apply_movement(&self, movement: Movement, cx: &mut App) -> bool;
    fn escape(&self, cx: &mut App) -> bool;
    fn start_search(&self, cx: &mut App) -> bool;
    fn search_key(&self, keystroke: &Keystroke, cx: &mut App) -> bool;
    fn reveal(&self, reveal: Reveal, cx: &mut App);
    fn set_page_focused(&self, focused: bool, cx: &mut App);
    fn set_frame(&self, frame: PageFrame, cx: &mut App);
    fn fills_page(&self) -> bool;
    fn full_width(&self) -> bool;
    fn heading_meta(&self, cx: &App) -> AnyElement;
    fn hints(&self, cx: &App) -> Vec<Hint>;
    fn page_status(&self, cx: &App) -> Option<PageStatus>;
    fn side_pane(&self, cx: &App) -> Option<AnyElement>;
    fn view(&self) -> AnyView;
    /// Re-emits the page's [`ViewEvent`]s, repaints the View on its changes and follows its row
    /// clicks.
    fn subscribe(&self, cx: &mut Context<'_, SettingsView>) -> Vec<Subscription>;
}

impl<T: Page> PageHandle for Entity<T> {
    fn module_key(&self, chord: &str, cx: &mut App) -> bool {
        self.update(cx, |page, cx| page.module_key(chord, cx))
    }

    fn key_ahead(&self, keystroke: &Keystroke, cx: &mut App) -> bool {
        self.update(cx, |page, cx| page.key_ahead(keystroke, cx))
    }

    fn handle_key(&self, keystroke: &Keystroke, cx: &mut App) -> bool {
        self.update(cx, |page, cx| page.handle_key(keystroke, cx))
    }

    fn apply_movement(&self, movement: Movement, cx: &mut App) -> bool {
        self.update(cx, |page, cx| page.apply_movement(movement, cx))
    }

    fn escape(&self, cx: &mut App) -> bool {
        self.update(cx, |page, cx| page.escape(cx))
    }

    fn start_search(&self, cx: &mut App) -> bool {
        self.update(cx, |page, cx| page.start_search(cx))
    }

    fn search_key(&self, keystroke: &Keystroke, cx: &mut App) -> bool {
        self.update(cx, |page, cx| page.search_key(keystroke, cx))
    }

    fn reveal(&self, reveal: Reveal, cx: &mut App) {
        self.update(cx, |page, cx| page.reveal(reveal, cx));
    }

    fn set_page_focused(&self, focused: bool, cx: &mut App) {
        self.update(cx, |page, cx| page.set_page_focused(focused, cx));
    }

    fn set_frame(&self, frame: PageFrame, cx: &mut App) {
        self.update(cx, |page, cx| page.set_frame(frame, cx));
    }

    fn fills_page(&self) -> bool {
        T::FILLS_PAGE
    }

    fn full_width(&self) -> bool {
        T::FULL_WIDTH
    }

    fn heading_meta(&self, cx: &App) -> AnyElement {
        self.read(cx).heading_meta(cx)
    }

    fn hints(&self, cx: &App) -> Vec<Hint> {
        self.read(cx).hints(cx)
    }

    fn page_status(&self, cx: &App) -> Option<PageStatus> {
        self.read(cx).page_status(cx)
    }

    fn side_pane(&self, cx: &App) -> Option<AnyElement> {
        self.read(cx).side_pane(self, cx)
    }

    fn view(&self) -> AnyView {
        self.clone().into()
    }

    fn subscribe(&self, cx: &mut Context<'_, SettingsView>) -> Vec<Subscription> {
        vec![
            cx.subscribe(self, |_view, _page, event: &ViewEvent, cx| {
                cx.emit(event.clone());
            }),
            cx.observe(self, |_view, _page, cx| cx.notify()),
            cx.subscribe(self, |view, _page, _event: &PageRowClicked, cx| {
                view.focus_page_from_click(cx);
            }),
        ]
    }
}

/// The Entity of every Settings page. Settings builds them and `Root` reaches them only through
/// the typed accessors on [`SettingsView`], for the palette commands that drive a page, the
/// Tracing capture feed and the test snapshots.
pub struct SettingsPages {
    pub general: Entity<GeneralPage>,
    pub display: Entity<DisplayPage>,
    pub units: Entity<UnitsPage>,
    pub institutions: Entity<InstitutionsPage>,
    pub accounts: Entity<AccountsPage>,
    pub categories: Entity<CategoriesPage>,
    pub tags: Entity<TagsPage>,
    pub payees: Entity<PayeesPage>,
    pub documents: Entity<DocumentsPage>,
    pub inventory: Entity<InventoryPage>,
    pub sync_server: Entity<SyncServerPage>,
    pub data_backup: Entity<DataBackupPage>,
    pub tracing: Entity<TracingPage>,
    pub about: Entity<AboutPage>,
}

impl SettingsPages {
    pub fn new(
        ledger: &Entity<LedgerStore>,
        clock: &lib_core::Clock,
        setup: SettingsSetup,
        cx: &mut Context<'_, SettingsView>,
    ) -> Self {
        let SettingsSetup {
            units,
            institutions,
            payees,
            accounts,
            categories,
            tags,
            inventory,
        } = setup;
        Self {
            general: cx.new(|cx| GeneralPage::new(ledger.clone(), units.ports.clone(), cx)),
            display: cx.new(|_| DisplayPage::default()),
            units: units_page(
                units.ports,
                units.deletion,
                ledger.clone(),
                units.keymap,
                units.prices_keymap,
                cx,
            ),
            institutions: institutions_page(
                institutions.ports,
                ledger.clone(),
                institutions.keymap,
                cx,
            ),
            accounts: accounts_page(accounts, ledger.clone(), clock.clone(), cx),
            categories: categories_page(categories, ledger.clone(), clock, cx),
            tags: tags_page(tags, ledger.clone(), clock.clone(), cx),
            payees: payees_page(
                payees.ports,
                ledger.clone(),
                clock.clone(),
                payees.keymap,
                cx,
            ),
            documents: cx.new(|cx| DocumentsPage::new(ledger.clone(), cx)),
            inventory: inventory_page(inventory, ledger.clone(), cx),
            sync_server: cx.new(|_| SyncServerPage),
            data_backup: cx.new(|_| DataBackupPage),
            // A private, empty capture until `set_log_capture` hands over the real one.
            tracing: cx.new(|_| {
                TracingPage::new(
                    LogView::new(
                        lib_tracing::LogBuffer::new(lib_tracing::LOG_CAPACITY),
                        TracingLevel::default(),
                    ),
                    ListState::new(0, ListAlignment::Top, LOG_LIST_OVERDRAW),
                )
            }),
            about: cx.new(|_| AboutPage),
        }
    }

    /// The page's handle. The one match on the page: a new page fails to compile here.
    pub fn get(&self, page: SettingsPage) -> &dyn PageHandle {
        match page {
            SettingsPage::General => &self.general,
            SettingsPage::Display => &self.display,
            SettingsPage::Units => &self.units,
            SettingsPage::Institutions => &self.institutions,
            SettingsPage::Accounts => &self.accounts,
            SettingsPage::Categories => &self.categories,
            SettingsPage::Tags => &self.tags,
            SettingsPage::Payees => &self.payees,
            SettingsPage::Documents => &self.documents,
            SettingsPage::Inventory => &self.inventory,
            SettingsPage::SyncServer => &self.sync_server,
            SettingsPage::DataBackup => &self.data_backup,
            SettingsPage::Tracing => &self.tracing,
            SettingsPage::About => &self.about,
        }
    }

    pub fn all(&self) -> impl Iterator<Item = &dyn PageHandle> {
        SettingsPage::ALL.into_iter().map(|page| self.get(page))
    }
}
