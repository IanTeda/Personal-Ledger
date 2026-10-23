//! The date concern of `Shell`: the clock the Desktop reads the calendar day from, and the day
//! poll that delivers `lib-shell`'s [`ShellInput::DayTick`]. The Shell's Presenter decides when a
//! month has turned (#822); the root Router's [`ShellRouter::month_turned`] generates its Bill
//! Schedule wherever the person is (#793). An `impl Shell` block; the generation itself is the
//! Bills module's (`bills::ports::new_month`).

use std::time::Duration;

use chrono::NaiveDate;
use gpui::{Context, Entity};
use lib_core::Clock;
use lib_shell::{ShellInput, ShellRouter};

use super::Shell;
use crate::ledger_store::LedgerStore;
use crate::view::dashboard::state::DashboardView;

/// How often the Shell reads its clock for a new month. A month turns at midnight, so a minute
/// late is soon enough. Public so the headless tests advance past exactly this.
#[doc(hidden)]
pub const DATE_POLL: Duration = Duration::from_secs(60);

/// `today` moving on with the local clock, so a Desktop left running crosses into a new month.
pub fn wall_clock(today: NaiveDate) -> Clock {
    let started = chrono::Local::now().date_naive();
    Clock::new(move || today + (chrono::Local::now().date_naive() - started))
}

/// The root Router's month turn. The rest of the root Router is still `Shell` itself, until it
/// moves to the bin's `shell/` router (#827).
struct RootRouter<'a, 'b> {
    bills: &'a crate::bills::DesktopPorts,
    ledger: &'a Entity<LedgerStore>,
    dashboard: &'a Entity<DashboardView>,
    /// The Shell's today, which its legacy pages are dated by.
    today: &'a mut NaiveDate,
    cx: &'a mut Context<'b, Shell>,
}

impl ShellRouter for RootRouter<'_, '_> {
    /// Generates the new month's Bill Schedule, committed as one write, and moves the Shell's and
    /// the Dashboard's today on to it, so Needs Attention and the rail badge date their entries by
    /// it. The Bills page catches up on its own next event too, which finds nothing left to
    /// generate.
    fn month_turned(&mut self, today: NaiveDate) {
        *self.today = today;
        crate::bills::ports::new_month(self.bills, self.ledger, today, self.cx);
        self.dashboard
            .update(self.cx, |dashboard, cx| dashboard.set_today(today, cx));
        self.cx.notify();
    }
}

impl Shell {
    /// Delivers a [`ShellInput::DayTick`] every [`DATE_POLL`] for the window's life. Stops once the
    /// window, and with it the Shell, has gone. Called once, by `build_shell`.
    pub fn start_date_clock(&self, cx: &mut Context<'_, Self>) {
        cx.spawn(async move |this, cx| {
            loop {
                // The executor's timer, not `Timer::after`, so headless tests can advance it.
                cx.background_executor().timer(DATE_POLL).await;
                if this.update(cx, |shell, cx| shell.day_tick(cx)).is_err() {
                    break;
                }
            }
        })
        .detach();
    }

    fn day_tick(&mut self, cx: &mut Context<'_, Self>) {
        let mut router = RootRouter {
            bills: &self.bills,
            ledger: &self.ledger,
            dashboard: &self.views.dashboard,
            today: &mut self.today,
            cx,
        };
        self.presenter.handle(ShellInput::DayTick, &mut router);
    }
}
