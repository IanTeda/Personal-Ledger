//! # Change Set Database Model
//!
//! Defines the `ChangeSet` struct, which represents one row of the Sync Server's durable
//! Change Set log (`change_sets` table) -- the unit of data pushed and pulled between
//! Clients to propagate one Client's local edits to the others, at field granularity.
//! See [ADR-0009](https://github.com/IanTeda/Personal-Ledger/blob/feasibility/docs/adr/0009-lww-sqlite-change-set-log.md)
//! for the schema decision and `CONTEXT.md`'s Change Set glossary entry.

/// Database row model representing one persisted Change Set.
///
/// Maps directly to the `change_sets` table. Each row is one field-level edit: the
/// target `table_name`/`row_id`/`field_name`, the new `value` (`None` means the field
/// was set to NULL), the [`HybridLogicalClock`](lib_core::HybridLogicalClock)
/// timestamp used for last-write-wins comparison, the originating `client_id` for
/// tie-breaking, and a `version` field held for a future CRDT/manual-merge upgrade path.
#[derive(Debug, sqlx::FromRow, serde::Deserialize, serde::Serialize, PartialEq, Clone)]
pub struct ChangeSet {
    /// Stable, time-ordered identifier for this Change Set. Also the Sync Server's
    /// pull cursor: Clients pull every Change Set with an `id` greater than the last
    /// one they have already applied.
    pub id: lib_core::RowID,

    /// The target table this Change Set applies to (e.g. `"categories"`).
    pub table_name: String,

    /// The target row's identifier within `table_name`.
    pub row_id: lib_core::RowID,

    /// The target column within the row.
    pub field_name: String,

    /// The field's new value, serialised as text. `None` represents SQL `NULL`.
    pub value: Option<String>,

    /// The Hybrid Logical Clock timestamp this edit was made at, used for
    /// last-write-wins conflict resolution between Change Sets targeting the same
    /// `table_name`/`row_id`/`field_name`.
    pub hlc: lib_core::HybridLogicalClock,

    /// The stable identifier of the Client that produced this Change Set, used to
    /// tie-break Change Sets whose `hlc` values are otherwise equal.
    pub client_id: lib_core::RowID,

    /// Version/parent-version placeholder for a future CRDT or manual-merge upgrade
    /// path (ADR-0009). Not consumed by plain last-write-wins.
    pub version: i64,

    /// UTC timestamp the Sync Server inserted this Change Set into its log. Distinct
    /// from `hlc`: this is the log's own bookkeeping time, not the edit's logical time.
    pub created_on: chrono::DateTime<chrono::Utc>,
}

impl ChangeSet {
    /// Generate a mock `ChangeSet` instance with randomised test data.
    ///
    /// **Note**: This function is only available in test builds.
    #[cfg(test)]
    pub fn mock() -> Self {
        use crate::change_sets::ChangeSetBuilder;

        ChangeSetBuilder::new()
            .with_id(lib_core::RowID::mock())
            .with_table_name(Self::generate_mock_table_name())
            .with_row_id(lib_core::RowID::mock())
            .with_field_name(Self::generate_mock_field_name())
            .with_value_opt(Self::generate_mock_value())
            .with_hlc(lib_core::HybridLogicalClock::mock())
            .with_client_id(lib_core::RowID::mock())
            .with_version_opt(Some(0))
            .with_created_on_opt(Some(chrono::Utc::now()))
            .build()
            .expect("Mock Change Set should always build successfully")
    }

    #[cfg(test)]
    fn generate_mock_table_name() -> String {
        "categories".to_string()
    }

    #[cfg(test)]
    fn generate_mock_field_name() -> String {
        use fake::Fake;
        use fake::faker::lorem::en::Word;

        Word().fake()
    }

    #[cfg(test)]
    fn generate_mock_value() -> Option<String> {
        use fake::Fake;
        use fake::faker::boolean::en::Boolean;
        use fake::faker::lorem::en::Words;

        let is_some: bool = Boolean(80).fake();
        if is_some {
            let words: Vec<String> = Words(1..4).fake();
            Some(words.join(" "))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mock_generates_valid_change_set() {
        let change_set = ChangeSet::mock();
        assert!(!change_set.table_name.is_empty());
        assert!(!change_set.field_name.is_empty());
        assert!(change_set.created_on <= chrono::Utc::now());
    }

    #[test]
    fn change_set_struct_derives_work() {
        let cs1 = ChangeSet::mock();
        let cs2 = cs1.clone();
        assert_eq!(cs1, cs2);

        let debug_str = format!("{:?}", cs1);
        assert!(debug_str.contains("ChangeSet"));

        let json = serde_json::to_string(&cs1).unwrap();
        let deserialized: ChangeSet = serde_json::from_str(&json).unwrap();
        assert_eq!(cs1, deserialized);
    }
}
