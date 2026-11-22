//! What the Settings module asks of the Shell and of the device, through ports each Client
//! implements. [`SettingsRouter`] is the frame's and [`DeviceRouter`] the This device pages', each
//! lent to every `handle` call; a refusal or a saved setting goes to the status line through
//! [`Notify`] (ADR-0048). A device's settings, Configuration in the user's configuration file (ADR-0053), are
//! reached through [`DeviceFile`](crate::DeviceFile), which the Client answers over its own
//! configuration writer.

use std::path::PathBuf;
use std::rc::Rc;

use lib_colour_theme::ColourAppearance;
use lib_core::configuration::{ConfigKey, ConfigValue};
use lib_core::{FinancialYearStart, LedgerName, Money, Owner, UnitId, UnitRef};
use lib_toast::Notify;

use crate::error::Error;
use crate::log::LogLine;
use crate::page::SettingsPage;
use crate::presenter::{
    ClearLogsPresenter, DisplayOutput, GeneralSettings, LogListChange, LogScroll,
};

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

    /// `page` became the selected page, on show or under the index's cursor, by anything but a
    /// restore, so its body starts at its top when it shows. A restored page keeps its scroll.
    fn page_selected(&mut self, page: SettingsPage);

    /// `page` lost the keyboard, to the index, to another page, or to its own reopening.
    fn keyboard_left(&mut self, page: SettingsPage);

    /// `page` has the keyboard, newly or reopened, so it puts it back on its first control. Sent
    /// after any [`Self::keyboard_left`] of the same input.
    fn keyboard_entered(&mut self, page: SettingsPage);

    /// The index shows with its cursor on `page`, newly placed there, so the Client scrolls its row
    /// into view.
    fn index_revealed(&mut self, page: SettingsPage);
}

/// What only the Shell does once a This device page has changed a setting: the Toasts, the colours
/// and the Views that draw with the Display output are the Client's.
pub trait DeviceRouter: Notify {
    /// Toasts were turned on or off and saved, so the Shell's Toasts take it.
    fn toasts_changed(&mut self, on: bool);

    /// The Display output changed, so the Shell hands it to every View that draws with it.
    fn display_changed(&mut self, display: DisplayOutput);

    /// The Colour Theme in effect changed, saved or put back by Undo or Redo, so the Client
    /// applies it to every window. Raw, so `None` is the default and an id this release doesn't
    /// know stays as written.
    fn colour_theme_changed(&mut self, colour_theme: Option<String>);

    /// The Appearance in effect changed, saved or put back, so the Client applies it to every
    /// window.
    fn colour_appearance_changed(&mut self, appearance: ColourAppearance);

    /// The Data folder in effect changed by Undo or Redo, so `:open` and `:new` start there. A
    /// pick through `GeneralPresenter::set_data_folder` answers its outcome instead.
    fn data_folder_changed(&mut self, folder: PathBuf);

    /// General's **Test** asked for the Toasts test, which the Shell's Toasts raise (DESKTOP-184).
    fn test_toasts(&mut self);

    /// General's **Change…** asked for the Data folder picker (DESKTOP-187), which the Shell's
    /// file explorer is. What it picks comes back to `GeneralPresenter::set_data_folder`.
    fn pick_data_folder(&mut self);
}

/// A Colour Theme or Appearance chosen on Display or by the palette.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColourChoice {
    /// A built-in Colour Theme, by its id.
    Theme(&'static str),
    Appearance(ColourAppearance),
}

/// What a device holds of one key.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct KeyState {
    /// The configuration file's own value, whatever outranks it; `None` when the file doesn't set
    /// the key.
    pub stored: Option<ConfigValue>,
    /// The value in force, as start-up's parse read it from every source; `None` for a key with
    /// no default that nothing sets.
    pub effective: Option<ConfigValue>,
    /// What sets the key above the file, by name, or `None` when a write to the file applies.
    pub shadow: Option<String>,
}

/// Every key Settings writes as a device holds it, read at once, and the file a write goes to.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DeviceSnapshot {
    /// The user's configuration file; `None` with no home folder to keep one in.
    pub file: Option<PathBuf>,
    /// One per [`ConfigKey::ALL`], in its order.
    keys: [KeyState; ConfigKey::ALL.len()],
}

impl DeviceSnapshot {
    /// A snapshot of `file` with each key's state from `state`.
    pub fn new(file: Option<PathBuf>, mut state: impl FnMut(ConfigKey) -> KeyState) -> Self {
        Self {
            file,
            keys: ConfigKey::ALL.map(&mut state),
        }
    }

    pub fn get(&self, key: ConfigKey) -> &KeyState {
        &self.keys[index(key)]
    }

    /// The value `key`'s control shows: the file's for a key that [writes when
    /// shadowed](ConfigKey::writes_when_shadowed), since that is what a write changes, else the
    /// value in force.
    pub fn shown(&self, key: ConfigKey) -> Option<&ConfigValue> {
        let state = self.get(key);
        if key.writes_when_shadowed() {
            state.stored.as_ref()
        } else {
            state.effective.as_ref()
        }
    }

    /// Whether non-Error Toasts show: Toasts' value in force, else [`GeneralSettings`]' default.
    /// The value in force for a reader other than a control (the Shell starts its Toasts on it);
    /// Settings › General reads its own value through [`DeviceSnapshot::shown`].
    pub fn toasts_on(&self) -> bool {
        match self.get(ConfigKey::Toasts).effective {
            Some(ConfigValue::Toasts(on)) => on,
            _ => GeneralSettings::default().toasts_on,
        }
    }

    /// The Data folder in effect, where `:open` and `:new` start: Data's value in force, whatever
    /// the file names; `None` when nothing sets it. A value in force for a reader other than a
    /// control, as [`DeviceSnapshot::toasts_on`] is; Settings › General reads its own value through
    /// [`DeviceSnapshot::shown`], the file's, since Data writes when shadowed.
    pub fn data_folder(&self) -> Option<&std::path::Path> {
        match &self.get(ConfigKey::Data).effective {
            Some(ConfigValue::Data(folder)) => Some(folder),
            _ => None,
        }
    }
}

/// `key`'s place in [`ConfigKey::ALL`].
fn index(key: ConfigKey) -> usize {
    // The discriminant, not a search that would need a fallback: `ALL` lists the keys in
    // declaration order, which `each_key_sits_at_its_place_in_all` holds.
    key as usize
}

/// How a write of a device's setting went: an answer for the person, never a failure for the
/// caller, so the port stays infallible (ADR-0046).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeviceAnswer<T> {
    /// Written: the value now in the file, and the file, as the status line names it.
    Saved { value: T, file: String },
    /// Not written, since a source above the file sets the key: a flag, an environment variable or
    /// a higher configuration file, named by `source`. Never the answer for a key that
    /// [writes when shadowed](ConfigKey::writes_when_shadowed).
    Shadowed { source: String },
    /// Not written, for the reason given.
    Refused(Error),
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
/// capture ring, which fills from any thread and signals the page ([`TracingEvent::LogChanged`])
/// when entries arrive.
///
/// [`TracingEvent::LogChanged`]: crate::TracingEvent::LogChanged
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

/// What the Client hands the Ledger page and the Settings index as one value: the Units read, the
/// summaries read and the base Unit write, all trait objects the persistence layer fills.
#[derive(Clone)]
pub struct SettingsLedger {
    pub units: Rc<dyn LedgerReads>,
    pub summaries: Rc<dyn LedgerSummaries>,
    pub preferences: Rc<dyn LedgerPreferences>,
}

/// What Settings › Ledger reads of the Units: the Client answers it over the Units store
/// (through `LedgerPorts::settings_ledger()`), so the page never holds a copy.
pub trait LedgerReads {
    /// The Units the Base unit select offers: the active ones, by code.
    fn choices(&self) -> Vec<UnitRef>;

    /// The Ledger's base Unit.
    fn base_unit(&self) -> Option<UnitId>;

    /// The Ledger's name, from the Ledger store.
    fn name(&self) -> LedgerName;

    /// Who the Ledger belongs to, if anyone.
    fn owner(&self) -> Option<Owner>;

    /// The month the Ledger's Financial year starts in.
    fn financial_year_start(&self) -> FinancialYearStart;
}

/// How a write of the base Unit went.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BaseUnitAnswer {
    Saved,
    /// The Unit was removed or deactivated under the select, which repaints without it.
    Refused,
}

/// Writes the Ledger's Preferences: its base Unit, name, Owner and Financial year start. The last
/// three are not refusable, since their value types have already checked the value.
pub trait LedgerPreferences {
    fn set_base_unit(&self, id: UnitId) -> BaseUnitAnswer;

    fn set_name(&self, name: LedgerName);

    /// `None` clears the Owner.
    fn set_owner(&self, owner: Option<Owner>);

    fn set_financial_year_start(&self, start: FinancialYearStart);
}

/// What only the Shell does once Settings › Ledger has set the base Unit: every view of the Units
/// is the Client's to bring in line. What History calls the change goes through [`Notify`].
pub trait LedgerRouter: Notify {
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
/// Client answers it over the stores (through `LedgerPorts::settings_ledger()`), so the index never
/// builds a domain's Presenter. Read only for the pane's page, and not at all with no Ledger open.
pub trait LedgerSummaries {
    fn counts(&self) -> LedgerCounts;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_key_sits_at_its_place_in_all() {
        for (place, key) in ConfigKey::ALL.into_iter().enumerate() {
            assert_eq!(index(key), place, "{key:?}");
        }
    }

    #[test]
    fn the_data_folder_in_effect_is_datas_value_in_force_not_the_files() {
        let snapshot = DeviceSnapshot::new(None, |key| match key {
            ConfigKey::Data => KeyState {
                stored: Some(ConfigValue::Data(PathBuf::from("/home/me/finance"))),
                effective: Some(ConfigValue::Data(PathBuf::from("/home/me/ledgers"))),
                shadow: Some("--data".to_string()),
            },
            _ => KeyState::default(),
        });

        assert_eq!(
            snapshot.data_folder(),
            Some(std::path::Path::new("/home/me/ledgers"))
        );
        assert_eq!(DeviceSnapshot::default().data_folder(), None);
    }

    #[test]
    fn toasts_on_with_no_value_in_force_is_generals_default() {
        assert_eq!(
            DeviceSnapshot::default().toasts_on(),
            GeneralSettings::default().toasts_on
        );
    }

    #[test]
    fn a_control_shows_the_value_in_force_but_the_files_for_a_key_written_while_shadowed() {
        let snapshot = DeviceSnapshot::new(None, |key| match key {
            ConfigKey::Toasts => KeyState {
                stored: Some(ConfigValue::Toasts(true)),
                effective: Some(ConfigValue::Toasts(false)),
                shadow: Some("--no-toasts".to_string()),
            },
            ConfigKey::Locale => KeyState {
                stored: Some(ConfigValue::Locale("en-GB".to_string())),
                effective: Some(ConfigValue::Locale("en-US".to_string())),
                shadow: Some("--locale en-US".to_string()),
            },
            _ => KeyState::default(),
        });

        assert_eq!(
            snapshot.shown(ConfigKey::Toasts),
            Some(&ConfigValue::Toasts(false))
        );
        assert_eq!(
            snapshot.shown(ConfigKey::Locale),
            Some(&ConfigValue::Locale("en-GB".to_string()))
        );
        assert_eq!(snapshot.shown(ConfigKey::Data), None);
        assert_eq!(
            snapshot.get(ConfigKey::Toasts).shadow.as_deref(),
            Some("--no-toasts")
        );
    }
}
