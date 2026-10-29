use lib_core::UnitId;

/// Refusals for Bill Plan and Bill Schedule operations: one variant per rule, typed and never a
/// string. The `#[error]` text is for logs; the Presenter maps a variant to its `msg::` Message.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    #[error("a bill plan needs a name")]
    NameRequired,
    #[error("a bill plan's category must be an expense category with no children")]
    CategoryNotExpenseLeaf,
    #[error("a bill plan needs an account that takes transactions")]
    AccountRequired,
    #[error("the unit must be the account's unit")]
    UnitMismatch,
    /// Carries the locked Unit.
    #[error("the unit is locked at {0}")]
    UnitLocked(UnitId),
    #[error("the planned amount must be greater than zero")]
    AmountNotPositive,
    #[error("ends on can't be before first due")]
    EndsBeforeFirstDue,
    #[error("a one-shot bill plan has no ends on date")]
    EndsOnForOneShot,
    #[error("bill plan not found")]
    PlanNotFound,
    #[error("bill schedule entry not found")]
    EntryNotFound,
    /// Paid, Skipped or superseded already, or a preview with no row behind it.
    #[error("this bill schedule entry isn't open")]
    EntryNotOpen,
    #[error("that split can't be matched to this bill")]
    NotACandidate,
    #[error("this bill schedule entry isn't paid")]
    NotPaid,
    #[error("this bill schedule entry isn't skipped")]
    NotSkipped,
}

pub type Result<T, E = Error> = std::result::Result<T, E>;
