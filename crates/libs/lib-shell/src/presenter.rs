//! The Shell's Presenter. It holds the Clock and the last calendar month it saw, and tells its
//! Router once the Clock reads a new one.

use lib_core::{Clock, Period};

use crate::protocols::ShellRouter;

/// What the Client delivers to the Shell.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShellInput {
    /// The Client's day poll fired: read the Clock for a new month.
    DayTick,
}

pub struct ShellPresenter {
    clock: Clock,
    /// The calendar month the Shell last saw the Clock read.
    month: Period,
}

impl ShellPresenter {
    /// The Shell on the month `clock` reads now, which the Client generated the Schedule for as
    /// it started.
    pub fn new(clock: Clock) -> Self {
        let month = Period::of(clock.today());
        Self { clock, month }
    }

    pub fn handle(&mut self, input: ShellInput, router: &mut dyn ShellRouter) {
        match input {
            ShellInput::DayTick => {
                let today = self.clock.today();
                if Period::of(today) != self.month {
                    self.month = Period::of(today);
                    router.month_turned(today);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;

    use super::*;
    use crate::fake::{RecordingRouter, Routed};

    fn date(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).expect("a real date")
    }

    #[test]
    fn a_day_tick_in_the_same_month_calls_nothing() {
        let (clock, day) = Clock::settable(date(2026, 9, 19));
        let mut shell = ShellPresenter::new(clock);
        let mut router = RecordingRouter::default();

        day.set(date(2026, 9, 30));
        shell.handle(ShellInput::DayTick, &mut router);

        assert!(router.routed.is_empty());
    }

    #[test]
    fn a_day_tick_in_a_new_month_turns_it_once_with_the_new_day() {
        let (clock, day) = Clock::settable(date(2026, 9, 30));
        let mut shell = ShellPresenter::new(clock);
        let mut router = RecordingRouter::default();

        day.set(date(2026, 10, 1));
        shell.handle(ShellInput::DayTick, &mut router);
        shell.handle(ShellInput::DayTick, &mut router);

        assert_eq!(router.routed, vec![Routed::MonthTurned(date(2026, 10, 1))]);
    }

    #[test]
    fn a_day_tick_in_the_same_month_of_another_year_turns_it() {
        let (clock, day) = Clock::settable(date(2026, 9, 30));
        let mut shell = ShellPresenter::new(clock);
        let mut router = RecordingRouter::default();

        day.set(date(2027, 9, 1));
        shell.handle(ShellInput::DayTick, &mut router);

        assert_eq!(router.routed, vec![Routed::MonthTurned(date(2027, 9, 1))]);
    }
}
