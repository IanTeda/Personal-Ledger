//! The Settings pages: [`SettingsPages`], every page's Entity. Every page is a
//! [`ModulePage`](crate::shell::view::module_page::ModulePage) (#1100), and
//! [`SettingsPageHandle`], the one hosting trait `Root` reaches the active View through (#1101)
//! with the Settings body's own verbs on top (#1129), is its surface over the `Entity`, so
//! [`SettingsPages::get`] hands the Settings View the open page as one `dyn` handle and `get` is the
//! only match on the page. Each page is framed in the Window Size as it is built and as the Window
//! Size changes, and nothing else: the Window Size is the whole frame, and Display measures its own
//! Colour Theme grid as it paints (#1222).

use gpui::{App, AppContext as _, Context, Entity, Subscription};

use std::cell::RefCell;
use std::rc::Rc;

use lib_settings::{DeviceConfiguration, DeviceSnapshot, LedgerCounts, LedgerSummaries};
use lib_viper::keymap::KeyBindingConfig;

use crate::{
    accounts::{AccountsPage, accounts_page},
    announcer::Announcer,
    categories::{CategoriesPage, categories_page},
    document_types::{DocumentTypesPage, document_types_page},
    institutions::{InstitutionsPage, institutions_page},
    inventory::{InventoryPage, inventory_page},
    payees::{PayeesPage, payees_page},
    settings::{
        SettingsPage,
        about::{AboutPage, about_page},
        data_backup::{DataBackupPage, data_backup_page},
        display::{DisplayPage, display_page},
        general::{GeneralPage, general_page},
        ledger::{LedgerPage, ledger_page},
        state::SettingsView,
        sync_server::{SyncServerPage, sync_server_page},
        tracing::{
            TracingPage,
            log::{CaptureLog, LogFeed},
            tracing_page,
        },
    },
    shell::view::{
        module_page::PageCue, page_handle::SettingsPageHandle, page_router::Built,
        window_size::WindowSize,
    },
    tags::{TagsPage, tags_page},
    units::{UnitsPage, units_page},
};
use lib_accounts::presenter::AccountsPagePresenter;
use lib_categories::presenter::CategoriesPagePresenter;
use lib_documents::presenter::DocumentTypesPagePresenter;
use lib_institutions::presenter::InstitutionsPagePresenter;
use lib_inventory::presenter::InventoryPagePresenter;
use lib_payees::presenter::PayeesPagePresenter;
use lib_tags::presenter::TagsPagePresenter;
use lib_units::presenter::UnitsPagePresenter;

/// A click on a row of a Settings page, which hands the page keyboard focus. Every page with rows
/// emits it, and the Settings View, which owns the focus, follows it.
#[derive(Debug, Clone, Copy)]
pub struct PageRowClicked;

/// What `shell::router::build` hands the This Ledger pages: each domain page's Presenter, which
/// owns its keymap, beside its Setup ([`Built`]), and the Ledger page's reads.
pub struct SettingsSetup {
    pub units: Built<UnitsPagePresenter>,
    pub institutions: Built<InstitutionsPagePresenter>,
    pub payees: Built<PayeesPagePresenter>,
    pub document_types: Built<DocumentTypesPagePresenter>,
    pub accounts: Built<AccountsPagePresenter>,
    pub categories: Built<CategoriesPagePresenter>,
    pub tags: Built<TagsPagePresenter>,
    pub inventory: Built<InventoryPagePresenter>,
    pub ledger: lib_settings::SettingsLedger,
}

/// The Entity of every Settings page. Settings builds them and `Root` reaches them only through
/// [`SettingsView::page`], for a module Router's entry point and the test snapshots. The This
/// device pages stand with no Ledger open; the This Ledger pages are built on one, so they are
/// `None` until a Ledger opens (#977).
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
    /// The This Ledger pages, the Ledger and info pages' keys built from `config`.
    pub fn new(
        announcer: &Entity<Announcer>,
        config: &KeyBindingConfig,
        setup: SettingsSetup,
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
            ledger,
        } = setup;
        let summaries = CachedCounts::new(ledger.summaries.clone());
        // Every domain's write can change a count, so any announcement drops them.
        let _ledger_changed = cx.subscribe(announcer, |view: &mut SettingsView, _, _, cx| {
            view.ledger_changed(cx);
        });
        Self {
            summaries,
            _ledger_changed,
            ledger: cx.new(|cx| ledger_page(ledger, config, announcer, cx)),
            units: units_page(units.0, units.1, announcer.clone(), cx),
            institutions: institutions_page(institutions.0, institutions.1, announcer.clone(), cx),
            accounts: accounts_page(accounts.0, accounts.1, announcer.clone(), cx),
            categories: categories_page(categories.0, categories.1, announcer.clone(), cx),
            tags: tags_page(tags.0, tags.1, announcer.clone(), cx),
            payees: payees_page(payees.0, payees.1, announcer.clone(), cx),
            documents: document_types_page(
                document_types.0,
                document_types.1,
                announcer.clone(),
                cx,
            ),
            inventory: inventory_page(inventory.0, inventory.1, announcer.clone(), cx),
            sync_server: cx.new(|cx| sync_server_page(config, announcer, cx)),
            data_backup: cx.new(|cx| data_backup_page(config, announcer, cx)),
        }
    }
}

impl SettingsPages {
    /// The This device pages, with no Ledger open, General and Display writing to `device` and
    /// showing `snapshot`, what it read as the window opened, their keys built from `config`, each
    /// framed in `window_size`.
    pub fn new(
        device: Rc<dyn DeviceConfiguration>,
        snapshot: &DeviceSnapshot,
        config: &KeyBindingConfig,
        window_size: WindowSize,
        cx: &mut Context<'_, SettingsView>,
    ) -> Self {
        let log_feed = cx.new(|_| LogFeed);
        let pages = Self {
            general: cx.new(|cx| general_page(device.clone(), snapshot.clone(), config, cx)),
            display: cx.new(|cx| display_page(device, snapshot.clone(), config, cx)),
            // A private, empty capture until `set_log_capture` hands over the real one.
            tracing: cx.new(|cx| {
                tracing_page(
                    Rc::new(CaptureLog::new(lib_tracing::LogBuffer::new(
                        lib_tracing::LOG_CAPACITY,
                    ))),
                    lib_settings::TracingLevel::default(),
                    config,
                    log_feed.clone(),
                    cx,
                )
            }),
            log_feed,
            about: cx.new(|cx| about_page(env!("CARGO_PKG_VERSION"), config, cx)),
            ledger: None,
        };
        pages.frame(window_size, cx);
        pages
    }

    /// Adds the This Ledger pages built on an Open Ledger, in place of any built before, each
    /// framed in `window_size`, so a palette Command reaching one before its first draw sees it.
    pub fn open_ledger(&mut self, pages: LedgerPages, window_size: WindowSize, cx: &mut App) {
        self.ledger = Some(pages);
        self.frame(window_size, cx);
    }

    /// Tells every page built the Window Size it is drawn at.
    pub fn frame(&self, window_size: WindowSize, cx: &mut App) {
        for page in self.all() {
            page.cue(PageCue::Framed(window_size), cx);
        }
    }

    /// The page's handle, `None` for a This Ledger page with no Ledger open. The one match on the
    /// page: a new page fails to compile here.
    pub fn get(&self, page: SettingsPage) -> Option<&dyn SettingsPageHandle> {
        let ledger = self.ledger.as_ref();
        match page {
            SettingsPage::General => Some(&self.general),
            SettingsPage::Display => Some(&self.display),
            SettingsPage::Tracing => Some(&self.tracing),
            SettingsPage::About => Some(&self.about),
            SettingsPage::Ledger => ledger.map(|pages| &pages.ledger as &dyn SettingsPageHandle),
            SettingsPage::Units => ledger.map(|pages| &pages.units as &dyn SettingsPageHandle),
            SettingsPage::Institutions => {
                ledger.map(|pages| &pages.institutions as &dyn SettingsPageHandle)
            }
            SettingsPage::Accounts => {
                ledger.map(|pages| &pages.accounts as &dyn SettingsPageHandle)
            }
            SettingsPage::Categories => {
                ledger.map(|pages| &pages.categories as &dyn SettingsPageHandle)
            }
            SettingsPage::Tags => ledger.map(|pages| &pages.tags as &dyn SettingsPageHandle),
            SettingsPage::Payees => ledger.map(|pages| &pages.payees as &dyn SettingsPageHandle),
            SettingsPage::Documents => {
                ledger.map(|pages| &pages.documents as &dyn SettingsPageHandle)
            }
            SettingsPage::Inventory => {
                ledger.map(|pages| &pages.inventory as &dyn SettingsPageHandle)
            }
            SettingsPage::SyncServer => {
                ledger.map(|pages| &pages.sync_server as &dyn SettingsPageHandle)
            }
            SettingsPage::DataBackup => {
                ledger.map(|pages| &pages.data_backup as &dyn SettingsPageHandle)
            }
        }
    }

    /// Every page built, with which page it is, in index order.
    pub fn each(&self) -> impl Iterator<Item = (SettingsPage, &dyn SettingsPageHandle)> {
        SettingsPage::ALL
            .into_iter()
            .filter_map(|page| self.get(page).map(|handle| (page, handle)))
    }

    /// Every page built, in index order.
    pub fn all(&self) -> impl Iterator<Item = &dyn SettingsPageHandle> {
        self.each().map(|(_, handle)| handle)
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
