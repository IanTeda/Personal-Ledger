/// Refusals for the Bill Payments module: typed, never a string. The `#[error]` text is for logs;
/// the Pay Dialog's Presenter maps a variant to its `msg::` Message.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    #[error("the payment amount must be greater than zero")]
    AmountNotPositive,
    /// A `lib-bills` refusal: from Match, which the Pay Dialog's Match panel runs, or Pay finding
    /// its entry gone, not open, or its Bill Plan gone.
    #[error(transparent)]
    Bills(#[from] lib_bills::Error),
}

pub type Result<T, E = Error> = std::result::Result<T, E>;
