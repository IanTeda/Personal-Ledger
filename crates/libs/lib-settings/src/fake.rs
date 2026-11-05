//! A recording Router and an in-memory [`DeviceConfiguration`], for the Presenters' tests and a
//! Client's UI-first work.

use std::cell::RefCell;

use lib_colour_theme::ColourAppearance;
use lib_core::{DateStyle, OnStart, StartView};
use lib_toast::{Notice, Notify};

use crate::page::SettingsPage;
use crate::presenter::DisplayOutput;
use crate::protocols::{
    ColourChoice, DeviceAnswer, DeviceConfiguration, DeviceKey, DeviceRefusal, DeviceRouter,
    SettingsRouter,
};

/// One thing a Presenter asked of the Shell, in the order asked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Routed {
    PageChanged(SettingsPage),
    PickerOpened,
    PickerClosed,
    ToastsChanged(bool),
    DisplayChanged(DisplayOutput),
    ColourChanged(ColourChoice),
    Notice(Notice),
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
}

/// What a [`DeviceConfiguration`] holds of each setting, `None` for a key never written.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StoredDevice {
    pub on_start: Option<OnStart>,
    pub start_view: Option<StartView>,
    pub toasts: Option<bool>,
    pub date_style: Option<Option<DateStyle>>,
    pub colour_theme: Option<String>,
    pub colour_appearance: Option<ColourAppearance>,
}

/// A configuration file in memory: the settings written, the keys a source above it sets, and
/// whether there is a file to write at all or writes fail.
#[derive(Debug, Default)]
pub struct InMemoryDeviceConfiguration {
    /// The file a write names; `None` refuses every write, as with no home folder.
    pub file: Option<String>,
    /// The keys a source above the file sets, with the source's name.
    pub shadows: Vec<(DeviceKey, String)>,
    /// Makes every write fail, as a file that can't be written does.
    pub fail_writes: bool,
    pub stored: RefCell<StoredDevice>,
}

impl InMemoryDeviceConfiguration {
    /// A file that takes every write.
    pub fn writable() -> Self {
        Self {
            file: Some("personal-ledger.conf".to_string()),
            ..Self::default()
        }
    }

    /// [`Self::writable`], with `key` set by `source` above the file.
    pub fn shadowing(key: DeviceKey, source: &str) -> Self {
        Self {
            shadows: vec![(key, source.to_string())],
            ..Self::writable()
        }
    }

    pub fn stored(&self) -> StoredDevice {
        self.stored.borrow().clone()
    }

    /// Writes `value` with `store` unless the key is shadowed, there's no file, or writes fail.
    fn write<T: Clone>(
        &self,
        key: DeviceKey,
        value: T,
        store: impl FnOnce(&mut StoredDevice, T),
    ) -> DeviceAnswer<T> {
        if let Some(source) = self.shadow(key) {
            return DeviceAnswer::Shadowed { source };
        }
        let Some(file) = &self.file else {
            return DeviceAnswer::Refused(DeviceRefusal::NoFile);
        };
        if self.fail_writes {
            return DeviceAnswer::Refused(DeviceRefusal::WriteFailed);
        }
        store(&mut self.stored.borrow_mut(), value.clone());
        DeviceAnswer::Saved {
            value,
            file: file.clone(),
        }
    }
}

impl DeviceConfiguration for InMemoryDeviceConfiguration {
    fn shadow(&self, key: DeviceKey) -> Option<String> {
        self.shadows
            .iter()
            .find(|(shadowed, _)| *shadowed == key)
            .map(|(_, source)| source.clone())
    }

    fn set_on_start(&self, on_start: OnStart) -> DeviceAnswer<OnStart> {
        self.write(DeviceKey::OnStart, on_start, |stored, value| {
            stored.on_start = Some(value);
        })
    }

    fn set_start_view(&self, start_view: StartView) -> DeviceAnswer<StartView> {
        self.write(DeviceKey::StartView, start_view, |stored, value| {
            stored.start_view = Some(value);
        })
    }

    fn set_toasts(&self, on: bool) -> DeviceAnswer<bool> {
        self.write(DeviceKey::Toasts, on, |stored, value| {
            stored.toasts = Some(value);
        })
    }

    fn set_date_style(&self, style: Option<DateStyle>) -> DeviceAnswer<Option<DateStyle>> {
        self.write(DeviceKey::DateStyle, style, |stored, value| {
            stored.date_style = Some(value);
        })
    }

    fn set_colour_theme(&self, id: &'static str) -> DeviceAnswer<&'static str> {
        self.write(DeviceKey::ColourTheme, id, |stored, value| {
            stored.colour_theme = Some(value.to_string());
        })
    }

    fn set_colour_appearance(
        &self,
        appearance: ColourAppearance,
    ) -> DeviceAnswer<ColourAppearance> {
        self.write(DeviceKey::ColourAppearance, appearance, |stored, value| {
            stored.colour_appearance = Some(value);
        })
    }
}
