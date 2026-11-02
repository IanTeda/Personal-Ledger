//! The Budgets Refusals (`GLOSSARY.md`): one variant per rule an Interactor enforces, named for what
//! the rule forbids. A variant carries nothing English: the `#[error]` text is for logs, and the
//! person sees the Presenter's `msg::` Message.

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    #[error("budget not found")]
    NotFound,
    #[error("a budget needs a name")]
    NameRequired,
    #[error("another budget already has that name")]
    NameTaken,
    #[error("an archived budget is read-only")]
    Archived,
    #[error("the default budget can't be archived")]
    DefaultCannotBeArchived,
    #[error("only a leaf expense category can be budgeted")]
    NotAnExpenseLeaf,
    #[error("closed months are read-only")]
    ClosedMonth,
    #[error("a budget amount can't be negative")]
    NegativeAmount,
    #[error("account not found")]
    AccountNotFound,
    #[error("an on-budget account must take transactions in the budget's unit")]
    AccountNotInUnit,
    #[error("that category has no budget amount this month")]
    NotBudgeted,
    #[error("that isn't an amount")]
    InvalidAmount,
}

pub type Result<T, E = Error> = std::result::Result<T, E>;
