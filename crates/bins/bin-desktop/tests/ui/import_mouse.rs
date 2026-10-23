//! The Import step driven by clicks: a row, Remember and Back. Only elements tagged with
//! `debug_selector` are clicked; state is read back from `Shell`.

use super::common;

use bin_desktop::navigation::nav::Noun;
use common::Harness;
use gpui::TestAppContext;

fn on_import(app: &mut TestAppContext) -> Harness<'_> {
    let mut ui = Harness::new(app);
    ui.press(": i m p o r t enter");
    assert!(ui.import().is_some());
    ui
}

#[gpui::test]
fn clicking_an_import_row_selects_it(app: &mut TestAppContext) {
    let mut ui = on_import(app);

    ui.click("import-row-3");
    assert_eq!(ui.import().expect("import is open").selected, 3);
}

#[gpui::test]
fn clicking_remember_toggles_it(app: &mut TestAppContext) {
    let mut ui = on_import(app);

    ui.click("import-remember");
    assert!(!ui.import().expect("import is open").remember);
    ui.click("import-remember");
    assert!(ui.import().expect("import is open").remember);
}

#[gpui::test]
fn clicking_back_leaves_the_import_step(app: &mut TestAppContext) {
    let mut ui = on_import(app);

    ui.click("import-back");
    assert_eq!(ui.import(), None);
    assert_eq!(ui.noun(), Noun::Transactions);
}
