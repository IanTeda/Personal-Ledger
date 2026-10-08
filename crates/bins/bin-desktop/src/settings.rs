//! Pure Settings-surface domain types (`docs/ux/desktop/16-settings/README.md`'s "2a resting
//! state" -- "Sections" table) -- `gpui`-free, the same "pure state, chrome renders it" split
//! `navigation/nav.rs` uses between `NavState` and `Shell`'s render tree. `chrome::rail::settings_index::SettingsIndexRail`
//! and `view::settings` are the chrome; this module only knows what pages exist, their index
//! order, and their label/scope-note/search text.

use lib_core::DateStyle;

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

/// The **Display** section's "Date format" segmented control: the nullable date style
/// Preference (ADR-0021). `None` is "Locale default" -- the Locale picks the form -- and only an
/// explicit choice is stored. The Locale itself is not chosen here: it is Configuration, shown
/// read-only beside this control. The number separator has no control, because the Locale owns it.
pub const DATE_STYLE_CHOICES: [Option<DateStyle>; 5] = [
    None,
    Some(DateStyle::Short),
    Some(DateStyle::Medium),
    Some(DateStyle::Long),
    Some(DateStyle::Iso),
];

/// The control's label for a choice, as a Message in the Locale in effect.
pub fn date_style_label(choice: Option<DateStyle>) -> String {
    match choice {
        None => crate::msg::desktop_display_date_style_default(),
        Some(DateStyle::Short) => crate::msg::desktop_display_date_style_short(),
        Some(DateStyle::Medium) => crate::msg::desktop_display_date_style_medium(),
        Some(DateStyle::Long) => crate::msg::desktop_display_date_style_long(),
        Some(DateStyle::Iso) => crate::msg::desktop_display_date_style_iso(),
    }
}

/// The same section's "Row density" segmented control (`regular` the mockup's own `checked`
/// option).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RowDensity {
    Compact,
    #[default]
    Regular,
    Roomy,
}

impl RowDensity {
    pub const ALL: [RowDensity; 3] = [Self::Compact, Self::Regular, Self::Roomy];

    pub fn label(self) -> String {
        match self {
            Self::Compact => crate::msg::desktop_settings_density_compact(),
            Self::Regular => crate::msg::desktop_settings_density_regular(),
            Self::Roomy => crate::msg::desktop_settings_density_roomy(),
        }
    }

    /// The PREVIEW table's own row vertical padding at this density -- the mockup only draws the
    /// `regular` state (`padding:8px 12px`), so `compact`/`roomy` step by the design tokens' own
    /// 4px rhythm (`docs/ux/desktop/16-settings/README.md`'s Spacing token list) either side of it,
    /// the same "reflects the currently-selected values" the README's own field list promises
    /// for this control.
    pub fn preview_row_padding_y(self) -> f32 {
        match self {
            Self::Compact => 4.0,
            Self::Regular => 8.0,
            Self::Roomy => 12.0,
        }
    }
}

/// The same section's "Status glyphs" radio group (dot-style, like [`TracingLevel`]'s own
/// radios, not segmented) -- `unicode` is the mockup's own `checked` option.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StatusGlyphs {
    #[default]
    Unicode,
    AsciiFallback,
}

impl StatusGlyphs {
    pub const ALL: [StatusGlyphs; 2] = [Self::Unicode, Self::AsciiFallback];

    pub fn label(self) -> String {
        match self {
            Self::Unicode => {
                crate::msg::desktop_settings_glyphs_unicode("\u{25cb} \u{25d0} \u{25cf} \u{2691}")
            }
            Self::AsciiFallback => crate::msg::desktop_settings_glyphs_ascii("o / x !"),
        }
    }
}

/// One PREVIEW-table row's transaction status (`docs/ux/desktop/16-settings/README.md`'s own three
/// drawn rows: cleared, pending, flagged) -- distinct from [`StatusGlyphs`], which picks *how* a
/// status renders, not *which* status a row has.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreviewStatus {
    Cleared,
    Pending,
    Flagged,
}

impl PreviewStatus {
    /// The glyph shown in the PREVIEW table's leftmost column at the given [`StatusGlyphs`]
    /// style. The ascii fallback's own three symbols aren't individually labelled in the
    /// mockup's copy ("o / x !"), so this keeps that same left-to-right order: pending stays the
    /// "open/unmarked" `o`, cleared takes the unambiguous `x`, flagged keeps `!`.
    pub fn glyph(self, style: StatusGlyphs) -> &'static str {
        match (self, style) {
            (Self::Cleared, StatusGlyphs::Unicode) => "\u{25cf}",
            (Self::Pending, StatusGlyphs::Unicode) => "\u{25d0}",
            (Self::Flagged, StatusGlyphs::Unicode) => "\u{2691}",
            (Self::Cleared, StatusGlyphs::AsciiFallback) => "x",
            (Self::Pending, StatusGlyphs::AsciiFallback) => "o",
            (Self::Flagged, StatusGlyphs::AsciiFallback) => "!",
        }
    }
}

/// One PREVIEW-table row -- the mockup's own three seeded rows
/// (`docs/ux/desktop/16-settings/README.md`'s "2a resting state" markup), keyed on a real
/// `(year, month, day)` and integer cents rather than pre-formatted strings so
/// [`format_preview_date`]/[`format_preview_amount`] can re-render them under any selected
/// [`DateFormat`]/[`DecimalSeparator`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DisplayPreviewRow {
    pub status: PreviewStatus,
    pub year: u32,
    pub month: u32,
    pub day: u32,
    pub payee: &'static str,
    pub amount_cents: i64,
}

/// The mockup's own three seeded rows, in its own order.
pub const DEFAULT_DISPLAY_PREVIEW_ROWS: &[DisplayPreviewRow] = &[
    DisplayPreviewRow {
        status: PreviewStatus::Cleared,
        year: 2026,
        month: 9,
        day: 12,
        payee: "Woolworths Metro",
        amount_cents: -8640,
    },
    DisplayPreviewRow {
        status: PreviewStatus::Pending,
        year: 2026,
        month: 9,
        day: 10,
        payee: "Telstra",
        amount_cents: -9900,
    },
    DisplayPreviewRow {
        status: PreviewStatus::Flagged,
        year: 2026,
        month: 9,
        day: 9,
        payee: "Dept of Education",
        amount_cents: 421_000,
    },
];

/// Formats a `(year, month, day)` in the selected date style, for the PREVIEW table, through the
/// same Locale formatting the rest of the app uses so the two cannot disagree.
pub fn format_preview_date(year: u32, month: u32, day: u32, style: Option<DateStyle>) -> String {
    i32::try_from(year)
        .ok()
        .and_then(|year| chrono::NaiveDate::from_ymd_opt(year, month, day))
        .map(|date| crate::view::format::date(date, style))
        .unwrap_or_else(|| "???".to_string())
}

/// Formats `amount_cents` as the PREVIEW table shows it (`"+4,210.00"`, `"\u{2212}86.40"`): an
/// explicit `+` for a positive amount and the Locale's grouping.
pub fn format_preview_amount(amount_cents: i64) -> String {
    let sign = if amount_cents < 0 { "-" } else { "" };
    let text = format!(
        "{sign}{}.{:02}",
        amount_cents.unsigned_abs() / 100,
        amount_cents.unsigned_abs() % 100
    );
    text.parse::<lib_core::Money>()
        .map(|money| crate::view::format::signed_amount(&money).1)
        .unwrap_or_default()
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

/// The **Tracing (Logs)** page's level radios: the most verbose level the log box shows. A view
/// filter over what the capture already holds (it records at `debug`), so changing it is instant
/// and retroactive. Session only, not a Preference (issue #501). There is no `trace` or `off`:
/// the capture never records `trace`, and `off` would only ever show the empty state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TracingLevel {
    Error,
    Warn,
    /// `lib_tracing::init`'s own default when no `log` level is configured.
    #[default]
    Info,
    Debug,
}

impl TracingLevel {
    pub const ALL: [TracingLevel; 4] = [Self::Error, Self::Warn, Self::Info, Self::Debug];

    pub fn label(self) -> String {
        match self {
            Self::Error => crate::msg::desktop_settings_tracing_level_error(),
            Self::Warn => crate::msg::desktop_settings_tracing_level_warn(),
            Self::Info => crate::msg::desktop_settings_tracing_level_info(),
            Self::Debug => crate::msg::desktop_settings_tracing_level_debug(),
        }
    }

    /// The radio the page opens on: the configured `log` level, clamped to the four offered, so
    /// the page starts by showing what the console and log file show.
    pub fn from_configured(level: Option<lib_tracing::Levels>) -> Self {
        use lib_tracing::Levels;
        match level {
            Some(Levels::OFF | Levels::ERROR) => Self::Error,
            Some(Levels::WARN) => Self::Warn,
            Some(Levels::INFO) | None => Self::Info,
            Some(Levels::DEBUG | Levels::TRACE) => Self::Debug,
        }
    }

    /// Whether an entry at `level` shows under this filter.
    pub fn admits(self, level: tracing::Level) -> bool {
        let most_verbose = match self {
            Self::Error => tracing::Level::ERROR,
            Self::Warn => tracing::Level::WARN,
            Self::Info => tracing::Level::INFO,
            Self::Debug => tracing::Level::DEBUG,
        };
        // `tracing` orders levels by verbosity: `ERROR` is the least.
        level <= most_verbose
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
    fn tracing_level_starts_from_the_configured_level_clamped() {
        use lib_tracing::Levels;
        let cases = [
            (Some(Levels::OFF), TracingLevel::Error),
            (Some(Levels::ERROR), TracingLevel::Error),
            (Some(Levels::WARN), TracingLevel::Warn),
            (Some(Levels::INFO), TracingLevel::Info),
            (None, TracingLevel::Info),
            (Some(Levels::DEBUG), TracingLevel::Debug),
            (Some(Levels::TRACE), TracingLevel::Debug),
        ];
        for (configured, expected) in cases {
            assert_eq!(TracingLevel::from_configured(configured), expected);
        }
    }

    #[test]
    fn tracing_level_admits_its_own_level_and_anything_more_severe() {
        use tracing::Level;
        assert!(TracingLevel::Warn.admits(Level::ERROR));
        assert!(TracingLevel::Warn.admits(Level::WARN));
        assert!(!TracingLevel::Warn.admits(Level::INFO));
        assert!(TracingLevel::Debug.admits(Level::DEBUG));
        assert!(!TracingLevel::Debug.admits(Level::TRACE));
        assert!(!TracingLevel::Error.admits(Level::WARN));
    }

    #[test]
    fn the_date_style_choices_start_with_the_locale_default() {
        assert_eq!(DATE_STYLE_CHOICES[0], None);
        assert_eq!(DATE_STYLE_CHOICES.len(), 5);
    }

    #[test]
    fn the_date_style_labels_are_messages() {
        crate::locale::init_for_tests();
        let labels: Vec<String> = DATE_STYLE_CHOICES
            .into_iter()
            .map(date_style_label)
            .collect();
        assert_eq!(labels, ["Locale", "Short", "Medium", "Long", "ISO"]);
    }

    #[test]
    fn row_density_defaults_to_regular() {
        assert_eq!(RowDensity::default(), RowDensity::Regular);
    }

    #[test]
    fn status_glyphs_defaults_to_unicode() {
        assert_eq!(StatusGlyphs::default(), StatusGlyphs::Unicode);
    }

    #[test]
    fn default_display_preview_rows_matches_the_mockups_own_three_seeded_rows() {
        let payees: Vec<_> = DEFAULT_DISPLAY_PREVIEW_ROWS
            .iter()
            .map(|row| row.payee)
            .collect();
        assert_eq!(
            payees,
            vec!["Woolworths Metro", "Telstra", "Dept of Education"]
        );
    }

    #[test]
    fn format_preview_date_follows_the_locale_and_style() {
        use lib_locale::{Locale, with_locale};
        with_locale(Locale::EnAu, || {
            assert_eq!(format_preview_date(2026, 9, 12, None), "12 Sept 2026");
            assert_eq!(
                format_preview_date(2026, 9, 12, Some(DateStyle::Long)),
                "12 September 2026"
            );
            assert_eq!(
                format_preview_date(2026, 9, 12, Some(DateStyle::Iso)),
                "2026-09-12"
            );
        });
        with_locale(Locale::EnUs, || {
            assert_eq!(
                format_preview_date(2026, 9, 12, Some(DateStyle::Short)),
                "9/12/26"
            );
        });
        assert_eq!(format_preview_date(2026, 13, 40, None), "???");
    }

    #[test]
    fn format_preview_amount_matches_the_mockups_own_values() {
        assert_eq!(format_preview_amount(-8640), "\u{2212}86.40");
        assert_eq!(format_preview_amount(421_000), "+4,210.00");
    }

    #[test]
    fn preview_status_glyph_matches_the_selected_style() {
        assert_eq!(
            PreviewStatus::Cleared.glyph(StatusGlyphs::Unicode),
            "\u{25cf}"
        );
        assert_eq!(
            PreviewStatus::Cleared.glyph(StatusGlyphs::AsciiFallback),
            "x"
        );
        assert_eq!(
            PreviewStatus::Flagged.glyph(StatusGlyphs::Unicode),
            "\u{2691}"
        );
        assert_eq!(
            PreviewStatus::Flagged.glyph(StatusGlyphs::AsciiFallback),
            "!"
        );
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

    #[test]
    fn local_enum_labels_come_from_the_catalogue() {
        crate::locale::init_for_tests();
        assert_eq!(RowDensity::Roomy.label(), "roomy");
        assert_eq!(TracingLevel::Warn.label(), "warn");
        assert_eq!(
            StatusGlyphs::AsciiFallback.label(),
            "ascii fallback \u{2014} o / x !"
        );
    }
}
