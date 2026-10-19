//! The page enum: which pages Settings has, their index order, and their label, scope note and id.

use crate::assets::icon::DesktopIcon;

/// A click on a row of a Settings page, which hands the page keyboard focus. Every page with rows
/// emits it, and the Settings View, which owns the focus, follows it.
#[derive(Debug, Clone, Copy)]
pub struct PageRowClicked;

/// The twelve pages of Settings, in the settings index rail's own row order. Settings is paged,
/// not one continuous scroll: each entry swaps the body to its own page. Accounts, Categories,
/// Tags and Payees joined the original eight when they left the primary rail.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SettingsPage {
    #[default]
    General,
    Display,
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
    /// Whether the page fills the body's height and scrolls inside itself (Tracing's log box)
    /// rather than the body scrolling the whole page.
    pub fn fills_page(self) -> bool {
        self == Self::Tracing
    }

    /// Every page, in index-rail order (the handoff's canonical `General · Display · Units ·
    /// Institutions · Accounts · Categories · Tags · Payees · Documents · Inventory · Sync server · Data & backup ·
    /// Tracing (Logs) · About`).
    pub const ALL: [SettingsPage; 14] = [
        SettingsPage::General,
        SettingsPage::Display,
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
        SettingsPage::Tracing,
        SettingsPage::About,
    ];

    /// The index-rail label / section heading text.
    pub fn label(self) -> String {
        match self {
            Self::General => crate::msg::desktop_settings_section_general(),
            Self::Display => crate::msg::desktop_settings_section_display(),
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
            Self::General => crate::msg::desktop_settings_scope_general(),
            Self::Units | Self::Institutions => {
                crate::msg::desktop_settings_scope_synced_change_sets()
            }
            Self::Accounts
            | Self::Categories
            | Self::Tags
            | Self::Payees
            | Self::Documents
            | Self::Inventory => crate::msg::desktop_settings_scope_ledger_data(),
            Self::Display => crate::msg::desktop_settings_scope_display(),
            Self::SyncServer => crate::msg::desktop_settings_scope_sync_server(30),
            Self::DataBackup => crate::msg::desktop_settings_scope_data_backup("aud", "2.84 mb"),
            Self::Tracing => crate::msg::desktop_settings_scope_tracing(
                i64::try_from(lib_tracing::LOG_CAPACITY).unwrap_or(i64::MAX),
            ),
            Self::About => crate::msg::desktop_settings_scope_about(),
        }
    }

    /// The Desktop Settings Surface ticket that builds this section's real content -- this
    /// scaffold (issue #173) only lays out the frame, so every section is a placeholder until
    /// its own ticket lands. `LedgerUnits`'s own #176 is gone along with the section it built
    /// (issue #189).
    pub fn placeholder_issue(self) -> u32 {
        match self {
            Self::General => 175,
            Self::Units => 177,
            Self::Institutions => 178,
            Self::Accounts => 416,
            Self::Categories => 417,
            Self::Tags => 418,
            Self::Payees => 419,
            Self::Documents => 480,
            Self::Inventory => 493,
            Self::Display => 179,
            Self::SyncServer => 180,
            Self::DataBackup => 181,
            Self::Tracing => 182,
            Self::About => 183,
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
                "tracing",
                "about",
            ]
        );
    }

    #[test]
    fn default_section_is_general() {
        assert_eq!(SettingsPage::default(), SettingsPage::General);
    }

    #[test]
    fn every_section_has_a_distinct_placeholder_issue() {
        let issues: std::collections::HashSet<_> = SettingsPage::ALL
            .iter()
            .map(|section| section.placeholder_issue())
            .collect();
        assert_eq!(issues.len(), SettingsPage::ALL.len());
    }

    #[test]
    fn scope_notes_keep_the_configuration_and_preference_terms_and_pluralise() {
        crate::locale::init_for_tests();
        assert_eq!(
            SettingsPage::Display.scope_note(),
            "client-scoped · never synced"
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
