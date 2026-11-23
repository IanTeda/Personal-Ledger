//! The contracts the device's ports keep. The write-through policy runs once, in the Interactors
//! over the files a [`DeviceFileFactory`] builds ([`run_all_policy`]), the in-memory one here and the
//! Client's. The raw [`DeviceFile`] contract runs against both the
//! in-memory file here and the Client's over its configuration file ([`run_all_device_file`]), so the
//! fake the Presenters' tests use can be trusted; it is generic over a [`DeviceFileFactory`], since
//! the Client's file needs its own set-up (a temporary file per check). Each check writes through
//! the port, reads it back, and expects what was saved, and only that. The writes run over
//! every [`ConfigKey`], so a new key is checked with no new arm here.

use std::path::PathBuf;

use lib_core::configuration::{ConfigKey, ConfigValue};
use lib_core::{
    ColourAppearance, DateStyle, FinancialYearStart, LedgerName, Levels, OnStart, Owner,
    RowDensity, StartView, StatusGlyphs, UnitId,
};

use std::rc::Rc;

use crate::device::{DeviceFile, DevicePorts};
use crate::error::Error;
use crate::fake::StoredDevice;
use crate::interactor::{RemoveSetting, WriteSetting};
use crate::log::LogLevel;
use crate::protocols::{BaseUnitAnswer, DeviceAnswer, LedgerPreferences, LedgerReads, TracingLog};

/// Builds the raw file a [`DeviceFile`] contract check runs on, each on a file of its own.
pub trait DeviceFileFactory {
    type File: DeviceFile + 'static;

    /// A file that takes writes, empty to start.
    fn writable(&mut self) -> Self::File;

    /// [`Self::writable`], with `key` set by a source above the file, and that source's name as
    /// the file says it.
    fn shadowing(&mut self, key: ConfigKey) -> (Self::File, String);

    /// A file with no place to be written, as with no home folder.
    fn without_file(&mut self) -> Self::File;

    /// A file that can't be written.
    fn unwritable(&mut self) -> Self::File;
}

/// Two values for every key, in [`ConfigKey::ALL`]'s order: the first other than its default, the
/// second other than the first, the Locale's default date style among the seconds.
fn sample_pairs() -> Vec<(ConfigValue, ConfigValue)> {
    vec![
        (
            ConfigValue::OnStart(OnStart::NoLedger),
            ConfigValue::OnStart(OnStart::ReopenLastLedger),
        ),
        (
            ConfigValue::StartView(StartView::Noun("bills".into())),
            ConfigValue::StartView(StartView::Dashboard),
        ),
        (ConfigValue::Toasts(false), ConfigValue::Toasts(true)),
        (
            ConfigValue::Locale("en-GB".to_string()),
            ConfigValue::Locale("en-US".to_string()),
        ),
        (
            ConfigValue::Data(PathBuf::from("/home/me/finance")),
            ConfigValue::Data(PathBuf::from("/home/me/money")),
        ),
        (
            ConfigValue::ColourTheme("nord".to_string()),
            ConfigValue::ColourTheme("gruvbox".to_string()),
        ),
        (
            ConfigValue::ColourAppearance(ColourAppearance::Light),
            ConfigValue::ColourAppearance(ColourAppearance::Dark),
        ),
        (
            ConfigValue::DateStyle(Some(DateStyle::Iso)),
            ConfigValue::DateStyle(None),
        ),
        (
            ConfigValue::RowDensity(RowDensity::Compact),
            ConfigValue::RowDensity(RowDensity::Roomy),
        ),
        (
            ConfigValue::StatusGlyphs(StatusGlyphs::AsciiFallback),
            ConfigValue::StatusGlyphs(StatusGlyphs::Unicode),
        ),
        (
            ConfigValue::StartSidebarMinimised(true),
            ConfigValue::StartSidebarMinimised(false),
        ),
        (ConfigValue::HintBar(false), ConfigValue::HintBar(true)),
        (
            ConfigValue::TerminalColours(true),
            ConfigValue::TerminalColours(false),
        ),
        (
            ConfigValue::Log(Levels::DEBUG),
            ConfigValue::Log(Levels::ERROR),
        ),
        (
            ConfigValue::LogFilePath(PathBuf::from("/home/me/ledger.log")),
            ConfigValue::LogFilePath(PathBuf::from("/home/me/other.log")),
        ),
    ]
}

/// A value for every key, each other than its default, in [`ConfigKey::ALL`]'s order.
fn first_values() -> Vec<ConfigValue> {
    sample_pairs().into_iter().map(|(first, _)| first).collect()
}

/// A second value for every key, each other than [`first_values`]', the Locale's default date
/// style among them.
fn second_values() -> Vec<ConfigValue> {
    sample_pairs()
        .into_iter()
        .map(|(_, second)| second)
        .collect()
}

/// The in-memory file and the ports the Interactors write it through.
struct Rig {
    ports: DevicePorts,
}

impl Rig {
    fn over(file: impl DeviceFile + 'static) -> Self {
        let file: Rc<dyn DeviceFile> = Rc::new(file);
        Self {
            ports: DevicePorts::new(file),
        }
    }

    fn write(&self, value: ConfigValue) -> DeviceAnswer<ConfigValue> {
        WriteSetting(value).run(&self.ports)
    }

    fn remove(&self, key: ConfigKey) -> DeviceAnswer<()> {
        RemoveSetting(key).run(&self.ports)
    }
}

/// What the rig's file holds, as its read says.
fn stored(device: &Rig) -> StoredDevice {
    let snapshot = device.ports.snapshot();
    StoredDevice(
        ConfigKey::ALL
            .into_iter()
            .filter_map(|key| Some((key, snapshot.get(key).stored.clone()?)))
            .collect(),
    )
}

/// The name the in-memory file gives a source above it that sets `key`.
fn shadow_source(key: ConfigKey) -> String {
    format!("PERSONAL_LEDGER_{}", key.name().to_uppercase())
}

/// What `file` holds, as its raw read says.
fn held(file: &impl DeviceFile) -> StoredDevice {
    StoredDevice(file.stored_all().into_iter().collect())
}

/// Every key written through a raw file reads back, and a later write replaces it.
pub fn every_key_is_written_and_read_back(factory: &mut impl DeviceFileFactory) {
    let file = factory.writable();
    for value in first_values() {
        assert_eq!(file.set(value), Ok(()));
    }
    assert_eq!(held(&file), StoredDevice::of(first_values()));

    for value in second_values() {
        assert_eq!(file.set(value), Ok(()));
    }
    assert_eq!(held(&file), StoredDevice::of(second_values()));
}

/// A removed key reads back unset and the others stay; removing it again is no error.
pub fn a_removed_key_reads_back_unset(factory: &mut impl DeviceFileFactory) {
    let file = factory.writable();
    for value in first_values() {
        assert_eq!(file.set(value), Ok(()));
    }
    for key in ConfigKey::ALL {
        assert_eq!(file.remove(key), Ok(()), "{key:?} removed");
        assert_eq!(file.remove(key), Ok(()), "{key:?} removed again");
        assert_eq!(file.raw(key), None, "{key:?} taken out");
    }
    assert_eq!(held(&file), StoredDevice::default());
}

/// A restore puts a key back as its raw text was read, and takes it out for `None`.
pub fn a_restore_puts_the_raw_value_back(factory: &mut impl DeviceFileFactory) {
    let file = factory.writable();
    for value in first_values() {
        let key = value.key();
        assert_eq!(file.set(value), Ok(()));
        let raw = file.raw(key);
        assert!(raw.is_some(), "{key:?} is read raw once written");

        assert_eq!(file.remove(key), Ok(()));
        assert_eq!(file.raw(key), None, "{key:?}");
        assert_eq!(file.restore(key, raw.as_deref()), Ok(()));
        assert_eq!(file.raw(key), raw, "{key:?} restored");

        assert_eq!(file.restore(key, None), Ok(()));
        assert_eq!(file.raw(key), None, "{key:?} restored to unset");
    }
}

/// The file reports where it is, and what sets a key above it and no other.
pub fn the_file_reports_its_target_and_shadows(factory: &mut impl DeviceFileFactory) {
    assert!(factory.writable().target().is_some());
    assert_eq!(factory.without_file().target(), None);

    for value in first_values() {
        let key = value.key();
        let (file, source) = factory.shadowing(key);
        assert_eq!(
            file.shadow(key).as_deref(),
            Some(source.as_str()),
            "{key:?}"
        );
        for other in ConfigKey::ALL.into_iter().filter(|other| *other != key) {
            assert_eq!(file.shadow(other), None, "{other:?} under {key:?}");
        }
    }
}

/// A file that can't be written answers every change with an error, and holds nothing.
pub fn an_unwritable_file_is_an_error(factory: &mut impl DeviceFileFactory) {
    let file = factory.unwritable();
    for value in first_values() {
        let key = value.key();
        assert_eq!(file.set(value), Err(Error::WriteFailed), "{key:?}");
        assert_eq!(file.remove(key), Err(Error::WriteFailed), "{key:?} removed");
        assert_eq!(
            file.restore(key, None),
            Err(Error::WriteFailed),
            "{key:?} restored"
        );
    }
    assert_eq!(held(&file), StoredDevice::default());
}

/// Runs every [`DeviceFile`] check.
pub fn run_all_device_file(factory: &mut impl DeviceFileFactory) {
    every_key_is_written_and_read_back(factory);
    a_removed_key_reads_back_unset(factory);
    a_restore_puts_the_raw_value_back(factory);
    the_file_reports_its_target_and_shadows(factory);
    an_unwritable_file_is_an_error(factory);
}

/// Every key written through the policy answers saved with its value, and reads back as stored.
pub fn every_key_is_saved_and_reads_back(factory: &mut impl DeviceFileFactory) {
    let device = Rig::over(factory.writable());

    for value in first_values() {
        assert_saved(device.write(value.clone()), &value);
    }

    assert_eq!(stored(&device), StoredDevice::of(first_values()));
}

/// A later write replaces the value, and a removed key reads back unset, the others kept.
pub fn a_later_write_replaces_the_value(factory: &mut impl DeviceFileFactory) {
    let device = Rig::over(factory.writable());
    for value in first_values() {
        device.write(value);
    }

    for value in second_values() {
        assert_saved(device.write(value.clone()), &value);
    }
    assert_eq!(stored(&device), StoredDevice::of(second_values()));

    for key in ConfigKey::ALL {
        assert!(
            matches!(device.remove(key), DeviceAnswer::Saved { .. }),
            "{key:?} removed"
        );
        assert!(
            matches!(device.remove(key), DeviceAnswer::Saved { .. }),
            "{key:?} removed again"
        );
        assert_eq!(
            device.ports.snapshot().get(key).stored,
            None,
            "{key:?} taken out"
        );
    }
    assert_eq!(stored(&device), StoredDevice::default());
}

/// A key a source above the file sets reads that source, answers shadowed naming it, and is not
/// written; the other keys are not shadowed. The Locale and the Data folder are named too, but
/// still written ([`ConfigKey::writes_when_shadowed`]).
pub fn a_shadowed_key_reads_its_source_and_is_left_unwritten(factory: &mut impl DeviceFileFactory) {
    for value in first_values() {
        let key = value.key();
        let (file, source) = factory.shadowing(key);
        let device = Rig::over(file);

        let snapshot = device.ports.snapshot();
        assert_eq!(
            snapshot.get(key).shadow.as_deref(),
            Some(source.as_str()),
            "{key:?}"
        );
        for other in ConfigKey::ALL.into_iter().filter(|other| *other != key) {
            assert_eq!(snapshot.get(other).shadow, None, "{other:?} under {key:?}");
        }

        if key.writes_when_shadowed() {
            assert_saved(device.write(value.clone()), &value);
            assert_eq!(
                stored(&device),
                StoredDevice::of([value]),
                "{key:?} is written under its override"
            );
        } else {
            assert_eq!(
                source_of(device.write(value)).as_deref(),
                Some(source.as_str()),
                "{key:?}"
            );
            assert_eq!(
                source_of(device.remove(key)).as_deref(),
                Some(source.as_str()),
                "{key:?} removed"
            );
            assert_eq!(stored(&device), StoredDevice::default(), "{key:?}");
        }
    }
}

/// The read names the file a write goes to, the one a save names, and no file without one.
pub fn the_read_names_the_file(factory: &mut impl DeviceFileFactory) {
    let device = Rig::over(factory.writable());
    let Some(file) = device.ports.snapshot().file else {
        panic!("a writable device names its file");
    };
    match device.write(ConfigValue::Toasts(false)) {
        DeviceAnswer::Saved { file: saved, .. } => {
            assert_eq!(saved, file.display().to_string());
        }
        other => panic!("expected a save, got {other:?}"),
    }

    assert_eq!(
        Rig::over(factory.without_file()).ports.snapshot().file,
        None
    );
}

/// With no file to write, every write is refused.
pub fn with_no_file_every_write_is_refused(factory: &mut impl DeviceFileFactory) {
    assert_refused(&Rig::over(factory.without_file()), Error::NoFile);
}

/// A file that can't be written refuses every write.
pub fn a_failed_write_is_refused(factory: &mut impl DeviceFileFactory) {
    assert_refused(&Rig::over(factory.unwritable()), Error::WriteFailed);
}

/// Runs every policy check through the Interactors over the files `factory` builds, so the
/// Client's file and the fake the Presenters' tests use keep the same policy.
pub fn run_all_policy(factory: &mut impl DeviceFileFactory) {
    every_key_is_saved_and_reads_back(factory);
    a_later_write_replaces_the_value(factory);
    a_shadowed_key_reads_its_source_and_is_left_unwritten(factory);
    the_read_names_the_file(factory);
    with_no_file_every_write_is_refused(factory);
    a_failed_write_is_refused(factory);
}

fn assert_saved(answer: DeviceAnswer<ConfigValue>, expected: &ConfigValue) {
    match answer {
        DeviceAnswer::Saved { value, file } => {
            assert_eq!(&value, expected);
            assert!(!file.is_empty(), "a save names its file");
        }
        other => panic!("expected {expected:?} saved, got {other:?}"),
    }
}

fn source_of<T: std::fmt::Debug>(answer: DeviceAnswer<T>) -> Option<String> {
    match answer {
        DeviceAnswer::Shadowed { source } => Some(source),
        _ => None,
    }
}

fn assert_refused(device: &Rig, refusal: Error) {
    fn refused<T>(answer: DeviceAnswer<T>) -> Option<Error> {
        match answer {
            DeviceAnswer::Refused(refusal) => Some(refusal),
            _ => None,
        }
    }
    for value in first_values() {
        let key = value.key();
        assert_eq!(refused(device.write(value)), Some(refusal), "{key:?}");
        assert_eq!(
            refused(device.remove(key)),
            Some(refusal),
            "{key:?} removed"
        );
    }
    assert_eq!(stored(device), StoredDevice::default());
}

/// Builds the capture a [`TracingLog`] contract check runs on. The capture fills from the Client's
/// own side, not through the port, so the factory also says how an event is captured.
pub trait TracingLogFactory {
    type Log: TracingLog;

    /// An empty capture keeping `capacity` entries.
    fn with_capacity(&mut self, capacity: usize) -> Self::Log;

    /// Captures an event at `level` from `bin_desktop::shell`, with no fields.
    fn capture(&mut self, log: &Self::Log, level: LogLevel, message: &str);
}

fn messages(log: &impl TracingLog, from: u64) -> Vec<String> {
    log.since(from)
        .iter()
        .map(|line| line.message.clone())
        .collect()
}

/// `since` hands back what arrived at or after a `seq`, oldest first, keeping what was captured.
pub fn since_hands_back_what_arrived_oldest_first(factory: &mut impl TracingLogFactory) {
    let log = factory.with_capacity(8);
    factory.capture(&log, LogLevel::Info, "a");
    factory.capture(&log, LogLevel::Warn, "b");
    factory.capture(&log, LogLevel::Error, "c");

    let all = log.since(0);
    assert_eq!(messages(&log, 0), ["a", "b", "c"]);
    assert!(all.windows(2).all(|pair| pair[0].seq < pair[1].seq));
    assert_eq!(
        all.iter().map(|line| line.level).collect::<Vec<_>>(),
        [LogLevel::Info, LogLevel::Warn, LogLevel::Error]
    );
    assert_eq!(all[0].target, "bin_desktop::shell");
    assert_eq!(messages(&log, all[1].seq), ["b", "c"]);
    assert!(log.since(log.next_seq()).is_empty());
}

/// A full capture evicts its oldest entry, and says how many it keeps.
pub fn a_full_capture_evicts_the_oldest(factory: &mut impl TracingLogFactory) {
    let log = factory.with_capacity(2);
    for message in ["a", "b", "c"] {
        factory.capture(&log, LogLevel::Info, message);
    }

    assert_eq!(log.capacity(), 2);
    assert_eq!(messages(&log, 0), ["b", "c"]);
}

/// Clear empties the capture for every reader, and the `seq` keeps counting up, so a reader's
/// cursor stays valid.
pub fn clear_empties_and_seq_keeps_counting(factory: &mut impl TracingLogFactory) {
    let log = factory.with_capacity(8);
    factory.capture(&log, LogLevel::Info, "old");
    let next = log.next_seq();

    log.clear();
    assert!(log.since(0).is_empty());
    assert_eq!(log.next_seq(), next);

    factory.capture(&log, LogLevel::Info, "new");
    assert_eq!(messages(&log, next), ["new"]);
    assert_eq!(messages(&log, 0), ["new"]);
}

/// Runs every [`TracingLog`] check.
pub fn run_all_tracing_log(factory: &mut impl TracingLogFactory) {
    since_hands_back_what_arrived_oldest_first(factory);
    a_full_capture_evicts_the_oldest(factory);
    clear_empties_and_seq_keeps_counting(factory);
}

/// Builds the read and write ports Settings › Ledger runs on, over the same Units.
pub trait LedgerFactory {
    type Reads: LedgerReads;
    type Writes: LedgerPreferences;

    /// The sample Units (aud, btc, vas, aud the base) with those of `deactivated` made inactive,
    /// read and written through ports on one store.
    fn create(&mut self, deactivated: &[UnitId]) -> (Self::Reads, Self::Writes);
}

/// The choices are the active Units by code, and the base is the sample's.
pub fn the_choices_are_the_active_units_by_code(factory: &mut impl LedgerFactory) {
    let (reads, _) = factory.create(&[UnitId::SEED_BTC]);

    let codes: Vec<_> = reads.choices().into_iter().map(|unit| unit.code).collect();
    assert_eq!(codes, ["AUD", "VAS"]);
    assert_eq!(reads.base_unit(), Some(UnitId::SEED_AUD));
}

/// A base Unit that is saved reads back, and only it moves.
pub fn a_saved_base_unit_reads_back(factory: &mut impl LedgerFactory) {
    let (reads, writes) = factory.create(&[]);

    assert_eq!(
        writes.set_base_unit(UnitId::SEED_BTC),
        BaseUnitAnswer::Saved
    );

    assert_eq!(reads.base_unit(), Some(UnitId::SEED_BTC));
    assert_eq!(reads.choices().len(), 3, "no Unit is added or removed");
}

/// A Unit that is gone, or deactivated, is refused and the base stays.
pub fn a_unit_that_is_gone_or_inactive_is_refused(factory: &mut impl LedgerFactory) {
    let (reads, writes) = factory.create(&[UnitId::SEED_VAS]);

    assert_eq!(writes.set_base_unit(UnitId::new()), BaseUnitAnswer::Refused);
    assert_eq!(
        writes.set_base_unit(UnitId::SEED_VAS),
        BaseUnitAnswer::Refused
    );
    assert_eq!(reads.base_unit(), Some(UnitId::SEED_AUD));
}

/// The sample's name, Owner and Financial year start are read before any write.
pub fn the_preferences_read_as_the_sample_holds_them(factory: &mut impl LedgerFactory) {
    let (reads, _) = factory.create(&[]);

    assert_eq!(reads.name().as_str(), "Sample Ledger");
    assert_eq!(reads.owner(), Owner::new("Sam Sample"));
    assert_eq!(reads.financial_year_start(), FinancialYearStart::JULY);
}

/// A name, Owner and Financial year start that are written read back, and the base Unit stays.
#[expect(
    clippy::expect_used,
    reason = "the name is a short literal that is never blank"
)]
pub fn a_saved_preference_reads_back(factory: &mut impl LedgerFactory) {
    let (reads, writes) = factory.create(&[]);

    writes.set_name(LedgerName::new("Household").expect("a name"));
    writes.set_owner(Owner::new("Ian"));
    writes.set_financial_year_start(FinancialYearStart::APRIL);

    assert_eq!(reads.name().as_str(), "Household");
    assert_eq!(reads.owner(), Owner::new("Ian"));
    assert_eq!(reads.financial_year_start(), FinancialYearStart::APRIL);
    assert_eq!(reads.base_unit(), Some(UnitId::SEED_AUD));
}

/// No Owner clears the one there was.
pub fn no_owner_clears_the_owner(factory: &mut impl LedgerFactory) {
    let (reads, writes) = factory.create(&[]);

    writes.set_owner(None);

    assert_eq!(reads.owner(), None);
}

/// Runs every Settings › Ledger check on `factory`.
pub fn run_all_ledger(factory: &mut impl LedgerFactory) {
    the_preferences_read_as_the_sample_holds_them(factory);
    a_saved_preference_reads_back(factory);
    no_owner_clears_the_owner(factory);
    the_choices_are_the_active_units_by_code(factory);
    a_saved_base_unit_reads_back(factory);
    a_unit_that_is_gone_or_inactive_is_refused(factory);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fake::{InMemoryDeviceFile, InMemoryLedgerUnits, InMemoryTracingLog};

    /// The sample pairs cover every key, once each, so the checks reach a new key, and each pair
    /// is two different values of the one key, so a later write is seen to replace the first.
    #[test]
    fn the_sample_values_cover_every_key() {
        let pairs = sample_pairs();
        assert_eq!(
            pairs
                .iter()
                .map(|(first, _)| first.key())
                .collect::<Vec<_>>(),
            ConfigKey::ALL
        );
        for (first, second) in &pairs {
            assert_eq!(first.key(), second.key(), "{first:?} and {second:?}");
            assert_ne!(
                first, second,
                "{first:?}: the pair's two values are the same"
            );
        }
    }

    struct FakeLogs;

    impl TracingLogFactory for FakeLogs {
        type Log = InMemoryTracingLog;

        fn with_capacity(&mut self, capacity: usize) -> Self::Log {
            InMemoryTracingLog::new(capacity)
        }

        fn capture(&mut self, log: &Self::Log, level: LogLevel, message: &str) {
            log.push(level, "bin_desktop::shell", message);
        }
    }

    #[test]
    fn the_in_memory_capture_keeps_the_tracing_log_contract() {
        run_all_tracing_log(&mut FakeLogs);
    }

    struct FakeLedger;

    impl LedgerFactory for FakeLedger {
        type Reads = InMemoryLedgerUnits;
        type Writes = InMemoryLedgerUnits;

        fn create(&mut self, deactivated: &[UnitId]) -> (InMemoryLedgerUnits, InMemoryLedgerUnits) {
            let units = InMemoryLedgerUnits::sample();
            for id in deactivated {
                units.deactivate(*id);
            }
            (units.clone(), units)
        }
    }

    #[test]
    fn the_in_memory_ledger_units_keep_the_contract() {
        run_all_ledger(&mut FakeLedger);
    }

    struct FakeFiles;

    impl DeviceFileFactory for FakeFiles {
        type File = InMemoryDeviceFile;

        fn writable(&mut self) -> Self::File {
            InMemoryDeviceFile::writable()
        }

        fn shadowing(&mut self, key: ConfigKey) -> (Self::File, String) {
            let source = shadow_source(key);
            (InMemoryDeviceFile::shadowing(key, &source), source)
        }

        fn without_file(&mut self) -> Self::File {
            InMemoryDeviceFile::default()
        }

        fn unwritable(&mut self) -> Self::File {
            InMemoryDeviceFile::unwritable()
        }
    }

    #[test]
    fn the_in_memory_file_keeps_the_device_file_contract() {
        run_all_device_file(&mut FakeFiles);
    }

    #[test]
    fn the_policy_over_the_in_memory_file_keeps_its_contract() {
        run_all_policy(&mut FakeFiles);
    }
}
