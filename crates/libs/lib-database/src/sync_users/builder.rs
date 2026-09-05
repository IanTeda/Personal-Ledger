//! # SyncUser Builder
//!
//! Provides a fluent API for constructing [`SyncUser`] records, mirroring
//! [`crate::categories::CategoriesBuilder`]'s shape.

use super::SyncUser;
use crate::DatabaseError;

/// Fluent builder for [`SyncUser`] rows.
#[derive(Debug, Default, Clone)]
pub struct SyncUserBuilder {
    id: Option<lib_core::RowID>,
    username: Option<String>,
    password_hash: Option<String>,
    refresh_token_hash: Option<Option<String>>,
    created_on: Option<chrono::DateTime<chrono::Utc>>,
    updated_on: Option<chrono::DateTime<chrono::Utc>>,
}

impl SyncUserBuilder {
    /// Start building a new sync user with no preset values.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Use an existing [`RowID`](lib_core::RowID) for the sync user.
    #[must_use]
    pub fn with_id(mut self, id: lib_core::RowID) -> Self {
        self.id = Some(id);
        self
    }

    /// Set the sync user's username.
    #[must_use]
    pub fn with_username(mut self, username: impl Into<String>) -> Self {
        self.username = Some(username.into());
        self
    }

    /// Set the Argon2 password hash (PHC string format).
    #[must_use]
    pub fn with_password_hash(mut self, password_hash: impl Into<String>) -> Self {
        self.password_hash = Some(password_hash.into());
        self
    }

    /// Set the current refresh-token hash (`None` clears it -- no active session).
    #[must_use]
    pub fn with_refresh_token_hash_opt(mut self, refresh_token_hash: Option<String>) -> Self {
        self.refresh_token_hash = Some(refresh_token_hash);
        self
    }

    /// Provide an optional creation timestamp, defaulting to now when unset.
    #[must_use]
    pub fn with_created_on_opt(
        mut self,
        created_on: Option<chrono::DateTime<chrono::Utc>>,
    ) -> Self {
        self.created_on = created_on;
        self
    }

    /// Provide an optional update timestamp, defaulting to now when unset.
    #[must_use]
    pub fn with_updated_on_opt(
        mut self,
        updated_on: Option<chrono::DateTime<chrono::Utc>>,
    ) -> Self {
        self.updated_on = updated_on;
        self
    }

    /// Build the [`SyncUser`], returning an error when required fields are missing.
    pub fn build(self) -> crate::DatabaseResult<SyncUser> {
        let username = self.username.ok_or(DatabaseError::SyncUserBuilder(
            "username is required but was not set".to_string(),
        ))?;
        let password_hash = self.password_hash.ok_or(DatabaseError::SyncUserBuilder(
            "password_hash is required but was not set".to_string(),
        ))?;

        Ok(SyncUser {
            // clippy's unwrap_or_default suggestion is WRONG here: RowID::default()
            // is a nil (version 0) UUID, not a usable row id -- RowID's Decode requires
            // version 7. RowID::new() must run whenever no id was explicitly provided.
            #[allow(clippy::unwrap_or_default)]
            id: self.id.unwrap_or_else(lib_core::RowID::new),
            username,
            password_hash,
            refresh_token_hash: self.refresh_token_hash.unwrap_or(None),
            created_on: self.created_on.unwrap_or_else(chrono::Utc::now),
            updated_on: self.updated_on.unwrap_or_else(chrono::Utc::now),
        })
    }
}
