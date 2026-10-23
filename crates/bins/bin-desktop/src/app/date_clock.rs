//! The date concern of `Shell`: the clock the Desktop reads the calendar day from, and the month
//! tick that generates a new month's Bill Schedule wherever the person is (#793). An `impl Shell`
//! block; the generation itself is the Bills module's (`bills::ports::new_month`).

use std::time::Duration;

use chrono::NaiveDate;
use gpui::Context;
use lib_core::Period;

use super::Shell;
use crate::bills::presenter::Clock;

/// How often the Shell reads its clock for a new month. A month turns at midnight, so a minute
/// late is soon enough. Public so the headless tests advance past exactly this.
#[doc(hidden)]
pub const DATE_POLL: Duration = Duration::from_secs(60);

/// `today` moving on with the local clock, so a Desktop left running crosses into a new month.
pub fn wall_clock(today: NaiveDate) -> Clock {
    let started = chrono::Local::now().date_naive();
    std::rc::Rc::new(move || today + (chrono::Local::now().date_naive() - started))
}

impl Shell {
    /// Reads the clock every [`DATE_POLL`] for the window's life. Stops once the window, and with
    /// it the Shell, has gone. Called once, by `build_shell`.
    pub fn start_date_clock(&self, cx: &mut Context<'_, Self>) {
        cx.spawn(async move |this, cx| {
            loop {
                // The executor's timer, not `Timer::after`, so headless tests can advance it.
                cx.background_executor().timer(DATE_POLL).await;
                if this.update(cx, |shell, cx| shell.read_clock(cx)).is_err() {
                    break;
                }
            }
        })
        .detach();
    }

    /// Once the clock reads a new calendar month: generates its Bill Schedule, committed as one
    /// write, and moves the Shell's and the Dashboard's today on to it, so Needs Attention and
    /// the rail badge date their entries by it. The Bills page catches up on its own next event
    /// too, which finds nothing left to generate.
    fn read_clock(&mut self, cx: &mut Context<'_, Self>) {
        let today = (self.clock)();
        if Period::of(today) == Period::of(self.today) {
            return;
        }
        self.today = today;
        crate::bills::ports::new_month(&self.bills, &self.ledger, today, cx);
        self.views
            .dashboard
            .update(cx, |dashboard, cx| dashboard.set_today(today, cx));
        cx.notify();
    }
}
