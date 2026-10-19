//! The Settings pages as one trait (ADR-0037's amendment, "Settings pages forward through a trait
//! with default methods"). A page implements [`Page`] for what it handles and takes the default for
//! the rest; [`PageHandle`] is the same surface over its `Entity`, so [`SettingsPages::get`] hands
//! the Settings View the open page as one `dyn` handle and `get` is the only match on the page.

use chrono::NaiveDate;
use gpui::{
    AnyElement, AnyView, App, AppContext as _, Context, Entity, EventEmitter, IntoElement,
    Keystroke, ListAlignment, ListState, Render, Subscription,
};

use crate::{
    chrome::{statusline::PageStatus, window_size::WindowSize},
    ledger_store::LedgerStore,
    navigation::key_router::Movement,
    settings::{
        SettingsPage,
        about::AboutPage,
        accounts::AccountsPage,
        categories::CategoriesPage,
        data_backup::DataBackupPage,
        display::DisplayPage,
        documents::DocumentsPage,
        general::GeneralPage,
        hints::settings_plain_page_hints,
        institutions::InstitutionsPage,
        inventory::InventoryPage,
        page::PageRowClicked,
        payees::{PayeesPage, PayeesSources},
        state::SettingsView,
        sync_server::SyncServerPage,
        tags::TagsPage,
        tracing::{
            TracingPage,
            log::{LogView, TracingLevel},
        },
        units::UnitsPage,
    },
    view::event::{Reveal, ViewEvent},
};

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

/// One Settings page. Every method but [`Self::PAGE`] has a default, so a page writes only what it
/// handles.
pub trait Page: Render + EventEmitter<ViewEvent> + Sized {
    /// The page's place in the index; the defaults read its scope note.
    const PAGE: SettingsPage;

    /// A key the page claims ahead of the router and Settings' focus keys.
    fn key_ahead(&mut self, _keystroke: &Keystroke, _cx: &mut Context<'_, Self>) -> bool {
        false
    }

    /// A key the router left over.
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

    /// Told whether the page, not the index rail, holds keyboard focus, as the View draws it.
    fn set_page_focused(&mut self, _focused: bool) {}

    fn set_frame(&mut self, _frame: PageFrame, _cx: &mut Context<'_, Self>) {}

    /// The heading's right-aligned meta.
    fn heading_meta(&self, _cx: &App) -> AnyElement {
        Self::PAGE.scope_note().into_any_element()
    }

    /// The page's keys as `(key, action)`, for the legend and the `?` cheat sheet.
    fn hints(&self, _cx: &App) -> Vec<(&'static str, String)> {
        settings_plain_page_hints()
    }

    /// The status line while the page has focus.
    fn page_status(&self, cx: &App) -> Option<PageStatus> {
        Some(PageStatus {
            hints: self.hints(cx),
            right: Self::PAGE.scope_note(),
        })
    }

    /// A pane drawn at the View's right edge, beside the page.
    fn side_pane(&self, _entity: &Entity<Self>, _cx: &App) -> Option<AnyElement> {
        None
    }

    /// Subscribes the Settings View to the page's row clicks, for a page with rows.
    fn subscribe_row_clicks(
        _entity: &Entity<Self>,
        _cx: &mut Context<'_, SettingsView>,
    ) -> Option<Subscription> {
        None
    }
}

/// Subscribes the Settings View to a page's [`PageRowClicked`], which hands the page focus.
pub fn focus_on_row_click<T: Page + EventEmitter<PageRowClicked>>(
    entity: &Entity<T>,
    cx: &mut Context<'_, SettingsView>,
) -> Option<Subscription> {
    Some(
        cx.subscribe(entity, |view, _page, _event: &PageRowClicked, cx| {
            view.focus_page_from_click(cx)
        }),
    )
}

/// [`Page`] over a page's `Entity`: what the Settings View calls, without knowing which page.
pub trait PageHandle {
    fn key_ahead(&self, keystroke: &Keystroke, cx: &mut App) -> bool;
    fn handle_key(&self, keystroke: &Keystroke, cx: &mut App) -> bool;
    fn apply_movement(&self, movement: Movement, cx: &mut App) -> bool;
    fn escape(&self, cx: &mut App) -> bool;
    fn start_search(&self, cx: &mut App) -> bool;
    fn search_key(&self, keystroke: &Keystroke, cx: &mut App) -> bool;
    fn reveal(&self, reveal: Reveal, cx: &mut App);
    fn set_page_focused(&self, focused: bool, cx: &mut App);
    fn set_frame(&self, frame: PageFrame, cx: &mut App);
    fn heading_meta(&self, cx: &App) -> AnyElement;
    fn hints(&self, cx: &App) -> Vec<(&'static str, String)>;
    fn page_status(&self, cx: &App) -> Option<PageStatus>;
    fn side_pane(&self, cx: &App) -> Option<AnyElement>;
    fn view(&self) -> AnyView;
    /// Re-emits the page's [`ViewEvent`]s, repaints the View on its changes and follows its row
    /// clicks.
    fn subscribe(&self, cx: &mut Context<'_, SettingsView>) -> Vec<Subscription>;
}

impl<T: Page> PageHandle for Entity<T> {
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
        self.update(cx, |page, _| page.set_page_focused(focused));
    }

    fn set_frame(&self, frame: PageFrame, cx: &mut App) {
        self.update(cx, |page, cx| page.set_frame(frame, cx));
    }

    fn heading_meta(&self, cx: &App) -> AnyElement {
        self.read(cx).heading_meta(cx)
    }

    fn hints(&self, cx: &App) -> Vec<(&'static str, String)> {
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
        let mut subscriptions = vec![
            cx.subscribe(self, |_view, _page, event: &ViewEvent, cx| {
                cx.emit(event.clone());
            }),
            cx.observe(self, |_view, _page, cx| cx.notify()),
        ];
        subscriptions.extend(T::subscribe_row_clicks(self, cx));
        subscriptions
    }
}

/// The Entity of every Settings page. Settings builds them and `Shell` reaches them only through
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
        today: NaiveDate,
        cx: &mut Context<'_, SettingsView>,
    ) -> Self {
        Self {
            general: cx.new(|_| GeneralPage),
            display: cx.new(|_| DisplayPage::default()),
            units: cx.new(|cx| UnitsPage::new(ledger.clone(), cx)),
            institutions: cx.new(|cx| InstitutionsPage::new(ledger.clone(), cx)),
            accounts: cx.new(|cx| AccountsPage::new(ledger.clone(), cx)),
            categories: cx.new(|cx| CategoriesPage::new(ledger.clone(), today, cx)),
            tags: cx.new(|cx| TagsPage::new(ledger.clone(), cx)),
            payees: cx.new(|cx| {
                PayeesPage::new(
                    PayeesSources {
                        ledger: ledger.clone(),
                    },
                    cx,
                )
            }),
            documents: cx.new(|cx| DocumentsPage::new(ledger.clone(), cx)),
            inventory: cx.new(|cx| InventoryPage::new(ledger.clone(), cx)),
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
