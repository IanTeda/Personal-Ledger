//! Start-up and the Ledger file, driven by the keyboard (#977): the Desktop starts with no Ledger
//! open unless On start or `file` names one, a named file that is missing or won't open leaves none
//! open and says so, and with none open every key that needs a Ledger is refused on the status line
//! while `:open`, `:new`, Help and the This device Settings pages still run. State is read back
//! from `Root`. `:open`'s file explorer picks a file by mouse alone (01-shell's "Row click selects
//! it"), so opening one is `ledger_file_mouse.rs`'s; here `:open` and `esc` run from the keys (#978).

use super::common;

use bin_desktop::{
    Started,
    navigation::nav::{InputMode, Noun},
    persistence::PersistedState,
};
use common::{Harness, config_for, new_ledger};
use gpui::TestAppContext;

/// The state file of a run that last showed `noun` on Settings' `page`.
fn remembering(noun: Noun, page: &str) -> PersistedState {
    PersistedState {
        noun,
        settings_page: Some(page.to_string()),
        ..Default::default()
    }
}

#[gpui::test]
fn with_no_ledger_named_the_desktop_starts_on_the_dashboard_with_none_open(
    app: &mut TestAppContext,
) {
    let mut ui = Harness::starting(app, Started::NoLedger, remembering(Noun::Bills, "units"));

    assert!(!ui.read(|shell| shell.ledger_open()));
    assert_eq!(ui.noun(), Noun::Dashboard, "whatever noun it last showed");
    assert!(ui.is_drawn("no-ledger-open"));
    assert!(ui.is_drawn("no-ledger-new"));
    assert!(ui.toasts().visible.is_empty(), "nothing went wrong to say");
}

#[gpui::test]
fn a_missing_named_ledger_starts_with_none_open_creates_nothing_and_says_so(
    app: &mut TestAppContext,
) {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let missing = dir.path().join("missing.pldb");

    let started = bin_desktop::start_on(&config_for(&missing), common::today());
    assert!(matches!(&started, Started::Failed(failure) if failure.path == missing));
    let mut ui = Harness::starting(app, started, PersistedState::default());

    assert!(!missing.exists(), "start-up never creates a Ledger file");
    assert!(!ui.read(|shell| shell.ledger_open()));
    assert_eq!(ui.noun(), Noun::Dashboard);
    let toast = ui
        .toasts()
        .visible
        .pop()
        .expect("a Toast says the Ledger didn't open");
    assert_eq!(toast.kind, "Error");
    assert_eq!(
        toast.text,
        "Couldn't open ledger missing.pldb: no such file"
    );
}

#[gpui::test]
fn a_named_ledger_that_opens_starts_on_it_as_before(app: &mut TestAppContext) {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let config = new_ledger(&dir.path().join("ledger.pldb"));

    let started = bin_desktop::start_on(&config, common::today());
    assert!(matches!(started, Started::Opened { .. }));
    let mut ui = Harness::starting(app, started, remembering(Noun::Bills, "units"));

    assert!(ui.read(|shell| shell.ledger_open()));
    assert_eq!(ui.noun(), Noun::Bills, "the remembered noun");
    assert_eq!(ui.unit_codes(), ["aud", "btc", "vas"]);
}

#[gpui::test]
fn a_g_jump_to_a_noun_that_needs_a_ledger_is_refused(app: &mut TestAppContext) {
    let mut ui = Harness::without_ledger(app);

    for jump in ["g l", "g f", "g w", "g b", "g c"] {
        ui.press(jump);
        assert_eq!(ui.noun(), Noun::Dashboard, "{jump} stays put");
        assert!(ui.refused_without_ledger(), "{jump} says why");
    }
    ui.press("g d");
    assert_eq!(ui.noun(), Noun::Dashboard);
    assert!(
        !ui.refused_without_ledger(),
        "the Dashboard stands without a Ledger"
    );
}

#[gpui::test]
fn committing_the_rail_to_a_noun_that_needs_a_ledger_is_refused(app: &mut TestAppContext) {
    let mut ui = Harness::without_ledger(app);

    ui.press("shift-tab");
    ui.press("j");
    ui.press("enter");
    assert_eq!(ui.noun(), Noun::Dashboard);
    assert!(ui.refused_without_ledger());
}

#[gpui::test]
fn g_s_opens_settings_on_the_remembered_this_device_page(app: &mut TestAppContext) {
    let mut ui = Harness::starting(
        app,
        Started::NoLedger,
        remembering(Noun::Dashboard, "display"),
    );

    ui.press("g s");
    assert_eq!(ui.noun(), Noun::Settings);
    assert_eq!(ui.settings_frame().page, "Display");
}

#[gpui::test]
fn g_s_opens_general_when_the_remembered_page_needs_a_ledger(app: &mut TestAppContext) {
    let mut ui = Harness::starting(app, Started::NoLedger, remembering(Noun::Settings, "units"));

    ui.press("g s");
    assert_eq!(ui.noun(), Noun::Settings);
    assert_eq!(ui.settings_frame().page, "General");
}

#[gpui::test]
fn a_palette_command_that_needs_a_ledger_is_refused(app: &mut TestAppContext) {
    let mut ui = Harness::without_ledger(app);

    ui.press(": a c c o u n t s enter");
    assert_eq!(ui.noun(), Noun::Dashboard);
    assert!(ui.refused_without_ledger());
    assert_eq!(ui.read(|shell| shell.nav().mode()), InputMode::Normal);

    ui.press(": i m p o r t enter");
    assert_eq!(ui.noun(), Noun::Dashboard);
    assert!(ui.refused_without_ledger());
}

#[gpui::test]
fn open_help_and_this_device_settings_run_with_no_ledger_open(app: &mut TestAppContext) {
    let mut ui = Harness::without_ledger(app);

    ui.press(": s e t t i n g s space d i s p l a y enter");
    assert_eq!(ui.noun(), Noun::Settings);
    assert_eq!(ui.settings_frame().page, "Display");
    assert!(!ui.refused_without_ledger());

    ui.press("?");
    assert!(ui.read(bin_desktop::shell::router::Root::help_open));
    ui.press("escape");

    ui.press(": o p e n enter");
    assert_eq!(
        ui.read(|shell| shell.nav().mode()),
        InputMode::Command,
        "the file explorer is open"
    );
    assert!(!ui.refused_without_ledger());
}

#[gpui::test]
fn stepping_the_settings_index_onto_a_this_ledger_page_is_refused(app: &mut TestAppContext) {
    let mut ui = Harness::without_ledger(app);
    ui.press("g s");

    ui.press("j j j");
    assert_eq!(
        ui.settings_frame().page,
        "About",
        "the last This device page"
    );
    ui.press("j");
    assert_eq!(ui.settings_frame().page, "About", "Ledger stays muted");
    assert!(ui.refused_without_ledger());
    ui.press("shift-g");
    assert_eq!(ui.settings_frame().page, "About");
    assert!(ui.refused_without_ledger());
}

#[gpui::test]
fn the_status_line_names_the_open_ledgers_file(app: &mut TestAppContext) {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let path = dir.path().join("ledger.pldb");
    let started = bin_desktop::start_on(&new_ledger(&path), common::today());
    let mut ui = Harness::starting(app, started, PersistedState::default());

    assert_eq!(ui.ledger_path(), Some(path.clone()));
    assert_eq!(
        ui.read(|shell| shell.ledger_label_snapshot()),
        path.display().to_string()
    );
}

#[gpui::test]
fn with_no_ledger_open_the_status_line_says_so(app: &mut TestAppContext) {
    let mut ui = Harness::without_ledger(app);

    assert_eq!(ui.ledger_path(), None);
    assert_eq!(
        ui.read(|shell| shell.ledger_label_snapshot()),
        "No ledger open…"
    );
}

#[gpui::test]
fn escape_closes_open_s_file_explorer_keeping_the_open_ledger(app: &mut TestAppContext) {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let path = dir.path().join("ledger.pldb");
    let started = bin_desktop::start_on(&new_ledger(&path), common::today());
    let mut ui = Harness::starting(app, started, PersistedState::default());
    ui.browse_from(dir.path());
    ui.press("g w");

    ui.press(": o p e n enter");
    assert_eq!(ui.read(|shell| shell.nav().mode()), InputMode::Command);
    ui.press("escape");

    assert_eq!(ui.read(|shell| shell.nav().mode()), InputMode::Normal);
    assert_eq!(ui.ledger_path(), Some(path));
    assert_eq!(ui.noun(), Noun::Bills);
    assert!(ui.toasts().visible.is_empty(), "nothing opened to say");
}
