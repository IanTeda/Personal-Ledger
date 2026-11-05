//! The contract every [`DeviceConfiguration`] keeps, run against the in-memory one here and the
//! Client's over its configuration file, so the fake the Presenters' tests use can be trusted.
//!
//! Each check is generic over a [`DeviceFactory`]: a Client runs it from its own tests, since its
//! device needs its own set-up (a temporary configuration file per check). Each writes through the
//! port, reads the file back as a restart would, and expects what was saved, and only that.

use lib_colour_theme::ColourAppearance;
use lib_core::{DateStyle, OnStart, RowDensity, StartView, StatusGlyphs, UnitId};

use crate::fake::StoredDevice;
use crate::log::LogLevel;
use crate::protocols::{
    BaseUnitAnswer, DeviceAnswer, DeviceConfiguration, DeviceKey, DeviceRefusal, LedgerPreferences,
    LedgerUnits, TracingLog,
};

/// Builds the device a contract check runs on.
pub trait DeviceFactory {
    type Device: DeviceConfiguration;

    /// A device writing to a file of its own, empty to start.
    fn writable(&mut self) -> Self::Device;

    /// [`Self::writable`], with `key` set by a source above the file, and that source's name as
    /// the device says it.
    fn shadowing(&mut self, key: DeviceKey) -> (Self::Device, String);

    /// A device with no file to write, as with no home folder.
    fn without_file(&mut self) -> Self::Device;

    /// A device whose file can't be written.
    fn unwritable(&mut self) -> Self::Device;

    /// What `device`'s file holds, read afresh as a restart would.
    fn reload(&mut self, device: &Self::Device) -> StoredDevice;
}

/// Every setting written through the port answers saved with its value, and reads back.
pub fn every_setting_is_saved_and_reads_back(factory: &mut impl DeviceFactory) {
    let device = factory.writable();

    assert_saved(device.set_on_start(OnStart::NoLedger), OnStart::NoLedger);
    assert_saved(
        device.set_start_view(StartView::Noun("bills".into())),
        StartView::Noun("bills".into()),
    );
    assert_saved(device.set_toasts(false), false);
    assert_saved(
        device.set_date_style(Some(DateStyle::Iso)),
        Some(DateStyle::Iso),
    );
    assert_saved(
        device.set_row_density(RowDensity::Compact),
        RowDensity::Compact,
    );
    assert_saved(
        device.set_status_glyphs(StatusGlyphs::AsciiFallback),
        StatusGlyphs::AsciiFallback,
    );
    assert_saved(device.set_colour_theme("nord"), "nord");
    assert_saved(
        device.set_colour_appearance(ColourAppearance::Light),
        ColourAppearance::Light,
    );

    assert_eq!(
        factory.reload(&device),
        StoredDevice {
            on_start: Some(OnStart::NoLedger),
            start_view: Some(StartView::Noun("bills".into())),
            toasts: Some(false),
            date_style: Some(Some(DateStyle::Iso)),
            row_density: Some(RowDensity::Compact),
            status_glyphs: Some(StatusGlyphs::AsciiFallback),
            colour_theme: Some("nord".to_string()),
            colour_appearance: Some(ColourAppearance::Light),
        }
    );
}

/// A later write replaces the value, the Locale's default date style included.
pub fn a_later_write_replaces_the_value(factory: &mut impl DeviceFactory) {
    let device = factory.writable();
    assert_saved(
        device.set_date_style(Some(DateStyle::Long)),
        Some(DateStyle::Long),
    );
    assert_saved(device.set_date_style(None), None);
    assert_saved(device.set_start_view(StartView::Last), StartView::Last);
    assert_saved(
        device.set_start_view(StartView::Dashboard),
        StartView::Dashboard,
    );

    assert_saved(
        device.set_status_glyphs(StatusGlyphs::AsciiFallback),
        StatusGlyphs::AsciiFallback,
    );
    assert_saved(
        device.set_status_glyphs(StatusGlyphs::Unicode),
        StatusGlyphs::Unicode,
    );

    let stored = factory.reload(&device);
    assert_eq!(stored.status_glyphs, Some(StatusGlyphs::Unicode));
    assert_eq!(stored.date_style, Some(None));
    assert_eq!(stored.start_view, Some(StartView::Dashboard));
}

/// A key a source above the file sets is named by `shadow`, answers shadowed naming it, and is not
/// written; the other keys are not shadowed.
pub fn a_shadowed_key_is_named_and_left_unwritten(factory: &mut impl DeviceFactory) {
    for key in DeviceKey::ALL {
        let (device, source) = factory.shadowing(key);

        assert_eq!(
            device.shadow(key).as_deref(),
            Some(source.as_str()),
            "{key:?}"
        );
        for other in DeviceKey::ALL.into_iter().filter(|other| *other != key) {
            assert_eq!(device.shadow(other), None, "{other:?} under {key:?}");
        }
        let shadowed = |answer: Option<String>| {
            assert_eq!(answer.as_deref(), Some(source.as_str()), "{key:?}");
        };
        match key {
            DeviceKey::OnStart => shadowed(source_of(device.set_on_start(OnStart::NoLedger))),
            DeviceKey::StartView => shadowed(source_of(device.set_start_view(StartView::Last))),
            DeviceKey::Toasts => shadowed(source_of(device.set_toasts(false))),
            DeviceKey::DateStyle => shadowed(source_of(device.set_date_style(None))),
            DeviceKey::RowDensity => {
                shadowed(source_of(device.set_row_density(RowDensity::Roomy)));
            }
            DeviceKey::StatusGlyphs => shadowed(source_of(
                device.set_status_glyphs(StatusGlyphs::AsciiFallback),
            )),
            DeviceKey::ColourTheme => shadowed(source_of(device.set_colour_theme("nord"))),
            DeviceKey::ColourAppearance => shadowed(source_of(
                device.set_colour_appearance(ColourAppearance::Dark),
            )),
        }
        assert_eq!(factory.reload(&device), StoredDevice::default(), "{key:?}");
    }
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
    every_setting_is_saved_and_reads_back(factory);
    a_later_write_replaces_the_value(factory);
    a_shadowed_key_is_named_and_left_unwritten(factory);
    with_no_file_every_write_is_refused(factory);
    a_failed_write_is_refused(factory);
}

fn assert_saved<T: std::fmt::Debug + PartialEq>(answer: DeviceAnswer<T>, expected: T) {
    match answer {
        DeviceAnswer::Saved { value, file } => {
            assert_eq!(value, expected);
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
    let refusals = [
        refused(device.set_on_start(OnStart::NoLedger)),
        refused(device.set_start_view(StartView::Last)),
        refused(device.set_toasts(false)),
        refused(device.set_date_style(None)),
        refused(device.set_row_density(RowDensity::Roomy)),
        refused(device.set_status_glyphs(StatusGlyphs::AsciiFallback)),
        refused(device.set_colour_theme("nord")),
        refused(device.set_colour_appearance(ColourAppearance::Dark)),
    ];
    assert_eq!(refusals, [Some(refusal); 8]);
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

    /// The stub Units (aud, btc, vas, aud the base) with those of `deactivated` made inactive,
    /// read and written through ports on one store.
    fn create(&mut self, deactivated: &[UnitId]) -> (Self::Reads, Self::Writes);
}

/// The choices are the active Units by code, and the base is the stub's.
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
            let units = InMemoryLedgerUnits::stub();
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

        fn shadowing(&mut self, key: DeviceKey) -> (Self::Device, String) {
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

        fn reload(&mut self, device: &Self::Device) -> StoredDevice {
            device.stored()
        }
    }

    #[test]
    fn the_in_memory_device_keeps_the_contract() {
        run_all(&mut Fakes);
    }
}
