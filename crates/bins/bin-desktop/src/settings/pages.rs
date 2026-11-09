//! The Settings pages as one trait (ADR-0037's amendment, "Settings pages forward through a trait
//! with default methods"). A page implements [`Page`] for what it handles and takes the default for
//! the rest; [`PageHandle`] is the same surface over its `Entity`, so [`SettingsPages::get`] hands
//! the Settings View the open page as one `dyn` handle and `get` is the only match on the page.

use gpui::{
    AnyElement, AnyView, App, AppContext as _, Context, Entity, EventEmitter, IntoElement, Render,
    Subscription,
};

use std::cell::RefCell;
use std::rc::Rc;

use lib_settings::{
    LedgerCounts, LedgerSummaries, SettingsFrameEvent, keymap::settings_frame_keymap,
};
use lib_shell::Hint;
use lib_viper::keymap::{KeyBindingConfig, Keymap};

use crate::{
    accounts::{AccountsPage, AccountsSetup, accounts_page},
    announcer::Announcer,
    categories::{CategoriesPage, CategoriesSetup, categories_page},
    document_types::{DocumentTypesPage, DocumentTypesSetup, document_types_page},
    institutions::{InstitutionsPage, InstitutionsSetup, institutions_page},
    inventory::{InventoryPage, InventorySetup, inventory_page},
    navigation::movement::Movement,
    payees::{PayeesPage, PayeesSetup, payees_page},
    settings::{
        SettingsPage,
        about::{AboutPage, about_page},
        data_backup::{DataBackupPage, data_backup_page},
        device::DeviceConfigFile,
        display::{DisplayPage, display_page},
        general::{GeneralPage, general_page},
        ledger::{LedgerKeys, LedgerPage, ledger_page},
        state::SettingsView,
        sync_server::{SyncServerPage, sync_server_page},
        tracing::{
            TracingPage,
            log::{CaptureLog, LogFeed},
            tracing_page,
        },
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

    /// Shows a record another View handed off.
    fn reveal(&mut self, _reveal: Reveal, _cx: &mut Context<'_, Self>) {}

    /// Told whether the page, not the index rail, holds keyboard focus, as the View draws it and
    /// as focus moves in or out.
    fn set_page_focused(&mut self, _focused: bool, _cx: &mut Context<'_, Self>) {}

    /// Told the space the page is drawn in, outside any input, which can be every draw. A module
    /// page hands it on as `PageView::framed`'s event.
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
    fn apply_movement(&self, movement: Movement, cx: &mut App) -> bool;
    fn escape(&self, cx: &mut App) -> bool;
    fn start_search(&self, cx: &mut App) -> bool;
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

    fn apply_movement(&self, movement: Movement, cx: &mut App) -> bool {
        self.update(cx, |page, cx| page.apply_movement(movement, cx))
    }

    fn escape(&self, cx: &mut App) -> bool {
        self.update(cx, |page, cx| page.escape(cx))
    }

    fn start_search(&self, cx: &mut App) -> bool {
        self.update(cx, |page, cx| page.start_search(cx))
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
    /// What the capture feed signals the Tracing page through.
    pub log_feed: Entity<LogFeed>,
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
    /// The Open Ledger's counts, which the X-Large index pane quotes for a This Ledger page.
    pub summaries: CachedCounts,
    /// Drops the cached counts as a write lands, so the pane recounts once rather than every draw.
    _ledger_changed: Subscription,
}

/// The Open Ledger's [`LedgerCounts`], counted once and kept until a write lands: counting clones
/// every store and groups the Tags for likely duplicates, too much for every repaint of the X-Large
/// index pane. It lives in [`LedgerPages`], so opening or closing a Ledger starts it afresh.
pub struct CachedCounts {
    source: Rc<dyn LedgerSummaries>,
    counts: RefCell<Option<LedgerCounts>>,
}

impl CachedCounts {
    fn new(source: Rc<dyn LedgerSummaries>) -> Self {
        Self {
            source,
            counts: RefCell::new(None),
        }
    }

    /// Forgets the counts, so the next read recounts.
    pub(super) fn invalidate(&self) {
        self.counts.replace(None);
    }
}

impl LedgerSummaries for CachedCounts {
    fn counts(&self) -> LedgerCounts {
        self.counts
            .borrow_mut()
            .get_or_insert_with(|| self.source.counts())
            .clone()
    }
}

impl LedgerPages {
    /// The This Ledger pages. `frame_keys` is the frame's keymap, which the Sync server and Data &
    /// backup legends read their chords from.
    pub fn new(
        announcer: &Entity<Announcer>,
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
        let summaries = CachedCounts::new(ledger_keys.settings_reads.clone());
        // Every domain's write can change a count, so any announcement drops them.
        let _ledger_changed = cx.subscribe(announcer, |view: &mut SettingsView, _, _, cx| {
            view.ledger_changed(cx);
        });
        Self {
            summaries,
            _ledger_changed,
            ledger: cx.new(|cx| ledger_page(ledger_keys, announcer, cx)),
            units: units_page(units, announcer.clone(), clock.clone(), cx),
            institutions: institutions_page(institutions, announcer.clone(), cx),
            accounts: accounts_page(accounts, announcer.clone(), clock.clone(), cx),
            categories: categories_page(categories, announcer.clone(), cx),
            tags: tags_page(tags, announcer.clone(), clock.clone(), cx),
            payees: payees_page(payees, announcer.clone(), clock.clone(), cx),
            documents: document_types_page(document_types, announcer.clone(), cx),
            inventory: inventory_page(inventory, announcer.clone(), cx),
            sync_server: cx.new(|cx| sync_server_page(frame_keys, announcer, cx)),
            data_backup: cx.new(|cx| data_backup_page(frame_keys, announcer, cx)),
        }
    }
}

impl SettingsPages {
    /// The This device pages, with no Ledger open.
    pub fn new(cx: &mut Context<'_, SettingsView>) -> Self {
        let log_feed = cx.new(|_| LogFeed);
        Self {
            // No configuration file until `configure_device` hands over start-up's, so a write
            // before then is refused.
            general: cx.new(|cx| {
                general_page(
                    Rc::new(DeviceConfigFile::default()),
                    &KeyBindingConfig::default(),
                    super::state::back_chord(&KeyBindingConfig::default()),
                    cx,
                )
            }),
            display: cx.new(|cx| {
                display_page(
                    Rc::new(DeviceConfigFile::default()),
                    &KeyBindingConfig::default(),
                    super::state::back_chord(&KeyBindingConfig::default()),
                    cx,
                )
            }),
            // A private, empty capture until `set_log_capture` hands over the real one.
            tracing: cx.new(|cx| {
                tracing_page(
                    Rc::new(CaptureLog::new(lib_tracing::LogBuffer::new(
                        lib_tracing::LOG_CAPACITY,
                    ))),
                    lib_settings::TracingLevel::default(),
                    &KeyBindingConfig::default(),
                    super::state::back_chord(&KeyBindingConfig::default()),
                    log_feed.clone(),
                    cx,
                )
            }),
            log_feed,
            about: cx.new(|cx| {
                about_page(
                    env!("CARGO_PKG_VERSION"),
                    &settings_frame_keymap(&KeyBindingConfig::default()),
                    cx,
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

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use super::*;

    /// Counts how often it is asked, so the cache's recounts show.
    #[derive(Default)]
    struct Counting {
        asked: Cell<usize>,
    }

    impl LedgerSummaries for Counting {
        fn counts(&self) -> LedgerCounts {
            self.asked.set(self.asked.get() + 1);
            LedgerCounts {
                tags: self.asked.get(),
                ..LedgerCounts::default()
            }
        }
    }

    #[test]
    fn the_counts_are_counted_once_until_a_write_drops_them() {
        let source = Rc::new(Counting::default());
        let cached = CachedCounts::new(source.clone());

        assert_eq!(cached.counts().tags, 1);
        assert_eq!(cached.counts().tags, 1, "a repaint reads the cache");
        assert_eq!(source.asked.get(), 1);

        cached.invalidate();
        assert_eq!(cached.counts().tags, 2, "the next read recounts");
        assert_eq!(source.asked.get(), 2);
    }
}
