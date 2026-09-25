//! Shared helpers for the in-memory mock fixtures (`account`, `category`, `payee`, `tag`).
//!
//! The fixtures are UI-first seed data rather than `#[cfg(test)]` code, so the workspace's
//! `expect_used` lint applies to them. The helpers here hold the few places where a hand-written
//! seed literal is trusted, each behind one targeted `#[expect]`, so the fixtures themselves
//! carry no `expect` at all.

use chrono::{Datelike, Duration, Months, NaiveDate};
use lib_core::RowID;

/// A seed-literal calendar date.
#[expect(
    clippy::expect_used,
    reason = "only called with hand-written fixture literals, and every fixture's own tests build it, so an invalid literal fails the test suite"
)]
pub const fn date(year: i32, month: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(year, month, day).expect("fixture seed literal is a valid date")
}

/// The first day of `as_of`'s month, shifted by `months` (negative is earlier).
#[expect(
    clippy::expect_used,
    reason = "fixtures shift by a few dozen months at most from 2020s dates, far inside chrono's representable range"
)]
pub fn month_start(as_of: NaiveDate, months: i32) -> NaiveDate {
    let first = as_of - Duration::days(i64::from(as_of.day0()));
    let shifted = if months >= 0 {
        first.checked_add_months(Months::new(months.unsigned_abs()))
    } else {
        first.checked_sub_months(Months::new(months.unsigned_abs()))
    };
    shifted.expect("fixture month offsets stay inside chrono's range")
}

/// A deterministic seed derived from a `RowID`, so the same row always generates the same fake
/// data. Takes the UUID's low 64 bits (its bytes `8..16`, big-endian).
pub fn seed_from_id(id: RowID) -> u64 {
    id.into_uuid().as_u128() as u64
}

/// Deterministic `RowID`s, one second apart from `start_secs` (Unix time).
///
/// `RowID::from_timestamp` delegates to `uuid::Uuid::new_v7`, which fills the non-timestamp
/// bits from the OS RNG, so two runs would seed different ids (and different `seed_from_id`
/// data) — issues #116/#124. A plain incrementing counter stands in for the random bits
/// instead, which still sorts by creation order like a real v7 id.
pub fn id_sequence(start_secs: u64) -> impl FnMut() -> RowID {
    let mut millis = start_secs * 1_000;
    let mut counter: u64 = 0;
    move || {
        counter += 1;
        let mut counter_bytes = [0u8; 10];
        counter_bytes[2..10].copy_from_slice(&counter.to_be_bytes());
        let uuid = uuid::Builder::from_unix_timestamp_millis(millis, &counter_bytes).into_uuid();
        millis += 1_000;
        RowID::from_uuid(uuid)
    }
}

/// `2021-01-01T00:00:00Z`, the account, payee and tag fixtures' first id timestamp.
pub const EPOCH_2021: u64 = 1_609_459_200;

/// `2024-01-01T00:00:00Z`, the category fixture's first id timestamp.
pub const EPOCH_2024: u64 = 1_704_067_200;

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{DateTime, Utc};

    #[test]
    fn epochs_match_their_rfc3339_literals() {
        let parse = |s| DateTime::parse_from_rfc3339(s).unwrap().with_timezone(&Utc);
        assert_eq!(parse("2021-01-01T00:00:00Z").timestamp() as u64, EPOCH_2021);
        assert_eq!(parse("2024-01-01T00:00:00Z").timestamp() as u64, EPOCH_2024);
    }

    #[test]
    fn seed_from_id_reads_the_low_eight_bytes() {
        let id = id_sequence(EPOCH_2021)();
        let bytes = id.into_uuid();
        let expected = u64::from_be_bytes(bytes.as_bytes()[8..16].try_into().unwrap());
        assert_eq!(seed_from_id(id), expected);
    }

    #[test]
    fn month_start_shifts_both_ways() {
        let as_of = date(2026, 3, 31);
        assert_eq!(month_start(as_of, 0), date(2026, 3, 1));
        assert_eq!(month_start(as_of, 1), date(2026, 4, 1));
        assert_eq!(month_start(as_of, -3), date(2025, 12, 1));
    }

    #[test]
    fn id_sequence_is_deterministic_and_ordered() {
        let first: Vec<_> = std::iter::repeat_with(id_sequence(EPOCH_2021))
            .take(3)
            .collect();
        let second: Vec<_> = std::iter::repeat_with(id_sequence(EPOCH_2021))
            .take(3)
            .collect();
        assert_eq!(first, second);
        assert!(first.windows(2).all(|pair| pair[0] < pair[1]));
    }
}
