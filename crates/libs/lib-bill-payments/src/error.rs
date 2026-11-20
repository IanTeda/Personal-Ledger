use lib_locale::format::DateInputError;

/// Refusals for the Bill Payments module: typed, never a string. The `#[error]` text is for logs;
/// the Pay Dialog's Presenter maps a variant to its `msg::` Message.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    /// The Bill Schedule entry to pay is gone.
    #[error("bill schedule entry not found")]
    EntryNotFound,
    /// The entry is Paid, Skipped or superseded already, or a preview with no row behind it.
    #[error("this bill schedule entry isn't open")]
    EntryNotOpen,
    /// The entry's Bill Plan is gone.
    #[error("bill plan not found")]
    PlanNotFound,
    #[error("the payment amount must be greater than zero")]
    AmountNotPositive,
    /// A refusal from `lib-bills`' Match, which the Pay Dialog's Match panel runs.
    #[error(transparent)]
    Bills(#[from] lib_bills::Error),
    /// A typed payment date that doesn't parse, wrapping the value-type error from `lib-locale`.
    #[error("the payment date is invalid: {0}")]
    PayDate(#[from] DateInputError),
}

pub type Result<T, E = Error> = std::result::Result<T, E>;
