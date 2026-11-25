//! The Settings Refusals: why a This device setting the file would take wasn't written, or why a
//! This Ledger write was turned away. The
//! `#[error]` text is for logs; the person sees the Presenter's `msg::` Message.

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    /// There is no home folder to keep the file in.
    #[error("there is no configuration file to write")]
    NoFile,
    /// The file or its folder couldn't be written; the Client logs the cause.
    #[error("the configuration file couldn't be written")]
    WriteFailed,
    /// The base Unit was removed or deactivated under the select, which repaints without it.
    #[error("the base Unit is gone or inactive")]
    BaseUnitRefused,
}

pub type Result<T, E = Error> = std::result::Result<T, E>;
