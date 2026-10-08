//! Proof that the headless harness drives the real key router and the real mouse handlers.

mod common;

use bin_desktop::navigation::nav::Noun;
use common::Harness;
use gpui::TestAppContext;

#[gpui::test]
fn g_f_jumps_to_documents(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);
    assert_ne!(ui.noun(), Noun::Documents, "must not start on Documents");

    ui.press("g f");

    assert_eq!(ui.noun(), Noun::Documents);
}

#[gpui::test]
fn clicking_the_rail_row_opens_documents(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);
    assert_ne!(ui.noun(), Noun::Documents, "must not start on Documents");

    ui.click("primary-rail-Documents");

    assert_eq!(ui.noun(), Noun::Documents);
}
