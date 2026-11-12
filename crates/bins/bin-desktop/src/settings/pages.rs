//! The Settings pages: [`SettingsPages`], every page's Entity, and [`PageFrame`], the space they are
//! drawn in. Every page is a [`ModulePage`](crate::shell::view::module_page::ModulePage) (#1100), and [`PageHandle`], the one hosting trait
//! `Root` reaches the active View through too (#1101), is its surface over the `Entity`, so
//! [`SettingsPages::get`] hands the Settings View the open page as one `dyn` handle and `get` is the
//! only match on the page.

use gpui::{AppContext as _, Context, Entity, Subscription};

use std::cell::RefCell;
use std::rc::Rc;

use lib_settings::{DeviceConfiguration, LedgerCounts, LedgerSummaries};
use lib_viper::keymap::KeyBindingConfig;

use crate::{
    accounts::{AccountsPage, AccountsSetup, accounts_page},
    announcer::Announcer,
    categories::{CategoriesPage, CategoriesSetup, categories_page},
    document_types::{DocumentTypesPage, DocumentTypesSetup, document_types_page},
    institutions::{InstitutionsPage, InstitutionsSetup, institutions_page},
    inventory::{InventoryPage, InventorySetup, inventory_page},
    payees::{PayeesPage, PayeesSetup, payees_page},
    settings::{
        SettingsPage,
        about::{AboutPage, about_page},
        data_backup::{DataBackupPage, data_backup_page},
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
    shell::view::{page_handle::PageHandle, window_size::WindowSize},
    tags::{TagsPage, TagsSetup, tags_page},
    units::{UnitsPage, UnitsSetup, units_page},
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
    /// The This Ledger pages, their keys built from `setup.ledger`'s config.
    pub fn new(
        announcer: &Entity<Announcer>,
        clock: &lib_core::Clock,
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
            ledger: ledger_keys,
        } = setup;
        let config = ledger_keys.config.clone();
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
            sync_server: cx.new(|cx| sync_server_page(&config, announcer, cx)),
            data_backup: cx.new(|cx| data_backup_page(&config, announcer, cx)),
        }
    }
}

impl SettingsPages {
    /// The This device pages, with no Ledger open, General and Display on `device`, each reading
    /// it once.
    pub fn new(device: Rc<dyn DeviceConfiguration>, cx: &mut Context<'_, SettingsView>) -> Self {
        let log_feed = cx.new(|_| LogFeed);
        Self {
            general: cx.new(|cx| general_page(device.clone(), &KeyBindingConfig::default(), cx)),
            display: cx.new(|cx| display_page(device, &KeyBindingConfig::default(), cx)),
            // A private, empty capture until `set_log_capture` hands over the real one.
            tracing: cx.new(|cx| {
                tracing_page(
                    Rc::new(CaptureLog::new(lib_tracing::LogBuffer::new(
                        lib_tracing::LOG_CAPACITY,
                    ))),
                    lib_settings::TracingLevel::default(),
                    &KeyBindingConfig::default(),
                    log_feed.clone(),
                    cx,
                )
            }),
            log_feed,
            about: cx
                .new(|cx| about_page(env!("CARGO_PKG_VERSION"), &KeyBindingConfig::default(), cx)),
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
