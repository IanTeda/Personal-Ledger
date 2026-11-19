//! The ports a This device setting is written through. A Client's configuration file answers the
//! raw [`DeviceFile`] port, which carries no policy: the [`interactor`](crate::interactor)s decide
//! whether a write goes through. [`DevicePorts`] is the file and the [`ChangeLog`] the Presenters
//! hand them.

use std::fmt;
use std::path::Path;
use std::rc::Rc;

use lib_core::configuration::{ConfigKey, ConfigValue};

use crate::protocols::{DeviceSnapshot, KeyState};

/// A write the file couldn't take. The adapter logs the cause; the policy only needs to know it
/// failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WriteError;

/// A device's configuration file as it is, with no policy: where it is, what outranks it, what it
/// holds and how to change it. Every write goes through, so the Interactors decide first.
pub trait DeviceFile {
    /// The file a write goes to; `None` with no home folder to keep one in.
    fn target(&self) -> Option<&Path>;

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
/// just before and just after. Never told of a [`RestoreSetting`](crate::interactor::RestoreSetting), so stepping
/// through History makes no entry.
pub trait ChangeLog {
    fn record(&self, key: ConfigKey, before: Option<String>, after: Option<String>);
}

/// The ports the This device Interactors take: the file they write and the log they note a saved
/// change in, shared by every This device page's Presenter.
#[derive(Clone)]
pub struct DevicePorts {
    file: Rc<dyn DeviceFile>,
    log: Option<Rc<dyn ChangeLog>>,
}

impl DevicePorts {
    /// Ports over `file`, noting no change.
    pub fn new(file: Rc<dyn DeviceFile>) -> Self {
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

    /// The raw file underneath.
    pub fn file(&self) -> &dyn DeviceFile {
        &*self.file
    }

    pub(crate) fn log(&self) -> Option<&dyn ChangeLog> {
        self.log.as_deref()
    }

    /// Every key's value in the file, the value in force and what sets it above the file.
    pub fn snapshot(&self) -> DeviceSnapshot {
        snapshot(&*self.file)
    }
}

/// What `file` reads: every key's value in the file, the value in force and what sets it above the
/// file. The file is read and parsed once for every key, not once per key.
pub fn snapshot(file: &dyn DeviceFile) -> DeviceSnapshot {
    let stored = file.stored_all();
    DeviceSnapshot::new(file.target().map(Path::to_path_buf), |key| KeyState {
        stored: stored
            .iter()
            .find(|(read, _)| *read == key)
            .map(|(_, value)| value.clone()),
        effective: file.effective(key),
        shadow: file.shadow(key),
    })
}

impl fmt::Debug for DevicePorts {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DevicePorts")
            .field("file", &self.file.target())
            .field("logging", &self.log.is_some())
            .finish()
    }
}
