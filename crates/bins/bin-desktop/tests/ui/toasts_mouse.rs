//! The Toast layer and the Toast history driven by clicks: a Toast's cross and the history's close
//! button. Only elements tagged with `debug_selector` are clicked; state is read back from `Shell`.

use super::common;

use bin_desktop::navigation::nav::InputMode;
use common::Harness;
use gpui::TestAppContext;
use lib_toast::ToastKind;

#[gpui::test]
fn a_toasts_cross_dismisses_just_that_toast(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);
    ui.raise_toast(ToastKind::Info, "first");
    ui.raise_toast(ToastKind::Success, "second");

    ui.click("toast-dismiss-0");
    let toasts = ui.toasts();
    assert_eq!(toasts.visible.len(), 1);
    assert_eq!(toasts.visible[0].text, "second");
    assert_eq!(
        toasts.history.len(),
        2,
        "the history keeps what was dismissed"
    );
}

#[gpui::test]
fn the_history_close_button_shuts_it(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);
    ui.raise_toast(ToastKind::Info, "one");
    ui.press(": m e s s a g e s enter");
    assert!(ui.toasts().history_open);

    ui.click("toast-history-close");
    assert!(!ui.toasts().history_open);
    assert_eq!(ui.read(|s| s.nav().mode()), InputMode::Normal);
}
