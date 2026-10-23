//! The Shell's Presenter. It holds the Clock and the last calendar month it saw, and tells its
//! Router once the Clock reads a new one. The Client's own inputs, a Notice, a legacy write and a
//! failed one, pass through it to the Router until the Presenter holds what they change (#826).

use lib_core::{Clock, Period};
use lib_toast::Notice;

use crate::protocols::ShellRouter;

/// What the Client delivers to the Shell, each through its one `send`. `F` is the Client's write
/// failure ([`ShellRouter::Failure`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShellInput<F> {
    /// The Client's day poll fired: read the Clock for a new month.
    DayTick,
    /// A View not yet a module raised a Toast or set the status line.
    Notice(Notice),
    /// A View not yet a module ran a write on the Client's legacy store, outside any input the
    /// Shell saw. The write has run; this input exists so the `send` it arrives in commits it,
    /// the one commit path (#823). It goes when the last such View migrates.
    LegacyApply,
    /// A committed write failed.
    WriteFailed(F),
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

    pub fn handle<R: ShellRouter + ?Sized>(
        &mut self,
        input: ShellInput<R::Failure>,
        router: &mut R,
    ) {
        match input {
            ShellInput::DayTick => {
                let today = self.clock.today();
                if Period::of(today) != self.month {
                    self.month = Period::of(today);
                    router.month_turned(today);
                }
            }
            ShellInput::Notice(notice) => router.notify(notice),
            ShellInput::LegacyApply => {}
            ShellInput::WriteFailed(failure) => router.write_failed(failure),
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
    fn a_notice_and_a_write_failure_go_to_the_router_and_a_legacy_write_asks_nothing() {
        let mut shell = ShellPresenter::new(Clock::fixed(date(2026, 9, 19)));
        let mut router = RecordingRouter::default();
        let saved = Notice::Toast(lib_toast::ToastKind::Success, "Saved".to_string());

        shell.handle(ShellInput::Notice(saved.clone()), &mut router);
        shell.handle(ShellInput::LegacyApply, &mut router);
        shell.handle(
            ShellInput::WriteFailed("disk full".to_string()),
            &mut router,
        );

        assert_eq!(
            router.routed,
            vec![
                Routed::Notice(saved),
                Routed::WriteFailed("disk full".to_string())
            ]
        );
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
