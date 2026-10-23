//! The root Router: what `lib-shell`'s Presenter asks of the Desktop, and the one place one module
//! is called from another (#820). [`Shell::send_shell`] delivers each [`ShellInput`] through the
//! one `send`, so a month turn's Bills, or a failed write's reload, lands in that input's commit.
//! An `impl Shell` block; the rest of the root Router is still `Shell` itself, until it moves to
//! the bin's `shell/` router (#827).

use chrono::NaiveDate;
use gpui::{Context, Entity};
use lib_database::WriteFailure;
use lib_shell::{ShellInput, ShellRouter};
use lib_toast::{Notice, Notify};

use super::Shell;
use crate::ledger_store::LedgerStore;
use crate::view::dashboard::state::DashboardView;

/// The parts of `Shell` the Shell's Presenter acts on, borrowed for one input.
struct RootRouter<'a, 'b> {
    units: &'a crate::units::DesktopPorts,
    bills: &'a crate::bills::DesktopPorts,
    ledger: &'a Entity<LedgerStore>,
    dashboard: &'a Entity<DashboardView>,
    /// The Shell's today, which its legacy pages are dated by.
    today: &'a mut NaiveDate,
    cx: &'a mut Context<'b, Shell>,
}

impl Notify for RootRouter<'_, '_> {
    /// Queues `notice`, for the input's `send` to drain into the Shell.
    fn notify(&mut self, notice: Notice) {
        crate::notices::Queue.notify(notice);
    }
}

impl ShellRouter for RootRouter<'_, '_> {
    type Failure = WriteFailure;

    /// Generates the new month's Bill Schedule, committed as the day poll's one write, and moves
    /// the Shell's and the Dashboard's today on to it, so Needs Attention and the rail badge date
    /// their entries by it. The Bills page catches up on its own next event too, which finds
    /// nothing left to generate.
    fn month_turned(&mut self, today: NaiveDate) {
        *self.today = today;
        crate::bills::ports::new_month(self.bills, self.ledger, today, self.cx);
        self.dashboard
            .update(self.cx, |dashboard, cx| dashboard.set_today(today, cx));
    }

    /// Puts each domain `failure` names back as stored, with an Error Toast for each.
    fn write_failed(&mut self, failure: WriteFailure) {
        for notice in super::write_failures::roll_back(&failure, self.units, self.ledger, self.cx) {
            self.notify(notice);
        }
    }
}

impl Shell {
    /// Delivers `input` to the Shell's Presenter as one input, with the root Router.
    pub(super) fn send_shell(
        &mut self,
        input: ShellInput<WriteFailure>,
        cx: &mut Context<'_, Self>,
    ) {
        crate::input::send(cx, |cx| {
            let mut router = RootRouter {
                units: &self.units,
                bills: &self.bills,
                ledger: &self.ledger,
                dashboard: &self.views.dashboard,
                today: &mut self.today,
                cx,
            };
            self.presenter.handle(input, &mut router);
        });
    }
}
