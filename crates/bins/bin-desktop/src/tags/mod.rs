//! Shared stub Tags, seeded from the Tags handoff's sample list (`docs/ux/desktop-mockups/19-tags/`) plus the
//! names the Transactions seed draws on. `gpui`-free and side-effect free, so every rule is
//! unit-tested without a window.
//!
//! The rules are ADR-0015's and its amendments (#352–#355):
//!
//! - A Tag's name is required and unique by its [`normalise`]d form (Unicode lower-cased, letters
//!   and digits only), so `work trip` is refused while `work-trip` exists. A name that normalises to
//!   nothing is refused too. Typing a Tag on a Split reuses an exact case-insensitive match.
//! - A Tag has an optional colour: the user's data, not a Colour Theme role.
//! - Removing a Tag always hard-deletes it, used or not: it untags every Split and never touches
//!   their amounts, Categories or Payees. `is_active` is a separate toggle; an inactive Tag drops
//!   out of the "N tags" count but keeps its name taken.
//! - Merging moves every Split from the source to the target (without doubling a Split that
//!   already carries the target) and deletes the source; the target keeps its name, colour and
//!   `is_active`. Likely duplicates exist only in data that broke the uniqueness rule (e.g. from
//!   Sync), so the seed carries one such pair on purpose.
//! - Usage is all time across every Account: distinct Transactions, the newest one's date, and a
//!   total that only sums when every Split carrying the Tag shares one Unit.

pub(crate) mod form;

pub use lib_tags::*;

#[cfg(test)]
mod tests {
    #[test]
    fn sorted_by_name_ignores_case() {
        let tags = default_tags();
        let names: Vec<String> = sorted_by_name(&tags)
            .iter()
            .map(|tag| tag.name.to_lowercase())
            .collect();
        let mut expected = names.clone();
        expected.sort();
        assert_eq!(names, expected);
    }

    use bigdecimal::BigDecimal;
    use chrono::NaiveDate;
    use lib_accounts::Account;
    use lib_core::{Money, Total, TransactionStatus};
    use lib_transactions::Transaction;

    use super::*;
    use crate::{
        accounts::default_accounts,
        categories::default_categories,
        payees::default_payees,
        transactions::{Split, default_transactions},
    };

    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 9, 19).unwrap()
    }

    pub(super) fn seeded() -> (Vec<Account>, Vec<Tag>, Vec<Transaction>) {
        let accounts = default_accounts();
        let tags = default_tags();
        let transactions = default_transactions(
            &accounts,
            &default_categories(),
            &default_payees(),
            &tags,
            today(),
        );
        (accounts, tags, transactions)
    }

    fn id(tags: &[Tag], name: &str) -> u32 {
        tags.iter().find(|t| t.name == name).unwrap().id
    }

    fn draft(name: &str) -> TagDraft {
        TagDraft {
            name: name.to_string(),
            color: None,
        }
    }

    fn split(cents: i64, tag_ids: &[u32]) -> Split {
        Split {
            amount: Money(BigDecimal::new(cents.into(), 2)),
            category_id: 7,
            payee_id: None,
            tag_ids: tag_ids.to_vec(),
        }
    }

    fn transaction(id: u32, account_id: u32, date: NaiveDate, splits: Vec<Split>) -> Transaction {
        Transaction {
            id,
            date,
            account_id,
            status: TransactionStatus::Open,
            is_flagged: false,
            description: None,
            splits,
        }
    }

    fn account_id(accounts: &[Account], name: &str) -> u32 {
        accounts.iter().find(|a| a.name == name).unwrap().id
    }

    #[test]
    fn normalise_lower_cases_and_keeps_only_letters_and_digits() {
        assert_eq!(normalise("Work-Trip"), "worktrip");
        assert_eq!(normalise(" work_trip 2 "), "worktrip2");
        assert_eq!(normalise("Überweisung"), "überweisung");
        assert_eq!(normalise("--- !"), "");
    }

    #[test]
    fn seeded_tags_break_the_rule_only_for_the_one_duplicate_pair() {
        let (_, tags, transactions) = seeded();
        let groups = duplicate_groups(&tags, &transactions);
        assert_eq!(
            groups,
            vec![DuplicateGroup {
                target: id(&tags, "work-trip"),
                duplicates: vec![id(&tags, "Work Trip")],
            }]
        );
    }

    #[test]
    fn the_seed_has_one_inactive_tag_left_out_of_the_count() {
        let (_, tags, _) = seeded();
        assert!(!get(&tags, id(&tags, "Bali 2025")).unwrap().is_active);
        assert_eq!(active_count(&tags), tags.len() - 1);
    }

    #[test]
    fn seeded_usage_keeps_the_mockups_order() {
        let (_, tags, transactions) = seeded();
        let order: Vec<&str> = sorted_by_usage(&tags, &transactions)
            .iter()
            .map(|t| t.name.as_str())
            .take(6)
            .collect();
        assert_eq!(
            order,
            [
                "shared",
                "reimbursable",
                "tax-deductible",
                "work-trip",
                "gift",
                "one-off"
            ]
        );
        assert_eq!(transaction_count(&transactions, id(&tags, "one-off")), 3);
        assert!(transaction_count(&transactions, id(&tags, "Work Trip")) > 0);
        assert!(transaction_count(&transactions, id(&tags, "Bali 2025")) > 0);
    }

    #[test]
    fn find_by_name_ignores_case_but_not_punctuation() {
        let tags = default_tags();
        assert_eq!(find_by_name(&tags, "SHARED"), Some(id(&tags, "shared")));
        assert_eq!(find_by_name(&tags, "tax deductible"), None);
        assert_eq!(find_by_name(&tags, "missing"), None);
    }

    #[test]
    fn insert_trims_the_name_and_starts_active() {
        let mut tags = default_tags();
        let new = insert_tag(
            &mut tags,
            &TagDraft {
                name: "  holiday ".to_string(),
                color: Some(swatches()[3].clone()),
            },
        )
        .unwrap();
        let tag = get(&tags, new).unwrap();
        assert_eq!(tag.name, "holiday");
        assert_eq!(tag.color, Some(swatches()[3].clone()));
        assert!(tag.is_active);
        assert_eq!(new, tags.iter().map(|t| t.id).max().unwrap());
    }

    #[test]
    fn insert_refuses_empty_punctuation_only_and_normalised_duplicates() {
        let mut tags = default_tags();
        assert_eq!(
            insert_tag(&mut tags, &draft("   ")),
            Err(TagError::NameRequired)
        );
        assert_eq!(
            insert_tag(&mut tags, &draft("- !")),
            Err(TagError::NoLetterOrDigit)
        );
        assert_eq!(
            insert_tag(&mut tags, &draft("Tax Deductible")),
            Err(TagError::DuplicateName("tax-deductible".to_string()))
        );
        // An inactive Tag's name stays taken.
        assert_eq!(
            insert_tag(&mut tags, &draft("bali-2025")),
            Err(TagError::DuplicateName("Bali 2025".to_string()))
        );
    }

    #[test]
    fn edit_renames_and_recolours_and_may_keep_its_own_name() {
        let mut tags = default_tags();
        let shared = id(&tags, "shared");
        edit_tag(&mut tags, shared, &draft("Shared")).unwrap();
        assert_eq!(get(&tags, shared).unwrap().name, "Shared");
        assert_eq!(get(&tags, shared).unwrap().color, None);
        assert_eq!(
            edit_tag(&mut tags, shared, &draft("gift!")),
            Err(TagError::DuplicateName("gift".to_string()))
        );
        assert_eq!(
            edit_tag(&mut tags, 999, &draft("x")),
            Err(TagError::NotFound)
        );
    }

    #[test]
    fn set_active_toggles() {
        let mut tags = default_tags();
        let gift = id(&tags, "gift");
        set_active(&mut tags, gift, false).unwrap();
        assert!(!get(&tags, gift).unwrap().is_active);
        set_active(&mut tags, gift, true).unwrap();
        assert!(get(&tags, gift).unwrap().is_active);
        assert_eq!(set_active(&mut tags, 999, false), Err(TagError::NotFound));
    }

    #[test]
    fn remove_untags_every_split_and_keeps_the_transactions() {
        let (_, mut tags, mut transactions) = seeded();
        let before = transactions.clone();
        let shared = id(&tags, "shared");
        remove_tag(&mut tags, &mut transactions, shared).unwrap();
        assert!(get(&tags, shared).is_none());
        assert_eq!(transaction_count(&transactions, shared), 0);
        assert_eq!(transactions.len(), before.len());
        for (after, before) in transactions.iter().zip(&before) {
            assert_eq!(after.total(), before.total());
            for (a, b) in after.splits.iter().zip(&before.splits) {
                assert_eq!((a.category_id, a.payee_id), (b.category_id, b.payee_id));
            }
        }
        assert_eq!(
            remove_tag(&mut tags, &mut transactions, shared),
            Err(TagError::NotFound)
        );
    }

    #[test]
    fn remove_deletes_an_unused_tag_too() {
        let (_, mut tags, mut transactions) = seeded();
        let new = insert_tag(&mut tags, &draft("unused")).unwrap();
        remove_tag(&mut tags, &mut transactions, new).unwrap();
        assert!(get(&tags, new).is_none());
    }

    #[test]
    fn merge_retags_without_doubling_and_deletes_the_source() {
        let mut tags = default_tags();
        let (source, target, other) = (id(&tags, "Work Trip"), id(&tags, "work-trip"), 1);
        let day = today();
        let mut transactions = vec![
            transaction(1, 2, day, vec![split(-100, &[source])]),
            transaction(
                2,
                2,
                day,
                vec![split(-200, &[source, target]), split(-50, &[other])],
            ),
            transaction(3, 2, day, vec![split(-300, &[target])]),
        ];
        merge_tags(&mut tags, &mut transactions, source, target).unwrap();
        assert!(get(&tags, source).is_none());
        assert_eq!(transactions[0].splits[0].tag_ids, vec![target]);
        assert_eq!(transactions[1].splits[0].tag_ids, vec![target]);
        assert_eq!(transactions[1].splits[1].tag_ids, vec![other]);
        assert_eq!(transaction_count(&transactions, target), 3);
        // The target keeps its own name, colour and state.
        let kept = get(&tags, target).unwrap();
        assert_eq!(kept.name, "work-trip");
        assert_eq!(kept.color, Some(swatches()[4].clone()));
    }

    #[test]
    fn merge_refuses_itself_and_missing_tags_and_allows_inactive_ones() {
        let (_, mut tags, mut transactions) = seeded();
        let gift = id(&tags, "gift");
        let bali = id(&tags, "Bali 2025");
        assert_eq!(
            merge_tags(&mut tags, &mut transactions, gift, gift),
            Err(TagError::MergeIntoItself)
        );
        assert_eq!(
            merge_tags(&mut tags, &mut transactions, 999, gift),
            Err(TagError::NotFound)
        );
        let expected =
            transaction_count(&transactions, gift) + transaction_count(&transactions, bali);
        merge_tags(&mut tags, &mut transactions, bali, gift).unwrap();
        assert_eq!(transaction_count(&transactions, gift), expected);
        assert!(get(&tags, gift).unwrap().is_active);
    }

    #[test]
    fn a_split_reuses_an_exact_match_and_refuses_a_normalised_one() {
        let tags = default_tags();
        assert_eq!(
            resolve_split_tag(&tags, " SHARED "),
            Ok(SplitTag::Existing(id(&tags, "shared")))
        );
        assert_eq!(
            resolve_split_tag(&tags, "one off"),
            Err(TagError::DuplicateName("one-off".to_string()))
        );
        assert_eq!(resolve_split_tag(&tags, "camping"), Ok(SplitTag::New));
        assert_eq!(resolve_split_tag(&tags, ""), Err(TagError::NameRequired));
    }

    #[test]
    fn usage_counts_distinct_transactions_sums_the_tagged_splits_and_takes_the_newest_date() {
        let accounts = default_accounts();
        let everyday = account_id(&accounts, "ANZ Everyday");
        let day = today();
        let transactions = vec![
            transaction(
                1,
                everyday,
                day - chrono::Duration::days(3),
                vec![split(-1_000, &[5]), split(-250, &[5]), split(-9_999, &[])],
            ),
            // Future-dated still counts as last used.
            transaction(
                2,
                everyday,
                day + chrono::Duration::days(2),
                vec![split(-500, &[5])],
            ),
            transaction(3, everyday, day, vec![split(-700, &[6])]),
        ];
        let used = usage(&transactions, &accounts, 5);
        assert_eq!(used.transactions, 2);
        assert_eq!(
            used.total,
            Total::Single {
                unit: "aud".to_string(),
                amount: Money(BigDecimal::new((-1_750).into(), 2)),
            }
        );
        assert_eq!(used.last_used, Some(day + chrono::Duration::days(2)));
    }

    #[test]
    fn usage_across_units_is_mixed_and_an_unused_tag_is_empty() {
        let accounts = default_accounts();
        let day = today();
        let transactions = vec![
            transaction(
                1,
                account_id(&accounts, "ANZ Everyday"),
                day,
                vec![split(-1_000, &[5])],
            ),
            transaction(
                2,
                account_id(&accounts, "Bitcoin"),
                day,
                vec![split(-500, &[5])],
            ),
        ];
        assert_eq!(usage(&transactions, &accounts, 5).total, Total::Mixed);
        let unused = usage(&transactions, &accounts, 6);
        assert_eq!(
            (unused.transactions, unused.total, unused.last_used),
            (0, Total::Empty, None)
        );
    }

    #[test]
    fn sort_ties_fall_back_to_the_name() {
        let tags = vec![
            Tag {
                id: 1,
                name: "zeta".to_string(),
                color: None,
                is_active: true,
            },
            Tag {
                id: 2,
                name: "Alpha".to_string(),
                color: None,
                is_active: true,
            },
            Tag {
                id: 3,
                name: "beta".to_string(),
                color: None,
                is_active: true,
            },
        ];
        let transactions = vec![transaction(1, 2, today(), vec![split(-1, &[3])])];
        let order: Vec<u32> = sorted_by_usage(&tags, &transactions)
            .iter()
            .map(|t| t.id)
            .collect();
        assert_eq!(order, [3, 2, 1]);
    }

    #[test]
    fn the_suggested_target_is_active_then_more_used_then_older() {
        let tag = |id, name: &str, is_active| Tag {
            id,
            name: name.to_string(),
            color: None,
            is_active,
        };
        let day = today();
        let used = |id, tag_id| transaction(id, 2, day, vec![split(-1, &[tag_id])]);

        // More Transactions wins among active Tags.
        let tags = vec![tag(1, "Shared", true), tag(2, "shared", true)];
        let transactions = vec![used(1, 2)];
        let groups = duplicate_groups(&tags, &transactions);
        assert_eq!(
            groups[0],
            DuplicateGroup {
                target: 2,
                duplicates: vec![1]
            }
        );
        assert_eq!(duplicate_of(&groups, 1), Some(2));
        assert_eq!(duplicate_of(&groups, 2), None);

        // Active beats more used.
        let tags = vec![tag(1, "Shared", true), tag(2, "shared", false)];
        assert_eq!(duplicate_groups(&tags, &transactions)[0].target, 1);

        // A tie goes to the older (lower id); every other member is flagged against it.
        let tags = vec![
            tag(4, "work trip", true),
            tag(3, "Work-Trip", true),
            tag(5, "WORK_TRIP", false),
        ];
        assert_eq!(
            duplicate_groups(&tags, &[]),
            vec![DuplicateGroup {
                target: 3,
                duplicates: vec![4, 5]
            }]
        );
    }
}
