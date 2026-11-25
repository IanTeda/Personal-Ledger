//! The Clock the TUI reads the calendar day and the moment now from: the local date and the real
//! time, read on each call, so a TUI left running crosses midnight. Built here, not in `lib-core`,
//! since reading the time is I/O.

use lib_core::Clock;

/// The local date and the real time, which an Open Ledger and its Units read.
pub fn wall_clock() -> Clock {
    Clock::with_time(|| chrono::Local::now().date_naive(), chrono::Utc::now)
}
