//! [`TagFixture`]: the in-memory [`TagStore`] every ticket in the "Tags catalog screen, views
//! and popup" map (issue #125) and the "Tags right pane" map (issue #131) builds against,
//! seeded with a handful of plausible cross-cutting Tags — ADR-0015's own worked example is
//! "Japan Trip 2026", so that's the first one seeded here.
//!
//! Seeded with a deliberate mix: `Home Renovation` has zero `tagged_transaction_count` (the
//! "nothing lost" delete case, and the right pane's own "zero widgets, no panic" case),
//! `Japan Trip 2026`/`Tax Deductible` have non-zero counts (the "N transactions will lose this
//! tag" delete case, and real right-pane data), and `Old Project` is inactive (the `za` case).
//!
//! **Right-pane data** (`TagTransaction` rows, and the `category_breakdown`/`monthly_spend`
//! derived from them) is generated on demand by [`transactions_for`], deterministically seeded
//! from a Tag's own id — mirroring `crate::account::fixture::ledger_for`'s own "generate,
//! don't store" technique — from a `category_pool` of real leaf category paths copied out of a
//! temporary `crate::category::CategoryFixture` at seed time (see [`expense_leaf_paths`]). Only
//! the plain path *strings* are copied; nothing here holds a `RowID` back to Category, and
//! `CategoryFixture` itself is discarded once its paths are read.

use bigdecimal::{BigDecimal, FromPrimitive};
use chrono::{DateTime, Datelike, Duration, Months, NaiveDate, Utc};
use lib_core::{Money, RowID};

use super::{Tag, TagError, TagStore, TagTransaction};
use crate::category::{CategoryFixture, CategoryStore};

/// The fixture's fixed "now" — matches `crate::account::fixture`'s own `2026-09-08`, so
/// anything cross-referencing multiple screens' fixtures (none do yet, but a future one might)
/// agrees on what "today" is.
const FIXTURE_NOW: NaiveDate = match NaiveDate::from_ymd_opt(2026, 9, 8) {
    Some(date) => date,
    None => panic!("fixed literal is a valid date"),
};

/// How many trailing months the "Tagged spend" sparkline plots — matches
/// `crate::account::fixture`'s own `CHART_MONTHS` convention (`view::accounts`'s 24-month
/// balance-line window).
const SPEND_MONTHS: usize = 24;

const FAKE_PAYEES: &[&str] = &[
    "JR East",
    "Qantas",
    "Hotel Granvia",
    "Kura Sushi",
    "Suica top-up",
    "BP",
    "Bunnings",
    "Officeworks",
];

/// A tiny xorshift PRNG step — deterministic across runs/platforms, the same technique
/// `crate::account::fixture`'s own fake ledger already uses (no `rand` dependency needed).
fn xorshift(seed: u64) -> u64 {
    let mut seed = seed;
    seed ^= seed << 13;
    seed ^= seed >> 7;
    seed ^= seed << 17;
    seed
}

/// A deterministic seed derived from a `RowID`, so the same Tag always generates the same fake
/// transactions — mirrors `crate::account::fixture::seed_from_id`.
fn seed_from_id(id: RowID) -> u64 {
    let uuid = id.into_uuid();
    let bytes = uuid.as_bytes();
    u64::from_be_bytes(
        bytes[8..16]
            .try_into()
            .expect("a uuid's byte array is always at least 16 bytes long"),
    )
}

fn round_money(amount: f64) -> Money {
    Money(
        BigDecimal::from_f64(amount)
            .unwrap_or_default()
            .with_scale(2),
    )
}

/// Walks `id` up through `categories` to its root, lowercasing each name along the way — a
/// local copy of `popup::category::path::ancestor_names`'s own technique (that helper lives
/// under `popup`, a UI concern this domain module has no business depending on).
fn ancestor_path(categories: &CategoryFixture, id: RowID) -> Vec<String> {
    let mut names = Vec::new();
    let mut current = categories.find(id);
    while let Some(node) = current {
        names.push(node.name.to_lowercase());
        current = node
            .parent_id
            .and_then(|parent_id| categories.find(parent_id));
    }
    names.reverse();
    names
}

/// Every leaf under `categories`' own `Expenses` root, as a `"parent/child"`-style path with
/// the root segment itself dropped (`"food/restaurants"`, not `"expenses/food/restaurants"`) —
/// plain display strings for "Where it lands", copied once from a temporary `CategoryFixture`
/// and never referenced back to it again (see this module's own doc).
fn expense_leaf_paths(categories: &CategoryFixture) -> Vec<String> {
    let mut paths: Vec<String> = categories
        .nodes()
        .iter()
        .filter(|node| categories.children(node.id).is_empty())
        .filter(|node| {
            categories
                .root(node.id)
                .is_some_and(|root| root.name.eq_ignore_ascii_case("expenses"))
        })
        .map(|node| {
            let mut segments = ancestor_path(categories, node.id);
            if segments.len() > 1 {
                segments.remove(0); // drop the "expenses" root segment
            }
            segments.join("/")
        })
        .collect();
    paths.sort();
    paths
}

/// Picks out the first path containing each of `needles`, in order — lets each seeded Tag get
/// a themed pool (e.g. a trip's food/transport spend) drawn from real category paths rather
/// than the whole undifferentiated leaf list.
fn pool_containing(paths: &[String], needles: &[&str]) -> Vec<String> {
    needles
        .iter()
        .filter_map(|needle| paths.iter().find(|path| path.contains(needle)).cloned())
        .collect()
}

/// `as_of`'s own month, `months_back` months earlier, as `(first day, last day)` — a local copy
/// of `crate::account::fixture::month_end`'s technique, extended to return the month's start
/// too (`monthly_spend_for` sums a window, not a running balance, so it needs both ends).
fn month_bounds(as_of: NaiveDate, months_back: usize) -> (NaiveDate, NaiveDate) {
    let first_of_as_of_month = NaiveDate::from_ymd_opt(as_of.year(), as_of.month(), 1)
        .expect("as_of's own year/month with day 1 is always valid");
    let first_of_target_month = first_of_as_of_month
        .checked_sub_months(Months::new(months_back as u32))
        .expect("months_back stays well within chrono's representable range");
    let first_of_next_month = first_of_target_month
        .checked_add_months(Months::new(1))
        .expect("adding one month to a valid first-of-month date stays in range");
    (
        first_of_target_month,
        first_of_next_month - Duration::days(1),
    )
}

/// Generates `tag`'s fixture-simulated transaction rows, newest first, deterministically seeded
/// from its id — see this module's own doc. Empty for `tagged_transaction_count == 0` or an
/// empty `category_pool` (a new, just-created Tag has both).
///
/// The window runs from `tag.created_on` (clamped to no earlier than [`SPEND_MONTHS`] months
/// before [`FIXTURE_NOW`]) through to [`FIXTURE_NOW`] itself, with the first and last generated
/// rows forced onto those two dates exactly — the same "force the endpoints, scatter the rest"
/// technique `crate::account::fixture::ledger_for` uses, which is what guarantees
/// `monthly_spend_for`'s own final month is never zero for a Tag that actually has
/// transactions, and that the flat run before a recently-created Tag's own `created_on` is
/// genuine (not an artefact of a row landing earlier by chance).
pub(super) fn transactions_for(tag: &Tag) -> Vec<TagTransaction> {
    let count = tag.tagged_transaction_count as usize;
    if count == 0 || tag.category_pool.is_empty() {
        return Vec::new();
    }

    let window_floor = FIXTURE_NOW
        .checked_sub_months(Months::new(SPEND_MONTHS as u32))
        .unwrap_or(FIXTURE_NOW);
    let window_start = tag.created_on.max(window_floor);
    let span_days = (FIXTURE_NOW - window_start).num_days().max(0) as u64;

    let mut seed = seed_from_id(tag.id);
    let mut rows = Vec::with_capacity(count);

    for index in 0..count {
        let offset_days = if index == 0 {
            0
        } else if index == count - 1 {
            span_days
        } else {
            seed = xorshift(seed);
            seed % (span_days + 1)
        };
        let date = window_start + Duration::days(offset_days as i64);

        seed = xorshift(seed);
        let category_path = tag.category_pool[(seed as usize) % tag.category_pool.len()].clone();

        seed = xorshift(seed);
        let payee = FAKE_PAYEES[(seed as usize) % FAKE_PAYEES.len()];

        seed = xorshift(seed);
        let wobble = 0.5 + (seed % 200) as f64 / 100.0; // 0.5..2.5
        let amount = round_money(60.0 * wobble);

        // Row index 1 always carries another Tag — deterministic, not left to chance, so a
        // Tag with two or more transactions always has real overlap evidence for the "N of M
        // carry another tag" footer statement (the ticket's own requirement).
        let other_tags = if index == 1 { 1 } else { 0 };

        rows.push(TagTransaction {
            date,
            payee,
            category_path,
            other_tags,
            amount,
        });
    }

    rows.sort_by_key(|row| std::cmp::Reverse(row.date));
    rows
}

/// `tag`'s spend grouped by category, biggest total first — derived from
/// [`transactions_for`], never stored.
pub(super) fn category_breakdown_for(tag: &Tag) -> Vec<(String, Money)> {
    let rows = transactions_for(tag);
    let mut totals: Vec<(String, BigDecimal)> = Vec::new();
    for row in &rows {
        match totals
            .iter_mut()
            .find(|(path, _)| *path == row.category_path)
        {
            Some(entry) => entry.1 += row.amount.0.clone(),
            None => totals.push((row.category_path.clone(), row.amount.0.clone())),
        }
    }
    totals.sort_by(|a, b| b.1.cmp(&a.1));
    totals
        .into_iter()
        .map(|(path, total)| (path, Money(total)))
        .collect()
}

/// One pass over `tag`'s generated transactions, summing spend into each trailing month —
/// mirrors `crate::account::fixture::monthly_balances_for`'s own "one pass, not `months`
/// separate queries" shape. Oldest month first.
pub(super) fn monthly_spend_for(tag: &Tag, months: usize, as_of: NaiveDate) -> Vec<Money> {
    let rows = transactions_for(tag);
    let mut result = Vec::with_capacity(months);
    for months_back in (0..months).rev() {
        let (start, end) = month_bounds(as_of, months_back);
        let total: BigDecimal = rows
            .iter()
            .filter(|row| row.date >= start && row.date <= end)
            .map(|row| row.amount.0.clone())
            .sum();
        result.push(Money(total));
    }
    result
}

/// An in-memory `TagStore`, seeded once with a fixed demo Tag list. Every mutating method
/// mutates this same `Vec` in place (discarded on quit, since nothing here is persisted).
pub struct TagFixture {
    tags: Vec<Tag>,
}

impl Default for TagFixture {
    fn default() -> Self {
        Self::new()
    }
}

impl TagFixture {
    /// Seeds the demo Tag list. Ids are built directly via `uuid::Builder::
    /// from_unix_timestamp_millis` with a plain incrementing counter standing in for a real
    /// v7 UUID's random bits — **not** `RowID::from_timestamp`, which delegates to
    /// `uuid::Uuid::new_v7` and is *not* actually deterministic across runs (the bug found and
    /// fixed in `crate::account::fixture`, issue #116's own resolution; still open against
    /// `crate::category::fixture`, issue #124). Confirmed deterministic here the same way: by
    /// diffing two separate `cargo test` process runs' generated ids.
    pub fn new() -> Self {
        let mut next = DateTime::parse_from_rfc3339("2021-01-01T00:00:00Z")
            .expect("fixed literal is a valid RFC3339 timestamp")
            .with_timezone(&Utc);
        let mut counter: u64 = 0;
        let mut id = move || {
            let millis = next.timestamp_millis() as u64;
            next += chrono::Duration::seconds(1);
            counter += 1;
            let mut counter_bytes = [0u8; 10];
            counter_bytes[2..10].copy_from_slice(&counter.to_be_bytes());
            let uuid =
                uuid::Builder::from_unix_timestamp_millis(millis, &counter_bytes).into_uuid();
            RowID::from_uuid(uuid)
        };

        let date = |year, month, day| {
            NaiveDate::from_ymd_opt(year, month, day).expect("seed literal is a valid date")
        };

        // Read once, then discarded — see this module's own doc on why nothing here keeps a
        // `CategoryFixture` or a `RowID` back to it.
        let categories = CategoryFixture::new();
        let leaves = expense_leaf_paths(&categories);

        let trip_pool = pool_containing(
            &leaves,
            &[
                "food/restaurants",
                "food/takeaway",
                "food/coffee",
                "transport/public",
            ],
        );
        let deductible_pool = pool_containing(
            &leaves,
            &[
                "transport/fuel",
                "health/pharmacy",
                "housing/insurance",
                "health/gp",
            ],
        );

        let tags = vec![
            Tag {
                id: id(),
                name: "Japan Trip 2026".to_string(),
                is_active: true,
                created_on: date(2025, 11, 1),
                updated_on: date(2026, 8, 15),
                tagged_transaction_count: 7,
                category_pool: trip_pool,
            },
            Tag {
                id: id(),
                name: "Home Renovation".to_string(),
                is_active: true,
                created_on: date(2026, 1, 10),
                updated_on: date(2026, 1, 10),
                tagged_transaction_count: 0,
                category_pool: leaves.clone(),
            },
            Tag {
                id: id(),
                name: "Tax Deductible".to_string(),
                is_active: true,
                created_on: date(2024, 7, 1),
                updated_on: date(2026, 6, 30),
                tagged_transaction_count: 23,
                category_pool: deductible_pool,
            },
            Tag {
                id: id(),
                name: "Old Project".to_string(),
                is_active: false,
                created_on: date(2023, 3, 15),
                updated_on: date(2023, 9, 1),
                tagged_transaction_count: 0,
                category_pool: leaves,
            },
        ];

        Self { tags }
    }

    /// Whether `name` case-insensitively clashes with any seeded Tag other than `excluding`
    /// (active or not — a deactivated Tag's name is still taken).
    fn name_taken(&self, name: &str, excluding: Option<RowID>) -> bool {
        self.tags
            .iter()
            .any(|tag| Some(tag.id) != excluding && tag.name.eq_ignore_ascii_case(name))
    }
}

impl TagStore for TagFixture {
    fn tags(&self) -> &[Tag] {
        &self.tags
    }

    fn find(&self, id: RowID) -> Option<&Tag> {
        self.tags.iter().find(|tag| tag.id == id)
    }

    fn create(&mut self, name: String, active: bool) -> Result<RowID, TagError> {
        if self.name_taken(&name, None) {
            return Err(TagError::DuplicateName { name });
        }
        let id = RowID::new();
        self.tags.push(Tag {
            id,
            name,
            is_active: active,
            created_on: FIXTURE_NOW,
            updated_on: FIXTURE_NOW,
            tagged_transaction_count: 0,
            category_pool: Vec::new(),
        });
        Ok(id)
    }

    fn update(&mut self, id: RowID, name: String, active: bool) -> Result<(), TagError> {
        if self.name_taken(&name, Some(id)) {
            return Err(TagError::DuplicateName { name });
        }
        let tag = self
            .tags
            .iter_mut()
            .find(|tag| tag.id == id)
            .ok_or(TagError::NotFound)?;
        tag.name = name;
        tag.is_active = active;
        tag.updated_on = FIXTURE_NOW;
        Ok(())
    }

    fn set_active(&mut self, id: RowID, active: bool) -> Result<(), TagError> {
        let tag = self
            .tags
            .iter_mut()
            .find(|tag| tag.id == id)
            .ok_or(TagError::NotFound)?;
        tag.is_active = active;
        tag.updated_on = FIXTURE_NOW;
        Ok(())
    }

    fn delete(&mut self, id: RowID) -> Result<(), TagError> {
        if self.find(id).is_none() {
            return Err(TagError::NotFound);
        }
        self.tags.retain(|tag| tag.id != id);
        Ok(())
    }

    fn transactions(&self, id: RowID) -> Vec<TagTransaction> {
        self.find(id).map(transactions_for).unwrap_or_default()
    }

    fn category_breakdown(&self, id: RowID) -> Vec<(String, Money)> {
        self.find(id)
            .map(category_breakdown_for)
            .unwrap_or_default()
    }

    fn monthly_spend(&self, id: RowID, months: usize, as_of: NaiveDate) -> Vec<Money> {
        match self.find(id) {
            Some(tag) => monthly_spend_for(tag, months, as_of),
            None => std::iter::repeat_with(|| Money(BigDecimal::from(0)))
                .take(months)
                .collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn find_by_name<'a>(store: &'a TagFixture, name: &str) -> &'a Tag {
        store
            .tags()
            .iter()
            .find(|tag| tag.name == name)
            .unwrap_or_else(|| panic!("fixture should seed a tag named {name}"))
    }

    #[test]
    fn seeds_four_tags_three_active_one_inactive() {
        let store = TagFixture::new();
        assert_eq!(store.tags().len(), 4);
        assert_eq!(store.tags().iter().filter(|tag| tag.is_active).count(), 3);
    }

    #[test]
    fn find_by_name_locates_every_seeded_tag() {
        let store = TagFixture::new();
        for name in [
            "Japan Trip 2026",
            "Home Renovation",
            "Tax Deductible",
            "Old Project",
        ] {
            find_by_name(&store, name);
        }
    }

    #[test]
    fn seeds_a_zero_count_tag_and_non_zero_count_tags() {
        let store = TagFixture::new();
        assert_eq!(
            find_by_name(&store, "Home Renovation").tagged_transaction_count,
            0
        );
        assert!(find_by_name(&store, "Japan Trip 2026").tagged_transaction_count > 0);
        assert!(find_by_name(&store, "Tax Deductible").tagged_transaction_count > 0);
    }

    #[test]
    fn create_rejects_a_case_insensitive_duplicate_against_an_active_tag() {
        let mut store = TagFixture::new();
        let result = store.create("japan trip 2026".to_string(), true);
        assert_eq!(
            result,
            Err(TagError::DuplicateName {
                name: "japan trip 2026".to_string()
            })
        );
    }

    #[test]
    fn create_rejects_a_case_insensitive_duplicate_against_an_inactive_tag() {
        let mut store = TagFixture::new();
        let result = store.create("OLD PROJECT".to_string(), true);
        assert_eq!(
            result,
            Err(TagError::DuplicateName {
                name: "OLD PROJECT".to_string()
            })
        );
    }

    #[test]
    fn create_succeeds_with_a_genuinely_new_name() {
        let mut store = TagFixture::new();
        let id = store
            .create("Wedding".to_string(), true)
            .expect("a new name should create successfully");
        let tag = store.find(id).expect("just created");
        assert_eq!(tag.name, "Wedding");
        assert!(tag.is_active);
        assert_eq!(tag.tagged_transaction_count, 0);
    }

    #[test]
    fn update_rejects_a_case_insensitive_clash_with_a_different_tag() {
        let mut store = TagFixture::new();
        let home_renovation = find_by_name(&store, "Home Renovation").id;
        let result = store.update(home_renovation, "tax deductible".to_string(), true);
        assert_eq!(
            result,
            Err(TagError::DuplicateName {
                name: "tax deductible".to_string()
            })
        );
    }

    #[test]
    fn update_allows_renaming_a_tag_to_its_own_current_name() {
        let mut store = TagFixture::new();
        let japan_trip = find_by_name(&store, "Japan Trip 2026").id;
        store
            .update(japan_trip, "Japan Trip 2026".to_string(), true)
            .expect("renaming to the same name should succeed");
    }

    #[test]
    fn update_changes_name_and_active_and_bumps_updated_on() {
        let mut store = TagFixture::new();
        let home_renovation = find_by_name(&store, "Home Renovation").id;
        store
            .update(home_renovation, "Renovation 2026".to_string(), false)
            .expect("should succeed");

        let tag = store.find(home_renovation).expect("still exists");
        assert_eq!(tag.name, "Renovation 2026");
        assert!(!tag.is_active);
        assert_eq!(tag.updated_on, FIXTURE_NOW);
    }

    #[test]
    fn set_active_toggles_without_touching_the_name() {
        let mut store = TagFixture::new();
        let old_project = find_by_name(&store, "Old Project").id;
        store.set_active(old_project, true).expect("should succeed");
        let tag = store.find(old_project).expect("still exists");
        assert_eq!(tag.name, "Old Project");
        assert!(tag.is_active);
    }

    #[test]
    fn delete_succeeds_regardless_of_tagged_transaction_count() {
        let mut store = TagFixture::new();
        let zero_count = find_by_name(&store, "Home Renovation").id;
        let non_zero_count = find_by_name(&store, "Japan Trip 2026").id;

        store
            .delete(zero_count)
            .expect("a zero-reference tag should delete");
        store
            .delete(non_zero_count)
            .expect("a non-zero-reference tag should delete unconditionally too");

        assert!(store.find(zero_count).is_none());
        assert!(store.find(non_zero_count).is_none());
        assert_eq!(store.tags().len(), 2);
    }

    #[test]
    fn delete_of_an_unknown_id_returns_not_found() {
        let mut store = TagFixture::new();
        assert_eq!(store.delete(RowID::new()), Err(TagError::NotFound));
    }

    // --- Right-pane fixture data (issue #132) ---

    #[test]
    fn expense_leaf_paths_are_real_leaves_under_expenses_only() {
        let categories = CategoryFixture::new();
        let leaves = expense_leaf_paths(&categories);
        assert!(!leaves.is_empty());
        assert!(leaves.contains(&"food/restaurants".to_string()));
        assert!(leaves.contains(&"transport/fuel".to_string()));
        // No income-side leaves (e.g. "salary/bonus") should have leaked in.
        assert!(!leaves.iter().any(|path| path.contains("salary")));
        // No non-leaf ("food" alone, with children) should appear.
        assert!(!leaves.contains(&"food".to_string()));
    }

    #[test]
    fn zero_reference_tag_has_no_transactions_breakdown_or_spend() {
        let store = TagFixture::new();
        let home_renovation = find_by_name(&store, "Home Renovation").id;

        assert!(store.transactions(home_renovation).is_empty());
        assert!(store.category_breakdown(home_renovation).is_empty());

        let spend = store.monthly_spend(home_renovation, SPEND_MONTHS, FIXTURE_NOW);
        assert_eq!(spend.len(), SPEND_MONTHS);
        assert!(spend.iter().all(|month| month.0 == 0));
    }

    #[test]
    fn transactions_len_matches_tagged_transaction_count() {
        let store = TagFixture::new();
        for name in ["Japan Trip 2026", "Tax Deductible"] {
            let tag = find_by_name(&store, name);
            assert_eq!(
                store.transactions(tag.id).len(),
                tag.tagged_transaction_count as usize,
                "{name}"
            );
        }
    }

    #[test]
    fn category_breakdown_sums_to_the_transactions_total_and_is_sorted_descending() {
        let store = TagFixture::new();
        let tag = find_by_name(&store, "Tax Deductible").id;

        let rows = store.transactions(tag);
        let rows_total: BigDecimal = rows.iter().map(|row| row.amount.0.clone()).sum();

        let breakdown = store.category_breakdown(tag);
        assert!(breakdown.len() > 1, "a genuinely mixed breakdown");
        let breakdown_total: BigDecimal =
            breakdown.iter().map(|(_, amount)| amount.0.clone()).sum();
        assert_eq!(breakdown_total, rows_total);

        for pair in breakdown.windows(2) {
            assert!(pair[0].1.0 >= pair[1].1.0, "should sort biggest first");
        }
    }

    #[test]
    fn monthly_spend_is_flat_zero_before_the_tags_own_created_on_month() {
        let store = TagFixture::new();
        let tag = find_by_name(&store, "Japan Trip 2026");
        let spend = store.monthly_spend(tag.id, SPEND_MONTHS, FIXTURE_NOW);
        assert_eq!(spend.len(), SPEND_MONTHS);

        // Japan Trip 2026 was created 2025-11-01 — comfortably inside the 24-month window
        // ending 2026-09-08, so its own first few months must be flat zero.
        assert_eq!(spend[0].0, BigDecimal::from(0));
    }

    #[test]
    fn monthly_spend_final_month_is_nonzero_for_a_tag_with_transactions() {
        let store = TagFixture::new();
        let tag = find_by_name(&store, "Tax Deductible").id;
        let spend = store.monthly_spend(tag, SPEND_MONTHS, FIXTURE_NOW);
        assert_ne!(
            *spend.last().expect("24 months requested"),
            Money(BigDecimal::from(0)),
            "the last generated row is forced onto FIXTURE_NOW, so its own month can't be empty"
        );
    }

    #[test]
    fn at_least_one_transaction_row_carries_another_tag() {
        let store = TagFixture::new();
        let tag = find_by_name(&store, "Japan Trip 2026").id;
        let rows = store.transactions(tag);
        assert!(rows.iter().any(|row| row.other_tags > 0));
    }

    #[test]
    fn transactions_are_sorted_newest_first() {
        let store = TagFixture::new();
        let tag = find_by_name(&store, "Tax Deductible").id;
        let rows = store.transactions(tag);
        for pair in rows.windows(2) {
            assert!(pair[0].date >= pair[1].date);
        }
    }
}
