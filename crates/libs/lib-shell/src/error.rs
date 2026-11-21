//! Why an Undo or Redo step didn't apply, as the Client's Router reports it, or a folder the
//! explorer browses couldn't be read, as the `LedgerFolders` port reports it. Typed, never a
//! pre-formatted string: the Shell words the status line from the variant, and the `Display` text
//! is for logs.

use std::fmt;

use lib_core::configuration::ConfigKey;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// The Undo or Redo's Ledger rows didn't save; `reloaded` is whether they read back as stored.
    LedgerNotSaved { reloaded: bool },
    /// The key is set elsewhere (an env var or another file), which outranks the file. A Refusal.
    ConfigurationShadowed { key: ConfigKey, source: String },
    /// There is no configuration file to write the key to.
    ConfigurationNoFile { key: ConfigKey },
    /// Writing the key to the configuration file failed.
    ConfigurationNotWritten { key: ConfigKey },
    /// A folder the explorer browses couldn't be read (permissions, or gone), by the I/O error's
    /// kind. The adapter logs the error itself.
    FolderUnreadable(std::io::ErrorKind),
}

// By hand, not `thiserror`: it reads a field named `source` as the error's cause, and this one is
// data (an env var or a path).
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LedgerNotSaved { reloaded } => {
                write!(f, "the Ledger rows didn't save (reloaded: {reloaded})")
            }
            Self::ConfigurationShadowed { key, source } => {
                write!(
                    f,
                    "{} is set by {source}, which outranks the file",
                    key.name()
                )
            }
            Self::ConfigurationNoFile { key } => {
                write!(f, "{} can't be saved: no configuration file", key.name())
            }
            Self::ConfigurationNotWritten { key } => {
                write!(f, "{} wasn't written to the configuration file", key.name())
            }
            Self::FolderUnreadable(kind) => write!(f, "the folder couldn't be read: {kind}"),
        }
    }
}

impl std::error::Error for Error {}

pub type Result<T, E = Error> = std::result::Result<T, E>;
