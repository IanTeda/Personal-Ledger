//! The root Router: what `lib-shell`'s Presenter asks of the Desktop, and the one place one module
//! is called from another (#820). [`Shell::send_shell`] delivers each [`ShellInput`] through the
//! one `send`, so a month turn's Bills, or a failed write's reload, lands in that input's commit,
//! and [`Shell::resolve_key`] each key the global keymap resolves. An `impl Shell` block; the rest
//! of the root Router is still `Shell` itself, until it moves to the bin's `shell/` router (#827).
//!
//! Halfway state: the focus, modes, rails and palette a global command acts on are still the
//! Shell's own fields, which the Router can't borrow beside the Presenter. So the Router hands
//! each command back and the Shell acts on it once the Presenter returns (#826 moves them into
//! `lib-shell`).

use chrono::NaiveDate;
use gpui::{Context, Entity};
use lib_database::WriteFailure;
use lib_shell::{ShellCommand, ShellInput, ShellKey, ShellRouter};
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
    /// The global commands the Presenter ran, for the Shell to act on once it returns.
    ran: Vec<ShellCommand>,
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

    fn run(&mut self, command: ShellCommand) {
        self.ran.push(command);
    }

    /// The handoff: "`g` + an unbound key is a no-op: clear the pending prefix and flash the hint
    /// strip."
    fn not_a_jump(&mut self, sequence: String) {
        self.notify(Notice::Status(crate::msg::desktop_status_not_a_jump(
            &sequence,
        )));
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
                ran: Vec::new(),
                cx,
            };
            self.presenter.handle(input, &mut router);
        });
    }

    /// Offers `chord` to the Shell's Presenter, returning its answer and the global commands it
    /// ran. A key the Shell takes is one input, committed once with its Notice drained; one it
    /// passes on is the next step's input, which commits through that step's own `send`. The Shell
    /// acts on the commands outside the input, since one may reach a module that sends its own.
    pub(super) fn resolve_key(
        &mut self,
        chord: &str,
        cx: &mut Context<'_, Self>,
    ) -> (ShellKey, Vec<ShellCommand>) {
        let mut router = RootRouter {
            units: &self.units,
            bills: &self.bills,
            ledger: &self.ledger,
            dashboard: &self.views.dashboard,
            today: &mut self.today,
            ran: Vec::new(),
            cx,
        };
        let answer = self.presenter.key(chord, &mut router);
        let ran = router.ran;
        if !matches!(answer, ShellKey::Passed { .. }) {
            crate::input::send(cx, |_| ());
        }
        (answer, ran)
    }
}
