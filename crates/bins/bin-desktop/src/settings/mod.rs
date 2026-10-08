//! Pure Settings-surface domain types (`docs/ux/desktop-mockups/16-settings/README.md`'s "2a resting
//! state" -- "Sections" table) -- `gpui`-free, the same "pure state, chrome renders it" split
//! `navigation/nav.rs` uses between `NavState` and `Shell`'s render tree. `chrome::rail::settings_index::SettingsIndexRail`
//! and `view::settings` are the chrome; this module only knows what pages exist, their index
//! order, and their label/scope-note/search text.

pub mod display;
pub mod tracing_log;

use crate::{
    chrome::dialog_host::Dialog,
    form::field::TextField,
    institutions::form::AddInstitutionForm,
    units::form::{DeleteUnitForm, UnitForm},
};

/// The twelve pages of Settings, in the settings index rail's own row order. Settings is paged,
/// not one continuous scroll: each entry swaps the body to its own page. Accounts, Categories,
/// Tags and Payees joined the original eight when they left the primary rail.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SettingsSection {
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

/// Where keyboard focus sits within Settings: the index rail, or the page it has open. Only
/// meaningful while the View zone has focus on the Settings noun.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SettingsFocus {
    #[default]
    Index,
    Page,
}

/// The Display page's keyboard-reachable controls above the Colour Theme grid, top to bottom:
/// Date format, Row density, Status glyphs, Start sidebar minimised, Toasts. The grid is the stop
/// after the last one.
pub const DISPLAY_FIELD_COUNT: usize = 5;

/// The Display field that is a checkbox rather than a segmented or radio control.
pub const DISPLAY_FIELD_SIDEBAR: usize = 3;

/// `delta` steps along a control's choices, stopping at either end rather than wrapping (the
/// grid's own edge rule), for `h`/`l` on a segmented or radio control.
pub fn step_choice<T: Copy + PartialEq>(choices: &[T], current: T, delta: isize) -> T {
    let Some(position) = choices.iter().position(|choice| *choice == current) else {
        return current;
    };
    let last = choices.len() - 1;
    choices[position.saturating_add_signed(delta).min(last)]
}

impl SettingsSection {
    /// Whether the page fills the body's height and scrolls inside itself (Tracing's log box)
    /// rather than the body scrolling the whole page.
    pub fn fills_page(self) -> bool {
        self == Self::Tracing
    }

    /// Every page, in index-rail order (the handoff's canonical `General · Display · Units ·
    /// Institutions · Accounts · Categories · Tags · Payees · Documents · Inventory · Sync server · Data & backup ·
    /// Tracing (Logs) · About`).
    pub const ALL: [SettingsSection; 14] = [
        SettingsSection::General,
        SettingsSection::Display,
        SettingsSection::Units,
        SettingsSection::Institutions,
        SettingsSection::Accounts,
        SettingsSection::Categories,
        SettingsSection::Tags,
        SettingsSection::Payees,
        SettingsSection::Documents,
        SettingsSection::Inventory,
        SettingsSection::SyncServer,
        SettingsSection::DataBackup,
        SettingsSection::Tracing,
        SettingsSection::About,
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
        reason = "ALL is a fixed-length array of every SettingsSection, and index_round_trips_every_section checks each one is present exactly once"
    )]
    pub fn index(self) -> usize {
        Self::ALL
            .iter()
            .position(|section| *section == self)
            .expect("SettingsSection::ALL must list every SettingsSection")
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

/// Every Settings dialog `Shell` can have open, `None` when none is -- the README's own `State`
/// block (`dialog: Option<Dialog>`). One variant per dialog ticket; `EditUnit`/`DeleteUnit`'s own
/// `usize` is the row's index in `Shell::settings_units` -- `Shell::confirm_settings_dialog`
/// needs it to know which row to overwrite/remove.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SettingsDialog {
    AddUnit(UnitForm),
    EditUnit(usize, UnitForm),
    DeleteUnit(usize, DeleteUnitForm),
    AddInstitution(AddInstitutionForm),
    /// Tracing's Clear logs confirm: nothing to fill in.
    ClearLogs,
}

impl Dialog for SettingsDialog {
    fn focused_text(&mut self) -> Option<&mut TextField> {
        match self {
            Self::AddUnit(form) | Self::EditUnit(_, form) => Some(form.focused_mut()),
            Self::DeleteUnit(_, form) => Some(&mut form.confirm_input),
            Self::AddInstitution(form) => Some(&mut form.name),
            Self::ClearLogs => None,
        }
    }

    fn cycle_field(&mut self) {
        if let Self::AddUnit(form) | Self::EditUnit(_, form) = self {
            form.cycle_field();
        }
    }

    fn is_valid(&self) -> bool {
        match self {
            Self::AddUnit(form) | Self::EditUnit(_, form) => form.is_valid(),
            Self::DeleteUnit(_, form) => form.is_valid(),
            Self::AddInstitution(form) => form.is_valid(),
            Self::ClearLogs => true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn step_choice_stops_at_both_ends() {
        let choices = [1, 2, 3];
        assert_eq!(step_choice(&choices, 2, 1), 3);
        assert_eq!(step_choice(&choices, 3, 1), 3);
        assert_eq!(step_choice(&choices, 1, -1), 1);
        assert_eq!(step_choice(&choices, 9, 1), 9);
    }

    #[test]
    fn index_round_trips_every_section() {
        for (index, section) in SettingsSection::ALL.into_iter().enumerate() {
            assert_eq!(
                section.index(),
                index,
                "{section:?} is listed more than once"
            );
        }
    }

    #[test]
    fn every_section_id_round_trips_and_is_unique() {
        let ids: std::collections::HashSet<_> = SettingsSection::ALL
            .iter()
            .map(|section| section.id())
            .collect();
        assert_eq!(ids.len(), SettingsSection::ALL.len());
        for section in SettingsSection::ALL {
            assert_eq!(SettingsSection::from_id(section.id()), section);
        }
    }

    #[test]
    fn an_unknown_id_falls_back_to_general() {
        assert_eq!(SettingsSection::from_id("nope"), SettingsSection::General);
    }

    #[test]
    fn the_index_runs_in_the_handoffs_canonical_order() {
        let ids: Vec<_> = SettingsSection::ALL
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

    #[test]
    fn scope_notes_keep_the_configuration_and_preference_terms_and_pluralise() {
        crate::locale::init_for_tests();
        assert_eq!(
            SettingsSection::Display.scope_note(),
            "client-scoped · never synced"
        );
        assert_eq!(SettingsSection::Units.scope_note(), "synced · change sets");
        assert_eq!(
            SettingsSection::SyncServer.scope_note(),
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
