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
//! Every key takes one route, [`route_key`], offered to each step in turn until one takes it:
//!
//! 1. An open Dialog takes every key.
//! 2. A mode left in the bin: the palette, Search, a filter, the file explorer, or a mode the
//!    active View entered for something of its own.
//! 3. `lib-shell`'s global keymap, which takes the keys the Shell keeps (`?`, `:`, `/`, the rail
//!    toggle, `shift+tab`, `esc`, the Toast keys and the `g` jumps) and passes the rest on.
//! 4. The focused module's keymap.
//! 5. A View that isn't a module yet, with its own key handler.
//!
//! A key no step takes falls back to the Shell when its global keymap binds it to focus `tab`, `a`,
//! a movement or `enter`, so a list's own `tab` or `a` wins over the Shell's; otherwise the View's
//! leftover handler has it. Each step that reaches a Presenter does so through its own `send`.
//!
//! Halfway state: the Notices drain into the Shell's own Toast stack and status line, and the
//! focus, modes and status line the route reads are still the bin's (#826 moves them into
//! `lib-shell`). The palette, Search, filter and file explorer stay in the bin behind `InputMode`
//! until their own slice.

use std::borrow::BorrowMut;
use std::rc::Rc;

use gpui::{App, Global, WeakEntity};
use lib_database::Commit;
use lib_shell::ShellCommand;

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

/// What the Shell's global keymap made of a key (step 3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Offered {
    /// It took the key; `true` when the window needs redrawing.
    Took(bool),
    /// Not the Shell's to take first: `chord` goes on to the module and the View, with the
    /// command the Shell falls back on if neither takes it.
    Passed {
        chord: String,
        fallback: Option<ShellCommand>,
    },
}

/// The steps a key is offered to, which the Shell implements over its own state. A step answers
/// `Some(redraw)` once it takes the key and `None` to pass it on.
pub(crate) trait KeyRoute {
    /// Step 1: an open Dialog takes every key.
    fn dialog(&mut self) -> Option<bool>;

    /// Step 2: a mode left in the bin takes every key.
    fn mode(&mut self) -> Option<bool>;

    /// Step 3: the global keymap.
    fn global(&mut self) -> Offered;

    /// Step 4: the focused module's keymap, by `chord`, which may be a whole sequence (`g g`).
    /// Whether it took the key.
    fn module(&mut self, chord: &str) -> bool;

    /// Step 5: the focused View's own key handler, for a single key. Whether it took the key.
    fn view(&mut self) -> bool;

    /// No step took the key: the Shell's `fallback`, else the View's leftover handler, unless the
    /// key finished a `sequence`. Whether the window needs redrawing.
    fn fall_back(&mut self, fallback: Option<ShellCommand>, sequence: bool) -> bool;
}

/// Offers a key to each step of `route` in order, until one takes it. Returns whether the window
/// needs redrawing.
pub(crate) fn route_key(route: &mut impl KeyRoute) -> bool {
    if let Some(redraw) = route.dialog() {
        return redraw;
    }
    if let Some(redraw) = route.mode() {
        return redraw;
    }
    let (chord, fallback) = match route.global() {
        Offered::Took(redraw) => return redraw,
        Offered::Passed { chord, fallback } => (chord, fallback),
    };
    if route.module(&chord) {
        return true;
    }
    // A View not yet a module reads one key at a time, so a sequence is the module's or the
    // Shell's.
    let sequence = chord.contains(' ');
    if !sequence && route.view() {
        return true;
    }
    route.fall_back(fallback, sequence)
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use gpui::TestAppContext;
    use lib_list_cursor::Step;

    use super::*;

    /// A route whose steps take the key from `taker` on, recording each step it was offered to.
    struct Steps {
        taker: &'static str,
        chord: &'static str,
        asked: Vec<&'static str>,
    }

    impl Steps {
        fn taken_by(taker: &'static str) -> Self {
            Self {
                taker,
                chord: "j",
                asked: Vec::new(),
            }
        }

        fn ask(&mut self, step: &'static str) -> bool {
            self.asked.push(step);
            self.taker == step
        }
    }

    impl KeyRoute for Steps {
        fn dialog(&mut self) -> Option<bool> {
            self.ask("dialog").then_some(true)
        }

        fn mode(&mut self) -> Option<bool> {
            self.ask("mode").then_some(true)
        }

        fn global(&mut self) -> Offered {
            if self.ask("global") {
                return Offered::Took(true);
            }
            Offered::Passed {
                chord: self.chord.to_string(),
                fallback: Some(ShellCommand::Move(Step::Next)),
            }
        }

        fn module(&mut self, _chord: &str) -> bool {
            self.ask("module")
        }

        fn view(&mut self) -> bool {
            self.ask("view")
        }

        fn fall_back(&mut self, _fallback: Option<ShellCommand>, _sequence: bool) -> bool {
            self.ask("fall_back")
        }
    }

    #[test]
    fn each_step_is_asked_in_order_and_the_first_to_take_the_key_ends_it() {
        let order = ["dialog", "mode", "global", "module", "view", "fall_back"];
        for (index, taker) in order.into_iter().enumerate() {
            let mut steps = Steps::taken_by(taker);
            assert!(route_key(&mut steps), "{taker}");
            assert_eq!(steps.asked, order[..=index], "{taker}");
        }
    }

    #[test]
    fn a_sequence_skips_the_view_that_reads_one_key_at_a_time() {
        let mut steps = Steps::taken_by("fall_back");
        steps.chord = "g g";

        route_key(&mut steps);

        assert_eq!(
            steps.asked,
            ["dialog", "mode", "global", "module", "fall_back"]
        );
    }

    // Module default keys

    /// Every migrated module's default bindings, as `(command, chord)`. A new VIPER module adds
    /// its keymap here.
    fn module_defaults() -> Vec<(&'static str, &'static str)> {
        let units = lib_units::keymap::UNITS_PAGE_BINDINGS
            .iter()
            .map(|binding| (binding.command, binding.chord));
        let bills = lib_bills::keymap::BILLS_PAGE_BINDINGS
            .iter()
            .map(|binding| (binding.command, binding.chord));
        units.chain(bills).collect()
    }

    /// The defaults among `bindings` the Shell takes before any module sees them (step 3).
    fn clashes(bindings: &[(&'static str, &'static str)]) -> Vec<(&'static str, &'static str)> {
        let keymap = lib_shell::global_keymap(&lib_config::KeyBindingConfig::default());
        bindings
            .iter()
            .copied()
            .filter(|(_, chord)| lib_shell::keymap::keeps(&keymap, chord))
            .collect()
    }

    #[test]
    fn no_module_default_chord_is_a_key_the_shell_keeps() {
        assert_eq!(clashes(&module_defaults()), []);
    }

    #[test]
    fn a_default_on_a_kept_key_is_a_clash_and_one_the_shell_falls_back_on_is_not() {
        let defaults = [
            ("demo.rail", "b"),
            ("demo.dismiss", "ctrl+l"),
            ("demo.back", "shift+tab"),
            ("demo.jump", "g x"),
            ("demo.help", "?"),
            ("demo.insert", "a"),
            ("demo.free", "B"),
            ("demo.page", "ctrl+u"),
            ("demo.tab", "tab"),
            (lib_viper::keymap::MOVE_FIRST, "g g"),
        ];
        assert_eq!(
            clashes(&defaults),
            [
                ("demo.rail", "b"),
                ("demo.dismiss", "ctrl+l"),
                ("demo.back", "shift+tab"),
                ("demo.jump", "g x"),
                ("demo.help", "?"),
            ]
        );
    }

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
