//! The contract every [`DeviceConfiguration`] keeps, run against the in-memory one here and the
//! Client's over its configuration file, so the fake the Presenters' tests use can be trusted.
//!
//! Each check is generic over a [`DeviceFactory`]: a Client runs it from its own tests, since its
//! device needs its own set-up (a temporary configuration file per check). Each writes through the
//! port, reads the file back as a restart would, and expects what was saved, and only that.

use lib_colour_theme::ColourAppearance;
use lib_core::{DateStyle, OnStart, StartView};

use crate::fake::StoredDevice;
use crate::protocols::{DeviceAnswer, DeviceConfiguration, DeviceKey, DeviceRefusal};

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

    let stored = factory.reload(&device);
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
        refused(device.set_colour_theme("nord")),
        refused(device.set_colour_appearance(ColourAppearance::Dark)),
    ];
    assert_eq!(refusals, [Some(refusal); 6]);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fake::InMemoryDeviceConfiguration;

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
