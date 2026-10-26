//! Why Account delete refused, typed so the Dialog words each case.

/// A delete that wrote nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum AccountDeletionError {
    #[error("account not found")]
    NotFound,
    /// Bill Plans pay from it, and a Plan has to pay from an Account. The file's foreign key on
    /// `bill_plans.account_id` refuses it too; this is the rule the person sees.
    #[error("{plans} bill plans pay from the account")]
    PaidFromByBillPlans { plans: usize },
}
