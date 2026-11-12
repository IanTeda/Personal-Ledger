//! The contract every [`DeviceConfiguration`] keeps, run against the in-memory one here and the
//! Client's over its configuration file, so the fake the Presenters' tests use can be trusted.
//!
//! Each check is generic over a [`DeviceFactory`]: a Client runs it from its own tests, since its
//! device needs its own set-up (a temporary configuration file per check). Each writes through the
//! port, reads the device back, and expects what was saved, and only that. The writes run over
//! every [`ConfigKey`], so a new key is checked with no new arm here.

use std::path::PathBuf;

use lib_core::configuration::{ConfigKey, ConfigValue};
use lib_core::{
    ColourAppearance, DateStyle, Levels, OnStart, RowDensity, StartView, StatusGlyphs, UnitId,
};

use crate::fake::StoredDevice;
use crate::log::LogLevel;
use crate::protocols::{
    BaseUnitAnswer, DeviceAnswer, DeviceConfiguration, DeviceRefusal, LedgerPreferences,
    LedgerUnits, TracingLog, writes_when_shadowed,
};

/// Builds the device a contract check runs on.
pub trait DeviceFactory {
    type Device: DeviceConfiguration;

    /// A device writing to a file of its own, empty to start.
    fn writable(&mut self) -> Self::Device;

    /// [`Self::writable`], with `key` set by a source above the file, and that source's name as
    /// the device says it.
    fn shadowing(&mut self, key: ConfigKey) -> (Self::Device, String);

    /// A device with no file to write, as with no home folder.
    fn without_file(&mut self) -> Self::Device;

    /// A device whose file can't be written.
    fn unwritable(&mut self) -> Self::Device;
}

/// A value for every key, each other than its default, in [`ConfigKey::ALL`]'s order.
pub fn first_values() -> Vec<ConfigValue> {
    vec![
        ConfigValue::OnStart(OnStart::NoLedger),
        ConfigValue::StartView(StartView::Noun("bills".into())),
        ConfigValue::Toasts(false),
        ConfigValue::Locale("en-GB".to_string()),
        ConfigValue::Data(PathBuf::from("/home/me/finance")),
        ConfigValue::ColourTheme("nord".to_string()),
        ConfigValue::ColourAppearance(ColourAppearance::Light),
        ConfigValue::DateStyle(Some(DateStyle::Iso)),
        ConfigValue::RowDensity(RowDensity::Compact),
        ConfigValue::StatusGlyphs(StatusGlyphs::AsciiFallback),
        ConfigValue::HintBar(false),
        ConfigValue::TerminalColours(true),
        ConfigValue::Log(Levels::DEBUG),
        ConfigValue::LogFilePath(PathBuf::from("/home/me/ledger.log")),
    ]
}

/// A second value for every key, each other than [`first_values`]', the Locale's default date
/// style among them.
pub fn second_values() -> Vec<ConfigValue> {
    vec![
        ConfigValue::OnStart(OnStart::ReopenLastLedger),
        ConfigValue::StartView(StartView::Dashboard),
        ConfigValue::Toasts(true),
        ConfigValue::Locale("en-US".to_string()),
        ConfigValue::Data(PathBuf::from("/home/me/money")),
        ConfigValue::ColourTheme("gruvbox".to_string()),
        ConfigValue::ColourAppearance(ColourAppearance::Dark),
        ConfigValue::DateStyle(None),
        ConfigValue::RowDensity(RowDensity::Roomy),
        ConfigValue::StatusGlyphs(StatusGlyphs::Unicode),
        ConfigValue::HintBar(true),
        ConfigValue::TerminalColours(false),
        ConfigValue::Log(Levels::ERROR),
        ConfigValue::LogFilePath(PathBuf::from("/home/me/other.log")),
    ]
}

/// What `device`'s file holds, as its read says.
fn stored(device: &impl DeviceConfiguration) -> StoredDevice {
    let snapshot = device.read();
    StoredDevice(
        ConfigKey::ALL
            .into_iter()
            .filter_map(|key| Some((key, snapshot.get(key).stored.clone()?)))
            .collect(),
    )
}

/// The sample values cover every key, once each, so the checks below reach a new key.
pub fn the_sample_values_cover_every_key() {
    for values in [first_values(), second_values()] {
        assert_eq!(
            values.iter().map(ConfigValue::key).collect::<Vec<_>>(),
            ConfigKey::ALL
        );
    }
}

/// Every key written through the port answers saved with its value, and reads back as stored.
pub fn every_key_is_saved_and_reads_back(factory: &mut impl DeviceFactory) {
    let device = factory.writable();

    for value in first_values() {
        assert_saved(device.write(value.clone()), &value);
    }

    assert_eq!(stored(&device), StoredDevice::of(first_values()));
}

/// A later write replaces the value, and a removed key reads back unset, the others kept.
pub fn a_later_write_replaces_the_value(factory: &mut impl DeviceFactory) {
    let device = factory.writable();
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
        assert_eq!(device.read().get(key).stored, None, "{key:?} taken out");
    }
    assert_eq!(stored(&device), StoredDevice::default());
}

/// A key a source above the file sets reads that source, answers shadowed naming it, and is not
/// written; the other keys are not shadowed. The Locale and the Data folder are named too, but
/// still written ([`writes_when_shadowed`]).
pub fn a_shadowed_key_reads_its_source_and_is_left_unwritten(factory: &mut impl DeviceFactory) {
    for value in first_values() {
        let key = value.key();
        let (device, source) = factory.shadowing(key);

        let snapshot = device.read();
        assert_eq!(
            snapshot.get(key).shadow.as_deref(),
            Some(source.as_str()),
            "{key:?}"
        );
        for other in ConfigKey::ALL.into_iter().filter(|other| *other != key) {
            assert_eq!(snapshot.get(other).shadow, None, "{other:?} under {key:?}");
        }

        if writes_when_shadowed(key) {
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
pub fn the_read_names_the_file(factory: &mut impl DeviceFactory) {
    let device = factory.writable();
    let Some(file) = device.read().file else {
        panic!("a writable device names its file");
    };
    match device.write(ConfigValue::Toasts(false)) {
        DeviceAnswer::Saved { file: saved, .. } => {
            assert_eq!(saved, file.display().to_string());
        }
        other => panic!("expected a save, got {other:?}"),
    }

    assert_eq!(factory.without_file().read().file, None);
}

/// With no file to write, every write is refused.
pub fn with_no_file_every_write_is_refused(factory: &mut impl DeviceFactory) {
    let device = factory.without_file();
    assert_refused(&device, DeviceRefusal::NoFile);
}

/// A file that can't be written refuses every write.
pub fn a_failed_write_is_refused(factory: &mut impl DeviceFactory) {
    let device = factory.unwritable();
    assert_refused(&device, DeviceRefusal::WriteFailed);
}

/// Runs every check.
pub fn run_all(factory: &mut impl DeviceFactory) {
    the_sample_values_cover_every_key();
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

fn assert_refused(device: &impl DeviceConfiguration, refusal: DeviceRefusal) {
    fn refused<T>(answer: DeviceAnswer<T>) -> Option<DeviceRefusal> {
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
    type Reads: LedgerUnits;
    type Writes: LedgerPreferences;

    /// The sample Units (aud, btc, vas, aud the base) with those of `deactivated` made inactive,
    /// read and written through ports on one store.
    fn create(&mut self, deactivated: &[UnitId]) -> (Self::Reads, Self::Writes);
}

/// The choices are the active Units by code, and the base is the sample's.
pub fn the_choices_are_the_active_units_by_code(factory: &mut impl LedgerFactory) {
    let (reads, _) = factory.create(&[UnitId::SEED_BTC]);

    let codes: Vec<_> = reads.choices().into_iter().map(|unit| unit.code).collect();
    assert_eq!(codes, ["aud", "vas"]);
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

/// Runs every Settings › Ledger check on `factory`.
pub fn run_all_ledger(factory: &mut impl LedgerFactory) {
    the_choices_are_the_active_units_by_code(factory);
    a_saved_base_unit_reads_back(factory);
    a_unit_that_is_gone_or_inactive_is_refused(factory);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fake::{InMemoryDeviceConfiguration, InMemoryLedgerUnits, InMemoryTracingLog};

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

    struct Fakes;

    impl DeviceFactory for Fakes {
        type Device = InMemoryDeviceConfiguration;

        fn writable(&mut self) -> Self::Device {
            InMemoryDeviceConfiguration::writable()
        }

        fn shadowing(&mut self, key: ConfigKey) -> (Self::Device, String) {
            let source = format!("PERSONAL_LEDGER_{}", key.name().to_uppercase());
            (InMemoryDeviceConfiguration::shadowing(key, &source), source)
        }

        fn without_file(&mut self) -> Self::Device {
            InMemoryDeviceConfiguration::default()
        }

        fn unwritable(&mut self) -> Self::Device {
            InMemoryDeviceConfiguration {
                fail_writes: true,
                ..InMemoryDeviceConfiguration::writable()
            }
        }
    }

    #[test]
    fn the_in_memory_device_keeps_the_contract() {
        run_all(&mut Fakes);
    }
}
