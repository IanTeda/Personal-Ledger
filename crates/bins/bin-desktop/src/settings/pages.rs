//! The Settings pages as one trait (ADR-0037's amendment, "Settings pages forward through a trait
//! with default methods"). A page implements [`Page`] for what it handles and takes the default for
//! the rest; [`PageHandle`] is the same surface over its `Entity`, so [`SettingsPages::get`] hands
//! the Settings View the open page as one `dyn` handle and `get` is the only match on the page.

use gpui::{
    AnyElement, AnyView, App, AppContext as _, Context, Entity, EventEmitter, IntoElement,
    Keystroke, ListAlignment, ListState, Render, Subscription,
};

use std::rc::Rc;

use lib_settings::{SettingsFrameEvent, keymap::settings_frame_keymap};
use lib_shell::Hint;
use lib_viper::keymap::{KeyBindingConfig, Keymap};

use crate::{
    accounts::{AccountsPage, AccountsSetup, accounts_page},
    categories::{CategoriesPage, CategoriesSetup, categories_page},
    document_types::{DocumentTypesPage, DocumentTypesSetup, document_types_page},
    institutions::{InstitutionsPage, InstitutionsSetup, institutions_page},
    inventory::{InventoryPage, InventorySetup, inventory_page},
    ledger_store::LedgerStore,
    navigation::movement::Movement,
    payees::{PayeesPage, PayeesSetup, payees_page},
    settings::{
        SettingsPage,
        about::AboutPage,
        data_backup::DataBackupPage,
        device::DeviceConfigFile,
        display::DisplayPage,
        general::GeneralPage,
        ledger::{LedgerKeys, LedgerPage},
        state::SettingsView,
        sync_server::SyncServerPage,
        tracing::{TracingPage, log::CaptureLog},
    },
    shell::view::{statusline::PageStatus, window_size::WindowSize},
    tags::{TagsPage, TagsSetup, tags_page},
    units::{UnitsPage, UnitsSetup, units_page},
    view::event::{Reveal, ViewEvent},
};

/// A click on a row of a Settings page, which hands the page keyboard focus. Every page with rows
/// emits it, and the Settings View, which owns the focus, follows it.
#[derive(Debug, Clone, Copy)]
pub struct PageRowClicked;

/// What `shell::router::build` hands the Settings pages that are modules' Views: the Units,
/// Institutions, Accounts, Categories, Payees and Document Types modules' ports and keymaps.
pub struct SettingsSetup {
    pub units: UnitsSetup,
    pub institutions: InstitutionsSetup,
    pub payees: PayeesSetup,
    pub document_types: DocumentTypesSetup,
    pub accounts: AccountsSetup,
    pub categories: CategoriesSetup,
    pub tags: TagsSetup,
    pub inventory: InventorySetup,
    pub ledger: LedgerKeys,
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

    /// A palette Command the page's module keymap binds, by its `command` name: the page sends the
    /// event the key would. Only a page that is a module's View has one.
    fn module_command(&mut self, _command: &str, _cx: &mut Context<'_, Self>) -> bool {
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
        super::scope_note(Self::PAGE).into_any_element()
    }

    /// The page's keys, for the legend and the `?` cheat sheet. A page returns its Presenter's
    /// hints here, rebound keys included, so the cheat sheet lists what its status line shows.
    fn hints(&self, cx: &App) -> Vec<Hint>;

    /// The status line while the page has focus.
    fn page_status(&self, cx: &App) -> Option<PageStatus> {
        Some(PageStatus {
            hints: self.hints(cx),
            right: self.status_note(cx),
        })
    }

    /// The status line's right-aligned note while the page has focus.
    fn status_note(&self, _cx: &App) -> String {
        super::scope_note(Self::PAGE)
    }

    /// A pane drawn at the View's right edge, beside the page.
    fn side_pane(&self, _entity: &Entity<Self>, _cx: &App) -> Option<AnyElement> {
        None
    }
}

/// [`Page`] over a page's `Entity`: what the Settings View calls, without knowing which page.
pub trait PageHandle {
    fn module_key(&self, chord: &str, cx: &mut App) -> bool;
    fn module_command(&self, command: &str, cx: &mut App) -> bool;
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

    fn module_command(&self, command: &str, cx: &mut App) -> bool {
        self.update(cx, |page, cx| page.module_command(command, cx))
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
/// [`SettingsView::page`], for a module Router's entry point and the test snapshots. The This device pages stand with no Ledger open; the This Ledger pages are built
/// on one, so they are `None` until a Ledger opens (#977).
pub struct SettingsPages {
    pub general: Entity<GeneralPage>,
    pub display: Entity<DisplayPage>,
    pub tracing: Entity<TracingPage>,
    pub about: Entity<AboutPage>,
    /// The This Ledger pages, `None` with no Ledger open.
    pub ledger: Option<LedgerPages>,
}

/// The This Ledger pages, built on the Open Ledger.
pub struct LedgerPages {
    pub ledger: Entity<LedgerPage>,
    pub units: Entity<UnitsPage>,
    pub institutions: Entity<InstitutionsPage>,
    pub accounts: Entity<AccountsPage>,
    pub categories: Entity<CategoriesPage>,
    pub tags: Entity<TagsPage>,
    pub payees: Entity<PayeesPage>,
    pub documents: Entity<DocumentTypesPage>,
    pub inventory: Entity<InventoryPage>,
    pub sync_server: Entity<SyncServerPage>,
    pub data_backup: Entity<DataBackupPage>,
}

impl LedgerPages {
    /// The This Ledger pages. `frame_keys` is the frame's keymap, which the Sync server and Data &
    /// backup legends read their chords from.
    pub fn new(
        ledger: &Entity<LedgerStore>,
        clock: &lib_core::Clock,
        setup: SettingsSetup,
        frame_keys: &Keymap<SettingsFrameEvent>,
        cx: &mut Context<'_, SettingsView>,
    ) -> Self {
        let SettingsSetup {
            units,
            institutions,
            payees,
            document_types,
            accounts,
            categories,
            tags,
            inventory,
            ledger: ledger_keys,
        } = setup;
        Self {
            ledger: cx
                .new(|cx| LedgerPage::new(ledger.clone(), units.ports.clone(), ledger_keys, cx)),
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
            documents: document_types_page(document_types, ledger.clone(), cx),
            inventory: inventory_page(inventory, ledger.clone(), cx),
            sync_server: cx.new(|_| SyncServerPage::new(frame_keys.clone())),
            data_backup: cx.new(|_| DataBackupPage::new(frame_keys.clone())),
        }
    }
}

impl SettingsPages {
    /// The This device pages, with no Ledger open.
    pub fn new(cx: &mut Context<'_, SettingsView>) -> Self {
        Self {
            // No configuration file until `configure_device` hands over start-up's, so a write
            // before then is refused.
            general: cx.new(|_| {
                GeneralPage::new(
                    Rc::new(DeviceConfigFile::default()),
                    &KeyBindingConfig::default(),
                    super::state::back_chord(&KeyBindingConfig::default()),
                )
            }),
            display: cx.new(|_| {
                DisplayPage::new(
                    Rc::new(DeviceConfigFile::default()),
                    &KeyBindingConfig::default(),
                    super::state::back_chord(&KeyBindingConfig::default()),
                )
            }),
            // A private, empty capture until `set_log_capture` hands over the real one.
            tracing: cx.new(|_| {
                TracingPage::new(
                    Rc::new(CaptureLog::new(lib_tracing::LogBuffer::new(
                        lib_tracing::LOG_CAPACITY,
                    ))),
                    lib_settings::TracingLevel::default(),
                    &KeyBindingConfig::default(),
                    super::state::back_chord(&KeyBindingConfig::default()),
                    ListState::new(0, ListAlignment::Top, LOG_LIST_OVERDRAW),
                )
            }),
            about: cx.new(|_| {
                AboutPage::new(
                    env!("CARGO_PKG_VERSION"),
                    settings_frame_keymap(&KeyBindingConfig::default()),
                )
            }),
            ledger: None,
        }
    }

    /// The page's handle, `None` for a This Ledger page with no Ledger open. The one match on the
    /// page: a new page fails to compile here.
    pub fn get(&self, page: SettingsPage) -> Option<&dyn PageHandle> {
        let ledger = self.ledger.as_ref();
        match page {
            SettingsPage::General => Some(&self.general),
            SettingsPage::Display => Some(&self.display),
            SettingsPage::Tracing => Some(&self.tracing),
            SettingsPage::About => Some(&self.about),
            SettingsPage::Ledger => ledger.map(|pages| &pages.ledger as &dyn PageHandle),
            SettingsPage::Units => ledger.map(|pages| &pages.units as &dyn PageHandle),
            SettingsPage::Institutions => {
                ledger.map(|pages| &pages.institutions as &dyn PageHandle)
            }
            SettingsPage::Accounts => ledger.map(|pages| &pages.accounts as &dyn PageHandle),
            SettingsPage::Categories => ledger.map(|pages| &pages.categories as &dyn PageHandle),
            SettingsPage::Tags => ledger.map(|pages| &pages.tags as &dyn PageHandle),
            SettingsPage::Payees => ledger.map(|pages| &pages.payees as &dyn PageHandle),
            SettingsPage::Documents => ledger.map(|pages| &pages.documents as &dyn PageHandle),
            SettingsPage::Inventory => ledger.map(|pages| &pages.inventory as &dyn PageHandle),
            SettingsPage::SyncServer => ledger.map(|pages| &pages.sync_server as &dyn PageHandle),
            SettingsPage::DataBackup => ledger.map(|pages| &pages.data_backup as &dyn PageHandle),
        }
    }

    /// Every page built, in index order.
    pub fn all(&self) -> impl Iterator<Item = &dyn PageHandle> {
        SettingsPage::ALL
            .into_iter()
            .filter_map(|page| self.get(page))
    }
}
