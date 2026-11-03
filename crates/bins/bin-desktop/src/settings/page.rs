//! The page enum: which pages Settings has, their index order, and their label, scope note and id.

use crate::{assets::icon::DesktopIcon, view::event::Reveal};

/// A click on a row of a Settings page, which hands the page keyboard focus. Every page with rows
/// emits it, and the Settings View, which owns the focus, follows it.
#[derive(Debug, Clone, Copy)]
pub struct PageRowClicked;

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
            Self::ThisDevice => crate::msg::desktop_settings_group_this_device(),
            Self::ThisLedger => crate::msg::desktop_settings_group_this_ledger(),
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
        let count = i64::try_from(self.pages().count()).unwrap_or(i64::MAX);
        match self {
            Self::ThisDevice => crate::msg::desktop_settings_group_this_device_scope(count),
            Self::ThisLedger => crate::msg::desktop_settings_group_this_ledger_scope(count),
        }
    }

    /// The line under the heading on the index page, saying where the group's settings live.
    pub fn description(self) -> String {
        match self {
            Self::ThisDevice => crate::msg::desktop_settings_group_this_device_description(),
            Self::ThisLedger => crate::msg::desktop_settings_group_this_ledger_description(),
        }
    }
}

/// The pages of Settings, in the settings index's own row order. Settings is paged, not one
/// continuous scroll: each entry swaps the body to its own page. The index groups them by
/// [`ScopeGroup`], this device's first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
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

    /// The group the page sits in on the index. A Router decision, like [`Self::for_reveal`], so
    /// it stays with the page's identity rather than on a page (#933, ADR-0037).
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
            Self::General => crate::msg::desktop_settings_section_general(),
            Self::Display => crate::msg::desktop_settings_section_display(),
            Self::Ledger => crate::msg::desktop_settings_section_ledger(),
            Self::Units => crate::msg::desktop_settings_section_units(),
            Self::Institutions => crate::msg::desktop_settings_section_institutions(),
            Self::Accounts => lib_locale::msg::nav_accounts(),
            Self::Categories => lib_locale::msg::nav_categories(),
            Self::Tags => lib_locale::msg::nav_tags(),
            Self::Payees => lib_locale::msg::nav_payees(),
            Self::Documents => crate::msg::desktop_nav_documents(),
            Self::Inventory => crate::msg::desktop_settings_section_inventory(),
            Self::SyncServer => crate::msg::desktop_settings_section_sync_server(),
            Self::DataBackup => crate::msg::desktop_settings_section_data_backup(),
            Self::Tracing => crate::msg::desktop_settings_section_tracing(),
            Self::About => crate::msg::desktop_settings_section_about(),
        }
    }

    /// The icon beside the page's name on the Medium index page (DESKTOP-119). A page that was a
    /// rail noun keeps that noun's icon, so it reads the same wherever it shows; Institutions
    /// shares Loans' bank.
    pub fn icon(self) -> DesktopIcon {
        match self {
            Self::General => DesktopIcon::SettingsGeneral,
            Self::Display => DesktopIcon::SettingsDisplay,
            Self::Ledger => DesktopIcon::SettingsLedger,
            Self::Units => DesktopIcon::SettingsUnits,
            Self::Institutions => DesktopIcon::Loans,
            Self::Accounts => DesktopIcon::Accounts,
            Self::Categories => DesktopIcon::Categories,
            Self::Tags => DesktopIcon::Tags,
            Self::Payees => DesktopIcon::Payees,
            Self::Documents => DesktopIcon::Documents,
            Self::Inventory => DesktopIcon::Inventory,
            Self::SyncServer => DesktopIcon::SettingsSyncServer,
            Self::DataBackup => DesktopIcon::SettingsDataBackup,
            Self::Tracing => DesktopIcon::SettingsTracing,
            Self::About => DesktopIcon::SettingsAbout,
        }
    }

    /// The one-line description under the page's name on the Medium index page (DESKTOP-119).
    pub fn description(self) -> String {
        match self {
            Self::General => crate::msg::desktop_settings_description_general(),
            Self::Display => crate::msg::desktop_settings_description_display(),
            Self::Ledger => crate::msg::desktop_settings_description_ledger(),
            Self::Units => crate::msg::desktop_settings_description_units(),
            Self::Institutions => crate::msg::desktop_settings_description_institutions(),
            Self::Accounts => crate::msg::desktop_settings_description_accounts(),
            Self::Categories => crate::msg::desktop_settings_description_categories(),
            Self::Tags => crate::msg::desktop_settings_description_tags(),
            Self::Payees => crate::msg::desktop_settings_description_payees(),
            Self::Documents => crate::msg::desktop_settings_description_documents(),
            Self::Inventory => crate::msg::desktop_settings_description_inventory(),
            Self::SyncServer => crate::msg::desktop_settings_description_sync_server(),
            Self::DataBackup => crate::msg::desktop_settings_description_data_backup(),
            Self::Tracing => crate::msg::desktop_settings_description_tracing(),
            Self::About => crate::msg::desktop_settings_description_about(),
        }
    }

    /// The section heading's right-aligned scope note -- the Configuration-vs-Preferences
    /// distinction the README calls "the UI must make ... legible" (issue #63/#101/ADR-0014).
    pub fn scope_note(self) -> String {
        match self {
            Self::General | Self::Display => crate::msg::desktop_settings_scope_this_device(),
            Self::Ledger => crate::msg::desktop_settings_scope_ledger(),
            Self::Units | Self::Institutions => {
                crate::msg::desktop_settings_scope_synced_change_sets()
            }
            Self::Accounts
            | Self::Categories
            | Self::Tags
            | Self::Payees
            | Self::Documents
            | Self::Inventory => crate::msg::desktop_settings_scope_ledger_data(),
            Self::SyncServer => crate::msg::desktop_settings_scope_sync_server(30),
            Self::DataBackup => crate::msg::desktop_settings_scope_data_backup("aud", "2.84 mb"),
            Self::Tracing => crate::msg::desktop_settings_scope_tracing(
                i64::try_from(lib_tracing::LOG_CAPACITY).unwrap_or(i64::MAX),
            ),
            Self::About => crate::msg::desktop_settings_scope_about(),
        }
    }

    /// This section's position among [`Self::ALL`].
    #[expect(
        clippy::expect_used,
        reason = "ALL is a fixed-length array of every SettingsPage, and index_round_trips_every_section checks each one is present exactly once"
    )]
    pub fn index(self) -> usize {
        Self::ALL
            .iter()
            .position(|section| *section == self)
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
            .find(|section| section.id() == id)
            .unwrap_or_default()
    }

    /// The page that owns a revealed record, or `None` for a record another View owns. Which page
    /// shows a record is a Router decision, so it sits with the page's identity, not on a page.
    pub fn for_reveal(reveal: &Reveal) -> Option<Self> {
        match reveal {
            Reveal::Account(_) => Some(Self::Accounts),
            Reveal::Payee(_) => Some(Self::Payees),
            Reveal::BillEntry(_) | Reveal::BillPlan(_) | Reveal::BillsPeriod(_) => None,
        }
    }

    /// Whether this page holds anything the keyboard can move into. `l`/`enter` on the index is a
    /// quiet no-op for a page with nothing to focus (the keyboard-model decision).
    pub fn has_controls(self) -> bool {
        !matches!(self, Self::About)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn index_round_trips_every_section() {
        for (index, section) in SettingsPage::ALL.into_iter().enumerate() {
            assert_eq!(
                section.index(),
                index,
                "{section:?} is listed more than once"
            );
        }
    }

    #[test]
    fn every_section_id_round_trips_and_is_unique() {
        let ids: std::collections::HashSet<_> = SettingsPage::ALL
            .iter()
            .map(|section| section.id())
            .collect();
        assert_eq!(ids.len(), SettingsPage::ALL.len());
        for section in SettingsPage::ALL {
            assert_eq!(SettingsPage::from_id(section.id()), section);
        }
    }

    #[test]
    fn an_unknown_id_falls_back_to_general() {
        assert_eq!(SettingsPage::from_id("nope"), SettingsPage::General);
    }

    #[test]
    fn the_index_runs_in_the_handoffs_canonical_order() {
        let ids: Vec<_> = SettingsPage::ALL
            .iter()
            .map(|section| section.id())
            .collect();
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
        assert_eq!(SettingsPage::ALL[split], SettingsPage::Ledger);
    }

    #[test]
    fn each_group_counts_its_own_pages_in_its_scope_line() {
        crate::locale::init_for_tests();
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
    fn default_section_is_general() {
        assert_eq!(SettingsPage::default(), SettingsPage::General);
    }

    #[test]
    fn a_revealed_account_or_payee_opens_its_page_and_a_bills_record_opens_none() {
        assert_eq!(
            SettingsPage::for_reveal(&Reveal::Account(lib_core::AccountId::new())),
            Some(SettingsPage::Accounts)
        );
        assert_eq!(
            SettingsPage::for_reveal(&Reveal::Payee(lib_core::PayeeId::new())),
            Some(SettingsPage::Payees)
        );
        assert_eq!(
            SettingsPage::for_reveal(&Reveal::BillPlan(lib_core::BillPlanId::new())),
            None
        );
    }

    #[test]
    fn scope_notes_keep_the_configuration_and_preference_terms_and_pluralise() {
        crate::locale::init_for_tests();
        assert_eq!(
            SettingsPage::Display.scope_note(),
            "this device · never synced"
        );
        assert_eq!(SettingsPage::Units.scope_note(), "synced · change sets");
        assert_eq!(
            SettingsPage::SyncServer.scope_note(),
            "synced · every 30 seconds"
        );
        assert_eq!(
            crate::msg::desktop_settings_scope_units(1),
            "1 unit · synced"
        );
        assert_eq!(
            crate::msg::desktop_settings_scope_institutions(7),
            "7 institutions · synced"
        );
    }
}
