//! The Clock the TUI reads the moment now, local time and the calendar day from: the real time and
//! the device's time zone, read on each call, so a TUI left running crosses midnight. Built here,
//! not in `lib-core`, since reading the time and the zone is I/O.

use lib_core::Clock;

/// The real time and the device's zone, which an Open Ledger and its Units read.
pub fn wall_clock() -> Clock {
    Clock::new(chrono::Utc::now, |at| {
        at.with_timezone(&chrono::Local).naive_local()
    })
}
