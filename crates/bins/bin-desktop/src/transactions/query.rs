//! The Transactions query engine lives in `lib_ledger` (`docs/ux/desktop-mockups/03-transactions/README.md`);
//! this module re-exports it and keeps the one piece that needs the Desktop's Message layer, the
//! Status filter's label.

pub use lib_ledger::query::*;

use lib_core::TransactionStatus;
use lib_locale::Label;

/// The word for a Status filter in the Locale in effect: the shared Status label, or "All".
pub fn status_filter_label(filter: StatusFilter) -> String {
    match filter {
        StatusFilter::All => crate::msg::desktop_transactions_status_all(),
        StatusFilter::Open => TransactionStatus::Open.label(),
        StatusFilter::Cleared => TransactionStatus::Cleared.label(),
        StatusFilter::Reconciled => TransactionStatus::Reconciled.label(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_status_filter_labels() {
        crate::locale::init_for_tests();
        assert_eq!(
            StatusFilter::ALL.map(status_filter_label),
            ["All", "Open", "Cleared", "Reconciled"]
        );
    }
}
