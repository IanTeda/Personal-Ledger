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

/// The **Ledger & units** section's "Default unit for new entries" segmented control
/// (`docs/ux/desktop/Settings/README.md`'s "2a resting state" markup -- the shared design
/// system's `.seg`/`.seg-opt` classes, not the "Field label"/`Input`/`select` pair General's
/// own fields use). `aud` is the mockup's own `checked` option.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DefaultUnit {
    #[default]
    Aud,
    Btc,
    Vas,
}

impl DefaultUnit {
    pub const ALL: [DefaultUnit; 3] = [DefaultUnit::Aud, DefaultUnit::Btc, DefaultUnit::Vas];

    pub fn label(self) -> &'static str {
        match self {
            Self::Aud => "aud",
            Self::Btc => "btc",
            Self::Vas => "vas",
        }
    }
}

/// The same section's "Budget period" segmented control. `monthly` is the mockup's own
/// `checked` option.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BudgetPeriod {
    Weekly,
    #[default]
    Monthly,
    Quarterly,
}

impl BudgetPeriod {
    pub const ALL: [BudgetPeriod; 3] = [
        BudgetPeriod::Weekly,
        BudgetPeriod::Monthly,
        BudgetPeriod::Quarterly,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Weekly => "weekly",
            Self::Monthly => "monthly",
            Self::Quarterly => "quarterly",
        }
    }
}

/// One row of the **Units** section's table (`docs/ux/desktop/Settings/README.md`'s "2a resting
/// state" markup: CODE / NAME / TYPE columns). Owned `String` fields, not `&'static str` --
/// issue #184's own Add unit dialog is this crate's first real typed-text input, so a row can
/// now hold text a person actually typed, not just compile-time dummy data. `kind` stays a
/// free-form string rather than [`UnitKind`]: the table just displays it, and legacy seeded rows
/// ("crypto", "etf") don't match any `UnitKind` label anyway. Issue #185's Edit dialog reads it
/// back through [`UnitKind::from_label`] to pre-fill the Type selector, but the row itself is
/// never typed narrower than a string -- `UnitKind` only governs the dialogs' own selector.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnitRow {
    pub code: String,
    pub name: String,
    pub kind: String,
}

/// The mockup's own three seeded rows, in its own order -- a function rather than a `const`
/// slice now that [`UnitRow`] owns its strings (`String` has no `const` constructor).
pub fn default_units() -> Vec<UnitRow> {
    vec![
        UnitRow {
            code: "aud".to_string(),
            name: "Australian Dollar".to_string(),
            kind: "currency".to_string(),
        },
        UnitRow {
            code: "btc".to_string(),
            name: "Bitcoin".to_string(),
            kind: "crypto".to_string(),
        },
        UnitRow {
            code: "vas".to_string(),
            name: "Vanguard Aus Shares".to_string(),
            kind: "etf".to_string(),
        },
    ]
}

/// The Add/Edit unit dialogs' own "Type" selector (`docs/ux/desktop/Settings/README.md`'s "2b —
/// Add unit": "Type (select: currency / cryptocurrency / custom)") -- rendered as a segmented
/// control (like [`DefaultUnit`]/[`BudgetPeriod`]/[`TracingLevel`]), not a real `<select>`
/// dropdown, same reasoning as every other "pick one of a few options" control this map has
/// built: dropdown-open behaviour has no precedent in this crate yet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum UnitKind {
    #[default]
    Currency,
    Cryptocurrency,
    Custom,
}

impl UnitKind {
    pub const ALL: [UnitKind; 3] = [Self::Currency, Self::Cryptocurrency, Self::Custom];

    pub fn label(self) -> &'static str {
        match self {
            Self::Currency => "currency",
            Self::Cryptocurrency => "cryptocurrency",
            Self::Custom => "custom",
        }
    }

    /// Maps a [`UnitRow::kind`] free-form string back to the closest `UnitKind` (issue #185's
    /// own Edit unit dialog: pre-filling the Type selector from an existing row). Falls back to
    /// `Custom` on no exact match -- `Custom` is the catch-all category by definition, and a
    /// legacy seeded row's own free-form label ("crypto", "etf") was never guaranteed to match
    /// one of these three canonical options in the first place (see [`UnitRow`]'s own doc).
    pub fn from_label(label: &str) -> Self {
        Self::ALL
            .into_iter()
            .find(|kind| kind.label() == label)
            .unwrap_or(Self::Custom)
    }
}

/// Which text field currently receives typed characters in the Add/Edit unit dialogs -- Type
/// has no equivalent variant since it's a click-select segmented control, not something you
/// type into.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AddUnitField {
    #[default]
    Code,
    Name,
}

/// The Add/Edit unit dialogs' own live form state (issues #184/#185) -- pure, `gpui`-free,
/// mirroring `nav.rs`/`palette.rs`'s "state here, chrome renders it" split. `Shell` owns
/// `Option<Self>` wrapped in [`SettingsDialog`]; `None` means no dialog is open. Shared between
/// both dialogs rather than a separate `EditUnitForm` -- issue #185's own body: "same form as
/// Add unit" -- so [`Self::is_valid`]/[`Self::push_char`]/[`Self::backspace`]/[`Self::cycle_field`]
/// only need writing once.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct UnitForm {
    pub code: String,
    pub name: String,
    pub kind: UnitKind,
    pub focused_field: AddUnitField,
}

impl UnitForm {
    /// Pre-fills a form from an existing row (issue #185's own Edit unit dialog) -- `kind` maps
    /// back through [`UnitKind::from_label`] since the row stores a free-form string, not the
    /// enum itself.
    pub fn from_row(row: &UnitRow) -> Self {
        Self {
            code: row.code.clone(),
            name: row.name.clone(),
            kind: UnitKind::from_label(&row.kind),
            focused_field: AddUnitField::default(),
        }
    }

    /// The README's own "Dialog lifecycle" row: "fill Code / Name / Type (all required)" --
    /// Type always has a value (a segmented control can't be empty), so only Code/Name gate the
    /// Add/Save button's enabled state.
    pub fn is_valid(&self) -> bool {
        !self.code.trim().is_empty() && !self.name.trim().is_empty()
    }

    /// Appends whichever character `text` is to the currently focused field -- `Shell` calls
    /// this once per typed character (see `Self::backspace`'s own doc for why there's no bulk
    /// "set text" method instead).
    pub fn push_char(&mut self, ch: char) {
        match self.focused_field {
            AddUnitField::Code => self.code.push(ch),
            AddUnitField::Name => self.name.push(ch),
        }
    }

    /// Pops one character from the focused field -- a no-op on an already-empty field, mirroring
    /// `String::pop`'s own behaviour rather than treating it as an error.
    pub fn backspace(&mut self) {
        match self.focused_field {
            AddUnitField::Code => {
                self.code.pop();
            }
            AddUnitField::Name => {
                self.name.pop();
            }
        }
    }

    /// `Tab` cycles Code -> Name -> Code -- the dialog's own two-field focus ring, independent
    /// of `NavState::cycle_focus_forward`'s three shell-wide zones (`InputMode::Dialog` routes
    /// `Tab` here instead, before the shell-wide tier ever sees it).
    pub fn cycle_field(&mut self) {
        self.focused_field = match self.focused_field {
            AddUnitField::Code => AddUnitField::Name,
            AddUnitField::Name => AddUnitField::Code,
        };
    }
}

/// Every Settings dialog `Shell` can have open, `None` when none is -- the README's own `State`
/// block (`dialog: Option<Dialog>`). One variant per dialog ticket; `EditUnit`'s own `usize` is
/// the row's index in `Shell::settings_units` -- `Shell::confirm_settings_dialog` needs it to
/// know which row to overwrite.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SettingsDialog {
    AddUnit(UnitForm),
    EditUnit(usize, UnitForm),
}

/// One row of the **Institutions** section's table (`docs/ux/desktop/Settings/README.md`'s "2a
/// resting state" markup: INSTITUTION / ACCOUNT TYPE columns) -- `&'static str` fields, unlike
/// [`UnitRow`]'s now-owned `String`s: no Institutions dialog types real text yet (issue #187 is
/// still just a stub), so there's nothing forcing these off compile-time dummy data yet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstitutionRow {
    pub name: &'static str,
    /// The ACCOUNT TYPE column, e.g. `"savings \u{b7} offset"` -- the mockup joins multiple
    /// types with the same `\u{b7}` separator the scope notes use, as one free-form label (not a
    /// `Vec` of chips -- that multi-select shape belongs to the Add institution dialog's own
    /// input, not this read-only table cell).
    pub account_type: &'static str,
}

/// The mockup's own six seeded rows, in its own order (the mockup's static scope note claims "7
/// institutions", but only six rows are actually drawn -- treated as the same kind of
/// mockup-authoring slip [`default_units`]'s own doc calls out elsewhere, not a seventh row to
/// invent; the scope note is dynamic and derived from `Shell::settings_institutions.len()`
/// regardless, so it self-corrects to whatever this slice actually holds).
pub const DEFAULT_INSTITUTIONS: &[InstitutionRow] = &[
    InstitutionRow {
        name: "ANZ Banking Group",
        account_type: "savings \u{b7} offset",
    },
    InstitutionRow {
        name: "American Express",
        account_type: "credit card",
    },
    InstitutionRow {
        name: "Vanguard Investments",
        account_type: "investment",
    },
    InstitutionRow {
        name: "Westpac Banking",
        account_type: "savings",
    },
    InstitutionRow {
        name: "Cryptocurrency Exchange",
        account_type: "crypto",
    },
    InstitutionRow {
        name: "Superannuation Fund",
        account_type: "retirement",
    },
];

/// The **Tracing (Logs)** section's level radios (`docs/ux/desktop/Settings/README.md`'s "2a
/// resting state" markup: `error`/`warn`/`info`/`debug`, `error` the mockup's own `checked`
/// option). Purely a selected-level preference, like [`DefaultUnit`]/[`BudgetPeriod`] -- there
/// are no real log lines to filter by level yet (see [`DEFAULT_LOG_LINES`]'s own doc).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TracingLevel {
    #[default]
    Error,
    Warn,
    Info,
    Debug,
}

impl TracingLevel {
    pub const ALL: [TracingLevel; 4] = [Self::Error, Self::Warn, Self::Info, Self::Debug];

    pub fn label(self) -> &'static str {
        match self {
            Self::Error => "error",
            Self::Warn => "warn",
            Self::Info => "info",
            Self::Debug => "debug",
        }
    }
}

/// The log viewport's own seeded lines, in the mockup's own order -- dummy data, not a real
/// `tracing`-subscriber feed (see the Desktop Settings Surface map's own Destination). `Shell`
/// clones this into a real `Vec` it owns, so "Clear logs" (issue #182) can empty it -- the one
/// real mutation this section makes, unlike Sync server's/Data & backup's own permanently
/// no-effect buttons.
pub const DEFAULT_LOG_LINES: &[&str] = &[
    "[14:22:18] sync: connected to server",
    "[14:22:15] txn: reconciled payment 312.80",
    "[14:22:12] budget: updated dining limit",
    "[14:22:08] import: 3 csv rows processed",
];

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

    #[test]
    fn default_unit_defaults_to_aud() {
        assert_eq!(DefaultUnit::default(), DefaultUnit::Aud);
    }

    #[test]
    fn budget_period_defaults_to_monthly() {
        assert_eq!(BudgetPeriod::default(), BudgetPeriod::Monthly);
    }

    #[test]
    fn default_units_matches_the_mockups_own_three_seeded_rows() {
        let codes: Vec<_> = default_units().into_iter().map(|unit| unit.code).collect();
        assert_eq!(codes, vec!["aud", "btc", "vas"]);
    }

    #[test]
    fn unit_kind_defaults_to_currency() {
        assert_eq!(UnitKind::default(), UnitKind::Currency);
    }

    #[test]
    fn unit_kind_from_label_matches_exactly_or_falls_back_to_custom() {
        assert_eq!(UnitKind::from_label("currency"), UnitKind::Currency);
        assert_eq!(
            UnitKind::from_label("cryptocurrency"),
            UnitKind::Cryptocurrency
        );
        assert_eq!(UnitKind::from_label("crypto"), UnitKind::Custom);
        assert_eq!(UnitKind::from_label("etf"), UnitKind::Custom);
    }

    #[test]
    fn unit_form_from_row_prefills_every_field() {
        let row = UnitRow {
            code: "btc".to_string(),
            name: "Bitcoin".to_string(),
            kind: "crypto".to_string(),
        };
        let form = UnitForm::from_row(&row);
        assert_eq!(form.code, "btc");
        assert_eq!(form.name, "Bitcoin");
        assert_eq!(form.kind, UnitKind::Custom);
        assert_eq!(form.focused_field, AddUnitField::Code);
    }

    #[test]
    fn add_unit_form_is_invalid_until_code_and_name_are_both_filled() {
        let mut form = UnitForm::default();
        assert!(!form.is_valid());
        form.push_char('a');
        assert!(!form.is_valid());
        form.cycle_field();
        form.push_char('b');
        assert!(form.is_valid());
    }

    #[test]
    fn add_unit_form_push_and_backspace_target_the_focused_field() {
        let mut form = UnitForm::default();
        form.push_char('a');
        form.push_char('u');
        form.push_char('d');
        assert_eq!(form.code, "aud");
        assert_eq!(form.name, "");
        form.backspace();
        assert_eq!(form.code, "au");

        form.cycle_field();
        form.push_char('x');
        assert_eq!(form.name, "x");
        assert_eq!(form.code, "au");
    }

    #[test]
    fn add_unit_form_cycle_field_toggles_between_code_and_name() {
        let mut form = UnitForm::default();
        assert_eq!(form.focused_field, AddUnitField::Code);
        form.cycle_field();
        assert_eq!(form.focused_field, AddUnitField::Name);
        form.cycle_field();
        assert_eq!(form.focused_field, AddUnitField::Code);
    }

    #[test]
    fn default_institutions_matches_the_mockups_own_six_seeded_rows() {
        let names: Vec<_> = DEFAULT_INSTITUTIONS
            .iter()
            .map(|institution| institution.name)
            .collect();
        assert_eq!(
            names,
            vec![
                "ANZ Banking Group",
                "American Express",
                "Vanguard Investments",
                "Westpac Banking",
                "Cryptocurrency Exchange",
                "Superannuation Fund",
            ]
        );
    }

    #[test]
    fn tracing_level_defaults_to_error() {
        assert_eq!(TracingLevel::default(), TracingLevel::Error);
    }

    #[test]
    fn default_log_lines_matches_the_mockups_own_four_seeded_lines() {
        assert_eq!(DEFAULT_LOG_LINES.len(), 4);
        assert!(DEFAULT_LOG_LINES[0].contains("sync: connected to server"));
    }
}
