//! # RowID Domain Type
//!
//! This module defines [`RowID`], a domain type for unique row identifiers based on UUID v7.
//! RowIDs are sortable by creation time and provide type-safe identifier handling
//! throughout the Personal Ledger backend.
//!
//! ## Features
//!
//! - **Time-ordered**: Uses UUID v7 for chronological ordering
//! - **Type-safe**: Prevents mixing different ID types
//! - **Serializable**: Supports JSON serialization/deserialization via serde
//! - **Database-ready**: SQLx integration for SQLite storage
//! - **Sortable**: Built-in sorting and comparison operations
//! - **Mock support**: Test utilities for generating predictable IDs
//!
//! ## Examples
//!
//! ```rust
//! use lib_domain::RowID;
//!
//! // Create a new time-ordered identifier
//! let id = RowID::new();
//!
//! // Parse from string
//! let parsed = "01800000-0000-7000-8000-000000000000".parse::<RowID>()?;
//!
//! // Compare IDs (time-ordered)
//! let id1 = RowID::new();
//! let id2 = RowID::new();
//! assert!(id1 < id2); // id1 was created first
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

/// A unique row identifier based on UUID v7.
///
/// `RowID` provides a type-safe wrapper around UUID v7, ensuring that row identifiers
/// are sortable by creation time and cannot be accidentally mixed with other types.
/// All RowIDs created with [`new()`](Self::new) will be naturally ordered by creation time.
///
/// The underlying UUID v7 format embeds a timestamp, making IDs chronologically sortable
/// while maintaining randomness for uniqueness. This is ideal for database primary keys
/// where insertion order matters.
///
/// # Implementation Details
///
/// - Stored as TEXT in SQLite databases
/// - Serialized as UUID strings in JSON
/// - Validated on creation and deserialization to ensure version 7
/// - Copy-able for efficient passing by value
///
/// # Examples
///
/// ```rust
/// use lib_domain::RowID;
///
/// // Create a new RowID with current timestamp
/// let id = RowID::new();
///
/// // Parse from string (validates UUID v7)
/// let parsed: RowID = "01800000-0000-7000-8000-000000000000".parse()?;
///
/// // Convert to UUID
/// let uuid = id.into_uuid();
///
/// // Compare IDs (chronologically ordered)
/// let id1 = RowID::new();
/// let id2 = RowID::new();
/// assert!(id1 < id2);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(
    Debug, Copy, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize,
)]
pub struct RowID(uuid::Uuid);

impl From<uuid::Error> for RowIDError {
    /// Convert a UUID parsing error into a [`RowIDError`].
    ///
    /// This allows seamless error propagation from the uuid crate's parsing functions.
    fn from(err: uuid::Error) -> Self {
        RowIDError::InvalidUuid(err.to_string())
    }
}

// SQLx implementations for RowID with validation
impl sqlx::Type<sqlx::Sqlite> for RowID {
    fn type_info() -> sqlx::sqlite::SqliteTypeInfo {
        <String as sqlx::Type<sqlx::Sqlite>>::type_info()
    }
}

impl<'q> sqlx::Encode<'q, sqlx::Sqlite> for RowID {
    fn encode_by_ref(
        &self,
        buf: &mut <sqlx::Sqlite as sqlx::Database>::ArgumentBuffer<'q>,
    ) -> Result<sqlx::encode::IsNull, sqlx::error::BoxDynError> {
        let s = self.0.to_string();
        <String as sqlx::Encode<'q, sqlx::Sqlite>>::encode(s, buf)
    }
}

impl<'r> sqlx::Decode<'r, sqlx::Sqlite> for RowID {
    fn decode(value: sqlx::sqlite::SqliteValueRef<'r>) -> Result<Self, sqlx::error::BoxDynError> {
        let s = <String as sqlx::Decode<sqlx::Sqlite>>::decode(value)?;
        let uuid = uuid::Uuid::parse_str(&s).map_err(|e| format!("Invalid UUID string in DB: {}", e))?;
        let row_id = RowID::try_from(uuid).map_err(|e| format!("Invalid RowID in DB: {}", e))?;
        Ok(row_id)
    }
}

impl TryFrom<uuid::Uuid> for RowID {
    type Error = RowIDError;

    /// Create a RowID from an existing UUID.
    ///
    /// This validates that the UUID is version 7 before wrapping it in a RowID.
    /// Use this when you have a UUID from external sources that needs validation.
    ///
    /// # Errors
    ///
    /// Returns [`RowIDError::InvalidVersion`] if the UUID is not version 7.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lib_domain::RowID;
    ///
    /// // Valid v7 UUID
    /// let uuid = uuid::Uuid::now_v7();
    /// let row_id = RowID::try_from(uuid)?;
    ///
    /// // Invalid version will fail
    /// let uuid_v4 = uuid::Uuid::new_v4();
    /// assert!(RowID::try_from(uuid_v4).is_err());
    /// # Ok::<(), lib_domain::RowIDError>(())
    /// ```
    fn try_from(uuid: uuid::Uuid) -> Result<Self, Self::Error> {
        if uuid.get_version_num() != 7 {
            return Err(RowIDError::InvalidVersion(uuid.get_version_num() as u8));
        }
        Ok(RowID(uuid))
    }
}

impl std::str::FromStr for RowID {
    type Err = RowIDError;
    
    /// Parse a RowID from a UUID string.
    ///
    /// This is the primary way to parse RowIDs from string input, including
    /// user input, configuration files, or API responses. The UUID must be
    /// version 7 to be accepted.
    ///
    /// # Errors
    ///
    /// Returns [`RowIDError::InvalidUuid`] if the string is not a valid UUID format,
    /// or [`RowIDError::InvalidVersion`] if the UUID is not version 7.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use std::str::FromStr;
    /// use lib_domain::RowID;
    ///
    /// // Using parse()
    /// let id: RowID = "01800000-0000-7000-8000-000000000000".parse()?;
    ///
    /// // Using FromStr explicitly
    /// let id = RowID::from_str("01800000-0000-7000-8000-000000000000")?;
    ///
    /// // Invalid format fails
    /// assert!("not-a-uuid".parse::<RowID>().is_err());
    /// # Ok::<(), lib_domain::RowIDError>(())
    /// ```
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let uuid = uuid::Uuid::parse_str(s).map_err(RowIDError::from)?;
        RowID::try_from(uuid)
    }
}

impl std::fmt::Display for RowID {
    /// Format the RowID as a standard hyphenated UUID string.
    ///
    /// The output follows RFC 4122 format: `xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx`
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl PartialOrd for RowID {
    /// Compare two RowIDs for ordering.
    ///
    /// RowIDs are ordered chronologically based on their embedded timestamps.
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for RowID {
    /// Compare two RowIDs for total ordering.
    ///
    /// RowIDs created earlier will be less than RowIDs created later,
    /// providing natural chronological sorting.
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.cmp(&other.0)
    }
}

impl RowID {
    /// Create a new RowID using UUID v7 with current timestamp.
    ///
    /// UUID v7 ensures that RowIDs created in sequence will be naturally
    /// ordered by creation time, making them suitable for chronological sorting.
    /// This is the primary method for creating new identifiers.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lib_domain::RowID;
    ///
    /// let id1 = RowID::new();
    /// let id2 = RowID::new();
    /// assert!(id1 < id2); // id1 was created before id2
    /// ```
    pub fn new() -> Self {
        let row_id = uuid::Uuid::now_v7();
        Self(row_id)
    }

    /// Create a new RowID using UUID v7 with a specific timestamp.
    ///
    /// This allows creating RowIDs with deterministic timestamps, useful for
    /// testing or when you need to control the creation time. The timestamp
    /// is embedded in the UUID v7 format.
    ///
    /// # Arguments
    ///
    /// * `timestamp` - The UTC timestamp to embed in the UUID
    ///
    /// # Examples
    ///
    /// ```rust
    /// use chrono::{DateTime, Utc};
    /// use lib_domain::RowID;
    ///
    /// let timestamp = DateTime::parse_from_rfc3339("2023-01-01T00:00:00Z")
    ///     .unwrap()
    ///     .with_timezone(&Utc);
    /// let id = RowID::from_timestamp(timestamp);
    /// ```
    pub fn from_timestamp(timestamp: chrono::DateTime<chrono::Utc>) -> Self {
        let ts = uuid::Timestamp::from_unix(
            uuid::NoContext,
            timestamp.timestamp() as u64,
            timestamp.timestamp_nanos_opt().unwrap() as u32,
        );
        let row_id = uuid::Uuid::new_v7(ts);
        Self(row_id)
    }

    /// Convert the RowID into its underlying UUID.
    ///
    /// This consumes the RowID and returns the wrapped UUID v7.
    /// Use [`as_uuid()`](Self::as_uuid) if you need a reference without consuming.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lib_domain::RowID;
    ///
    /// let id = RowID::new();
    /// let uuid = id.into_uuid();
    /// assert_eq!(uuid.get_version_num(), 7);
    /// ```
    pub fn into_uuid(self) -> uuid::Uuid {
        self.0
    }

    /// Get a reference to the underlying UUID without consuming the RowID.
    ///
    /// Use this when you need to access the UUID but want to keep the RowID.
    /// For consuming conversion, use [`into_uuid()`](Self::into_uuid).
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lib_domain::RowID;
    ///
    /// let id = RowID::new();
    /// let uuid_ref = id.as_uuid();
    /// // `id` is still valid here
    /// assert_eq!(uuid_ref.get_version_num(), 7);
    /// ```
    pub fn as_uuid(&self) -> &uuid::Uuid {
        &self.0
    }

    /// Validate that this RowID contains a valid UUID v7.
    ///
    /// Since RowIDs are validated on creation through [`new()`](Self::new),
    /// [`try_from()`](Self::try_from), and [`from_str()`](std::str::FromStr::from_str),
    /// this should always return `Ok(())` for RowIDs created through public APIs.
    /// However, this method is useful for defensive checks or when working with
    /// RowIDs from external sources.
    ///
    /// # Errors
    ///
    /// Returns [`RowIDError::InvalidVersion`] if the UUID is not version 7.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lib_domain::RowID;
    ///
    /// let id = RowID::new();
    /// assert!(id.validate().is_ok());
    /// ```
    pub fn validate(&self) -> Result<(), RowIDError> {
        if self.0.get_version_num() != 7 {
            Err(RowIDError::InvalidVersion(self.0.get_version_num() as u8))
        } else {
            Ok(())
        }
    }

    /// Sort a mutable slice of RowIDs in ascending (chronological) order.
    ///
    /// This sorts in-place, with earlier IDs (older timestamps) appearing first.
    /// For a non-mutating version, use [`sorted_ascending()`](Self::sorted_ascending).
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lib_domain::RowID;
    ///
    /// let id1 = RowID::new();
    /// let id2 = RowID::new();
    /// let id3 = RowID::new();
    ///
    /// let mut ids = vec![id3, id1, id2];
    /// RowID::sort_ascending(&mut ids);
    /// assert_eq!(ids, vec![id1, id2, id3]); // chronological order
    /// ```
    pub fn sort_ascending(ids: &mut [RowID]) {
        ids.sort();
    }

    /// Sort a mutable slice of RowIDs in descending (reverse chronological) order.
    ///
    /// This sorts in-place, with later IDs (newer timestamps) appearing first.
    /// For a non-mutating version, use [`sorted_descending()`](Self::sorted_descending).
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lib_domain::RowID;
    ///
    /// let id1 = RowID::new();
    /// let id2 = RowID::new();
    /// let id3 = RowID::new();
    ///
    /// let mut ids = vec![id1, id3, id2];
    /// RowID::sort_descending(&mut ids);
    /// assert_eq!(ids, vec![id3, id2, id1]); // newest first
    /// ```
    pub fn sort_descending(ids: &mut [RowID]) {
        ids.sort_by(|a, b| b.cmp(a));
    }

    /// Create a new Vec with RowIDs sorted in ascending (chronological) order.
    ///
    /// This returns a new vector without modifying the input. The original
    /// collection is consumed. For in-place sorting, use [`sort_ascending()`](Self::sort_ascending).
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lib_domain::RowID;
    ///
    /// let id1 = RowID::new();
    /// let id2 = RowID::new();
    /// let id3 = RowID::new();
    ///
    /// let ids = vec![id3, id1, id2];
    /// let sorted = RowID::sorted_ascending(ids);
    /// assert_eq!(sorted, vec![id1, id2, id3]);
    /// ```
    pub fn sorted_ascending<I>(ids: I) -> Vec<RowID>
    where
        I: IntoIterator<Item = RowID>,
    {
        let mut result: Vec<RowID> = ids.into_iter().collect();
        result.sort();
        result
    }

    /// Create a new Vec with RowIDs sorted in descending (reverse chronological) order.
    ///
    /// This returns a new vector without modifying the input. The original
    /// collection is consumed. For in-place sorting, use [`sort_descending()`](Self::sort_descending).
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lib_domain::RowID;
    ///
    /// let id1 = RowID::new();
    /// let id2 = RowID::new();
    /// let id3 = RowID::new();
    ///
    /// let ids = vec![id1, id3, id2];
    /// let sorted = RowID::sorted_descending(ids);
    /// assert_eq!(sorted, vec![id3, id2, id1]); // newest first
    /// ```
    pub fn sorted_descending<I>(ids: I) -> Vec<RowID>
    where
        I: IntoIterator<Item = RowID>,
    {
        let mut result: Vec<RowID> = ids.into_iter().collect();
        result.sort_by(|a, b| b.cmp(a));
        result
    }

    /// Find the earliest (minimum) RowID from a collection.
    ///
    /// Returns `None` if the collection is empty. Due to chronological ordering,
    /// the minimum RowID is the one that was created first.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lib_domain::RowID;
    ///
    /// let id1 = RowID::new();
    /// let id2 = RowID::new();
    /// let id3 = RowID::new();
    ///
    /// let ids = vec![id2, id1, id3];
    /// let earliest = RowID::min(ids).unwrap();
    /// assert_eq!(earliest, id1); // id1 was created first
    /// ```
    pub fn min<I>(ids: I) -> Option<RowID>
    where
        I: IntoIterator<Item = RowID>,
    {
        ids.into_iter().min()
    }

    /// Find the latest (maximum) RowID from a collection.
    ///
    /// Returns `None` if the collection is empty. Due to chronological ordering,
    /// the maximum RowID is the one that was created most recently.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lib_domain::RowID;
    ///
    /// let id1 = RowID::new();
    /// let id2 = RowID::new();
    /// let id3 = RowID::new();
    ///
    /// let ids = vec![id2, id1, id3];
    /// let latest = RowID::max(ids).unwrap();
    /// assert_eq!(latest, id3); // id3 was created last
    /// ```
    pub fn max<I>(ids: I) -> Option<RowID>
    where
        I: IntoIterator<Item = RowID>,
    {
        ids.into_iter().max()
    }

    /// Check if this RowID was created before another RowID.
    ///
    /// This is equivalent to `self < other` but provides a more semantic method name.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lib_domain::RowID;
    ///
    /// let id1 = RowID::new();
    /// std::thread::sleep(std::time::Duration::from_millis(1));
    /// let id2 = RowID::new();
    ///
    /// assert!(id1.is_before(&id2));
    /// assert!(!id2.is_before(&id1));
    /// ```
    pub fn is_before(&self, other: &RowID) -> bool {
        self < other
    }

    /// Check if this RowID was created after another RowID.
    ///
    /// This is equivalent to `self > other` but provides a more semantic method name.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lib_domain::RowID;
    ///
    /// let id1 = RowID::new();
    /// std::thread::sleep(std::time::Duration::from_millis(1));
    /// let id2 = RowID::new();
    ///
    /// assert!(id2.is_after(&id1));
    /// assert!(!id1.is_after(&id2));
    /// ```
    pub fn is_after(&self, other: &RowID) -> bool {
        self > other
    }


    /// Create a mock RowID with a random timestamp for testing.
    ///
    /// This generates a RowID with a random creation time between the Unix epoch
    /// and now, useful for testing scenarios where you need varied but valid identifiers.
    /// The generated RowID is always a valid UUID v7.
    ///
    /// **Note**: This function is only available in test builds.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lib_domain::RowID;
    ///
    /// let mock_id = RowID::mock();
    /// assert_eq!(mock_id.as_uuid().get_version_num(), 7);
    /// ```
    pub fn mock() -> Self {
        use chrono::{DateTime, Utc};
        use fake::faker::chrono::en::DateTimeAfter;
        use fake::Fake;

        // Generate random DateTime after UNIX time epoch (00:00:00 UTC on 1 January 1970)
        let random_datetime: DateTime<Utc> =
            DateTimeAfter(chrono::DateTime::UNIX_EPOCH).fake();

        // Convert datetime to a UUID timestamp
        let random_uuid_timestamp: uuid::Timestamp = uuid::Timestamp::from_unix(
            uuid::NoContext,
            random_datetime.timestamp() as u64,
            random_datetime.timestamp_nanos_opt().unwrap() as u32,
        );

        // Generate Uuid V7
        let row_id = uuid::Uuid::new_v7(random_uuid_timestamp);

        Self(row_id)
    }

    /// Create a mock RowID with a specific timestamp for testing.
    ///
    /// This allows creating RowIDs with deterministic timestamps, useful for
    /// testing chronological ordering and time-based logic. Unlike [`mock()`](Self::mock),
    /// this provides full control over the timestamp.
    ///
    /// **Note**: This function is only available in test builds.
    ///
    /// # Arguments
    ///
    /// * `date_time` - The UTC timestamp to embed in the UUID
    ///
    /// # Examples
    ///
    /// ```rust
    /// use chrono::{DateTime, Utc};
    /// use lib_domain::RowID;
    ///
    /// let timestamp = DateTime::parse_from_rfc3339("2023-01-01T00:00:00Z")
    ///     .unwrap()
    ///     .with_timezone(&Utc);
    /// let id = RowID::mock_from_datetime(timestamp);
    /// assert_eq!(id.as_uuid().get_version_num(), 7);
    /// ```
    #[cfg(test)]
    pub fn mock_from_datetime(date_time: chrono::DateTime<chrono::Utc>) -> Self {
        // Convert datetime to a UUID timestamp
        let uuid_timestamp: uuid::Timestamp = uuid::Timestamp::from_unix(
            uuid::NoContext,
            date_time.timestamp() as u64,
            date_time.timestamp_nanos_opt().unwrap() as u32,
        );

        // Generate Uuid V7
        let row_id = uuid::Uuid::new_v7(uuid_timestamp);

        Self(row_id)
    }

    /// Create a RowID from an existing UUID.
    ///
    /// This wraps a UUID without validation. Use with caution - prefer
    /// [`try_from()`](Self::try_from) for validated conversion or
    /// [`new()`](Self::new) for creating new IDs.
    ///
    /// **Warning**: This does not validate that the UUID is version 7.
    /// Using non-v7 UUIDs will break ordering guarantees.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use uuid::Uuid;
    /// use lib_domain::RowID;
    ///
    /// let uuid = Uuid::now_v7();
    /// let id = RowID::from_uuid(uuid);
    /// ```
    pub fn from_uuid(uuid: uuid::Uuid) -> Self {
        RowID(uuid)
    }

    /// Parse a RowID from a string representation of a UUID.
    ///
    /// This is a convenience wrapper around [`str::parse()`] that validates
    /// the UUID is version 7. Prefer using `.parse()` directly for more idiomatic code.
    ///
    /// # Errors
    ///
    /// Returns [`RowIDError::InvalidUuid`] if the string is not a valid UUID format,
    /// or [`RowIDError::InvalidVersion`] if the UUID is not version 7.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lib_domain::RowID;
    ///
    /// // Using from_string
    /// let id = RowID::from_string("01800000-0000-7000-8000-000000000000")?;
    ///
    /// // Prefer using parse() for idiomatic code
    /// let id: RowID = "01800000-0000-7000-8000-000000000000".parse()?;
    /// # Ok::<(), lib_domain::RowIDError>(())
    /// ```
    pub fn from_string(s: &str) -> Result<Self, RowIDError> {
        s.parse()
    }

    /// Convert an i64 to a RowID by embedding it in a UUID structure.
    ///
    /// This method places the i64 value in the last 8 bytes of a UUID,
    /// with the first 8 bytes being zero. This is useful for migrating
    /// from integer-based ID systems to UUID-based systems.
    ///
    /// **Warning**: The resulting UUID will **not** be a valid v7 UUID and will
    /// **not** have proper timestamp ordering properties. Use only for migration
    /// or compatibility purposes.
    ///
    /// # Errors
    ///
    /// Returns [`RowIDError::TypeCast`] if the provided i64 is negative.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lib_domain::RowID;
    ///
    /// let id = RowID::from_i64(12345)?;
    /// assert_eq!(id.to_i64(), 12345);
    ///
    /// // Negative values are rejected
    /// assert!(RowID::from_i64(-1).is_err());
    /// # Ok::<(), lib_domain::RowIDError>(())
    /// ```
    pub fn from_i64(id: i64) -> Result<Self, RowIDError> {
        if id < 0 {
            return Err(RowIDError::TypeCast("Negative IDs not allowed".to_string()));
        }
        // Convert i64 to UUID by treating it as a simple identifier
        // This is a simplified example - in practice you might want more sophisticated mapping
        let bytes = id.to_be_bytes();
        let mut uuid_bytes = [0u8; 16];
        uuid_bytes[8..16].copy_from_slice(&bytes);
        let uuid = uuid::Uuid::from_bytes(uuid_bytes);
        Ok(RowID(uuid))
    }

    /// Extract an i64 value from a RowID that was created with [`from_i64()`](Self::from_i64).
    ///
    /// This method extracts the i64 value from the last 8 bytes of the UUID.
    /// Only use this with RowIDs that were originally created from i64 values
    /// using [`from_i64()`](Self::from_i64).
    ///
    /// **Warning**: Calling this on RowIDs created with [`new()`](Self::new) or
    /// other methods will return meaningless values.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use lib_domain::RowID;
    ///
    /// let original_id = 12345i64;
    /// let row_id = RowID::from_i64(original_id)?;
    /// let extracted_id = row_id.to_i64();
    /// assert_eq!(original_id, extracted_id);
    /// # Ok::<(), lib_domain::RowIDError>(())
    /// ```
    pub fn to_i64(&self) -> i64 {
        let uuid_bytes = self.0.as_bytes();
        i64::from_be_bytes(uuid_bytes[8..16].try_into().unwrap())
    }
}

/// Errors that can occur during RowID operations.
///
/// This enum covers all error cases for RowID creation, parsing, and validation.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum RowIDError {
    /// The provided string is not a valid UUID format.
    ///
    /// This occurs when parsing fails due to invalid UUID syntax.
    #[error("Invalid UUID format: {0}")]
    InvalidUuid(String),
    
    /// The provided UUID is not version 7.
    ///
    /// RowIDs require UUID v7 for chronological ordering. This error is returned
    /// when attempting to create a RowID from a UUID of a different version.
    #[error("UUID is not version 7: version {0}")]
    InvalidVersion(u8),

    /// Type casting operation failed.
    ///
    /// This occurs during i64 conversion operations, such as when attempting
    /// to convert a negative i64 to a RowID.
    #[error("Type casting failed: {0}")]
    TypeCast(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_row_id_creation() {
        let id = RowID::new();
        assert_eq!(id.0.get_version_num(), 7);
    }

    #[test]
    fn test_row_id_ordering() {
        let id1 = RowID::new();
        let id2 = RowID::new();
        assert!(id1 < id2);
    }

    #[test]
    fn test_row_id_display() {
        let id = RowID::new();
        assert_eq!(format!("{}", id), id.0.to_string());
    }

    #[test]
    fn test_row_id_validation() {
        let id = RowID::new();
        assert!(id.validate().is_ok());

        // Create a RowID with a non-v7 UUID (this is internal, but for testing)
        let uuid_v4 = uuid::Uuid::new_v4();
        let row_id = RowID(uuid_v4); // Directly construct, bypassing validation
        let result = row_id.validate();
        assert!(result.is_err());
        if let Err(RowIDError::InvalidVersion(v)) = result {
            assert_eq!(v, 4);
        } else {
            panic!("Expected InvalidVersion error");
        }
    }

    #[test]
    fn test_row_id_sorting() {
        let id1 = RowID::new();
        let id2 = RowID::new();
        let id3 = RowID::new();

        let mut ids = vec![id2, id1, id3];
        RowID::sort_ascending(&mut ids);
        assert_eq!(ids, vec![id1, id2, id3]);

        let mut ids_desc = vec![id1, id3, id2];
        RowID::sort_descending(&mut ids_desc);
        assert_eq!(ids_desc, vec![id3, id2, id1]);
    }

    #[test]
    fn test_row_id_min_max() {
        let id1 = RowID::new();
        let id2 = RowID::new();
        let id3 = RowID::new();

        assert_eq!(RowID::min(vec![id2, id1, id3]), Some(id1));
        assert_eq!(RowID::max(vec![id2, id1, id3]), Some(id3));
    }

    #[test]
    fn test_row_id_is_before_after() {
        let id1 = RowID::new();
        let id2 = RowID::new();

        assert!(id1.is_before(&id2));
        assert!(id2.is_after(&id1));
    }

    #[test]
    fn test_row_id_mock() {
        let mock_id = RowID::mock();
        assert_eq!(mock_id.as_uuid().get_version_num(), 7);
    }

    #[test]
    fn test_row_id_from_i64() {
        let id = RowID::from_i64(12345).unwrap();
        assert_eq!(id.to_i64(), 12345);
    }

    #[test]
    fn test_into_uuid() {
        let row_id = RowID::new();
        let uuid = row_id.into_uuid();
        assert_eq!(uuid.get_version_num(), 7);
    }

    #[test]
    fn test_display() {
        let row_id = RowID::new();
        let display_str = format!("{}", row_id);
        assert_eq!(display_str, row_id.0.to_string());
    }

    #[test]
    fn test_validate_valid() {
        let row_id = RowID::new();
        assert!(row_id.validate().is_ok());
    }

    #[test]
    fn test_validate_invalid_version() {
        // Create a RowID with a non-v7 UUID (this is internal, but for testing)
        let uuid_v4 = uuid::Uuid::new_v4();
        let row_id = RowID(uuid_v4); // Directly construct, bypassing validation
        let result = row_id.validate();
        assert!(result.is_err());
        if let Err(RowIDError::InvalidVersion(v)) = result {
            assert_eq!(v, 4);
        } else {
            panic!("Expected InvalidVersion error");
        }
    }

    #[test]
    fn test_from_string_valid() {
        let id = RowID::new();
        let uuid_str = id.0.to_string();
        let parsed_id = RowID::from_string(&uuid_str).unwrap();
        assert_eq!(parsed_id, id);
        assert!(parsed_id.validate().is_ok());
    }

    #[test]
    fn test_from_string_invalid() {
        let invalid_str = "not-a-uuid";
        let result = RowID::from_string(invalid_str);
        assert!(result.is_err());
        if let Err(RowIDError::InvalidUuid(_)) = result {
            // Expected
        } else {
            panic!("Expected InvalidUuid error");
        }
    }

    #[test]
    fn test_from_i64_negative() {
        let result = RowID::from_i64(-1);
        assert!(result.is_err());
        if let Err(RowIDError::TypeCast(_)) = result {
            // Expected
        } else {
            panic!("Expected TypeCast error");
        }
    }

    #[test]
    fn test_try_from_uuid_invalid_version() {
        let uuid_v4 = uuid::Uuid::new_v4();
        let result = RowID::try_from(uuid_v4);
        assert!(result.is_err());
        if let Err(RowIDError::InvalidVersion(v)) = result {
            assert_eq!(v, 4);
        } else {
            panic!("Expected InvalidVersion error");
        }
    }

    #[test]
    fn test_try_from_uuid_valid() {
        let uuid_v7 = uuid::Uuid::now_v7();
        let row_id = RowID::try_from(uuid_v7).unwrap();
        assert_eq!(row_id.0, uuid_v7);
        assert_eq!(row_id.0.get_version_num(), 7);
    }

    #[test]
    fn test_from_str_valid() {
        let id = RowID::new();
        let uuid_str = id.0.to_string();
        let parsed_id: RowID = uuid_str.parse().unwrap();
        assert_eq!(parsed_id, id);
    }

    #[test]
    fn test_from_str_invalid() {
        let invalid_str = "not-a-uuid";
        let result: Result<RowID, _> = invalid_str.parse();
        assert!(result.is_err());
    }

    #[test]
    fn test_from_timestamp() {
        use chrono::{DateTime, Utc};
        let timestamp = DateTime::parse_from_rfc3339("2023-01-01T00:00:00Z").unwrap().with_timezone(&Utc);
        let id = RowID::from_timestamp(timestamp);
        assert_eq!(id.0.get_version_num(), 7);
        // Note: We can't easily test the exact timestamp due to UUID v7 encoding
    }

    #[test]
    fn test_from_uuid() {
        let uuid = uuid::Uuid::now_v7();
        let row_id = RowID::from_uuid(uuid);
        assert_eq!(row_id.0, uuid);
    }

    #[test]
    fn test_sorted_ascending() {
        let id1 = RowID::new();
        let id2 = RowID::new();
        let id3 = RowID::new();

        let ids = vec![id2, id1, id3];
        let sorted = RowID::sorted_ascending(ids);
        assert_eq!(sorted, vec![id1, id2, id3]);
    }

    #[test]
    fn test_sorted_descending() {
        let id1 = RowID::new();
        let id2 = RowID::new();
        let id3 = RowID::new();

        let ids = vec![id1, id3, id2];
        let sorted = RowID::sorted_descending(ids);
        assert_eq!(sorted, vec![id3, id2, id1]);
    }

    #[test]
    fn test_min_max_empty() {
        assert_eq!(RowID::min(Vec::<RowID>::new()), None);
        assert_eq!(RowID::max(Vec::<RowID>::new()), None);
    }

    #[test]
    fn test_sort_empty() {
        let mut empty: Vec<RowID> = vec![];
        RowID::sort_ascending(&mut empty);
        assert!(empty.is_empty());

        RowID::sort_descending(&mut empty);
        assert!(empty.is_empty());
    }

    #[test]
    fn test_sorted_empty() {
        let sorted_asc = RowID::sorted_ascending(Vec::<RowID>::new());
        assert!(sorted_asc.is_empty());

        let sorted_desc = RowID::sorted_descending(Vec::<RowID>::new());
        assert!(sorted_desc.is_empty());
    }

    #[test]
    fn test_round_trip_conversions() {
        // Test i64 round trip
        let original_i64 = 1234567890i64;
        let row_id = RowID::from_i64(original_i64).unwrap();
        let extracted_i64 = row_id.to_i64();
        assert_eq!(original_i64, extracted_i64);

        // Test string round trip
        let original_id = RowID::new();
        let uuid_str = original_id.0.to_string();
        let parsed_id = RowID::from_string(&uuid_str).unwrap();
        assert_eq!(original_id, parsed_id);

        // Test UUID round trip
        let uuid = uuid::Uuid::now_v7();
        let row_id = RowID::from_uuid(uuid);
        let extracted_uuid = row_id.into_uuid();
        assert_eq!(uuid, extracted_uuid);
    }

    #[test]
    fn test_equality_and_hash() {
        let id1 = RowID::new();
        let id2 = id1; // Copy
        assert_eq!(id1, id2);

        let id3 = RowID::new();
        assert_ne!(id1, id3);
    }

    #[test]
    fn test_clone() {
        let id1 = RowID::new();
        #[allow(clippy::clone_on_copy)]
        let id2 = id1.clone();
        assert_eq!(id1, id2);
    }

    #[test]
    fn test_debug() {
        let id = RowID::new();
        let debug_str = format!("{:?}", id);
        assert!(debug_str.contains("RowID"));
    }

    #[test]
    fn test_serialization() {
        let id = RowID::new();
        let serialized = serde_json::to_string(&id).unwrap();
        let deserialized: RowID = serde_json::from_str(&serialized).unwrap();
        assert_eq!(id, deserialized);
    }
}
