//! Pure Settings-surface domain types (`docs/ux/desktop/Settings/README.md`'s "2a resting
//! state" -- "Sections, in scroll order" table) -- `gpui`-free, the same "pure state, chrome
//! renders it" split `nav.rs` uses between `NavState` and `Shell`'s render tree.
//! `rail::settings_index::SettingsIndexRail` and `view::settings` are the chrome; this module
//! only knows what sections exist, their scroll order, and their label/scope-note/filter text.

/// The nine sections of the Settings body, in scroll order -- also the settings index rail's
/// own row order (`docs/ux/desktop/Settings/README.md`'s "Sections, in scroll order" table).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SettingsSection {
    #[default]
    General,
    LedgerUnits,
    Units,
    Institutions,
    Display,
    SyncServer,
    DataBackup,
    Tracing,
    About,
}

impl SettingsSection {
    /// Every section, in scroll/index-rail order.
    pub const ALL: [SettingsSection; 9] = [
        SettingsSection::General,
        SettingsSection::LedgerUnits,
        SettingsSection::Units,
        SettingsSection::Institutions,
        SettingsSection::Display,
        SettingsSection::SyncServer,
        SettingsSection::DataBackup,
        SettingsSection::Tracing,
        SettingsSection::About,
    ];

    /// The index-rail label / section heading text.
    pub fn label(self) -> &'static str {
        match self {
            Self::General => "General",
            Self::LedgerUnits => "Ledger & units",
            Self::Units => "Units",
            Self::Institutions => "Institutions",
            Self::Display => "Display",
            Self::SyncServer => "Sync server",
            Self::DataBackup => "Data & backup",
            Self::Tracing => "Tracing (Logs)",
            Self::About => "About",
        }
    }

    /// The section heading's right-aligned scope note -- the Configuration-vs-Preferences
    /// distinction the README calls "the UI must make ... legible" (issue #63/#101/ADR-0014).
    pub fn scope_note(self) -> &'static str {
        match self {
            Self::General => "ledger identity",
            Self::LedgerUnits => "synced · change sets",
            Self::Units => "synced · change sets",
            Self::Institutions => "synced · change sets",
            Self::Display => "client-scoped · never synced",
            Self::SyncServer => "synced · every 30 seconds",
            Self::DataBackup => "local files · aud · 2.84 mb",
            Self::Tracing => "diagnostic · last 1000 entries",
            Self::About => "version info",
        }
    }

    /// The Desktop Settings Surface ticket that builds this section's real content -- this
    /// scaffold (issue #173) only lays out the frame, so every section is a placeholder until
    /// its own ticket lands.
    pub fn placeholder_issue(self) -> u32 {
        match self {
            Self::General => 175,
            Self::LedgerUnits => 176,
            Self::Units => 177,
            Self::Institutions => 178,
            Self::Display => 179,
            Self::SyncServer => 180,
            Self::DataBackup => 181,
            Self::Tracing => 182,
            Self::About => 183,
        }
    }

    /// This section's position among [`Self::ALL`].
    pub fn index(self) -> usize {
        Self::ALL
            .iter()
            .position(|section| *section == self)
            .expect("SettingsSection::ALL must list every SettingsSection")
    }

    /// The scroll body's own child index for this section (`gpui::ScrollHandle::scroll_to_top_of_item`
    /// operates on direct children): the page heading block occupies child `0`, so every
    /// section sits one past its [`Self::index`] -- see `view::settings::render`.
    pub fn body_child_index(self) -> usize {
        self.index() + 1
    }

    /// Whether this section's label contains `needle`, case-insensitively -- the index rail's
    /// own `/ filter` (`docs/ux/desktop/Settings/README.md`'s "Navigation" bullet: "live
    /// substring filter over the index entries only"). An empty needle matches everything, the
    /// resting (unfiltered) state.
    pub fn matches_filter(self, needle: &str) -> bool {
        needle.is_empty() || self.label().to_lowercase().contains(&needle.to_lowercase())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_section_has_a_unique_body_child_index_after_the_heading() {
        let mut indices: Vec<_> = SettingsSection::ALL
            .iter()
            .map(|section| section.body_child_index())
            .collect();
        indices.sort_unstable();
        assert_eq!(indices, vec![1, 2, 3, 4, 5, 6, 7, 8, 9]);
    }

    #[test]
    fn empty_filter_matches_every_section() {
        for section in SettingsSection::ALL {
            assert!(section.matches_filter(""));
        }
    }

    #[test]
    fn filter_matches_case_insensitively_and_by_substring() {
        assert!(SettingsSection::SyncServer.matches_filter("sync"));
        assert!(SettingsSection::SyncServer.matches_filter("SYNC"));
        assert!(!SettingsSection::SyncServer.matches_filter("units"));
    }

    #[test]
    fn default_section_is_general() {
        assert_eq!(SettingsSection::default(), SettingsSection::General);
    }

    #[test]
    fn every_section_has_a_distinct_placeholder_issue() {
        let issues: std::collections::HashSet<_> = SettingsSection::ALL
            .iter()
            .map(|section| section.placeholder_issue())
            .collect();
        assert_eq!(issues.len(), SettingsSection::ALL.len());
    }
}
