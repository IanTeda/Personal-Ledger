//! # Hybrid Logical Clock domain type
//!
//! This module defines [`HybridLogicalClock`], the timestamp type
//! [ADR-0009](../../../../docs/adr/0009-lww-sqlite-change-set-log.md) requires for Change
//! Set conflict resolution: physical time plus a monotonic logical counter, rather than a
//! bare wall-clock value that can collide or run backwards across independent Clients.
//!
//! [`HlcClock`] is the generator a Client or the Sync Server holds to mint
//! [`HybridLogicalClock`] values for its own events (`tick`) and to merge in a
//! timestamp observed on an incoming Change Set (`observe`), following the standard
//! Hybrid Logical Clock algorithm (Kulkarni et al., "Logical Physical Clocks").
//!
//! ## Examples
//!
//! ```rust
//! use lib_core::HlcClock;
//!
//! let mut clock = HlcClock::new();
//! let first = clock.tick();
//! let second = clock.tick();
//! assert!(first < second);
//! ```

/// A Hybrid Logical Clock timestamp: physical time plus a logical tie-breaking counter.
///
/// Two [`HybridLogicalClock`] values compare by physical time first, and by the logical
/// counter when the physical component is equal -- the ordering ADR-0009's per-field
/// last-write-wins rule needs. Client-ID tie-breaking on an exact match (same physical
/// *and* logical) is the caller's responsibility (see the Change Set's own `client_id`
/// field), since a bare `HybridLogicalClock` doesn't know which Client produced it.
///
/// # Examples
///
/// ```rust
/// use lib_core::HybridLogicalClock;
///
/// let a: HybridLogicalClock = "2026-01-01T00:00:00Z:0".parse()?;
/// let b: HybridLogicalClock = "2026-01-01T00:00:00Z:1".parse()?;
/// assert!(a < b);
/// # Ok::<(), lib_core::HybridLogicalClockError>(())
/// ```
#[derive(Debug, Copy, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct HybridLogicalClock {
    physical: chrono::DateTime<chrono::Utc>,
    logical: u32,
}

impl HybridLogicalClock {
    /// The physical-time component.
    pub fn physical(&self) -> chrono::DateTime<chrono::Utc> {
        self.physical
    }

    /// The logical tie-breaking counter.
    pub fn logical(&self) -> u32 {
        self.logical
    }

    /// Create a mock `HybridLogicalClock` for testing.
    ///
    /// Not `#[cfg(test)]`-gated so downstream crates (e.g. `lib_database`'s own
    /// `ChangeSet::mock()`) can use it in their own test builds -- same convention as
    /// [`RowID::mock()`](../row_id/struct.RowID.html#method.mock).
    pub fn mock() -> Self {
        Self {
            physical: chrono::Utc::now(),
            logical: 0,
        }
    }
}

impl PartialOrd for HybridLogicalClock {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for HybridLogicalClock {
    /// Compare by physical time first, then by the logical counter.
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.physical
            .cmp(&other.physical)
            .then(self.logical.cmp(&other.logical))
    }
}

impl std::fmt::Display for HybridLogicalClock {
    /// Format as `<RFC 3339 timestamp>:<logical counter>`, the canonical wire/storage form.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.physical.to_rfc3339(), self.logical)
    }
}

impl std::str::FromStr for HybridLogicalClock {
    type Err = HybridLogicalClockError;

    /// Parse a `HybridLogicalClock` from its `<RFC 3339 timestamp>:<logical counter>` form.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (physical_str, logical_str) = s
            .rsplit_once(':')
            .ok_or_else(|| HybridLogicalClockError::InvalidFormat(s.to_string()))?;

        let physical = chrono::DateTime::parse_from_rfc3339(physical_str)
            .map_err(|e| HybridLogicalClockError::InvalidTimestamp(e.to_string()))?
            .with_timezone(&chrono::Utc);

        let logical = logical_str
            .parse::<u32>()
            .map_err(|e| HybridLogicalClockError::InvalidLogicalCounter(e.to_string()))?;

        Ok(Self { physical, logical })
    }
}

// SQLx implementations for HybridLogicalClock, stored as TEXT in SQLite via its Display form.
impl sqlx::Type<sqlx::Sqlite> for HybridLogicalClock {
    fn type_info() -> sqlx::sqlite::SqliteTypeInfo {
        <String as sqlx::Type<sqlx::Sqlite>>::type_info()
    }
}

impl<'q> sqlx::Encode<'q, sqlx::Sqlite> for HybridLogicalClock {
    fn encode_by_ref(
        &self,
        buf: &mut <sqlx::Sqlite as sqlx::Database>::ArgumentBuffer<'q>,
    ) -> Result<sqlx::encode::IsNull, sqlx::error::BoxDynError> {
        <String as sqlx::Encode<'q, sqlx::Sqlite>>::encode(self.to_string(), buf)
    }
}

impl<'r> sqlx::Decode<'r, sqlx::Sqlite> for HybridLogicalClock {
    fn decode(value: sqlx::sqlite::SqliteValueRef<'r>) -> Result<Self, sqlx::error::BoxDynError> {
        let s = <String as sqlx::Decode<sqlx::Sqlite>>::decode(value)?;
        Ok(s.parse()?)
    }
}

/// Errors that can occur parsing a [`HybridLogicalClock`] from its string form.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum HybridLogicalClockError {
    /// The string didn't contain the `<timestamp>:<logical>` separator.
    #[error("Invalid HybridLogicalClock format: {0}")]
    InvalidFormat(String),

    /// The timestamp portion wasn't a valid RFC 3339 timestamp.
    #[error("Invalid HybridLogicalClock timestamp: {0}")]
    InvalidTimestamp(String),

    /// The logical-counter portion wasn't a valid `u32`.
    #[error("Invalid HybridLogicalClock logical counter: {0}")]
    InvalidLogicalCounter(String),
}

/// A generator of monotonically increasing [`HybridLogicalClock`] values.
///
/// One `HlcClock` belongs to one Client (or the Sync Server), tracking the last
/// [`HybridLogicalClock`] it has produced or observed so that every subsequent value it
/// mints is strictly greater -- the property ADR-0009's last-write-wins rule relies on.
#[derive(Debug, Default)]
pub struct HlcClock {
    last: Option<HybridLogicalClock>,
}

impl HlcClock {
    /// Create a new `HlcClock` with no prior history.
    pub fn new() -> Self {
        Self::default()
    }

    /// Mint a [`HybridLogicalClock`] for a local event.
    ///
    /// If wall-clock time has moved on since the last tick/observe, the new value uses
    /// that physical time with logical reset to `0`. If wall-clock time hasn't advanced
    /// (or has gone backwards -- clock skew), the physical component is held at the last
    /// known value and the logical counter is incremented, guaranteeing strict ordering.
    pub fn tick(&mut self) -> HybridLogicalClock {
        let now = chrono::Utc::now();
        let next = match self.last {
            Some(last) if now > last.physical => HybridLogicalClock {
                physical: now,
                logical: 0,
            },
            Some(last) => HybridLogicalClock {
                physical: last.physical,
                logical: last.logical + 1,
            },
            None => HybridLogicalClock {
                physical: now,
                logical: 0,
            },
        };
        self.last = Some(next);
        next
    }

    /// Merge in a [`HybridLogicalClock`] observed on an incoming Change Set, and mint the
    /// local event's own timestamp from the merge -- the receive-side half of the HLC
    /// algorithm, ensuring a Client's own clock never falls behind one it has seen.
    pub fn observe(&mut self, remote: HybridLogicalClock) -> HybridLogicalClock {
        let now = chrono::Utc::now();
        let last_physical = self.last.map(|l| l.physical);
        let max_physical = [Some(now), last_physical, Some(remote.physical)]
            .into_iter()
            .flatten()
            .max()
            .expect("at least one physical timestamp is always present");

        let logical = match (
            last_physical == Some(max_physical),
            remote.physical == max_physical,
        ) {
            (true, true) => {
                self.last
                    .map(|l| l.logical)
                    .unwrap_or(0)
                    .max(remote.logical)
                    + 1
            }
            (true, false) => self.last.map(|l| l.logical).unwrap_or(0) + 1,
            (false, true) => remote.logical + 1,
            (false, false) => 0,
        };

        let next = HybridLogicalClock {
            physical: max_physical,
            logical,
        };
        self.last = Some(next);
        next
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tick_produces_strictly_increasing_values() {
        let mut clock = HlcClock::new();
        let first = clock.tick();
        let second = clock.tick();
        assert!(first < second);
    }

    #[test]
    fn tick_bumps_logical_when_physical_time_has_not_advanced() {
        // Two ticks in immediate succession may land in the same instant; the logical
        // counter must still guarantee strict ordering.
        let mut clock = HlcClock::new();
        let first = clock.tick();
        let second = clock.tick();
        if first.physical() == second.physical() {
            assert_eq!(second.logical(), first.logical() + 1);
        }
    }

    #[test]
    fn observe_advances_past_a_remote_timestamp_ahead_of_local_clock() {
        let mut clock = HlcClock::new();
        let remote = HybridLogicalClock {
            physical: chrono::Utc::now() + chrono::Duration::days(1),
            logical: 5,
        };
        let merged = clock.observe(remote);
        assert!(merged > remote);
    }

    #[test]
    fn observe_advances_past_a_remote_timestamp_behind_local_clock() {
        let mut clock = HlcClock::new();
        let local = clock.tick();
        let remote = HybridLogicalClock {
            physical: chrono::Utc::now() - chrono::Duration::days(1),
            logical: 99,
        };
        let merged = clock.observe(remote);
        assert!(merged > local);
        assert!(merged > remote);
    }

    #[test]
    fn display_and_from_str_round_trip() {
        let mut clock = HlcClock::new();
        let value = clock.tick();
        let parsed: HybridLogicalClock = value.to_string().parse().unwrap();
        assert_eq!(value, parsed);
    }

    #[test]
    fn from_str_rejects_missing_separator() {
        let result: Result<HybridLogicalClock, _> = "not-a-clock".parse();
        assert!(matches!(
            result,
            Err(HybridLogicalClockError::InvalidFormat(_))
        ));
    }

    #[test]
    fn from_str_rejects_invalid_timestamp() {
        let result: Result<HybridLogicalClock, _> = "not-a-timestamp:0".parse();
        assert!(matches!(
            result,
            Err(HybridLogicalClockError::InvalidTimestamp(_))
        ));
    }

    #[test]
    fn from_str_rejects_invalid_logical_counter() {
        let result: Result<HybridLogicalClock, _> = "2026-01-01T00:00:00Z:not-a-number".parse();
        assert!(matches!(
            result,
            Err(HybridLogicalClockError::InvalidLogicalCounter(_))
        ));
    }

    #[test]
    fn ordering_compares_physical_then_logical() {
        let base = chrono::Utc::now();
        let earlier = HybridLogicalClock {
            physical: base,
            logical: 5,
        };
        let later_logical = HybridLogicalClock {
            physical: base,
            logical: 6,
        };
        let later_physical = HybridLogicalClock {
            physical: base + chrono::Duration::seconds(1),
            logical: 0,
        };
        assert!(earlier < later_logical);
        assert!(later_logical < later_physical);
    }
}
