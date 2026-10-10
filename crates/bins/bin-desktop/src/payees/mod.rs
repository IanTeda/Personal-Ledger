pub(crate) mod form;

pub use lib_payees::*;

#[cfg(test)]
mod tests {
    use bigdecimal::BigDecimal;
    use chrono::NaiveDate;
    use lib_core::Money;

    use super::*;
    use crate::{
        accounts::default_accounts, categories, categories::default_categories, tags::default_tags,
        transactions::default_transactions,
    };
    use form::DeleteAction;
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

    fn draft(name: &str, aliases: &[&str]) -> PayeeDraft {
        PayeeDraft {
            name: name.to_string(),
            default_category: None,
            aliases: aliases.iter().map(|a| a.to_string()).collect(),
        }
    }

    fn id_of(payees: &[Payee], name: &str) -> u32 {
        find_by_name(payees, name).unwrap()
    }

    #[test]
    fn ids_names_and_aliases_are_unique() {
        let payees = default_payees();
        let mut ids: Vec<_> = payees.iter().map(|p| p.id).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), payees.len());
        let mut names: Vec<_> = payees.iter().map(|p| p.name.to_lowercase()).collect();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), payees.len());
        let mut aliases: Vec<_> = payees.iter().flat_map(|p| p.aliases.clone()).collect();
        let count = aliases.len();
        aliases.sort();
        aliases.dedup();
        assert_eq!(aliases.len(), count);
    }

    #[test]
    fn seeded_aliases_are_normalised_and_default_categories_are_leaves() {
        let categories = default_categories();
        for payee in default_payees() {
            for alias in &payee.aliases {
                assert_eq!(normalise_alias(alias).as_ref(), Some(alias));
            }
            if let Some(category) = payee.default_category {
                assert!(categories::is_leaf(&categories, category), "{}", payee.name);
            }
        }
    }

    #[test]
    fn find_by_name_is_exact_and_aliases_are_not_names() {
        let payees = default_payees();
        assert_eq!(find_by_name(&payees, "Woolworths"), Some(1));
        assert_eq!(find_by_name(&payees, "WOOLIES"), None);
        assert!(payees[0].aliases.contains(&"WOOLIES".to_string()));
    }

    #[test]
    fn normalise_alias_trims_upper_cases_and_ignores_blank() {
        assert_eq!(normalise_alias("  ww metro "), Some("WW METRO".to_string()));
        assert_eq!(normalise_alias("   "), None);
        assert_eq!(normalise_alias(""), None);
    }

    #[test]
    fn insert_normalises_and_dedupes_aliases_and_assigns_a_fresh_id() {
        let mut payees = default_payees();
        let max = payees.iter().map(|p| p.id).max().unwrap();
        let mut new = draft(
            " Aussie Candle Co ",
            &["aussie candle", "AUSSIE CANDLE ", " ", ""],
        );
        new.default_category = Some(10);
        let id = insert_payee(&mut payees, &new).unwrap();
        assert_eq!(id, max + 1);
        let payee = get(&payees, id).unwrap();
        assert_eq!(payee.name, "Aussie Candle Co");
        assert_eq!(payee.aliases, vec!["AUSSIE CANDLE".to_string()]);
        assert_eq!(payee.default_category, Some(10));
        assert!(payee.is_active);
    }

    #[test]
    fn insert_requires_a_name_unique_case_insensitively() {
        let mut payees = default_payees();
        assert_eq!(
            insert_payee(&mut payees, &draft("  ", &[])),
            Err(PayeeError::NameRequired)
        );
        assert_eq!(
            insert_payee(&mut payees, &draft("woolworths", &[])),
            Err(PayeeError::DuplicateName("Woolworths".to_string()))
        );
    }

    #[test]
    fn an_alias_another_payee_owns_is_rejected_naming_the_owner() {
        let mut payees = default_payees();
        assert_eq!(
            insert_payee(&mut payees, &draft("Woolies Metro", &["woolies"])),
            Err(PayeeError::AliasTaken {
                alias: "WOOLIES".to_string(),
                owner: "Woolworths".to_string(),
            })
        );
    }

    #[test]
    fn a_name_equal_to_another_payees_alias_is_rejected() {
        let mut payees = default_payees();
        assert_eq!(
            insert_payee(&mut payees, &draft("Woolies", &[])),
            Err(PayeeError::AliasTaken {
                alias: "WOOLIES".to_string(),
                owner: "Woolworths".to_string(),
            })
        );
    }

    #[test]
    fn edit_replaces_category_and_aliases() {
        let mut payees = default_payees();
        let coles = id_of(&payees, "Coles");
        let mut change = draft("Coles", &["coles express"]);
        change.default_category = Some(10);
        edit_payee(&mut payees, coles, &change).unwrap();
        let payee = get(&payees, coles).unwrap();
        assert_eq!(payee.aliases, vec!["COLES EXPRESS".to_string()]);
        assert_eq!(payee.default_category, Some(10));
    }

    #[test]
    fn a_rename_keeps_the_old_name_as_an_alias() {
        let mut payees = default_payees();
        let kmart = id_of(&payees, "Kmart");
        edit_payee(&mut payees, kmart, &draft("Kmart AU", &["KMART"])).unwrap();
        let payee = get(&payees, kmart).unwrap();
        assert_eq!(payee.name, "Kmart AU");
        // The old name normalises to the alias already there, so it isn't duplicated.
        assert_eq!(payee.aliases, vec!["KMART".to_string()]);

        let uber = id_of(&payees, "Uber");
        edit_payee(&mut payees, uber, &draft("Uber Eats", &[])).unwrap();
        assert_eq!(
            get(&payees, uber).unwrap().aliases,
            vec!["UBER".to_string()]
        );
    }

    #[test]
    fn renaming_to_an_own_alias_drops_that_alias() {
        let mut payees = default_payees();
        let woolworths = id_of(&payees, "Woolworths");
        let aliases: Vec<&str> = vec!["WOOLWORTHS", "WW SUPERMARKET", "WOOLIES"];
        edit_payee(&mut payees, woolworths, &draft("Woolies", &aliases)).unwrap();
        let payee = get(&payees, woolworths).unwrap();
        assert_eq!(payee.name, "Woolies");
        assert_eq!(
            payee.aliases,
            vec!["WOOLWORTHS".to_string(), "WW SUPERMARKET".to_string()]
        );
    }

    #[test]
    fn renaming_to_another_payees_name_or_alias_is_rejected() {
        let mut payees = default_payees();
        let coles = id_of(&payees, "Coles");
        assert_eq!(
            edit_payee(&mut payees, coles, &draft("bp", &[])),
            Err(PayeeError::DuplicateName("BP".to_string()))
        );
        assert_eq!(
            edit_payee(&mut payees, coles, &draft("Woolies", &[])),
            Err(PayeeError::AliasTaken {
                alias: "WOOLIES".to_string(),
                owner: "Woolworths".to_string(),
            })
        );
        assert_eq!(get(&payees, coles).unwrap().name, "Coles");
    }

    #[test]
    fn editing_keeping_the_same_name_in_another_case_is_allowed() {
        let mut payees = default_payees();
        let bp = id_of(&payees, "BP");
        edit_payee(&mut payees, bp, &draft("Bp", &[])).unwrap();
        // A change of case only is not a rename: the old name would equal the new one's alias.
        let payee = get(&payees, bp).unwrap();
        assert_eq!(payee.name, "Bp");
        assert!(payee.aliases.is_empty());
    }

    #[test]
    fn an_unreferenced_payee_hard_deletes_and_a_referenced_one_is_in_use() {
        let mut payees = default_payees();
        let transactions = seeded_transactions(&payees);
        let netflix = id_of(&payees, "Netflix");
        let j_smith = id_of(&payees, "J Smith");
        assert!(!is_referenced(&transactions, netflix));
        assert_eq!(
            delete_payee(&mut payees, &transactions, j_smith),
            Err(PayeeError::InUse)
        );
        delete_payee(&mut payees, &transactions, netflix).unwrap();
        assert_eq!(get(&payees, netflix), None);
        assert_eq!(
            delete_payee(&mut payees, &transactions, netflix),
            Err(PayeeError::NotFound)
        );
    }

    #[test]
    fn the_delete_action_follows_references_and_activity() {
        let mut payees = default_payees();
        let transactions = seeded_transactions(&payees);
        let netflix = id_of(&payees, "Netflix");
        let j_smith = id_of(&payees, "J Smith");
        let action =
            |payees: &[Payee], id| DeleteAction::for_payee(get(payees, id).unwrap(), &transactions);

        assert_eq!(action(&payees, netflix), DeleteAction::Delete);
        assert_eq!(action(&payees, j_smith), DeleteAction::Deactivate);
        set_active(&mut payees, j_smith, false).unwrap();
        assert!(!get(&payees, j_smith).unwrap().is_active);
        assert_eq!(action(&payees, j_smith), DeleteAction::Reactivate);
        set_active(&mut payees, j_smith, true).unwrap();
        assert!(get(&payees, j_smith).unwrap().is_active);
        delete_payee(&mut payees, &transactions, netflix).unwrap();
        assert_eq!(get(&payees, netflix), None);
    }

    #[test]
    fn set_active_toggles_and_counts_leave_inactive_payees_out() {
        let mut payees = default_payees();
        let active = active_count(&payees);
        let without = without_default_category_count(&payees);
        assert_eq!(active, payees.len() - 1);
        assert_eq!(without, 2); // J Smith and Hudson News; Don Quijote is inactive.

        let j_smith = id_of(&payees, "J Smith");
        set_active(&mut payees, j_smith, false).unwrap();
        assert_eq!(active_count(&payees), active - 1);
        assert_eq!(without_default_category_count(&payees), without - 1);
        set_active(&mut payees, j_smith, true).unwrap();
        assert_eq!(active_count(&payees), active);
        assert_eq!(
            set_active(&mut payees, 999, true),
            Err(PayeeError::NotFound)
        );
    }

    #[test]
    fn match_alias_is_a_case_insensitive_contains() {
        let payees = default_payees();
        assert_eq!(
            match_alias(&payees, "woolworths 2137 sydney au"),
            Some(id_of(&payees, "Woolworths"))
        );
        assert_eq!(
            match_alias(&payees, "NETFLIX.COM 866-579-7172"),
            Some(id_of(&payees, "Netflix"))
        );
        assert_eq!(match_alias(&payees, "SP AUSSIE CANDLE CO"), None);
    }

    #[test]
    fn the_longest_alias_wins_then_the_name_alphabetically() {
        let mut payees = default_payees();
        // "BP SERVICE STN" beats a shorter alias another Payee holds.
        insert_payee(&mut payees, &draft("Bpay Biller", &["BP SERV"])).unwrap();
        assert_eq!(
            match_alias(&payees, "BP SERVICE STN 4471"),
            Some(id_of(&payees, "BP"))
        );
        // Equal lengths: the alphabetically first name wins.
        insert_payee(&mut payees, &draft("Zed", &["ZZZZ"])).unwrap();
        insert_payee(&mut payees, &draft("Abe", &["YYYY"])).unwrap();
        assert_eq!(
            match_alias(&payees, "ZZZZ YYYY"),
            Some(id_of(&payees, "Abe"))
        );
    }

    #[test]
    fn inactive_payees_do_not_match() {
        let mut payees = default_payees();
        let bp = id_of(&payees, "BP");
        set_active(&mut payees, bp, false).unwrap();
        assert_eq!(match_alias(&payees, "BP SERVICE STN 4471"), None);
    }

    #[test]
    fn usage_counts_every_split_and_totals_base_unit_ones() {
        let payees = default_payees();
        let accounts = default_accounts();
        let transactions = seeded_transactions(&payees);
        let j_smith = id_of(&payees, "J Smith");
        let j_smith_usage = usage(&transactions, &accounts, Some("aud"), j_smith);
        assert_eq!(j_smith_usage.splits, 4);
        assert_eq!(
            j_smith_usage.total,
            Money(BigDecimal::new((-64_000).into(), 2))
        );

        let other_unit = usage(&transactions, &accounts, Some("xyz"), j_smith);
        assert_eq!(other_unit.splits, 4);
        assert_eq!(other_unit.total, Money(BigDecimal::new(0.into(), 2)));
    }
}
