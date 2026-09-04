//! # Change Set Builder
//!
//! Provides a fluent API for constructing [`ChangeSet`] records, mirroring
//! [`crate::categories::CategoriesBuilder`]'s shape. Useful for tests, fixtures, and
//! Client-side code assembling Change Sets from a local edit before pushing them.

use super::ChangeSet;
use crate::DatabaseError;

/// Fluent builder for [`ChangeSet`] rows.
#[derive(Debug, Default, Clone)]
pub struct ChangeSetBuilder {
    id: Option<lib_core::RowID>,
    table_name: Option<String>,
    row_id: Option<lib_core::RowID>,
    field_name: Option<String>,
    value: Option<String>,
    hlc: Option<lib_core::HybridLogicalClock>,
    client_id: Option<lib_core::RowID>,
    version: Option<i64>,
    created_on: Option<chrono::DateTime<chrono::Utc>>,
}

impl ChangeSetBuilder {
    /// Start building a new Change Set with no preset values.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Use an existing [`RowID`](lib_core::RowID) for the Change Set.
    #[must_use]
    pub fn with_id(mut self, id: lib_core::RowID) -> Self {
        self.id = Some(id);
        self
    }

    /// Set the target table name.
    #[must_use]
    pub fn with_table_name(mut self, table_name: impl Into<String>) -> Self {
        self.table_name = Some(table_name.into());
        self
    }

    /// Set the target row's identifier.
    #[must_use]
    pub fn with_row_id(mut self, row_id: lib_core::RowID) -> Self {
        self.row_id = Some(row_id);
        self
    }

    /// Set the target field name.
    #[must_use]
    pub fn with_field_name(mut self, field_name: impl Into<String>) -> Self {
        self.field_name = Some(field_name.into());
        self
    }

    /// Set the field's new value (`None` represents SQL `NULL`).
    #[must_use]
    pub fn with_value_opt(mut self, value: Option<String>) -> Self {
        self.value = value;
        self
    }

    /// Set the Hybrid Logical Clock timestamp.
    #[must_use]
    pub fn with_hlc(mut self, hlc: lib_core::HybridLogicalClock) -> Self {
        self.hlc = Some(hlc);
        self
    }

    /// Set the originating Client's stable identifier.
    #[must_use]
    pub fn with_client_id(mut self, client_id: lib_core::RowID) -> Self {
        self.client_id = Some(client_id);
        self
    }

    /// Provide an optional version number, defaulting to `0` when unset.
    #[must_use]
    pub fn with_version_opt(mut self, version: Option<i64>) -> Self {
        self.version = version;
        self
    }

    /// Provide an optional log-insertion timestamp, defaulting to now when unset.
    #[must_use]
    pub fn with_created_on_opt(
        mut self,
        created_on: Option<chrono::DateTime<chrono::Utc>>,
    ) -> Self {
        self.created_on = created_on;
        self
    }

    /// Build the [`ChangeSet`], returning an error when required fields are missing.
    pub fn build(self) -> crate::DatabaseResult<ChangeSet> {
        let table_name = self.table_name.ok_or(DatabaseError::ChangeSetBuilder(
            "table_name is required but was not set".to_string(),
        ))?;
        let row_id = self.row_id.ok_or(DatabaseError::ChangeSetBuilder(
            "row_id is required but was not set".to_string(),
        ))?;
        let field_name = self.field_name.ok_or(DatabaseError::ChangeSetBuilder(
            "field_name is required but was not set".to_string(),
        ))?;
        let hlc = self.hlc.ok_or(DatabaseError::ChangeSetBuilder(
            "hlc is required but was not set".to_string(),
        ))?;
        let client_id = self.client_id.ok_or(DatabaseError::ChangeSetBuilder(
            "client_id is required but was not set".to_string(),
        ))?;

        Ok(ChangeSet {
            id: self.id.unwrap_or_default(),
            table_name,
            row_id,
            field_name,
            value: self.value,
            hlc,
            client_id,
            version: self.version.unwrap_or(0),
            created_on: self.created_on.unwrap_or_else(chrono::Utc::now),
        })
    }
}
