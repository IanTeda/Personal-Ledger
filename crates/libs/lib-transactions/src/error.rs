use lib_locale::format::DateInputError;

/// Refusals for the Transactions page's filter popover: typed, never a string. The `#[error]`
/// text is for logs; the Presenter maps a variant to its `msg::` Message.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    /// A typed From or To date that doesn't parse, wrapping the value-type error from `lib-locale`.
    #[error("the filter's date is invalid: {0}")]
    FilterDate(#[from] DateInputError),
}

pub type Result<T, E = Error> = std::result::Result<T, E>;
