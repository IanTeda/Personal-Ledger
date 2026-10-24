//! The date concern of `Root`: the clock the Desktop reads the calendar day from, and the day
//! poll that delivers `lib-shell`'s [`ShellInput::DayTick`] as one input. The Shell's Presenter
//! decides when a month has turned (#822); the root Router's `month_turned` (`root_router.rs`)
//! generates its Bill Schedule wherever the person is (#793). An `impl Root` block; the
//! generation itself is the Bills module's (`bills::ports::new_month`).

use std::time::Duration;

use chrono::NaiveDate;
use gpui::Context;
use lib_core::Clock;
use lib_shell::ShellInput;

use super::Root;

/// How often the Shell reads its clock for a new month. A month turns at midnight, so a minute
/// late is soon enough. Public so the headless tests advance past exactly this.
#[doc(hidden)]
pub const DATE_POLL: Duration = Duration::from_secs(60);

/// `today` moving on with the local clock, so a Desktop left running crosses into a new month.
pub fn wall_clock(today: NaiveDate) -> Clock {
    let started = chrono::Local::now().date_naive();
    Clock::new(move || today + (chrono::Local::now().date_naive() - started))
}

impl Root {
    /// Delivers a [`ShellInput::DayTick`] every [`DATE_POLL`] for the window's life. Stops once the
    /// window, and with it the Shell, has gone. Called once, by `shell::router::build`.
    pub fn start_date_clock(&self, cx: &mut Context<'_, Self>) {
        cx.spawn(async move |this, cx| {
            loop {
                // The executor's timer, not `Timer::after`, so headless tests can advance it.
                cx.background_executor().timer(DATE_POLL).await;
                if this
                    .update(cx, |shell, cx| shell.send_shell(ShellInput::DayTick, cx))
                    .is_err()
                {
                    break;
                }
            }
        })
        .detach();
    }
}
