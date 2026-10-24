//! The Payee Entity: a Payee with its Payee Aliases, the stub seed, and the rules every Payees
//! write and read keeps (the crate root lists them). Plain functions over a slice of Payees, so
//! the Interactors run them on the rows a port returns, and a Client not yet migrated (the TUI,
//! Import) on its own copy.

use lib_core::{CategoryId, PayeeId};

use crate::protocols::{PayeeUsage, PayeeUse};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Payee {
    pub id: PayeeId,
    /// The current, canonical name.
    pub name: String,
    /// Payee Aliases: former names and hand-authored match rules, normalised by
    /// [`normalise_alias`].
    pub aliases: Vec<String>,
    /// A leaf the Category id that pre-fills new Splits for this Payee.
    pub default_category: Option<CategoryId>,
    pub is_active: bool,
}

fn payee(id: PayeeId, name: &str, default_category: Option<CategoryId>, aliases: &[&str]) -> Payee {
    Payee {
        id,
        name: name.to_string(),
        aliases: aliases.iter().map(|alias| alias.to_string()).collect(),
        default_category,
        is_active: true,
    }
}

/// The handoff's sample Payees first (J Smith with no default Category), then the rest of the
/// names the Transactions seed draws on. Category ids are `categories::default_categories`' leaves.
pub fn default_payees() -> Vec<Payee> {
    const RENT: CategoryId = CategoryId::seed(2);
    const ELECTRICITY: CategoryId = CategoryId::seed(4);
    const WATER: CategoryId = CategoryId::seed(5);
    const GROCERIES: CategoryId = CategoryId::seed(7);
    const DINING: CategoryId = CategoryId::seed(8);
    const TRANSPORT: CategoryId = CategoryId::seed(9);
    const HOUSEHOLD: CategoryId = CategoryId::seed(10);
    const SALARY: CategoryId = CategoryId::seed(11);
    const INTEREST: CategoryId = CategoryId::seed(12);
    let mut payees = vec![
        payee(
            PayeeId::seed(1),
            "Woolworths",
            Some(GROCERIES),
            &["WOOLWORTHS", "WW SUPERMARKET", "WOOLIES"],
        ),
        payee(
            PayeeId::seed(2),
            "Coles",
            Some(GROCERIES),
            &["COLES", "COLES ONLINE"],
        ),
        payee(
            PayeeId::seed(3),
            "Netflix",
            Some(HOUSEHOLD),
            &["NETFLIX.COM"],
        ),
        payee(
            PayeeId::seed(4),
            "BP",
            Some(TRANSPORT),
            &["BP SERVICE STN", "BP CONNECT"],
        ),
        payee(
            PayeeId::seed(5),
            "Origin Energy",
            Some(ELECTRICITY),
            &["ORIGIN ENERGY", "ORIGIN DIRECT DEBIT"],
        ),
        payee(
            PayeeId::seed(6),
            "Officeworks",
            Some(HOUSEHOLD),
            &["OFFICEWORKS"],
        ),
        payee(
            PayeeId::seed(7),
            "Employer Pty Ltd",
            Some(SALARY),
            &["EMPLOYER PTY LTD PAY"],
        ),
        payee(PayeeId::seed(8), "J Smith", None, &[]),
        payee(
            PayeeId::seed(9),
            "Aldi Kelvin Grove",
            Some(GROCERIES),
            &["ALDI"],
        ),
        payee(PayeeId::seed(10), "Kura Sushi", Some(DINING), &[]),
        payee(PayeeId::seed(11), "Cafe Vittoria", Some(DINING), &[]),
        payee(PayeeId::seed(12), "Uber", Some(TRANSPORT), &["UBER"]),
        payee(
            PayeeId::seed(13),
            "Bunnings Warehouse",
            Some(HOUSEHOLD),
            &["BUNNINGS"],
        ),
        payee(PayeeId::seed(14), "Kmart", Some(HOUSEHOLD), &["KMART"]),
        payee(
            PayeeId::seed(15),
            "Sydney Water",
            Some(WATER),
            &["SYDNEY WATER"],
        ),
        payee(
            PayeeId::seed(16),
            "ANZ Banking Group",
            Some(INTEREST),
            &["ANZ INTEREST"],
        ),
        payee(PayeeId::seed(17), "Hudson News", None, &[]),
        payee(PayeeId::seed(18), "Don Quijote", None, &[]),
        payee(
            PayeeId::seed(19),
            "Ray White Rentals",
            Some(RENT),
            &["RAY WHITE"],
        ),
        // The Bills handoff's sample Bill Plans' Payees.
        payee(PayeeId::seed(20), "Telstra", Some(HOUSEHOLD), &["TELSTRA"]),
        payee(PayeeId::seed(21), "Fitness First", Some(HOUSEHOLD), &[]),
        payee(PayeeId::seed(22), "AAMI", Some(TRANSPORT), &[]),
        payee(
            PayeeId::seed(23),
            "Brisbane City Council",
            Some(HOUSEHOLD),
            &[],
        ),
    ];
    // One inactive Payee, so the list's dimmed "inactive" state has something to show.
    if let Some(don_quijote) = payees.iter_mut().find(|p| p.id == PayeeId::seed(18)) {
        don_quijote.is_active = false;
    }
    payees
}

/// The id of the Payee named `name` (exact, first match), if any.
pub fn find_by_name(payees: &[Payee], name: &str) -> Option<PayeeId> {
    payees
        .iter()
        .find(|payee| payee.name == name)
        .map(|payee| payee.id)
}

pub fn get(payees: &[Payee], id: PayeeId) -> Option<&Payee> {
    payees.iter().find(|payee| payee.id == id)
}

/// An alias as stored: trimmed and upper-cased, or `None` for empty or whitespace-only input.
pub fn normalise_alias(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_uppercase())
}

/// Errors for Payee operations. A variant carries the names the Presenter's `msg::` Message needs,
/// but the `#[error]` text never repeats them: it is for logs, which never hold a Payee's name.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PayeeError {
    #[error("a payee needs a name")]
    NameRequired,
    /// Another Payee already has this name (case-insensitively); carries that Payee's name.
    #[error("another payee has this name")]
    DuplicateName(String),
    /// Another Payee already owns this alias.
    #[error("another payee has this match rule")]
    AliasTaken { alias: String, owner: String },
    /// A Split or a Bill Plan names the Payee, so it can only be deactivated.
    #[error("a payee in use can only be deactivated")]
    InUse,
    #[error("payee not found")]
    NotFound,
}

/// What the Add and Edit dialogs submit: the aliases as typed, normalised on the way in.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PayeeDraft {
    pub name: String,
    pub default_category: Option<CategoryId>,
    pub aliases: Vec<String>,
}

/// Normalises and dedupes `aliases` (keeping first-seen order) and checks none belongs to another
/// Payee than `own_id`.
fn clean_aliases(
    payees: &[Payee],
    own_id: Option<PayeeId>,
    aliases: &[String],
) -> Result<Vec<String>, PayeeError> {
    let mut cleaned: Vec<String> = Vec::new();
    for alias in aliases.iter().filter_map(|raw| normalise_alias(raw)) {
        if cleaned.contains(&alias) {
            continue;
        }
        if let Some(owner) = alias_owner(payees, &alias).filter(|p| Some(p.id) != own_id) {
            return Err(PayeeError::AliasTaken {
                alias,
                owner: owner.name.clone(),
            });
        }
        cleaned.push(alias);
    }
    Ok(cleaned)
}

/// The Payee owning `alias` (as stored), if any.
pub fn alias_owner<'a>(payees: &'a [Payee], alias: &str) -> Option<&'a Payee> {
    payees
        .iter()
        .find(|payee| payee.aliases.iter().any(|a| a == alias))
}

/// Checks `name` is present, not another Payee's name and not another Payee's alias; returns it
/// trimmed.
pub fn clean_name(
    payees: &[Payee],
    own_id: Option<PayeeId>,
    name: &str,
) -> Result<String, PayeeError> {
    let name = name.trim();
    if name.is_empty() {
        return Err(PayeeError::NameRequired);
    }
    let folded = name.to_lowercase();
    if let Some(other) = payees
        .iter()
        .find(|p| Some(p.id) != own_id && p.name.to_lowercase() == folded)
    {
        return Err(PayeeError::DuplicateName(other.name.clone()));
    }
    let as_alias = name.to_uppercase();
    if let Some(owner) = alias_owner(payees, &as_alias).filter(|p| Some(p.id) != own_id) {
        return Err(PayeeError::AliasTaken {
            alias: as_alias,
            owner: owner.name.clone(),
        });
    }
    Ok(name.to_string())
}

/// Whether a new Payee could take `name`: present, and neither another Payee's name nor alias.
pub fn name_available(payees: &[Payee], name: &str) -> bool {
    clean_name(payees, None, name).is_ok()
}

/// Adds a new, active Payee and returns its id.
pub fn insert_payee(payees: &mut Vec<Payee>, draft: &PayeeDraft) -> Result<PayeeId, PayeeError> {
    let name = clean_name(payees, None, &draft.name)?;
    let aliases = clean_aliases(payees, None, &draft.aliases)?;
    let id = PayeeId::new();
    payees.push(Payee {
        id,
        name,
        aliases,
        default_category: draft.default_category,
        is_active: true,
    });
    Ok(id)
}

/// Replaces Payee `id`'s name, default Category and aliases with `draft`'s. A rename keeps the old
/// name as an alias, and drops the alias the new name equals.
pub fn edit_payee(payees: &mut [Payee], id: PayeeId, draft: &PayeeDraft) -> Result<(), PayeeError> {
    let current = get(payees, id).ok_or(PayeeError::NotFound)?;
    let name = clean_name(payees, Some(id), &draft.name)?;
    let mut typed = draft.aliases.clone();
    if name != current.name {
        typed.push(current.name.clone());
    }
    let new_name_alias = name.to_uppercase();
    let mut aliases = clean_aliases(payees, Some(id), &typed)?;
    aliases.retain(|alias| *alias != new_name_alias);

    let payee = payees
        .iter_mut()
        .find(|p| p.id == id)
        .ok_or(PayeeError::NotFound)?;
    payee.name = name;
    payee.aliases = aliases;
    payee.default_category = draft.default_category;
    Ok(())
}

/// Why a Payee delete was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum DeletePayeeRefusal {
    #[error("payee not found")]
    NotFound,
    /// A Split or a Bill Plan names the Payee, so it can only be deactivated.
    #[error("a payee in use can only be deactivated")]
    InUse { splits: usize, bill_plans: usize },
}

/// Removes Payee `id` and its aliases and returns the row, refused while a Split or a Bill Plan
/// names it: such a Payee can only be deactivated. Checked before anything is removed, so a
/// refusal leaves `payees` untouched.
pub fn delete_payee(
    payees: &mut Vec<Payee>,
    usage: &dyn PayeeUsage,
    id: PayeeId,
) -> Result<Payee, DeletePayeeRefusal> {
    let index = payees
        .iter()
        .position(|p| p.id == id)
        .ok_or(DeletePayeeRefusal::NotFound)?;
    let PayeeUse { splits, bill_plans } = usage.payee_usage(id);
    if splits + bill_plans > 0 {
        return Err(DeletePayeeRefusal::InUse { splits, bill_plans });
    }
    Ok(payees.remove(index))
}

/// Deactivates or reactivates Payee `id`.
pub fn set_active(payees: &mut [Payee], id: PayeeId, is_active: bool) -> Result<(), PayeeError> {
    let payee = payees
        .iter_mut()
        .find(|p| p.id == id)
        .ok_or(PayeeError::NotFound)?;
    payee.is_active = is_active;
    Ok(())
}

/// The active Payee whose alias `raw` contains (case-insensitively): the longest alias wins, then
/// the Payee name alphabetically.
pub fn match_alias(payees: &[Payee], raw: &str) -> Option<PayeeId> {
    let haystack = raw.to_uppercase();
    payees
        .iter()
        .filter(|payee| payee.is_active)
        .flat_map(|payee| {
            payee
                .aliases
                .iter()
                .filter(|alias| haystack.contains(alias.as_str()))
                .map(move |alias| (alias.len(), payee))
        })
        .max_by(|(a_len, a), (b_len, b)| a_len.cmp(b_len).then_with(|| b.name.cmp(&a.name)))
        .map(|(_, payee)| payee.id)
}

/// Payees A–Z, ignoring case: the Settings page's order (it has no usage columns to rank by).
pub fn sorted_by_name(payees: &[Payee]) -> Vec<&Payee> {
    let mut sorted: Vec<&Payee> = payees.iter().collect();
    sorted.sort_by(|a, b| {
        a.name
            .to_lowercase()
            .cmp(&b.name.to_lowercase())
            .then_with(|| a.id.cmp(&b.id))
    });
    sorted
}

/// Active Payees: the sidebar badge and the "N payees" figure.
pub fn active_count(payees: &[Payee]) -> usize {
    payees.iter().filter(|p| p.is_active).count()
}

/// Active Payees with no default Category: the "N without a default category" figure.
pub fn without_default_category_count(payees: &[Payee]) -> usize {
    payees
        .iter()
        .filter(|p| p.is_active && p.default_category.is_none())
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fake::InMemoryPayeeUsage;

    #[test]
    fn delete_returns_the_removed_row_and_refuses_an_unknown_id() {
        let mut payees = default_payees();
        let seeded = payees.len();
        let unused = InMemoryPayeeUsage::default();

        let removed = delete_payee(&mut payees, &unused, PayeeId::seed(1)).unwrap();
        assert_eq!(removed.id, PayeeId::seed(1));
        assert_eq!(payees.len(), seeded - 1);
        assert!(get(&payees, PayeeId::seed(1)).is_none());
        assert_eq!(
            delete_payee(&mut payees, &unused, PayeeId::seed(1)),
            Err(DeletePayeeRefusal::NotFound)
        );
    }

    #[test]
    fn a_payee_a_split_or_bill_plan_names_is_refused_and_left_in_place() {
        let mut payees = default_payees();
        let before = payees.clone();
        let mut usage = InMemoryPayeeUsage::default();
        let by_splits = PayeeUse {
            splits: 3,
            bill_plans: 0,
        };
        let by_plan = PayeeUse {
            splits: 0,
            bill_plans: 1,
        };
        usage.uses.insert(PayeeId::seed(1), by_splits);
        usage.uses.insert(PayeeId::seed(2), by_plan);

        assert_eq!(
            delete_payee(&mut payees, &usage, PayeeId::seed(1)),
            Err(DeletePayeeRefusal::InUse {
                splits: 3,
                bill_plans: 0
            })
        );
        assert_eq!(
            delete_payee(&mut payees, &usage, PayeeId::seed(2)),
            Err(DeletePayeeRefusal::InUse {
                splits: 0,
                bill_plans: 1
            })
        );
        assert_eq!(payees, before);
        // A named Payee can still be deactivated.
        set_active(&mut payees, PayeeId::seed(1), false).unwrap();
        assert!(!get(&payees, PayeeId::seed(1)).unwrap().is_active);
    }

    #[test]
    fn refused_writes_leave_the_rows_untouched() {
        let mut payees = default_payees();
        let before = payees.clone();
        let draft = PayeeDraft {
            name: before[0].name.clone(),
            aliases: Vec::new(),
            default_category: None,
        };

        assert!(insert_payee(&mut payees, &draft).is_err());
        assert!(edit_payee(&mut payees, before[1].id, &draft).is_err());
        assert!(edit_payee(&mut payees, PayeeId::seed(u32::MAX), &draft).is_err());
        assert_eq!(payees, before);
    }

    fn draft(name: &str, aliases: &[&str]) -> PayeeDraft {
        PayeeDraft {
            name: name.to_string(),
            default_category: None,
            aliases: aliases.iter().map(|a| a.to_string()).collect(),
        }
    }

    fn id_of(payees: &[Payee], name: &str) -> PayeeId {
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

    /// That each default Category is a leaf is another domain's rule, checked in `lib-ledger`'s
    /// seed tests, which see both seeds.
    #[test]
    fn seeded_aliases_are_normalised() {
        for payee in default_payees() {
            for alias in &payee.aliases {
                assert_eq!(normalise_alias(alias).as_ref(), Some(alias));
            }
        }
    }

    #[test]
    fn find_by_name_is_exact_and_aliases_are_not_names() {
        let payees = default_payees();
        assert_eq!(find_by_name(&payees, "Woolworths"), Some(PayeeId::seed(1)));
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
        let mut new = draft(
            " Aussie Candle Co ",
            &["aussie candle", "AUSSIE CANDLE ", " ", ""],
        );
        new.default_category = Some(CategoryId::seed(10));
        let id = insert_payee(&mut payees, &new).unwrap();
        assert!(!default_payees().iter().any(|p| p.id == id));
        let payee = get(&payees, id).unwrap();
        assert_eq!(payee.name, "Aussie Candle Co");
        assert_eq!(payee.aliases, vec!["AUSSIE CANDLE".to_string()]);
        assert_eq!(payee.default_category, Some(CategoryId::seed(10)));
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
        change.default_category = Some(CategoryId::seed(10));
        edit_payee(&mut payees, coles, &change).unwrap();
        let payee = get(&payees, coles).unwrap();
        assert_eq!(payee.aliases, vec!["COLES EXPRESS".to_string()]);
        assert_eq!(payee.default_category, Some(CategoryId::seed(10)));
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
            set_active(&mut payees, PayeeId::seed(999), true),
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
    fn a_refusals_log_text_never_holds_a_payee_name_or_match_rule() {
        let refusals = [
            PayeeError::DuplicateName("Coles".into()),
            PayeeError::AliasTaken {
                alias: "WOOLIES".into(),
                owner: "Woolworths".into(),
            },
        ];
        for refusal in refusals {
            let text = refusal.to_string();
            for name in ["Coles", "WOOLIES", "Woolworths"] {
                assert!(!text.contains(name), "{text:?} holds {name}");
            }
        }
    }
}
