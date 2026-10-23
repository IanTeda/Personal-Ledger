//! The Desktop's one door for input (#820): every module View click and key, every Dialog key and
//! button, the Shell's day poll and failed writes, and each Toast, status or write a View not yet
//! a module reports reach a Presenter through [`send`]. It runs the Presenter's `handle`, commits
//! exactly once, whether or not anything was written (an empty commit is a no-op in the writer),
//! then drains the Notices the Routers queued into the Shell and redraws. No port or store
//! commits for itself.
//!
//! A call from one module to another (the root Router's month turn, a Dialog handing its outcome
//! to its page) runs inside the current `send`, so the one commit covers every module's writes.
//! A nested `send` is a bug, refused by `debug_assert!`.
//!
//! Halfway state: keys still reach the modules through `route_key` and the Dialog host, each
//! ending in a `send` (#825 routes them here), and the Notices drain into the Shell's own Toast
//! stack and status line (#826 moves them into `lib-shell`).

use std::borrow::BorrowMut;
use std::rc::Rc;

use gpui::{App, Global, WeakEntity};
use lib_database::Commit;

use crate::app::Shell;

/// What [`send`] reaches through: the commit every store's writes end in, and the Shell the
/// Notices drain into. One per window's App, installed by `build_shell`.
struct Door {
    commit: Rc<dyn Commit>,
    shell: WeakEntity<Shell>,
    /// Whether a `send` is running, so a second one inside it is caught.
    open: bool,
}

impl Global for Door {}

/// Opens the door on `commit`, draining into `shell`. Called once, by `build_shell`.
pub(crate) fn install(commit: Rc<dyn Commit>, shell: WeakEntity<Shell>, cx: &mut App) {
    cx.set_global(Door {
        commit,
        shell,
        open: false,
    });
}

/// Whether an input is being handled now, so a write made inside it waits for its commit rather
/// than asking for one of its own.
pub(crate) fn sending(cx: &App) -> bool {
    cx.try_global::<Door>().is_some_and(|door| door.open)
}

/// Delivers one input: runs `handle`, which hands it to its Presenter, then commits once, drains
/// the queued Notices into the Shell and redraws it. The drain waits for the current update to
/// end, since the Shell may be the Entity handling the input. Without a door (a unit test that
/// builds a part with no Shell), it only runs `handle`.
pub(crate) fn send<C: BorrowMut<App>, R>(cx: &mut C, handle: impl FnOnce(&mut C) -> R) -> R {
    if !cx.borrow_mut().has_global::<Door>() {
        return handle(cx);
    }
    let door = cx.borrow_mut().global_mut::<Door>();
    debug_assert!(
        !door.open,
        "an input was sent inside another; a call between modules runs in the current send"
    );
    door.open = true;
    let handled = handle(cx);
    let app: &mut App = cx.borrow_mut();
    let door = app.global_mut::<Door>();
    door.open = false;
    door.commit.commit();
    let shell = door.shell.clone();
    app.defer(move |cx| {
        // Gone with the window, so there is nothing left to show the Notices on.
        let _ = shell.update(cx, |shell, cx| {
            shell.drain_notices();
            cx.notify();
        });
    });
    handled
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use gpui::TestAppContext;

    use super::*;

    /// A door on a counting commit, draining into no Shell.
    fn door(cx: &mut TestAppContext) -> Rc<Cell<usize>> {
        let commit = lib_database::CountingCommit::new(Rc::new(lib_database::InMemoryCommit));
        let count = commit.count();
        cx.update(|cx| install(Rc::new(commit), WeakEntity::new_invalid(), cx));
        count
    }

    #[gpui::test]
    fn a_send_commits_once_after_handle_whether_or_not_it_wrote(cx: &mut TestAppContext) {
        let count = door(cx);

        let seen = cx.update(|cx| {
            send(cx, |cx| {
                assert!(sending(cx), "handle runs inside the send");
                count.get()
            })
        });

        assert_eq!(seen, 0, "the commit comes after handle");
        assert_eq!(count.get(), 1);
        assert!(!cx.update(|cx| sending(cx)));
    }

    #[gpui::test]
    #[should_panic(expected = "an input was sent inside another")]
    fn a_send_inside_a_send_is_refused(cx: &mut TestAppContext) {
        door(cx);

        cx.update(|cx| send(cx, |cx| send(cx, |_| ())));
    }
}
