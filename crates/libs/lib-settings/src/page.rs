//! Which pages Settings has: their index order, their This device and This Ledger grouping, and
//! their name, description, scope note and persisted id. Lifted from the Desktop unchanged but for
//! what is the toolkit's or another View's: a page's icon stays in the bin (ADR-0037, 2026-10-30),
//! and so does which page shows a revealed record, since `Reveal` is the Desktop's.

use lib_viper::view::count;

use crate::msg;

/// Which of Settings' two groups a page sits in (ADR-0051, `GLOSSARY.md` **Preference**): what
/// belongs to this device and works with no Ledger open, or what is stored in the Open Ledger or
/// acts on its file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScopeGroup {
    ThisDevice,
    ThisLedger,
}

impl ScopeGroup {
    /// The groups, in index order.
    pub const ALL: [ScopeGroup; 2] = [Self::ThisDevice, Self::ThisLedger];

    /// The group's heading.
    pub fn label(self) -> String {
        match self {
            Self::ThisDevice => msg::settings_group_this_device(),
            Self::ThisLedger => msg::settings_group_this_ledger(),
        }
    }

    /// The group's pages, in index order.
    pub fn pages(self) -> impl Iterator<Item = SettingsPage> {
        SettingsPage::ALL
            .into_iter()
            .filter(move |page| page.scope_group() == self)
    }

    /// The right-aligned line beside the heading on the index page: its page count and whether
    /// it syncs.
    pub fn scope_line(self) -> String {
        let pages = count(self.pages().count());
        match self {
            Self::ThisDevice => msg::settings_group_this_device_scope(pages),
            Self::ThisLedger => msg::settings_group_this_ledger_scope(pages),
        }
    }

    /// The line under the heading on the index page, saying where the group's settings live.
    pub fn description(self) -> String {
        match self {
            Self::ThisDevice => msg::settings_group_this_device_description(),
            Self::ThisLedger => msg::settings_group_this_ledger_description(),
        }
    }
}

/// What a page's scope note reads that Settings doesn't own: how many entries the log capture
/// keeps is `lib-tracing`'s, which this crate may not depend on, so the Client hands it in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScopeFacts {
    pub log_capacity: usize,
}

/// The pages of Settings, in the settings index's own row order. Settings is paged, not one
/// continuous scroll: each entry swaps the body to its own page. The index groups them by
/// [`ScopeGroup`], this device's first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum SettingsPage {
    #[default]
    General,
    Display,
    Ledger,
    Units,
    Institutions,
    Accounts,
    Categories,
    Tags,
    Payees,
    Documents,
    Inventory,
    SyncServer,
    DataBackup,
    Tracing,
    About,
}

impl SettingsPage {
    /// Every page, in index order: the mockups' THIS DEVICE group (`General · Display · Logs ·
    /// About`), then THIS LEDGER (`Ledger · Units · Institutions · Accounts · Categories · Tags ·
    /// Payees · Documents · Inventory · Sync server · Backup`). The mockups' AI Connections and
    /// History have no page yet.
    pub const ALL: [SettingsPage; 15] = [
        SettingsPage::General,
        SettingsPage::Display,
        SettingsPage::Tracing,
        SettingsPage::About,
        SettingsPage::Ledger,
        SettingsPage::Units,
        SettingsPage::Institutions,
        SettingsPage::Accounts,
        SettingsPage::Categories,
        SettingsPage::Tags,
        SettingsPage::Payees,
        SettingsPage::Documents,
        SettingsPage::Inventory,
        SettingsPage::SyncServer,
        SettingsPage::DataBackup,
    ];

    /// The group the page sits in on the index. It stays with the page's identity rather than on
    /// a page (#933, ADR-0037).
    pub fn scope_group(self) -> ScopeGroup {
        match self {
            Self::General | Self::Display | Self::Tracing | Self::About => ScopeGroup::ThisDevice,
            Self::Ledger
            | Self::Units
            | Self::Institutions
            | Self::Accounts
            | Self::Categories
            | Self::Tags
            | Self::Payees
            | Self::Documents
            | Self::Inventory
            | Self::SyncServer
            | Self::DataBackup => ScopeGroup::ThisLedger,
        }
    }

    /// Whether the page is built on a Ledger, so it is shown muted and refused with none open.
    pub fn needs_ledger(self) -> bool {
        self.scope_group() == ScopeGroup::ThisLedger
    }

    /// The index-rail label / section heading text.
    pub fn label(self) -> String {
        match self {
            Self::General => msg::settings_section_general(),
            Self::Display => msg::settings_section_display(),
            Self::Ledger => msg::settings_section_ledger(),
            Self::Units => msg::settings_section_units(),
            Self::Institutions => msg::settings_section_institutions(),
            Self::Accounts => msg::settings_section_accounts(),
            Self::Categories => msg::settings_section_categories(),
            Self::Tags => msg::settings_section_tags(),
            Self::Payees => msg::settings_section_payees(),
            Self::Documents => msg::settings_section_documents(),
            Self::Inventory => msg::settings_section_inventory(),
            Self::SyncServer => msg::settings_section_sync_server(),
            Self::DataBackup => msg::settings_section_data_backup(),
            Self::Tracing => msg::settings_section_tracing(),
            Self::About => msg::settings_section_about(),
        }
    }

    /// The one-line description under the page's name on the Medium index page (DESKTOP-119).
    pub fn description(self) -> String {
        match self {
            Self::General => msg::settings_description_general(),
            Self::Display => msg::settings_description_display(),
            Self::Ledger => msg::settings_description_ledger(),
            Self::Units => msg::settings_description_units(),
            Self::Institutions => msg::settings_description_institutions(),
            Self::Accounts => msg::settings_description_accounts(),
            Self::Categories => msg::settings_description_categories(),
            Self::Tags => msg::settings_description_tags(),
            Self::Payees => msg::settings_description_payees(),
            Self::Documents => msg::settings_description_documents(),
            Self::Inventory => msg::settings_description_inventory(),
            Self::SyncServer => msg::settings_description_sync_server(),
            Self::DataBackup => msg::settings_description_data_backup(),
            Self::Tracing => msg::settings_description_tracing(),
            Self::About => msg::settings_description_about(),
        }
    }

    /// The section heading's right-aligned scope note: the Configuration-vs-Preferences
    /// distinction the README says the UI must make legible (#63, #101, ADR-0014).
    pub fn scope_note(self, facts: ScopeFacts) -> String {
        match self {
            Self::General | Self::Display => msg::settings_scope_this_device(),
            Self::Ledger => msg::settings_scope_ledger(),
            Self::Units | Self::Institutions => msg::settings_scope_synced_change_sets(),
            Self::Accounts
            | Self::Categories
            | Self::Tags
            | Self::Payees
            | Self::Documents
            | Self::Inventory => msg::settings_scope_ledger_data(),
            Self::SyncServer => msg::settings_scope_sync_server(),
            Self::DataBackup => msg::settings_scope_data_backup(),
            Self::Tracing => msg::settings_scope_tracing(count(facts.log_capacity)),
            Self::About => msg::settings_scope_about(),
        }
    }

    /// This page's position among [`Self::ALL`].
    #[expect(
        clippy::expect_used,
        reason = "ALL is a fixed-length array of every SettingsPage, and index_round_trips_every_page checks each one is present exactly once"
    )]
    pub fn index(self) -> usize {
        Self::ALL
            .iter()
            .position(|page| *page == self)
            .expect("SettingsPage::ALL must list every SettingsPage")
    }

    /// A stable id for persistence, never translated and never reused: a renamed label must not
    /// orphan a saved last-visited page.
    pub fn id(self) -> &'static str {
        match self {
            Self::General => "general",
            Self::Display => "display",
            Self::Ledger => "ledger",
            Self::Units => "units",
            Self::Institutions => "institutions",
            Self::Accounts => "accounts",
            Self::Categories => "categories",
            Self::Tags => "tags",
            Self::Payees => "payees",
            Self::Documents => "documents",
            Self::Inventory => "inventory",
            Self::SyncServer => "sync-server",
            Self::DataBackup => "data-backup",
            Self::Tracing => "tracing",
            Self::About => "about",
        }
    }

    /// The page for a persisted id. An unknown id (a page since removed, a hand-edited file)
    /// falls back to the default page rather than failing the load.
    pub fn from_id(id: &str) -> Self {
        Self::ALL
            .into_iter()
            .find(|page| page.id() == id)
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn locale() {
        lib_locale::init_with_layers("en-US", &[msg::LAYER]);
    }

    #[test]
    fn index_round_trips_every_page() {
        for (index, page) in SettingsPage::ALL.into_iter().enumerate() {
            assert_eq!(page.index(), index, "{page:?} is listed more than once");
        }
    }

    #[test]
    fn every_page_id_round_trips_and_is_unique() {
        let ids: std::collections::HashSet<_> =
            SettingsPage::ALL.iter().map(|page| page.id()).collect();
        assert_eq!(ids.len(), SettingsPage::ALL.len());
        for page in SettingsPage::ALL {
            assert_eq!(SettingsPage::from_id(page.id()), page);
        }
    }

    #[test]
    fn an_unknown_id_falls_back_to_general() {
        assert_eq!(SettingsPage::from_id("nope"), SettingsPage::General);
        assert_eq!(SettingsPage::default(), SettingsPage::General);
    }

    #[test]
    fn the_index_runs_in_the_handoffs_canonical_order() {
        let ids: Vec<_> = SettingsPage::ALL.iter().map(|page| page.id()).collect();
        assert_eq!(
            ids,
            [
                "general",
                "display",
                "tracing",
                "about",
                "ledger",
                "units",
                "institutions",
                "accounts",
                "categories",
                "tags",
                "payees",
                "documents",
                "inventory",
                "sync-server",
                "data-backup",
            ]
        );
    }

    #[test]
    fn this_device_holds_the_first_pages_and_this_ledger_the_rest() {
        let groups: Vec<_> = SettingsPage::ALL
            .iter()
            .map(|page| page.scope_group())
            .collect();
        let split = groups
            .iter()
            .position(|group| *group == ScopeGroup::ThisLedger)
            .expect("THIS LEDGER has pages");
        assert_eq!(split, 4);
        assert!(groups[..split].iter().all(|g| *g == ScopeGroup::ThisDevice));
        assert!(groups[split..].iter().all(|g| *g == ScopeGroup::ThisLedger));
        assert!(SettingsPage::ALL[split..].iter().all(|p| p.needs_ledger()));
        assert!(!SettingsPage::ALL[..split].iter().any(|p| p.needs_ledger()));
    }

    #[test]
    fn each_group_counts_its_own_pages_in_its_scope_line() {
        locale();
        assert_eq!(
            ScopeGroup::ThisDevice.scope_line(),
            "4 pages · never synced"
        );
        assert_eq!(
            ScopeGroup::ThisLedger.scope_line(),
            "11 pages · synced with the Ledger"
        );
    }

    #[test]
    fn scope_notes_keep_the_configuration_and_preference_terms_and_pluralise() {
        locale();
        let facts = ScopeFacts { log_capacity: 500 };
        assert_eq!(
            SettingsPage::Display.scope_note(facts),
            "this device · never synced"
        );
        assert_eq!(
            SettingsPage::Units.scope_note(facts),
            "synced · change sets"
        );
        assert_eq!(
            SettingsPage::SyncServer.scope_note(facts),
            "synced",
            "no interval until the Sync server has one"
        );
        assert_eq!(
            SettingsPage::DataBackup.scope_note(facts),
            "local files",
            "no size until Data & backup reads one"
        );
        assert_eq!(
            SettingsPage::Tracing.scope_note(ScopeFacts { log_capacity: 1 }),
            "this device · diagnostic · last 1 entry"
        );
    }
}
