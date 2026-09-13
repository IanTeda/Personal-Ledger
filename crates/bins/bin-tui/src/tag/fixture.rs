//! [`TagFixture`]: the in-memory [`TagStore`] every ticket in the "Tags catalog screen, views
//! and popup" map builds against, seeded with a handful of plausible cross-cutting Tags —
//! ADR-0015's own worked example is "Japan Trip 2026", so that's the first one seeded here.
//!
//! Seeded with a deliberate mix: `Home Renovation` has zero `tagged_transaction_count` (the
//! "nothing lost" delete case), `Japan Trip 2026`/`Tax Deductible` have non-zero counts (the
//! "N transactions will lose this tag" delete case), and `Old Project` is inactive (the `za`
//! case).

use chrono::{DateTime, NaiveDate, Utc};
use lib_core::RowID;

use super::{Tag, TagError, TagStore};

/// The fixture's fixed "now" — matches `crate::account::fixture`'s own `2026-09-08`, so
/// anything cross-referencing multiple screens' fixtures (none do yet, but a future one might)
/// agrees on what "today" is.
const FIXTURE_NOW: NaiveDate = match NaiveDate::from_ymd_opt(2026, 9, 8) {
    Some(date) => date,
    None => panic!("fixed literal is a valid date"),
};

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
    /// diffing two separate `cargo test` process runs' generated ids, not by inspection alone.
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

        let tags = vec![
            Tag {
                id: id(),
                name: "Japan Trip 2026".to_string(),
                is_active: true,
                created_on: date(2025, 11, 1),
                updated_on: date(2026, 8, 15),
                tagged_transaction_count: 7,
            },
            Tag {
                id: id(),
                name: "Home Renovation".to_string(),
                is_active: true,
                created_on: date(2026, 1, 10),
                updated_on: date(2026, 1, 10),
                tagged_transaction_count: 0,
            },
            Tag {
                id: id(),
                name: "Tax Deductible".to_string(),
                is_active: true,
                created_on: date(2024, 7, 1),
                updated_on: date(2026, 6, 30),
                tagged_transaction_count: 23,
            },
            Tag {
                id: id(),
                name: "Old Project".to_string(),
                is_active: false,
                created_on: date(2023, 3, 15),
                updated_on: date(2023, 9, 1),
                tagged_transaction_count: 0,
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
}
