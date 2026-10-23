//! # Clock
//!
//! [`Clock`] is where a Client reads the calendar day from. A Client builds it on the local date
//! (each bin's `wall_clock`, since reading the time is I/O), a test on a day it sets, so every
//! View and module dates its rows from one source and only the Shell notices a new month.

use std::cell::Cell;
use std::fmt;
use std::rc::Rc;

use chrono::NaiveDate;

/// The calendar day as a Client reads it. Cheap to clone: every clone reads the same source.
/// Dated by the day alone, so it reads no time.
#[derive(Clone)]
pub struct Clock(Rc<dyn Fn() -> NaiveDate>);

impl Clock {
    /// A Clock reading the day from `read`, which a bin points at the local date.
    pub fn new(read: impl Fn() -> NaiveDate + 'static) -> Self {
        Self(Rc::new(read))
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
        (self.0)()
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
    fn a_fixed_clock_always_reads_its_day() {
        assert_eq!(Clock::fixed(date(2026, 6, 15)).today(), date(2026, 6, 15));
    }
}
