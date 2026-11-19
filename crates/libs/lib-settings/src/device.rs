//! The write-through policy for a This device setting, owned once. A Client's configuration file
//! answers the raw [`DeviceFile`] port, which carries no policy; [`Policed`] wraps it as the
//! [`DeviceConfiguration`] the Presenters call, and decides in one place whether a write goes
//! through: a source above the file that sets the key refuses it (except for a key that
//! [writes when shadowed](writes_when_shadowed)), then no file to write refuses it, then the write
//! itself, whose failure becomes [`DeviceRefusal::WriteFailed`].

use std::fmt;
use std::path::PathBuf;
use std::rc::Rc;

use lib_core::configuration::{ConfigKey, ConfigValue};

use crate::protocols::{
    DeviceAnswer, DeviceConfiguration, DeviceRefusal, DeviceSnapshot, KeyState,
    writes_when_shadowed,
};

/// A write the file couldn't take. The adapter logs the cause; the policy only needs to know it
/// failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WriteError;

/// A device's configuration file as it is, with no policy: where it is, what outranks it, what it
/// holds and how to change it. Every write goes through, so [`Policed`] decides first.
pub trait DeviceFile {
    /// The file a write goes to; `None` with no home folder to keep one in.
    fn target(&self) -> Option<PathBuf>;

    /// What sets `key` above the file, by name, or `None` when nothing does.
    fn shadow(&self, key: ConfigKey) -> Option<String>;

    /// Every key the file itself sets, each as the parser reads it.
    fn stored_all(&self) -> Vec<(ConfigKey, ConfigValue)>;

    /// `key`'s value in force, as start-up's parse read it from every source.
    fn effective(&self, key: ConfigKey) -> Option<ConfigValue>;

    /// `key`'s raw text in the file, `None` where the file doesn't hold it.
    fn raw(&self, key: ConfigKey) -> Option<String>;

    /// Writes `value` under its key, spelt as the parser reads it back.
    ///
    /// # Errors
    /// [`WriteError`] when the file or its folder can't be written.
    fn set(&self, value: ConfigValue) -> Result<(), WriteError>;

    /// Takes `key` out of the file.
    ///
    /// # Errors
    /// As [`Self::set`].
    fn remove(&self, key: ConfigKey) -> Result<(), WriteError>;

    /// Puts `key` back as [`Self::raw`] read it, or takes it out for `None`.
    ///
    /// # Errors
    /// As [`Self::set`].
    fn restore(&self, key: ConfigKey, value: Option<&str>) -> Result<(), WriteError>;
}

/// Where a saved change is noted for the input's History entry: the key's raw value in the file
/// just before and just after. Never told of a [`DeviceConfiguration::restore`], so stepping
/// through History makes no entry.
pub trait ChangeLog {
    fn record(&self, key: ConfigKey, before: Option<String>, after: Option<String>);
}

/// A [`DeviceFile`] with the write-through policy on top: the [`DeviceConfiguration`] the
/// Presenters call.
pub struct Policed<F> {
    file: F,
    log: Option<Rc<dyn ChangeLog>>,
}

impl<F: DeviceFile> Policed<F> {
    pub fn new(file: F) -> Self {
        Self { file, log: None }
    }

    /// `self`, noting each saved write and removal in `log`.
    #[must_use]
    pub fn logging(self, log: Rc<dyn ChangeLog>) -> Self {
        Self {
            log: Some(log),
            ..self
        }
    }

    /// The file underneath, with no policy.
    pub fn into_file(self) -> F {
        self.file
    }

    /// The raw file underneath.
    pub fn file(&self) -> &F {
        &self.file
    }

    /// [`Self::change`], noting the key's raw value before and after for the input's History entry
    /// (#1244), even under an override.
    fn recorded<T>(
        &self,
        key: ConfigKey,
        value: T,
        edit: impl FnOnce(&F) -> Result<(), WriteError>,
    ) -> DeviceAnswer<T> {
        let before = self.log.as_ref().map(|_| self.file.raw(key));
        let answer = self.change(key, value, edit);
        if let (Some(log), Some(before), DeviceAnswer::Saved { .. }) = (&self.log, before, &answer)
        {
            log.record(key, before, self.file.raw(key));
        }
        answer
    }

    /// Changes `key` in the file with `edit`, answering `value` once saved, unless a source above
    /// sets a key that refuses then.
    fn change<T>(
        &self,
        key: ConfigKey,
        value: T,
        edit: impl FnOnce(&F) -> Result<(), WriteError>,
    ) -> DeviceAnswer<T> {
        if !writes_when_shadowed(key)
            && let Some(source) = self.file.shadow(key)
        {
            return DeviceAnswer::Shadowed { source };
        }
        let Some(path) = self.file.target() else {
            return DeviceAnswer::Refused(DeviceRefusal::NoFile);
        };
        match edit(&self.file) {
            Ok(()) => DeviceAnswer::Saved {
                value,
                file: path.display().to_string(),
            },
            Err(WriteError) => DeviceAnswer::Refused(DeviceRefusal::WriteFailed),
        }
    }
}

impl<F: Default + DeviceFile> Default for Policed<F> {
    fn default() -> Self {
        Self::new(F::default())
    }
}

impl<F: fmt::Debug> fmt::Debug for Policed<F> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Policed")
            .field("file", &self.file)
            .field("logging", &self.log.is_some())
            .finish()
    }
}

impl<F: DeviceFile> DeviceConfiguration for Policed<F> {
    fn read(&self) -> DeviceSnapshot {
        // The file is read and parsed once for every key, not once per key.
        let stored = self.file.stored_all();
        DeviceSnapshot::new(self.file.target(), |key| KeyState {
            stored: stored
                .iter()
                .find(|(read, _)| *read == key)
                .map(|(_, value)| value.clone()),
            effective: self.file.effective(key),
            shadow: self.file.shadow(key),
        })
    }

    fn write(&self, value: ConfigValue) -> DeviceAnswer<ConfigValue> {
        let written = value.clone();
        self.recorded(value.key(), value, |file| file.set(written))
    }

    fn remove(&self, key: ConfigKey) -> DeviceAnswer<()> {
        self.recorded(key, (), |file| file.remove(key))
    }

    fn restore(&self, key: ConfigKey, value: Option<&str>) -> DeviceAnswer<()> {
        self.change(key, (), |file| file.restore(key, value))
    }
}
