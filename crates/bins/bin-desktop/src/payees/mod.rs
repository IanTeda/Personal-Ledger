pub(crate) mod apply;
pub(crate) mod form;
pub(crate) mod store;

pub use lib_payees::*;
pub use store::{PayeesEvent, PayeesStore};

#[cfg(test)]
mod tests {
    use bigdecimal::BigDecimal;
    use chrono::NaiveDate;
    use lib_core::Money;

    use super::*;
    use crate::{
        accounts::default_accounts, categories::default_categories, tags::default_tags,
        transactions::default_transactions,
    };
    use form::DeleteAction;
    use lib_ledger::PayeeUse;
    use lib_transactions::Transaction;

    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 9, 19).unwrap()
    }

    fn seeded_transactions(payees: &[Payee]) -> Vec<Transaction> {
        default_transactions(
            &default_accounts(),
            &default_categories(),
            payees,
            &default_tags(),
            today(),
        )
    }

    fn id_of(payees: &[Payee], name: &str) -> u32 {
        find_by_name(payees, name).unwrap()
    }

    #[test]
    fn the_delete_action_follows_references_and_activity() {
        let mut payees = default_payees();
        let netflix = id_of(&payees, "Netflix");
        let j_smith = id_of(&payees, "J Smith");
        let used = PayeeUse {
            splits: 1,
            bill_plans: 0,
        };
        let action = |payees: &[Payee], id, payee_use| {
            DeleteAction::for_payee(get(payees, id).unwrap(), payee_use)
        };

        assert_eq!(
            action(&payees, netflix, PayeeUse::default()),
            DeleteAction::Delete
        );
        assert_eq!(action(&payees, j_smith, used), DeleteAction::Deactivate);
        set_active(&mut payees, j_smith, false).unwrap();
        assert_eq!(action(&payees, j_smith, used), DeleteAction::Reactivate);
    }

    #[test]
    fn usage_counts_every_split_and_totals_base_unit_ones() {
        let payees = default_payees();
        let accounts = default_accounts();
        let transactions = seeded_transactions(&payees);
        let j_smith = id_of(&payees, "J Smith");
        let j_smith_usage = usage(
            &transactions,
            &accounts,
            Some(lib_core::UnitId::SEED_AUD),
            j_smith,
        );
        assert_eq!(j_smith_usage.splits, 4);
        assert_eq!(
            j_smith_usage.total,
            Money(BigDecimal::new((-64_000).into(), 2))
        );

        let other_unit = usage(
            &transactions,
            &accounts,
            Some(lib_core::UnitId::new()),
            j_smith,
        );
        assert_eq!(other_unit.splits, 4);
        assert_eq!(other_unit.total, Money(BigDecimal::new(0.into(), 2)));
    }
}
