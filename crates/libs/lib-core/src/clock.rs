//! # Clock
//!
//! [`Clock`] is where a Client reads the calendar day, the moment now, and the local time of a
//! moment from. A Client builds it on the local date, the real time and the device's time zone
//! (each bin's `wall_clock`, since reading them is I/O), a test on a day and a zone it sets, so
//! every View and module dates its rows from one source, no module reads the machine's zone, and
//! only the Shell notices a new month.

use std::cell::Cell;
use std::fmt;
use std::rc::Rc;

use chrono::{DateTime, NaiveDate, NaiveDateTime, NaiveTime, Utc};

/// The calendar day, the moment now, and where the person is, as a Client reads them. Cheap to
/// clone: every clone reads the same source.
#[derive(Clone)]
pub struct Clock {
    day: Rc<dyn Fn() -> NaiveDate>,
    now: Rc<dyn Fn() -> DateTime<Utc>>,
    local: Rc<dyn Fn(DateTime<Utc>) -> NaiveDateTime>,
}

impl Clock {
    /// A Clock reading the day from `read`, now as midday UTC on that day, and local time in UTC:
    /// the moment the Sample Ledger prices its Units at, so a test that sets the day reads the same
    /// ages and hours on every machine. A Client reads the real time through [`Clock::with_time`].
    pub fn new(read: impl Fn() -> NaiveDate + 'static) -> Self {
        let day: Rc<dyn Fn() -> NaiveDate> = Rc::new(read);
        let read_day = day.clone();
        Self {
            day,
            now: Rc::new(move || {
                read_day().and_time(NaiveTime::MIN).and_utc() + chrono::Duration::hours(12)
            }),
            local: Rc::new(|at| at.naive_utc()),
        }
    }

    /// A Clock reading the day from `day`, now from `now`, and a moment's local time through
    /// `local`, which a bin points at the local date, the real time and the device's time zone. A
    /// conversion rather than a zone, so the bin keeps the zone a daylight-saving change moves.
    pub fn with_time(
        day: impl Fn() -> NaiveDate + 'static,
        now: impl Fn() -> DateTime<Utc> + 'static,
        local: impl Fn(DateTime<Utc>) -> NaiveDateTime + 'static,
    ) -> Self {
        Self {
            day: Rc::new(day),
            now: Rc::new(now),
            local: Rc::new(local),
        }
    }

    /// A Clock that reads `day` until the returned handle moves it, so a test can cross a month.
    pub fn settable(day: NaiveDate) -> (Self, Rc<Cell<NaiveDate>>) {
        let set = Rc::new(Cell::new(day));
        let read = set.clone();
        (Self::new(move || read.get()), set)
    }

    /// A Clock that always reads `day`.
    pub fn fixed(day: NaiveDate) -> Self {
        Self::new(move || day)
    }

    /// The calendar day now.
    pub fn today(&self) -> NaiveDate {
        (self.day)()
    }

    /// The moment now, which a price's age is measured from.
    pub fn now(&self) -> DateTime<Utc> {
        (self.now)()
    }

    /// `at` as the wall-clock time where the person is, which an hour's price and a time of day
    /// are read in.
    pub fn local_time(&self, at: DateTime<Utc>) -> NaiveDateTime {
        (self.local)(at)
    }
}

impl fmt::Debug for Clock {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Clock").field(&self.today()).finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).expect("a real date")
    }

    #[test]
    fn a_settable_clock_reads_the_day_its_handle_last_set_in_every_clone() {
        let (clock, day) = Clock::settable(date(2026, 9, 30));
        let other = clock.clone();
        assert_eq!(clock.today(), date(2026, 9, 30));

        day.set(date(2026, 10, 1));

        assert_eq!(clock.today(), date(2026, 10, 1));
        assert_eq!(other.today(), date(2026, 10, 1));
    }

    #[test]
    fn a_clock_on_a_day_alone_reads_midday_utc_on_that_day_as_now() {
        let (clock, day) = Clock::settable(date(2026, 6, 15));
        let midday = |d: NaiveDate| d.and_hms_opt(12, 0, 0).expect("a real time").and_utc();
        assert_eq!(clock.now(), midday(date(2026, 6, 15)));

        day.set(date(2026, 7, 1));

        assert_eq!(clock.now(), midday(date(2026, 7, 1)));
        assert_eq!(
            Clock::fixed(date(2026, 6, 15)).now(),
            midday(date(2026, 6, 15))
        );
    }

    #[test]
    fn a_clock_with_a_time_reads_now_from_it() {
        let at = date(2026, 6, 15)
            .and_hms_opt(8, 30, 0)
            .expect("a real time")
            .and_utc();
        let clock = Clock::with_time(|| date(2026, 6, 15), move || at, |at| at.naive_utc());
        assert_eq!(clock.now(), at);
        assert_eq!(clock.today(), date(2026, 6, 15));
    }

    #[test]
    fn a_clock_on_a_day_alone_reads_local_time_in_utc() {
        let at = date(2026, 6, 15)
            .and_hms_opt(23, 30, 0)
            .expect("a real time")
            .and_utc();
        assert_eq!(
            Clock::fixed(date(2026, 6, 15)).local_time(at),
            date(2026, 6, 15)
                .and_hms_opt(23, 30, 0)
                .expect("a real time")
        );
    }

    #[test]
    fn a_clock_with_a_time_reads_local_time_through_its_conversion() {
        let brisbane = chrono::FixedOffset::east_opt(10 * 3600).expect("a real offset");
        let at = date(2026, 6, 15)
            .and_hms_opt(23, 30, 0)
            .expect("a real time")
            .and_utc();
        let clock = Clock::with_time(
            || date(2026, 6, 15),
            move || at,
            move |at| at.with_timezone(&brisbane).naive_local(),
        );
        assert_eq!(
            clock.local_time(at),
            date(2026, 6, 16)
                .and_hms_opt(9, 30, 0)
                .expect("a real time")
        );
    }

    #[test]
    fn a_fixed_clock_always_reads_its_day() {
        assert_eq!(Clock::fixed(date(2026, 6, 15)).today(), date(2026, 6, 15));
    }
}
