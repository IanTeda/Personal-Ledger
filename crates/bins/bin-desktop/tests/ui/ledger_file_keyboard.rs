//! Start-up and the Ledger file, driven by the keyboard (#977): the Desktop starts with no Ledger
//! open unless On start or `file` names one, a named file that is missing or won't open leaves none
//! open and says so, and with none open every key that needs a Ledger is refused on the status line
//! while `:open`, `:new`, Help and the This device Settings pages still run. State is read back
//! from `Root`. `:open`'s file explorer picks a file by mouse alone (01-shell's "Row click selects
//! it"), so opening one is `ledger_file_mouse.rs`'s; here `:open` and `esc` run from the keys (#978).
//! `:new` names its file in a focused File name field, so creating one runs from the keys here
//! (#980): a fresh name opens the Create ledger Dialog (#1068), which asks for the base Unit and
//! writes nothing until Create, then creates the Ledger with what every new Ledger starts with
//! (#1067) and opens it; Esc leaves what was open and writes nothing, a
//! name already in the folder is refused
//! inline, as is one whose SQLite journal is still there, and a folder it can't write keeps the
//! Open Ledger. Start view (#983) chooses where it
//! lands: each value with a Ledger open, and with none, where a noun lands on the Dashboard.

use super::common;
use lib_database::Domain;

use bin_desktop::{
    Started,
    navigation::nav::{InputMode, Noun},
    persistence::PersistedState,
};
use common::{Harness, config_for, new_ledger};
use gpui::TestAppContext;
use lib_core::StartView;

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
fn a_named_ledger_that_opens_starts_on_it_on_the_dashboard(app: &mut TestAppContext) {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let config = new_ledger(&dir.path().join("ledger.pldb"));

    let started = bin_desktop::start_on(&config, common::today());
    assert!(matches!(started, Started::Opened { .. }));
    let mut ui = Harness::starting(app, started, remembering(Noun::Bills, "units"));

    assert!(ui.read(|shell| shell.ledger_open()));
    assert_eq!(
        ui.noun(),
        Noun::Dashboard,
        "Start view's default, whatever noun it last showed"
    );
    assert_eq!(ui.unit_codes(), ["aud", "btc", "vas"]);
}

/// The Shell started on a new Ledger file in `dir`, under Start view `start_view`, from a state
/// file that last showed `noun` on Settings' `page`.
fn landing_on<'a>(
    app: &'a mut TestAppContext,
    dir: &tempfile::TempDir,
    start_view: StartView,
    noun: Noun,
    page: &str,
) -> Harness<'a> {
    let started = bin_desktop::start_on(
        &new_ledger(&dir.path().join("ledger.pldb")),
        common::today(),
    );
    assert!(matches!(started, Started::Opened { .. }));
    starting_on(app, started, start_view, noun, page)
}

/// The Shell from `started`, under Start view `start_view`, from a state file that last showed
/// `noun` on Settings' `page`.
fn starting_on<'a>(
    app: &'a mut TestAppContext,
    started: Started,
    start_view: StartView,
    noun: Noun,
    page: &str,
) -> Harness<'a> {
    let mut config = lib_config::PersonalLedgerConfig::default();
    config.start_view = start_view;
    Harness::starting_with(app, started, remembering(noun, page), &config)
}

#[gpui::test]
fn start_view_last_lands_on_the_noun_showing_at_the_last_quit(app: &mut TestAppContext) {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let mut ui = landing_on(app, &dir, StartView::Last, Noun::Bills, "units");

    assert_eq!(ui.noun(), Noun::Bills);
}

#[gpui::test]
fn start_view_names_a_noun_to_land_on(app: &mut TestAppContext) {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let mut ui = landing_on(
        app,
        &dir,
        StartView::Noun("budgets".into()),
        Noun::Bills,
        "units",
    );

    assert_eq!(ui.noun(), Noun::Budgets, "not the remembered noun");
}

#[gpui::test]
fn start_view_settings_lands_on_the_remembered_settings_page(app: &mut TestAppContext) {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let mut ui = landing_on(app, &dir, StartView::Settings, Noun::Bills, "units");

    assert_eq!(ui.noun(), Noun::Settings);
    assert_eq!(
        ui.settings_frame().page,
        "Units",
        "the cursor on the remembered page"
    );
    assert!(!ui.settings_frame().page_focused);
    assert!(
        ui.is_drawn("settings-index-page"),
        "a start lands on the index"
    );
}

#[gpui::test]
fn start_view_naming_a_noun_the_desktop_doesnt_have_lands_on_the_dashboard(
    app: &mut TestAppContext,
) {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let mut ui = landing_on(
        app,
        &dir,
        StartView::Noun("units".into()),
        Noun::Bills,
        "units",
    );

    assert_eq!(ui.noun(), Noun::Dashboard);
}

#[gpui::test]
fn with_no_ledger_open_start_view_naming_a_noun_lands_on_the_dashboard(app: &mut TestAppContext) {
    let mut ui = starting_on(
        app,
        Started::NoLedger,
        StartView::Noun("bills".into()),
        Noun::Budgets,
        "units",
    );

    assert_eq!(ui.noun(), Noun::Dashboard);
    assert!(ui.is_drawn("no-ledger-open"), "its no Ledger open content");
}

#[gpui::test]
fn with_no_ledger_open_start_view_last_lands_on_the_dashboard_for_a_noun(app: &mut TestAppContext) {
    let mut ui = starting_on(
        app,
        Started::NoLedger,
        StartView::Last,
        Noun::Bills,
        "units",
    );

    assert_eq!(ui.noun(), Noun::Dashboard);
    assert!(ui.is_drawn("no-ledger-open"));
}

#[gpui::test]
fn with_no_ledger_open_start_view_settings_lands_on_the_remembered_this_device_page(
    app: &mut TestAppContext,
) {
    let mut ui = starting_on(
        app,
        Started::NoLedger,
        StartView::Settings,
        Noun::Bills,
        "display",
    );

    assert_eq!(ui.noun(), Noun::Settings);
    assert_eq!(ui.settings_frame().page, "Display");
}

#[gpui::test]
fn with_no_ledger_open_start_view_settings_lands_on_general_for_a_this_ledger_page(
    app: &mut TestAppContext,
) {
    let mut ui = starting_on(
        app,
        Started::NoLedger,
        StartView::Settings,
        Noun::Bills,
        "units",
    );

    assert_eq!(ui.noun(), Noun::Settings);
    assert_eq!(ui.settings_frame().page, "General");
    assert!(!ui.refused_without_ledger(), "landing is not a refusal");
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

/// `:new`'s File name field: the name typed, and the file it clashes with, if any; `None` while
/// `:new`'s explorer isn't open.
fn file_name_field(ui: &mut Harness<'_>) -> Option<(String, Option<String>)> {
    assert!(ui.is_drawn("explorer-file-name"), "the File name field");
    ui.read(|shell| shell.explorer_file_name_snapshot())
}

#[gpui::test]
fn new_with_a_fresh_name_creates_the_ledger_file_and_opens_it(app: &mut TestAppContext) {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let mut ui = Harness::without_ledger(app);
    ui.browse_from(dir.path());

    ui.press(": n e w enter");
    ui.press("t e d a 2 0 2 7");
    assert_eq!(
        file_name_field(&mut ui),
        Some(("teda2027".to_string(), None))
    );
    ui.press("enter");

    let created = dir.path().join("teda2027.pldb");
    assert_eq!(ui.dialog(), Some("CreateLedger"), "the Dialog asks first");
    assert_eq!(
        ui.dialog_value("file").as_deref(),
        Some("teda2027.pldb"),
        ".pldb is added and shown, read-only"
    );
    assert_eq!(ui.dialog_value("code").as_deref(), Some("usd"));
    assert_eq!(ui.dialog_value("name").as_deref(), Some("US Dollar"));
    assert_eq!(ui.dialog_value("kind").as_deref(), Some("Currency"));
    assert!(!created.exists(), "nothing is written until Create");
    assert!(!ui.read(|shell| shell.ledger_open()));
    ui.press("enter");

    assert_eq!(ui.dialog(), None);
    assert!(created.is_file(), "the file written");
    assert_eq!(ui.ledger_path(), Some(created));
    assert_eq!(ui.noun(), Noun::Dashboard);
    assert_eq!(ui.read(|shell| shell.nav().mode()), InputMode::Normal);
    assert_eq!(
        ui.unit_codes(),
        ["usd"],
        "the Locale's Currency is the only Unit"
    );
    assert!(ui.dashboard_bills().is_empty(), "none of the sample rows");
    let toast = ui.toasts().visible.pop().expect("a Toast says so");
    assert_eq!(toast.kind, "Success");
    assert_eq!(toast.text, "Created ledger teda2027.pldb");
}

#[gpui::test]
fn new_refuses_a_name_already_in_the_folder_inline_and_writes_nothing(app: &mut TestAppContext) {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let existing = dir.path().join("teda.pldb");
    std::fs::write(&existing, "someone's file").expect("a writable file");
    let mut ui = Harness::without_ledger(app);
    ui.browse_from(dir.path());

    ui.press(": n e w enter");
    ui.press("t e d a");
    assert_eq!(
        file_name_field(&mut ui),
        Some(("teda".to_string(), Some("teda.pldb".to_string()))),
        "refused inline"
    );
    assert!(ui.is_drawn("explorer-name-taken"));
    ui.press("enter");

    assert_eq!(
        ui.read(|shell| shell.nav().mode()),
        InputMode::Command,
        "the explorer stays open"
    );
    assert!(!ui.read(|shell| shell.ledger_open()));
    assert_eq!(
        std::fs::read_to_string(&existing).ok().as_deref(),
        Some("someone's file"),
        "the file there is left alone"
    );

    ui.press("backspace backspace backspace backspace e m m a");
    assert_eq!(
        file_name_field(&mut ui),
        Some(("emma".to_string(), None)),
        "a free name is taken back"
    );
    ui.press("escape");
    assert_eq!(ui.read(|shell| shell.nav().mode()), InputMode::Normal);
    assert!(
        !dir.path().join("emma.pldb").exists(),
        "esc creates nothing"
    );
    assert!(ui.toasts().visible.is_empty());
}

#[gpui::test]
fn new_refuses_a_name_whose_journal_is_still_in_the_folder(app: &mut TestAppContext) {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let journal = dir.path().join("teda.pldb-wal");
    std::fs::write(&journal, "an old Ledger's journal").expect("a writable file");
    let mut ui = Harness::without_ledger(app);
    ui.browse_from(dir.path());

    ui.press(": n e w enter");
    ui.press("t e d a");
    assert_eq!(
        file_name_field(&mut ui),
        Some(("teda".to_string(), Some("teda.pldb".to_string()))),
        "a stale journal would replay into the new Ledger"
    );
    ui.press("enter");

    assert!(!ui.read(|shell| shell.ledger_open()));
    assert!(!dir.path().join("teda.pldb").exists());
    assert!(journal.exists(), "the journal is left alone");
}

#[cfg(unix)]
#[gpui::test]
fn new_in_a_folder_it_cannot_write_keeps_the_open_ledger_and_says_why(app: &mut TestAppContext) {
    use std::os::unix::fs::PermissionsExt;

    let dir = tempfile::tempdir().expect("a temporary directory");
    let path = dir.path().join("ledger.pldb");
    let started = bin_desktop::start_on(&new_ledger(&path), common::today());
    let mut ui = Harness::starting(app, started, PersistedState::default());
    let locked = dir.path().join("locked");
    std::fs::create_dir(&locked).expect("a folder");
    std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o555))
        .expect("a read-only folder");
    ui.browse_from(&locked);
    ui.press("g w");

    ui.press(": n e w enter");
    ui.press("t e d a enter");
    assert_eq!(ui.dialog(), Some("CreateLedger"));
    ui.press("enter");
    std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o755))
        .expect("the folder writable again, for the clean-up");

    assert_eq!(ui.ledger_path(), Some(path), "the Open Ledger stays");
    assert_eq!(ui.noun(), Noun::Bills, "stays where it was");
    let toast = ui.toasts().visible.pop().expect("a Toast says why");
    assert_eq!(toast.kind, "Error");
    assert_eq!(
        toast.text,
        "Couldn't create ledger teda.pldb: it can't be written there"
    );
    assert!(!locked.join("teda.pldb").exists());
    assert_eq!(
        ui.dialog(),
        Some("CreateLedger"),
        "the Dialog stays for another try"
    );
}

#[gpui::test]
fn esc_on_the_create_ledger_dialog_writes_nothing_and_leaves_the_open_ledger(
    app: &mut TestAppContext,
) {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let path = dir.path().join("ledger.pldb");
    let started = bin_desktop::start_on(&new_ledger(&path), common::today());
    let mut ui = Harness::starting(app, started, PersistedState::default());
    ui.browse_from(dir.path());
    ui.press("g w");

    ui.press(": n e w enter");
    ui.press("t e d a enter");
    assert_eq!(ui.dialog(), Some("CreateLedger"));
    ui.press("escape");

    assert_eq!(ui.dialog(), None);
    assert!(!dir.path().join("teda.pldb").exists(), "esc writes nothing");
    assert_eq!(ui.ledger_path(), Some(path), "the Open Ledger stays");
    assert_eq!(ui.noun(), Noun::Bills);
    assert!(ui.toasts().visible.is_empty());
    assert_eq!(
        file_name_field(&mut ui),
        Some(("teda".to_string(), None)),
        "the explorer is still up for another name"
    );
}

#[gpui::test]
fn create_ledger_with_a_changed_base_unit_makes_it_the_new_ledgers_base(app: &mut TestAppContext) {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let mut ui = Harness::without_ledger(app);
    ui.browse_from(dir.path());

    ui.press(": n e w enter");
    ui.press("t e d a enter");
    ui.press("backspace backspace backspace b t c");
    ui.press("tab down");
    assert_eq!(ui.dialog_value("field").as_deref(), Some("Type"));
    assert_eq!(ui.dialog_value("kind").as_deref(), Some("Crypto"));
    assert_eq!(
        ui.dialog_value("precision").as_deref(),
        Some("8"),
        "Precision follows the Type until typed"
    );
    ui.press("tab");
    ui.press(&"backspace ".repeat("US Dollar".len()));
    ui.press("B i t c o i n");
    assert_eq!(ui.dialog_value("name").as_deref(), Some("Bitcoin"));
    ui.press("enter");

    assert_eq!(ui.dialog(), None);
    assert_eq!(ui.noun(), Noun::Dashboard);
    assert_eq!(ui.base_unit_code().as_deref(), Some("btc"));
    assert_eq!(
        ui.unit_rows(),
        [(
            "btc".to_string(),
            "Bitcoin".to_string(),
            "Crypto".to_string()
        )]
    );
}

#[gpui::test]
fn create_is_off_while_a_field_is_invalid_and_enter_writes_nothing(app: &mut TestAppContext) {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let mut ui = Harness::without_ledger(app);
    ui.browse_from(dir.path());

    ui.press(": n e w enter");
    ui.press("t e d a enter");
    ui.press("backspace backspace backspace");
    assert_eq!(ui.dialog_value("code").as_deref(), Some(""));
    assert_eq!(ui.dialog_value("can_confirm").as_deref(), Some("false"));
    ui.press("enter");

    assert_eq!(ui.dialog(), Some("CreateLedger"), "Create is refused");
    assert!(!dir.path().join("teda.pldb").exists());
    ui.press("u s d");
    assert_eq!(ui.dialog_value("can_confirm").as_deref(), Some("true"));
}

/// The Shell started on a new Ledger file `ledger.pldb` in `dir`.
fn started_on_new<'a>(app: &'a mut TestAppContext, dir: &tempfile::TempDir) -> Harness<'a> {
    let started = bin_desktop::start_on(
        &new_ledger(&dir.path().join("ledger.pldb")),
        common::today(),
    );
    assert!(matches!(started, Started::Opened { .. }));
    Harness::starting(app, started, PersistedState::default())
}

#[gpui::test]
fn close_drops_the_ledger_and_lands_on_the_dashboard_with_none_open(app: &mut TestAppContext) {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let mut ui = started_on_new(app, &dir);
    ui.press("g w");
    assert_eq!(ui.noun(), Noun::Bills);

    ui.press(": c l o s e enter");

    assert!(!ui.read(|shell| shell.ledger_open()));
    assert_eq!(ui.ledger_path(), None);
    assert_eq!(ui.noun(), Noun::Dashboard);
    assert_eq!(ui.read(|shell| shell.nav().mode()), InputMode::Normal);
    assert_eq!(
        ui.read(|shell| shell.ledger_label_snapshot()),
        "No ledger open…"
    );
    ui.press("g w");
    assert_eq!(ui.noun(), Noun::Dashboard, "Bills is muted again");
    assert!(ui.refused_without_ledger());
}

#[gpui::test]
fn close_moves_focus_off_the_context_rail(app: &mut TestAppContext) {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let mut ui = started_on_new(app, &dir);
    ui.press("g w");
    ui.press("ctrl-w l");
    ui.press(": c l o s e enter");

    assert_ne!(
        ui.read(|shell| shell.nav().focus()),
        bin_desktop::navigation::nav::FocusZone::ContextRail
    );
}

#[gpui::test]
fn close_with_none_open_is_refused_like_any_ledger_command(app: &mut TestAppContext) {
    let mut ui = Harness::without_ledger(app);

    ui.press(": c l o s e enter");

    assert!(ui.refused_without_ledger());
    assert!(!ui.read(|shell| shell.ledger_open()));
}

#[gpui::test]
fn close_flushes_the_last_writes_into_the_file(app: &mut TestAppContext) {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let path = dir.path().join("ledger.pldb");
    let mut ui = started_on_new(app, &dir);
    ui.press("g s");
    ui.click("settings-index-Tags");
    ui.press("l");
    ui.press("n h o l i d a y enter");
    ui.press(": c l o s e enter");
    drop(ui);

    let reopened = bin_desktop::start_on(&config_for(&path), common::today());
    let Started::Opened { storage, .. } = reopened else {
        panic!("the closed Ledger file opens again");
    };
    let names: Vec<String> = storage
        .tags
        .tags()
        .tags()
        .into_iter()
        .map(|tag| tag.name)
        .collect();
    assert!(names.iter().any(|name| name == "holiday"), "{names:?}");
}

#[gpui::test]
fn quitting_after_close_remembers_no_last_ledger(app: &mut TestAppContext) {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let mut ui = started_on_new(app, &dir);
    assert!(!ui.read(|shell| shell.last_ledger_forgotten()));

    ui.press(": c l o s e enter");

    assert!(ui.read(|shell| shell.last_ledger_forgotten()));
}

#[gpui::test]
fn a_write_that_failed_just_before_close_is_reported(app: &mut TestAppContext) {
    let mut ui = Harness::arrange(app, |fakes| fakes.fails.writes = vec![Domain::Documents]);
    ui.press("g s");
    ui.press("j j j j j j j j j j j");
    ui.press("l");
    ui.press("n l e a s e s enter");

    ui.press(": c l o s e enter");

    let toast = ui.toasts().visible.pop().expect("an Error Toast");
    assert_eq!(toast.kind, "Error");
    assert!(toast.text.contains("wasn't saved"), "{}", toast.text);
    assert!(!ui.read(|shell| shell.ledger_open()));
}
