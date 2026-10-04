//! Pure Settings-surface domain types (`docs/ux/desktop/16-settings/README.md`'s "2a resting
//! state" -- "Sections" table) -- `gpui`-free, the same "pure state, chrome renders it" split
//! `nav.rs` uses between `NavState` and `Shell`'s render tree. `rail::settings_index::SettingsIndexRail`
//! and `view::settings` are the chrome; this module only knows what pages exist, their index
//! order, and their label/scope-note/search text.

use lib_core::DateStyle;

use crate::{dialog_host::Dialog, field::TextField};

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
        .map(|date| crate::format::date(date, style))
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
        .map(|money| crate::format::signed_amount(&money).1)
        .unwrap_or_default()
}

/// One row of the **Units** section's table (`docs/ux/desktop/16-settings/README.md`'s "2a resting
/// state" markup: CODE / NAME / FLAGS / SOURCE / TYPE / ACTIONS columns as of issue #189, up
/// from CODE / NAME / TYPE). Owned `String` fields, not `&'static str` --
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
    /// The SOURCE column shorthand (issue #189) -- e.g. `"USD"` (aud's rate is sourced against
    /// USD as the reference currency), `"CoinGecko"`, `"Manual entry"`. A unit added via the Add
    /// unit dialog (issue #184) has no real price-source integration, so it defaults to `"Manual
    /// entry"`, the same fallback `vas` already uses.
    pub source: String,
    /// The FLAGS column's `base` pill (`.tag.tag-accent`) -- the ledger's own base unit. Only
    /// `aud` carries this in the seeded data; nothing in this map lets a user change which unit
    /// is base, so a dialog-created row always defaults to `false`.
    pub is_base: bool,
    /// The FLAGS column's `default` pill (`.tag.tag-outline`) -- the default unit for new
    /// entries (formerly the removed "Ledger & units" section's own standalone control, now
    /// folded into this one flag). Same "only `aud`, dialogs default to `false`" reasoning as
    /// [`Self::is_base`].
    pub is_default: bool,
}

/// The mockup's own three seeded rows, in its own order -- a function rather than a `const`
/// slice now that [`UnitRow`] owns its strings (`String` has no `const` constructor).
pub fn default_units() -> Vec<UnitRow> {
    vec![
        UnitRow {
            code: "aud".to_string(),
            name: "Australian Dollar".to_string(),
            kind: "currency".to_string(),
            source: "USD".to_string(),
            is_base: true,
            is_default: true,
        },
        UnitRow {
            code: "btc".to_string(),
            name: "Bitcoin".to_string(),
            kind: "crypto".to_string(),
            source: "CoinGecko".to_string(),
            is_base: false,
            is_default: false,
        },
        UnitRow {
            code: "vas".to_string(),
            name: "Vanguard Aus Shares".to_string(),
            kind: "etf".to_string(),
            source: "Manual entry".to_string(),
            is_base: false,
            is_default: false,
        },
    ]
}

/// One row of the **Units** section's own "Price Sources" subsection (issue #189's own README
/// revision) -- a table about the *relationship* to an external price source, not the unit
/// record itself, so it's keyed by the unit's `name`, not its `code` (the README's own "the one
/// place the unit is referenced by name rather than code" callout). Only `btc`/`vas` appear here
/// -- `aud` has no price-source relationship of this kind (its own SOURCE column value, "USD",
/// is a reference-currency note on the Units table itself, not an external feed).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PriceSourceRow {
    pub unit_name: String,
    pub source: String,
    pub last_updated: String,
}

/// The mockup's own two seeded rows, in its own order.
pub fn default_price_sources() -> Vec<PriceSourceRow> {
    vec![
        PriceSourceRow {
            unit_name: "Bitcoin".to_string(),
            source: "CoinGecko \u{2014} auto, every 15 min".to_string(),
            last_updated: "5 minutes ago".to_string(),
        },
        PriceSourceRow {
            unit_name: "Vanguard Aus Shares".to_string(),
            source: "Manual entry".to_string(),
            last_updated: "\u{2014}".to_string(),
        },
    ]
}

/// The Add/Edit unit dialogs' own "Type" selector (`docs/ux/desktop/16-settings/README.md`'s "2b —
/// Add unit": "Type (select: currency / cryptocurrency / custom)") -- rendered as a segmented
/// control (like [`TracingLevel`]), not a real `<select>` dropdown, same reasoning as every
/// other "pick one of a few options" control this map has built: dropdown-open behaviour has no
/// precedent in this crate yet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum UnitKind {
    #[default]
    Currency,
    Cryptocurrency,
    Custom,
}

impl UnitKind {
    pub const ALL: [UnitKind; 3] = [Self::Currency, Self::Cryptocurrency, Self::Custom];

    pub fn label(self) -> String {
        match self {
            Self::Currency => crate::msg::desktop_settings_unit_kind_currency(),
            Self::Cryptocurrency => crate::msg::desktop_settings_unit_kind_cryptocurrency(),
            Self::Custom => crate::msg::desktop_settings_unit_kind_custom(),
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
/// Add unit" -- so [`Self::is_valid`]/[`Self::cycle_field`] only need writing once.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct UnitForm {
    pub code: TextField,
    pub name: TextField,
    pub kind: UnitKind,
    pub focused_field: AddUnitField,
}

impl UnitForm {
    /// Pre-fills a form from an existing row (issue #185's own Edit unit dialog) -- `kind` maps
    /// back through [`UnitKind::from_label`] since the row stores a free-form string, not the
    /// enum itself.
    pub fn from_row(row: &UnitRow) -> Self {
        Self {
            code: TextField::new(row.code.as_str()),
            name: TextField::new(row.name.as_str()),
            kind: UnitKind::from_label(&row.kind),
            focused_field: AddUnitField::default(),
        }
    }

    /// The README's own "Dialog lifecycle" row: "fill Code / Name / Type (all required)" --
    /// Type always has a value (a segmented control can't be empty), so only Code/Name gate the
    /// Add/Save button's enabled state.
    pub fn is_valid(&self) -> bool {
        !self.code.is_blank() && !self.name.is_blank()
    }

    /// The field typing and `Backspace` edit.
    pub fn focused_mut(&mut self) -> &mut TextField {
        match self.focused_field {
            AddUnitField::Code => &mut self.code,
            AddUnitField::Name => &mut self.name,
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

/// The Delete unit dialog's own live form state (issue #186) -- pure, `gpui`-free. Just the one
/// typed-back confirmation field: unlike [`UnitForm`], there's nothing to `Tab` between, so no
/// `focused_field` -- the confirmation input is implicitly the only thing you can type into
/// while this dialog is open. The code to type back is copied in when the dialog opens, so the
/// form validates without reading `Shell`'s Units (the dialog is modal: they can't change under
/// it).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeleteUnitForm {
    pub code: String,
    pub confirm_input: TextField,
}

impl DeleteUnitForm {
    pub fn new(code: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            confirm_input: TextField::default(),
        }
    }

    /// The README's own "disabled until the typed value matches the code exactly" -- a plain
    /// case-sensitive `==`, not a trim/lowercase-tolerant comparison, per the ticket's own body.
    pub fn is_valid(&self) -> bool {
        self.confirm_input.text() == self.code
    }
}

/// One row of the **Institutions** section's table (`docs/ux/desktop/16-settings/README.md`'s "2a
/// resting state" markup: INSTITUTION / ACCOUNT TYPE columns). Owned `String` fields, not
/// `&'static str` -- issue #187's own Add institution dialog produces real typed text, same
/// reasoning as [`UnitRow`]'s own migration for issue #184.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstitutionRow {
    pub name: String,
    /// The ACCOUNT TYPE column, e.g. `"savings \u{b7} offset"` -- the mockup joins multiple
    /// types with the same `\u{b7}` separator the scope notes use, as one free-form label (not a
    /// `Vec` of chips -- that multi-select shape belongs to the Add institution dialog's own
    /// input, not this read-only table cell). `AddInstitutionForm::account_types` joins the same
    /// way when a dialog-created row is appended (see `Shell::confirm_settings_dialog`).
    pub account_type: String,
}

/// The mockup's own six seeded rows, in its own order (the mockup's static scope note claims "7
/// institutions", but only six rows are actually drawn -- treated as the same kind of
/// mockup-authoring slip [`default_units`]'s own doc calls out elsewhere, not a seventh row to
/// invent; the scope note is dynamic and derived from `Shell::settings_institutions.len()`
/// regardless, so it self-corrects to whatever this slice actually holds). A function rather
/// than a `const` slice now that [`InstitutionRow`] owns its strings.
pub fn default_institutions() -> Vec<InstitutionRow> {
    vec![
        InstitutionRow {
            name: "ANZ Banking Group".to_string(),
            account_type: "savings \u{b7} offset".to_string(),
        },
        InstitutionRow {
            name: "American Express".to_string(),
            account_type: "credit card".to_string(),
        },
        InstitutionRow {
            name: "Vanguard Investments".to_string(),
            account_type: "investment".to_string(),
        },
        InstitutionRow {
            name: "Westpac Banking".to_string(),
            account_type: "savings".to_string(),
        },
        InstitutionRow {
            name: "Cryptocurrency Exchange".to_string(),
            account_type: "crypto".to_string(),
        },
        InstitutionRow {
            name: "Superannuation Fund".to_string(),
            account_type: "retirement".to_string(),
        },
    ]
}

/// The Add institution dialog's own Account types multi-select chips
/// (`docs/ux/desktop/16-settings/README.md`'s "2e — Add institution": savings / credit card /
/// offset / loan / investment).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccountType {
    Savings,
    CreditCard,
    Offset,
    Loan,
    Investment,
}

impl AccountType {
    pub const ALL: [AccountType; 5] = [
        Self::Savings,
        Self::CreditCard,
        Self::Offset,
        Self::Loan,
        Self::Investment,
    ];

    pub fn label(self) -> String {
        match self {
            Self::Savings => crate::msg::desktop_settings_account_type_savings(),
            Self::CreditCard => lib_locale::msg::account_kind_credit_card(),
            Self::Offset => crate::msg::desktop_settings_account_type_offset(),
            Self::Loan => lib_locale::msg::account_kind_loan(),
            Self::Investment => lib_locale::msg::account_kind_investment(),
        }
    }
}

/// The Add institution dialog's own live form state (issue #187) -- pure, `gpui`-free.
/// Institution name is a real text field with no `focused_field`, mirroring
/// [`DeleteUnitForm`]'s own one-field shape: it's the only text field, so always implicitly
/// focused, nothing to `Tab` between. Account types are a real multi-select (more than one chip
/// may be checked at once, unlike [`UnitForm`]'s single-choice Type) -- `Vec` rather than a
/// fixed-size set since membership toggles are simpler as push/remove.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AddInstitutionForm {
    pub name: TextField,
    pub account_types: Vec<AccountType>,
    /// The selected default unit's own *code*, not an index into `Shell::settings_units` -- a
    /// code stays meaningful even if that `Vec`'s shape changes, though nothing in this map
    /// actually opens two dialogs at once for that to matter yet.
    pub default_unit_code: Option<String>,
}

impl AddInstitutionForm {
    /// Seeds Account types with the mockup's own `checked` default (savings) and Default unit
    /// with `units`' own first entry, if any -- the same "first option is the resting default"
    /// precedent `TracingLevel`/etc. already establish, just read from a runtime `Vec` instead
    /// of a compile-time enum's own `ALL`.
    pub fn new(units: &[UnitRow]) -> Self {
        Self {
            name: TextField::default(),
            account_types: vec![AccountType::Savings],
            default_unit_code: units.first().map(|unit| unit.code.clone()),
        }
    }

    /// Toggles `account_type`'s own membership -- present removes it, absent adds it.
    pub fn toggle_account_type(&mut self, account_type: AccountType) {
        match self
            .account_types
            .iter()
            .position(|&selected| selected == account_type)
        {
            Some(index) => {
                self.account_types.remove(index);
            }
            None => self.account_types.push(account_type),
        }
    }

    /// The README's own "Dialog lifecycle" row: "name + at least one account type + default
    /// unit" (all required).
    pub fn is_valid(&self) -> bool {
        !self.name.is_blank() && !self.account_types.is_empty() && self.default_unit_code.is_some()
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
    fn default_units_matches_the_mockups_own_three_seeded_rows() {
        let codes: Vec<_> = default_units().into_iter().map(|unit| unit.code).collect();
        assert_eq!(codes, vec!["aud", "btc", "vas"]);
    }

    #[test]
    fn only_aud_carries_the_base_and_default_flags() {
        let units = default_units();
        let flagged: Vec<_> = units
            .iter()
            .filter(|unit| unit.is_base || unit.is_default)
            .map(|unit| unit.code.as_str())
            .collect();
        assert_eq!(flagged, vec!["aud"]);
    }

    #[test]
    fn default_price_sources_matches_the_mockups_own_two_seeded_rows() {
        let names: Vec<_> = default_price_sources()
            .into_iter()
            .map(|row| row.unit_name)
            .collect();
        assert_eq!(names, vec!["Bitcoin", "Vanguard Aus Shares"]);
    }

    #[test]
    fn unit_kind_defaults_to_currency() {
        assert_eq!(UnitKind::default(), UnitKind::Currency);
    }

    #[test]
    fn unit_kind_from_label_matches_exactly_or_falls_back_to_custom() {
        crate::locale::init_for_tests();
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
            source: "CoinGecko".to_string(),
            is_base: false,
            is_default: false,
        };
        let form = UnitForm::from_row(&row);
        assert_eq!(form.code.text(), "btc");
        assert_eq!(form.name.text(), "Bitcoin");
        assert_eq!(form.kind, UnitKind::Custom);
        assert_eq!(form.focused_field, AddUnitField::Code);
    }

    #[test]
    fn add_institution_form_new_seeds_savings_and_the_first_unit() {
        let units = default_units();
        let form = AddInstitutionForm::new(&units);
        assert_eq!(form.account_types, vec![AccountType::Savings]);
        assert_eq!(form.default_unit_code.as_deref(), Some("aud"));
    }

    #[test]
    fn add_institution_form_new_with_no_units_has_no_default_unit() {
        let form = AddInstitutionForm::new(&[]);
        assert_eq!(form.default_unit_code, None);
    }

    #[test]
    fn add_institution_form_toggle_account_type_adds_and_removes() {
        let mut form = AddInstitutionForm::new(&default_units());
        assert!(form.account_types.contains(&AccountType::Savings));
        form.toggle_account_type(AccountType::Savings);
        assert!(!form.account_types.contains(&AccountType::Savings));
        form.toggle_account_type(AccountType::Loan);
        assert!(form.account_types.contains(&AccountType::Loan));
    }

    #[test]
    fn add_unit_form_is_invalid_until_code_and_name_are_both_filled() {
        let mut form = UnitForm::default();
        assert!(!form.is_valid());
        form.focused_mut().push('a');
        assert!(!form.is_valid());
        form.cycle_field();
        form.focused_mut().push(' ');
        assert!(!form.is_valid());
        form.focused_mut().push('b');
        assert!(form.is_valid());
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
        let names: Vec<_> = default_institutions()
            .into_iter()
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
        assert_eq!(AccountType::CreditCard.label(), "Credit card");
        assert_eq!(TracingLevel::Warn.label(), "warn");
        assert_eq!(
            StatusGlyphs::AsciiFallback.label(),
            "ascii fallback \u{2014} o / x !"
        );
    }
}
