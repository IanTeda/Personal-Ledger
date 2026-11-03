//! The root Router: what `lib-shell`'s Presenter asks of the Desktop, and the one place one module
//! is called from another (#820). [`Root::send_shell`] delivers each [`ShellInput`] through the
//! one `send`, so a month turn's Bills, or a failed write's reload, lands in that input's commit,
//! and [`Root::resolve_key`] each key the global keymap resolves. An `impl Root` block of the root
//! Router, which is `Root` itself.
//!
//! The rails, palette and Views a global command acts on are `Root`'s own fields, and the focus and
//! mode it changes are the Presenter's, which the Router can't borrow while the Presenter calls it.
//! So the Router hands each command back and `Root` acts on it once the Presenter returns.

use chrono::NaiveDate;
use gpui::Context;
use lib_database::WriteFailure;
use lib_shell::{ShellCommand, ShellInput, ShellKey, ShellRouter};
use lib_toast::{Notice, Notify};

use super::{OpenLedger, Root};

/// The parts of `Root` the Shell's Presenter acts on, borrowed for one input.
struct RootRouter<'a, 'b> {
    /// The Open Ledger a month turn and a failed write act on, `None` with no Ledger open.
    open: Option<&'a OpenLedger>,
    /// The Shell's today, which its legacy pages are dated by.
    today: &'a mut NaiveDate,
    /// The global commands the Presenter ran, for the Shell to act on once it returns.
    ran: Vec<ShellCommand>,
    cx: &'a mut Context<'b, Root>,
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
    /// their entries by it. The Budgets page moves a month on its old one on to the new, and the
    /// Documents page bands its Key Dates against the new day and drops a Financial Year scope
    /// that has left the rail. The Bills page catches up on its own next event too, which finds
    /// nothing left to generate.
    /// With no Ledger open only the Shell's today moves on.
    fn month_turned(&mut self, today: NaiveDate) {
        *self.today = today;
        let Some(open) = self.open else {
            return;
        };
        crate::bills::ports::new_month(&open.bills, &open.ledger, today, self.cx);
        open.views
            .dashboard
            .update(self.cx, |dashboard, cx| dashboard.set_today(today, cx));
        open.views
            .budgets
            .update(self.cx, |budgets, cx| budgets.month_turned(cx));
        open.views
            .documents
            .update(self.cx, |documents, cx| documents.month_turned(cx));
    }

    /// Puts each domain `failure` names back as stored, with an Error Toast for each. A failure
    /// arriving with no Ledger open has nothing left to put back.
    fn write_failed(&mut self, failure: WriteFailure) {
        let Some(open) = self.open else {
            return;
        };
        for notice in super::write_failures::roll_back(&failure, &open.units, &open.ledger, self.cx)
        {
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

impl Root {
    /// Delivers `input` to the Shell's Presenter as one input, with the root Router.
    pub(super) fn send_shell(
        &mut self,
        input: ShellInput<WriteFailure>,
        cx: &mut Context<'_, Self>,
    ) {
        crate::input::send(cx, |cx| {
            let mut router = RootRouter {
                open: self.open.as_ref(),
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
            open: self.open.as_ref(),
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
