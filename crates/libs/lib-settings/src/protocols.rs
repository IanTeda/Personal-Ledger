//! What the Settings module asks of the Shell and of the device, through ports each Client
//! implements. [`SettingsRouter`] is the frame's and [`DeviceRouter`] the This device pages', each
//! lent to every `handle` call; a refusal or a saved setting goes to the status line through
//! [`Notify`] (ADR-0048). [`DeviceConfiguration`] writes a device's settings, Configuration in the
//! user's configuration file (ADR-0053), which the Client answers over its own configuration writer.

use std::rc::Rc;

use lib_colour_theme::ColourAppearance;
use lib_core::{DateStyle, Money, OnStart, RowDensity, StartView, StatusGlyphs, UnitId};
use lib_toast::Notify;

use crate::log::LogLine;
use crate::page::SettingsPage;
use crate::presenter::{ClearLogsPresenter, DisplayOutput, LogListChange, LogScroll};

/// What only the Shell does for the Settings frame.
pub trait SettingsRouter: Notify {
    /// The page on show changed by the person's hand (a key, a click, a command), so the Shell
    /// records it in its history (`NavInput::SettingsPage`) for Back and Forward. A restored page
    /// is not announced, since the Shell restored it.
    fn page_changed(&mut self, page: SettingsPage);

    /// The page picker opened, so the Shell's input mode becomes the picker's and every key
    /// reaches the frame until it closes.
    fn picker_opened(&mut self);

    /// The page picker closed, so the Shell's input mode returns to normal.
    fn picker_closed(&mut self);
}

/// What only the Shell does once a This device page has changed a setting: the Toasts, the colours
/// and the Views that draw with the Display output are the Client's.
pub trait DeviceRouter: Notify {
    /// Toasts were turned on or off and saved, so the Shell's Toasts take it.
    fn toasts_changed(&mut self, on: bool);

    /// The Display output changed, so the Shell hands it to every View that draws with it.
    fn display_changed(&mut self, display: DisplayOutput);

    /// A Colour Theme or Appearance was saved, so the Client applies it to every window.
    fn colour_changed(&mut self, colour: ColourChoice);
}

/// A Colour Theme or Appearance chosen on Display or by the palette.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColourChoice {
    /// A built-in Colour Theme, by its id.
    Theme(&'static str),
    Appearance(ColourAppearance),
}

/// A device's setting that is Configuration, by the key the configuration file spells it with,
/// which the status line and a control's shadowed note name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceKey {
    OnStart,
    StartView,
    Toasts,
    DateStyle,
    RowDensity,
    StatusGlyphs,
    ColourTheme,
    ColourAppearance,
}

impl DeviceKey {
    pub const ALL: [DeviceKey; 8] = [
        Self::OnStart,
        Self::StartView,
        Self::Toasts,
        Self::DateStyle,
        Self::RowDensity,
        Self::StatusGlyphs,
        Self::ColourTheme,
        Self::ColourAppearance,
    ];

    /// The key as the configuration file spells it, the TUI's `:set` and the environment variable
    /// naming it too.
    pub fn name(self) -> &'static str {
        match self {
            Self::OnStart => "on_start",
            Self::StartView => "start_view",
            Self::Toasts => "toasts",
            Self::DateStyle => "date_style",
            Self::RowDensity => "row_density",
            Self::StatusGlyphs => "status_glyphs",
            Self::ColourTheme => "colour_theme",
            Self::ColourAppearance => "colour_appearance",
        }
    }
}

/// How a write of a device's setting went: an answer for the person, never a failure for the
/// caller, so the port stays infallible (ADR-0046).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeviceAnswer<T> {
    /// Written: the value now in the file, and the file, as the status line names it.
    Saved { value: T, file: String },
    /// Not written, since a source above the file sets the key: an environment variable or a
    /// higher configuration file, named by `source`.
    Shadowed { source: String },
    /// Not written, for the reason given.
    Refused(DeviceRefusal),
}

/// Why a device's setting the file would take wasn't written.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceRefusal {
    /// There is no home folder to keep the file in.
    NoFile,
    /// The file or its folder couldn't be written; the Client logs the cause.
    WriteFailed,
}

/// Writes a device's settings to the user's configuration file, one typed method per setting, and
/// says which keys a source above the file sets, so a control can say so before it is changed.
pub trait DeviceConfiguration {
    /// What sets `key` above the file, by name, or `None` when a write to the file applies.
    fn shadow(&self, key: DeviceKey) -> Option<String>;

    fn set_on_start(&self, on_start: OnStart) -> DeviceAnswer<OnStart>;

    fn set_start_view(&self, start_view: StartView) -> DeviceAnswer<StartView>;

    fn set_toasts(&self, on: bool) -> DeviceAnswer<bool>;

    /// `None` is the Locale's default.
    fn set_date_style(&self, style: Option<DateStyle>) -> DeviceAnswer<Option<DateStyle>>;

    fn set_row_density(&self, density: RowDensity) -> DeviceAnswer<RowDensity>;

    /// Shared with the TUI, which reads the same key.
    fn set_status_glyphs(&self, glyphs: StatusGlyphs) -> DeviceAnswer<StatusGlyphs>;

    /// A built-in Colour Theme, by its id.
    fn set_colour_theme(&self, id: &'static str) -> DeviceAnswer<&'static str>;

    fn set_colour_appearance(&self, appearance: ColourAppearance)
    -> DeviceAnswer<ColourAppearance>;
}

/// What only the Shell does for Settings › Tracing: the log box's list is the Client's, and Clear
/// logs is a Dialog it hosts.
pub trait TracingRouter: Notify {
    /// Scrolls the log box.
    fn scroll(&mut self, scroll: LogScroll);

    /// The rows the log box shows changed, so the Client keeps its list in step.
    fn log_changed(&mut self, change: LogListChange);

    /// Opens the Clear logs Dialog, which `dialog` is the Presenter of.
    fn open_clear_logs(&mut self, dialog: ClearLogsPresenter);
}

/// The log capture Settings › Tracing reads and Clear logs empties. The Client answers it over its
/// capture ring, which fills from any thread and signals the page ([`TracingEvent::Appended`])
/// when entries arrive.
///
/// [`TracingEvent::Appended`]: crate::TracingEvent::Appended
pub trait TracingLog {
    /// How many entries the capture keeps before it evicts the oldest.
    fn capacity(&self) -> usize;

    /// The entries whose `seq` is `from` or later, oldest first: what arrived since a reader last
    /// saw [`Self::next_seq`].
    fn since(&self, from: u64) -> Vec<Rc<LogLine>>;

    /// The `seq` the next entry will get.
    fn next_seq(&self) -> u64;

    /// Empties the capture for every reader. Sequence numbers keep counting up.
    fn clear(&self);
}

/// A Unit the Base unit select offers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnitChoice {
    pub id: UnitId,
    pub code: String,
    pub name: String,
}

/// What Settings › Ledger reads of the Units: the Client answers it over the Units store
/// (`lib-database`'s `StoredSettingsReads`), so the page never holds a copy.
pub trait LedgerUnits {
    /// The Units the Base unit select offers: the active ones, by code.
    fn choices(&self) -> Vec<UnitChoice>;

    /// The Ledger's base Unit.
    fn base_unit(&self) -> Option<UnitId>;
}

/// How a write of the base Unit went.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BaseUnitAnswer {
    Saved,
    /// The Unit was removed or deactivated under the select, which repaints without it.
    Refused,
}

/// Writes the Ledger's one Preference, its base Unit.
pub trait LedgerPreferences {
    fn set_base_unit(&self, id: UnitId) -> BaseUnitAnswer;
}

/// What only the Shell does once Settings › Ledger has set the base Unit: every view of the Units
/// is the Client's to bring in line.
pub trait LedgerRouter {
    /// The base Unit was saved, so the Client announces the Units domain.
    fn base_unit_changed(&mut self);
}

/// A Unit's figure as a list page's headline quotes it: the amount, the places the Unit shows
/// and its code, formatted by the summary so the port stays free of the Locale.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeadlineAmount {
    pub amount: Money,
    pub places: u32,
    pub code: String,
}

/// What the list pages' summaries count in the Open Ledger: each page's count and its one headline
/// fact, matching the page's heading note. Counts only, never rows.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LedgerCounts {
    pub units: usize,
    /// The base Unit's code, which the Ledger page quotes.
    pub base_unit: Option<String>,
    pub institutions: usize,
    pub accounts: usize,
    /// The Accounts counted in the base Unit, summed; `None` when the base Unit is unknown.
    pub net_worth: Option<HeadlineAmount>,
    pub categories: usize,
    /// How many levels the Category tree runs to.
    pub category_levels: u32,
    /// The active Tags.
    pub tags: usize,
    pub likely_duplicate_tags: usize,
    /// The active Payees.
    pub payees: usize,
    pub payees_without_default_category: usize,
    pub document_types: usize,
    pub files_typed: usize,
    pub files: usize,
    pub locations: usize,
    pub areas: usize,
    pub items: usize,
}

/// What the Settings index's detail pane counts of the domains Settings doesn't own (#1109): the
/// Client answers it over the stores (`lib-database`'s `StoredSettingsReads`), so the index never
/// builds a domain's Presenter. Read only for the pane's page, and not at all with no Ledger open.
pub trait LedgerSummaries {
    fn counts(&self) -> LedgerCounts;
}
