//! The Toast layer and the Toast history driven by keys and commands: raising and merging Toasts,
//! `ctrl-l` and `:dismiss all`, `:messages` opening the history, `:toasts off`/`on`, and the Toast
//! timer running a Toast's lifetime out. State is read back from `Root`.

use std::time::Duration;

use super::common;

use bin_desktop::navigation::nav::InputMode;
use common::Harness;
use gpui::TestAppContext;
use lib_toast::ToastKind;

fn mode(ui: &mut Harness<'_>) -> InputMode {
    ui.read(|shell| shell.nav().mode())
}

#[gpui::test]
fn a_raised_toast_shows_in_the_stack_and_the_history(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);
    assert!(ui.toasts().on);
    assert!(ui.toasts().visible.is_empty());

    ui.raise_toast(ToastKind::Success, "Saved");
    let toasts = ui.toasts();
    assert_eq!(toasts.visible.len(), 1);
    assert_eq!(toasts.visible[0].kind, "Success");
    assert_eq!(toasts.visible[0].text, "Saved");
    assert_eq!(toasts.history.len(), 1);
}

#[gpui::test]
fn a_repeated_toast_merges_into_one_with_a_count(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);

    ui.raise_toast(ToastKind::Info, "Synced");
    ui.raise_toast(ToastKind::Info, "Synced");
    let toasts = ui.toasts();
    assert_eq!(toasts.visible.len(), 1);
    assert_eq!(toasts.visible[0].count, 2);
    assert_eq!(toasts.history.len(), 1);
    assert_eq!(toasts.history[0].count, 2);
}

#[gpui::test]
fn ctrl_l_dismisses_every_toast_but_keeps_the_history(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);
    ui.raise_toast(ToastKind::Info, "one");
    ui.raise_toast(ToastKind::Warning, "two");
    assert_eq!(ui.toasts().visible.len(), 2);

    ui.press("ctrl-l");
    let toasts = ui.toasts();
    assert!(toasts.visible.is_empty());
    assert_eq!(toasts.history.len(), 2);
}

#[gpui::test]
fn the_dismiss_all_command_clears_the_stack(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);
    ui.raise_toast(ToastKind::Info, "one");

    ui.press(": d i s m i s s space a l l enter");
    assert!(ui.toasts().visible.is_empty());
    assert_eq!(mode(&mut ui), InputMode::Normal);
}

#[gpui::test]
fn the_toasts_command_opens_the_history_newest_first_and_escape_closes_it(
    app: &mut TestAppContext,
) {
    let mut ui = Harness::new(app);
    ui.raise_toast(ToastKind::Info, "older");
    ui.raise_toast(ToastKind::Error, "newer");

    ui.press(": m e s s a g e s enter");
    let toasts = ui.toasts();
    assert!(toasts.history_open);
    assert_eq!(mode(&mut ui), InputMode::Dialog);
    assert_eq!(toasts.history[0].text, "newer");
    assert_eq!(toasts.history[1].text, "older");

    ui.press("escape");
    assert!(!ui.toasts().history_open);
    assert_eq!(mode(&mut ui), InputMode::Normal);
    assert_eq!(ui.toasts().visible.len(), 2, "viewing dismissed nothing");
}

#[gpui::test]
fn toasts_off_and_on_set_the_preference(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);

    ui.press(": t o a s t s space o f f enter");
    assert!(!ui.toasts().on);
    ui.raise_toast(ToastKind::Info, "hidden");
    assert!(ui.toasts().visible.is_empty());

    ui.press(": t o a s t s space o n enter");
    assert!(ui.toasts().on);
}

#[gpui::test]
fn the_toast_timer_runs_a_toast_out_and_leaves_an_error(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);
    ui.raise_toast(ToastKind::Success, "Saved");
    ui.raise_toast(ToastKind::Error, "Save failed");

    ui.advance_clock(Duration::from_secs(3));
    assert_eq!(ui.toasts().visible.len(), 2, "a Success lives four seconds");

    ui.advance_clock(Duration::from_secs(2));
    let texts: Vec<String> = ui.toasts().visible.into_iter().map(|t| t.text).collect();
    assert_eq!(texts, ["Save failed"], "an Error stays until dismissed");
}

#[gpui::test]
fn a_sticky_toast_ticks_without_an_input(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);
    ui.raise_toast(ToastKind::Error, "Save failed");
    let commits = ui.commits();

    ui.advance_clock(Duration::from_secs(10));

    assert_eq!(ui.toasts().visible.len(), 1);
    assert_eq!(
        ui.commits(),
        commits,
        "a tick writes nothing, so commits nothing"
    );
}
