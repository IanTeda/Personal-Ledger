//! A recording Router, an in-memory [`DeviceConfiguration`] and an in-memory [`TracingLog`], for the Presenters' tests and a
//! Client's UI-first work.

use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::rc::Rc;
use std::time::SystemTime;

use lib_core::UnitId;
use lib_core::configuration::{ConfigKey, ConfigValue};
use lib_toast::{Notice, Notify};

use crate::log::{LogLevel, LogLine};
use crate::page::SettingsPage;
use crate::presenter::{ClearLogsPresenter, DisplayOutput, LogListChange, LogScroll};
use crate::protocols::{
    BaseUnitAnswer, HeadlineAmount, LedgerCounts, LedgerPreferences, LedgerRouter, LedgerSummaries,
    LedgerUnits, UnitChoice,
};
use crate::protocols::{
    ColourChoice, DeviceAnswer, DeviceConfiguration, DeviceRefusal, DeviceRouter, DeviceSnapshot,
    KeyState, SettingsRouter, TracingLog, TracingRouter, writes_when_shadowed,
};

/// One thing a Presenter asked of the Shell, in the order asked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Routed {
    PageChanged(SettingsPage),
    PickerOpened,
    PickerClosed,
    PageSelected(SettingsPage),
    KeyboardLeft(SettingsPage),
    KeyboardEntered(SettingsPage),
    IndexRevealed(SettingsPage),
    ToastsChanged(bool),
    DisplayChanged(DisplayOutput),
    ColourChanged(ColourChoice),
    TestToasts,
    PickDataFolder,
    LogScrolled(LogScroll),
    LogChanged(LogListChange),
    ClearLogsOpened,
    BaseUnitChanged,
    Notice(Notice),
}

impl Routed {
    /// The frame reporting the selected page, the keyboard or the index, for the Client's pages
    /// and scrolls rather than the Shell.
    fn is_keyboard(&self) -> bool {
        matches!(
            self,
            Self::PageSelected(_)
                | Self::KeyboardLeft(_)
                | Self::KeyboardEntered(_)
                | Self::IndexRevealed(_)
        )
    }
}

/// A [`SettingsRouter`] and [`DeviceRouter`] that records every call.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RecordingRouter {
    pub routed: Vec<Routed>,
}

impl RecordingRouter {
    /// The pages announced, in order.
    pub fn pages_changed(&self) -> Vec<SettingsPage> {
        self.routed
            .iter()
            .filter_map(|routed| match routed {
                Routed::PageChanged(page) => Some(*page),
                _ => None,
            })
            .collect()
    }

    /// What the frame reported of the selected page, the keyboard and the index, in order.
    pub fn keyboard(&self) -> Vec<Routed> {
        self.routed
            .iter()
            .filter(|routed| routed.is_keyboard())
            .cloned()
            .collect()
    }

    /// Everything else asked of the Shell, in order: history, the picker's mode and Notices.
    pub fn shell(&self) -> Vec<Routed> {
        self.routed
            .iter()
            .filter(|routed| !routed.is_keyboard())
            .cloned()
            .collect()
    }

    /// The status-line messages sent, in order.
    pub fn statuses(&self) -> Vec<String> {
        self.routed
            .iter()
            .filter_map(|routed| match routed {
                Routed::Notice(Notice::Status(text)) => Some(text.clone()),
                _ => None,
            })
            .collect()
    }

    /// The Display outputs handed the Shell, in order.
    pub fn displays(&self) -> Vec<DisplayOutput> {
        self.routed
            .iter()
            .filter_map(|routed| match routed {
                Routed::DisplayChanged(display) => Some(*display),
                _ => None,
            })
            .collect()
    }
}

impl Notify for RecordingRouter {
    fn notify(&mut self, notice: Notice) {
        self.routed.push(Routed::Notice(notice));
    }
}

impl SettingsRouter for RecordingRouter {
    fn page_changed(&mut self, page: SettingsPage) {
        self.routed.push(Routed::PageChanged(page));
    }

    fn picker_opened(&mut self) {
        self.routed.push(Routed::PickerOpened);
    }

    fn picker_closed(&mut self) {
        self.routed.push(Routed::PickerClosed);
    }

    fn page_selected(&mut self, page: SettingsPage) {
        self.routed.push(Routed::PageSelected(page));
    }

    fn keyboard_left(&mut self, page: SettingsPage) {
        self.routed.push(Routed::KeyboardLeft(page));
    }

    fn keyboard_entered(&mut self, page: SettingsPage) {
        self.routed.push(Routed::KeyboardEntered(page));
    }

    fn index_revealed(&mut self, page: SettingsPage) {
        self.routed.push(Routed::IndexRevealed(page));
    }
}

impl DeviceRouter for RecordingRouter {
    fn toasts_changed(&mut self, on: bool) {
        self.routed.push(Routed::ToastsChanged(on));
    }

    fn display_changed(&mut self, display: DisplayOutput) {
        self.routed.push(Routed::DisplayChanged(display));
    }

    fn colour_changed(&mut self, colour: ColourChoice) {
        self.routed.push(Routed::ColourChanged(colour));
    }

    fn test_toasts(&mut self) {
        self.routed.push(Routed::TestToasts);
    }

    fn pick_data_folder(&mut self) {
        self.routed.push(Routed::PickDataFolder);
    }
}

impl LedgerRouter for RecordingRouter {
    fn base_unit_changed(&mut self) {
        self.routed.push(Routed::BaseUnitChanged);
    }
}

/// The Units Settings › Ledger reads and the base Unit it writes, in memory: the sample's aud, btc
/// and vas with aud the base. Clones share the rows, so a Presenter's write reads back.
#[derive(Debug, Clone)]
pub struct InMemoryLedgerUnits {
    rows: Rc<RefCell<LedgerRows>>,
}

#[derive(Debug)]
struct LedgerRows {
    /// `(choice, is_active)`, in code order.
    units: Vec<(UnitChoice, bool)>,
    base: UnitId,
}

impl InMemoryLedgerUnits {
    /// aud, btc and vas, all active, with aud the base.
    pub fn sample() -> Self {
        let unit = |id, code: &str, name: &str| {
            (
                UnitChoice {
                    id,
                    code: code.to_string(),
                    name: name.to_string(),
                },
                true,
            )
        };
        Self {
            rows: Rc::new(RefCell::new(LedgerRows {
                units: vec![
                    unit(UnitId::SEED_AUD, "aud", "Australian dollar"),
                    unit(UnitId::SEED_BTC, "btc", "Bitcoin"),
                    unit(UnitId::SEED_VAS, "vas", "Vanguard Australian Shares"),
                ],
                base: UnitId::SEED_AUD,
            })),
        }
    }

    /// Deactivates `id`, as the Units page does.
    pub fn deactivate(&self, id: UnitId) {
        for (choice, active) in &mut self.rows.borrow_mut().units {
            if choice.id == id {
                *active = false;
            }
        }
    }
}

impl LedgerUnits for InMemoryLedgerUnits {
    fn choices(&self) -> Vec<UnitChoice> {
        self.rows
            .borrow()
            .units
            .iter()
            .filter(|(_, active)| *active)
            .map(|(choice, _)| choice.clone())
            .collect()
    }

    fn base_unit(&self) -> Option<UnitId> {
        Some(self.rows.borrow().base)
    }
}

impl LedgerPreferences for InMemoryLedgerUnits {
    fn set_base_unit(&self, id: UnitId) -> BaseUnitAnswer {
        let mut rows = self.rows.borrow_mut();
        if rows
            .units
            .iter()
            .any(|(choice, active)| choice.id == id && *active)
        {
            rows.base = id;
            BaseUnitAnswer::Saved
        } else {
            BaseUnitAnswer::Refused
        }
    }
}

impl TracingRouter for RecordingRouter {
    fn scroll(&mut self, scroll: LogScroll) {
        self.routed.push(Routed::LogScrolled(scroll));
    }

    fn log_changed(&mut self, change: LogListChange) {
        self.routed.push(Routed::LogChanged(change));
    }

    fn open_clear_logs(&mut self, _dialog: ClearLogsPresenter) {
        self.routed.push(Routed::ClearLogsOpened);
    }
}

/// What a [`DeviceConfiguration`]'s file holds, a value per key it sets.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StoredDevice(pub HashMap<ConfigKey, ConfigValue>);

impl StoredDevice {
    /// The file holding each of `values` under its key.
    pub fn of(values: impl IntoIterator<Item = ConfigValue>) -> Self {
        Self(
            values
                .into_iter()
                .map(|value| (value.key(), value))
                .collect(),
        )
    }

    pub fn get(&self, key: ConfigKey) -> Option<&ConfigValue> {
        self.0.get(&key)
    }
}

/// A configuration file in memory: the settings written, the values start-up read, the keys a
/// source above it sets, and whether there is a file to write at all or writes fail.
#[derive(Debug, Default)]
pub struct InMemoryDeviceConfiguration {
    /// The file a write names; `None` refuses every write, as with no home folder.
    pub file: Option<PathBuf>,
    /// The values in force, as start-up read them; a write leaves them, as it does a run's.
    pub effective: StoredDevice,
    /// The keys a source above the file sets, with the source's name.
    pub shadows: Vec<(ConfigKey, String)>,
    /// Makes every write fail, as a file that can't be written does.
    pub fail_writes: bool,
    pub stored: RefCell<StoredDevice>,
}

impl InMemoryDeviceConfiguration {
    /// A file that takes every write.
    pub fn writable() -> Self {
        Self {
            file: Some(PathBuf::from("personal-ledger.conf")),
            ..Self::default()
        }
    }

    /// [`Self::writable`], with `key` set by `source` above the file.
    pub fn shadowing(key: ConfigKey, source: &str) -> Self {
        Self {
            shadows: vec![(key, source.to_string())],
            ..Self::writable()
        }
    }

    /// `self`, with start-up having read each of `values`.
    pub fn reading(self, values: impl IntoIterator<Item = ConfigValue>) -> Self {
        Self {
            effective: StoredDevice::of(values),
            ..self
        }
    }

    /// `self`, with the file holding each of `values`.
    pub fn storing(self, values: impl IntoIterator<Item = ConfigValue>) -> Self {
        Self {
            stored: RefCell::new(StoredDevice::of(values)),
            ..self
        }
    }

    pub fn stored(&self) -> StoredDevice {
        self.stored.borrow().clone()
    }

    fn shadow(&self, key: ConfigKey) -> Option<String> {
        self.shadows
            .iter()
            .find(|(shadowed, _)| *shadowed == key)
            .map(|(_, source)| source.clone())
    }

    /// Changes `key` in the file with `edit`, answering `value`, unless the key is shadowed,
    /// there's no file, or writes fail.
    fn change<T>(
        &self,
        key: ConfigKey,
        value: T,
        edit: impl FnOnce(&mut StoredDevice),
    ) -> DeviceAnswer<T> {
        if !writes_when_shadowed(key)
            && let Some(source) = self.shadow(key)
        {
            return DeviceAnswer::Shadowed { source };
        }
        let Some(file) = &self.file else {
            return DeviceAnswer::Refused(DeviceRefusal::NoFile);
        };
        if self.fail_writes {
            return DeviceAnswer::Refused(DeviceRefusal::WriteFailed);
        }
        edit(&mut self.stored.borrow_mut());
        DeviceAnswer::Saved {
            value,
            file: file.display().to_string(),
        }
    }
}

impl DeviceConfiguration for InMemoryDeviceConfiguration {
    fn read(&self) -> DeviceSnapshot {
        let stored = self.stored.borrow();
        DeviceSnapshot::new(self.file.clone(), |key| KeyState {
            stored: stored.get(key).cloned(),
            effective: self.effective.get(key).cloned(),
            shadow: self.shadow(key),
        })
    }

    fn write(&self, value: ConfigValue) -> DeviceAnswer<ConfigValue> {
        let written = value.clone();
        self.change(value.key(), value, |stored| {
            stored.0.insert(written.key(), written);
        })
    }

    fn remove(&self, key: ConfigKey) -> DeviceAnswer<()> {
        self.change(key, (), |stored| {
            stored.0.remove(&key);
        })
    }
}

/// A log capture in memory: a ring of `capacity` lines whose `seq` keeps counting up past a clear.
#[derive(Debug)]
pub struct InMemoryTracingLog {
    capacity: usize,
    lines: RefCell<VecDeque<Rc<LogLine>>>,
    next_seq: RefCell<u64>,
}

impl InMemoryTracingLog {
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            lines: RefCell::default(),
            next_seq: RefCell::default(),
        }
    }

    /// Captures an event with no fields, evicting the oldest line when the ring is full.
    pub fn push(&self, level: LogLevel, target: &str, message: &str) {
        let mut next_seq = self.next_seq.borrow_mut();
        let mut lines = self.lines.borrow_mut();
        if self.capacity == 0 {
            return;
        }
        if lines.len() == self.capacity {
            lines.pop_front();
        }
        lines.push_back(Rc::new(LogLine {
            seq: *next_seq,
            time: SystemTime::UNIX_EPOCH,
            level,
            target: target.to_string(),
            message: message.to_string(),
            fields: Vec::new(),
        }));
        *next_seq += 1;
    }
}

impl TracingLog for InMemoryTracingLog {
    fn capacity(&self) -> usize {
        self.capacity
    }

    fn since(&self, from: u64) -> Vec<Rc<LogLine>> {
        self.lines
            .borrow()
            .iter()
            .filter(|line| line.seq >= from)
            .cloned()
            .collect()
    }

    fn next_seq(&self) -> u64 {
        *self.next_seq.borrow()
    }

    fn clear(&self) {
        self.lines.borrow_mut().clear();
    }
}

/// A [`LedgerSummaries`] holding the counts it is given, for the index summaries' tests and a
/// Client's UI-first work.
#[derive(Debug, Clone, Default)]
pub struct FixedLedgerSummaries(pub LedgerCounts);

impl FixedLedgerSummaries {
    /// The Sample Ledger's counts as the mockups draw them: 3 units over aud, 7 institutions, 7
    /// accounts worth 83,995.87 aud, 12 categories 3 levels deep, 18 tags with 2 likely
    /// duplicates, 11 payees with 3 lacking a default category, 7 types with 392 of 412 files
    /// typed, and 3 locations, 14 areas and 241 items.
    #[expect(
        clippy::expect_used,
        reason = "the net worth is a decimal literal that always parses"
    )]
    pub fn sample() -> Self {
        Self(LedgerCounts {
            units: 3,
            base_unit: Some("aud".to_string()),
            institutions: 7,
            accounts: 7,
            net_worth: Some(HeadlineAmount {
                amount: lib_core::Money("83995.87".parse().expect("a decimal literal")),
                places: 2,
                code: "aud".to_string(),
            }),
            categories: 12,
            category_levels: 3,
            tags: 18,
            likely_duplicate_tags: 2,
            payees: 11,
            payees_without_default_category: 3,
            document_types: 7,
            files_typed: 392,
            files: 412,
            locations: 3,
            areas: 14,
            items: 241,
        })
    }
}

impl LedgerSummaries for FixedLedgerSummaries {
    fn counts(&self) -> LedgerCounts {
        self.0.clone()
    }
}
