//! The Ledger file, driven by clicks: with no Ledger open (#977) the muted rail and the This Ledger
//! Settings entries refuse a click on the status line and stay put, while Settings, its This device
//! pages and the Dashboard's `:open` still run; `:open`'s file explorer opens the file picked, from
//! no Ledger open or in place of another, and a file that won't open keeps whatever was open
//! (#978). `:new`'s explorer browses by mouse too: a file row is inert, a folder carries the name
//! into it and Create makes the file named there, in place of the Open Ledger (#980). Start view
//! (#983) is only where start-up lands: a file opened by click lands on the Dashboard. After
//! `:close` Back has nowhere to go, a Ledger opened by click gets the remembered Documents view,
//! and a swap reports the replaced Ledger's lost write as `:close` does. State is read back from
//! `Root`.

#![expect(
    clippy::expect_used,
    reason = "test support: a folder that can't be made or read should fail the test loudly"
)]

use super::common;

use std::path::{Path, PathBuf};

use bin_desktop::{
    Started, WindowSize,
    navigation::nav::{InputMode, Noun},
    persistence::PersistedState,
    shell::router::SettingsTagsSnapshot,
};
use common::{Harness, config_for, new_ledger};
use gpui::TestAppContext;

/// A folder to browse holding, in the explorer's order, two Ledger files, an empty `.pldb` and a
/// `.pldb` that isn't a Ledger: rows 0 to 3.
struct Folder {
    dir: tempfile::TempDir,
}

impl Folder {
    fn new() -> Self {
        let folder = Self {
            dir: tempfile::tempdir().expect("a temporary directory"),
        };
        new_ledger(&folder.household());
        new_ledger(&folder.business());
        std::fs::write(folder.empty(), "").expect("a writable file");
        std::fs::write(folder.notes(), "not a ledger").expect("a writable file");
        folder
    }

    fn path(&self) -> &Path {
        self.dir.path()
    }

    fn household(&self) -> PathBuf {
        self.path().join("a-household.pldb")
    }

    fn business(&self) -> PathBuf {
        self.path().join("b-business.pldb")
    }

    fn empty(&self) -> PathBuf {
        self.path().join("c-empty.pldb")
    }

    fn notes(&self) -> PathBuf {
        self.path().join("d-notes.pldb")
    }

    /// What the folder holds, so a test sees `:open` created nothing.
    fn entries(&self) -> usize {
        std::fs::read_dir(self.path())
            .expect("a readable folder")
            .count()
    }
}

/// The Shell as start-up leaves it on `path`, browsing `folder`.
fn started_on<'a>(app: &'a mut TestAppContext, folder: &Folder, path: &Path) -> Harness<'a> {
    let started = bin_desktop::start_on(&config_for(path), common::today());
    assert!(matches!(started, Started::Opened { .. }));
    let mut ui = Harness::starting(app, started, PersistedState::default());
    ui.browse_from(folder.path());
    ui
}

/// What the explorer lists of a [`Folder`], in order.
const ROWS: [&str; 4] = [
    "a-household.pldb",
    "b-business.pldb",
    "c-empty.pldb",
    "d-notes.pldb",
];

/// `:open` from the palette, then clicks explorer row `row` and Open.
fn open_row(ui: &mut Harness<'_>, row: usize) {
    ui.press(": o p e n enter");
    assert_eq!(
        ui.read(|shell| shell.explorer_rows_snapshot()),
        Some(ROWS.map(String::from).to_vec())
    );
    ui.click(&format!("explorer-row-{row}"));
    ui.click("explorer-open");
}

/// The newest Toast's text.
fn last_toast(ui: &mut Harness<'_>) -> String {
    ui.toasts().visible.pop().expect("a Toast").text
}

/// Settings' Tags page's Tag names.
fn tag_names(ui: &mut Harness<'_>) -> Vec<String> {
    ui.press("g s");
    ui.click("settings-index-Tags");
    let names = ui.settings::<SettingsTagsSnapshot>().tag_names;
    ui.press("g d");
    names
}

#[gpui::test]
fn a_file_opened_by_click_lands_on_the_dashboard_whatever_start_view_says(
    app: &mut TestAppContext,
) {
    let folder = Folder::new();
    let started = bin_desktop::start_on(&config_for(&folder.household()), common::today());
    let mut config = lib_config::PersonalLedgerConfig::default();
    config.start_view = lib_core::StartView::Noun("bills".into());
    let mut ui = Harness::starting_with(app, started, PersistedState::default(), &config);
    ui.browse_from(folder.path());
    assert_eq!(
        ui.noun(),
        Noun::Bills,
        "start-up lands where Start view says"
    );

    open_row(&mut ui, 1);

    assert_eq!(ui.ledger_path(), Some(folder.business()));
    assert_eq!(ui.noun(), Noun::Dashboard);
}

#[gpui::test]
fn clicking_a_rail_noun_that_needs_a_ledger_is_refused(app: &mut TestAppContext) {
    let mut ui = Harness::without_ledger(app);

    for noun in ["Transactions", "Bills", "Budgets", "Documents"] {
        ui.click(&format!("primary-rail-{noun}"));
        assert_eq!(ui.noun(), Noun::Dashboard, "{noun} stays muted");
        assert!(ui.refused_without_ledger(), "{noun} says why");
    }
}

#[gpui::test]
fn clicking_settings_on_the_rail_opens_its_this_device_pages(app: &mut TestAppContext) {
    let mut ui = Harness::without_ledger(app);

    ui.click("primary-rail-Settings");
    assert_eq!(ui.noun(), Noun::Settings);
    assert_eq!(ui.settings_frame().page, "General");
    ui.click("settings-index-Tracing");
    assert_eq!(ui.settings_frame().page, "Tracing");
}

#[gpui::test]
fn clicking_a_this_ledger_settings_entry_is_refused(app: &mut TestAppContext) {
    let mut ui = Harness::without_ledger(app);
    ui.click("primary-rail-Settings");

    ui.click("settings-index-Units");
    assert_eq!(
        ui.settings_frame().page,
        "General",
        "the page on show stays"
    );
    assert!(ui.refused_without_ledger());
}

#[gpui::test]
fn picking_a_this_ledger_page_in_the_medium_picker_is_refused(app: &mut TestAppContext) {
    let mut ui = Harness::without_ledger(app);
    ui.resize_to(WindowSize::Medium);
    ui.click("primary-rail-collapsed-Settings");
    ui.click("settings-index-page-Display");
    assert_eq!(ui.settings_frame().page, "Display");

    ui.click("settings-crumb-picker");
    ui.click("settings-picker-Accounts");
    let frame = ui.settings_frame();
    assert_eq!(frame.page, "Display");
    assert!(frame.picker_chosen.is_some(), "the picker stays open");
    assert!(ui.refused_without_ledger());
}

#[gpui::test]
fn clicking_open_on_the_dashboard_opens_the_file_explorer(app: &mut TestAppContext) {
    let mut ui = Harness::without_ledger(app);

    ui.click("no-ledger-open");
    assert_eq!(
        ui.read(|shell| shell.nav().mode()),
        InputMode::Command,
        "the file explorer is open"
    );
    assert!(!ui.refused_without_ledger());
}

#[gpui::test]
fn opening_a_ledger_file_with_none_open_lands_on_its_dashboard(app: &mut TestAppContext) {
    let folder = Folder::new();
    let mut ui = Harness::without_ledger(app);
    ui.browse_from(folder.path());

    ui.click("no-ledger-open");
    ui.click("explorer-row-0");
    ui.click("explorer-open");

    assert!(ui.read(|shell| shell.ledger_open()));
    assert_eq!(ui.ledger_path(), Some(folder.household()));
    assert_eq!(ui.noun(), Noun::Dashboard);
    assert_eq!(ui.read(|shell| shell.nav().mode()), InputMode::Normal);
    assert_eq!(last_toast(&mut ui), "Opened ledger a-household.pldb");
    assert_eq!(
        ui.read(|shell| shell.ledger_label_snapshot()),
        folder.household().display().to_string(),
        "the status line names the file, outside the home folder in full"
    );
}

#[gpui::test]
fn double_clicking_a_ledger_file_swaps_it_in_for_the_open_ledger(app: &mut TestAppContext) {
    let folder = Folder::new();
    let mut ui = started_on(app, &folder, &folder.household());
    ui.press("g w");
    assert_eq!(ui.noun(), Noun::Bills);

    ui.press(": o p e n enter");
    ui.click("explorer-row-1");
    ui.double_click("explorer-row-1");

    assert_eq!(ui.ledger_path(), Some(folder.business()));
    assert_eq!(ui.noun(), Noun::Dashboard, "lands on the Dashboard");
    assert_eq!(last_toast(&mut ui), "Opened ledger b-business.pldb");
}

#[gpui::test]
fn swapping_ledgers_keeps_the_old_ledgers_writes(app: &mut TestAppContext) {
    let folder = Folder::new();
    let mut ui = started_on(app, &folder, &folder.household());
    ui.press("g s");
    ui.click("settings-index-Tags");
    ui.press("l");
    ui.press("n h o l i d a y enter");
    assert_eq!(ui.dialog(), None, "the Add Dialog confirmed");
    ui.press("g d");
    assert!(tag_names(&mut ui).iter().any(|name| name == "holiday"));

    open_row(&mut ui, 1);
    assert_eq!(ui.ledger_path(), Some(folder.business()));
    assert!(
        !tag_names(&mut ui).iter().any(|name| name == "holiday"),
        "the other Ledger's own Tags"
    );

    open_row(&mut ui, 0);
    assert_eq!(ui.ledger_path(), Some(folder.household()));
    assert!(
        tag_names(&mut ui).iter().any(|name| name == "holiday"),
        "the Tag added before the swap reached the file"
    );
}

#[gpui::test]
fn a_file_that_will_not_open_keeps_no_ledger_open_and_says_why(app: &mut TestAppContext) {
    let folder = Folder::new();
    let entries = folder.entries();
    let mut ui = Harness::without_ledger(app);
    ui.browse_from(folder.path());

    open_row(&mut ui, 3);

    assert!(!ui.read(|shell| shell.ledger_open()));
    assert_eq!(ui.noun(), Noun::Dashboard);
    assert!(
        ui.read(|shell| shell.explorer_rows_snapshot()).is_some(),
        "the explorer stays open for another pick"
    );
    assert_eq!(ui.read(|shell| shell.nav().mode()), InputMode::Command);
    let toast = ui.toasts().visible.pop().expect("a Toast says why");
    assert_eq!(toast.kind, "Error");
    assert_eq!(
        toast.text,
        "Couldn't open ledger d-notes.pldb: it isn't a Ledger file, or can't be read"
    );
    assert_eq!(folder.entries(), entries, ":open creates no file");
}

#[gpui::test]
fn a_file_that_will_not_open_keeps_the_open_ledger(app: &mut TestAppContext) {
    let folder = Folder::new();
    let entries = folder.entries();
    let mut ui = started_on(app, &folder, &folder.household());
    ui.press("g w");

    open_row(&mut ui, 2);

    assert_eq!(ui.ledger_path(), Some(folder.household()));
    assert_eq!(ui.noun(), Noun::Bills, "stays where it was");
    assert_eq!(
        last_toast(&mut ui),
        "Couldn't open ledger c-empty.pldb: the file is empty"
    );
    assert_eq!(
        std::fs::metadata(folder.empty())
            .map(|file| file.len())
            .ok(),
        Some(0),
        "the empty file is left alone, not made a Ledger"
    );
    assert_eq!(folder.entries(), entries, ":open creates no file");
}

/// A migrated file with no base Unit, as an interrupted create leaves one, is refused, never
/// completed (ADR-0054), and the Open Ledger stays.
#[gpui::test]
fn a_file_that_is_not_a_complete_ledger_keeps_the_open_ledger_and_says_so(
    app: &mut TestAppContext,
) {
    let folder = Folder::new();
    let mut ui = started_on(app, &folder, &folder.household());
    ui.press("g w");
    let cut_short = tempfile::tempdir().expect("a temporary directory");
    let path = cut_short.path().join("cut-short.pldb");
    let (writer, _) = lib_database::Writer::open_ledger(&path).expect("a migrated file");
    writer.close().expect("the writer is running");
    drop(writer);
    ui.browse_from(cut_short.path());

    ui.press(": o p e n enter");
    ui.click("explorer-row-0");
    ui.click("explorer-open");

    assert_eq!(
        ui.ledger_path(),
        Some(folder.household()),
        "the Open Ledger stays"
    );
    assert_eq!(ui.noun(), Noun::Bills, "stays where it was");
    let toast = ui.toasts().visible.pop().expect("a Toast says why");
    assert_eq!(toast.kind, "Error");
    assert_eq!(
        toast.text,
        "Couldn't open ledger cut-short.pldb: it isn't a complete Ledger; create it again with :new"
    );
}

#[gpui::test]
fn clicking_create_makes_the_named_ledger_in_place_of_the_open_one(app: &mut TestAppContext) {
    let folder = Folder::new();
    let mut ui = started_on(app, &folder, &folder.household());
    ui.press("g w");

    ui.press(": n e w enter");
    ui.click("explorer-row-1");
    ui.press("e m m a");
    ui.click("explorer-open");

    let created = folder.path().join("emma.pldb");
    assert_eq!(ui.dialog(), Some("CreateLedger"), "the Dialog asks first");
    assert!(!created.exists(), "nothing is written until Create");
    assert_eq!(ui.ledger_path(), Some(folder.household()), "still open");
    ui.click("create-ledger-confirm");

    assert_eq!(ui.dialog(), None);
    assert_eq!(ui.ledger_path(), Some(created.clone()));
    assert_eq!(ui.noun(), Noun::Dashboard, "lands on the Dashboard");
    assert_eq!(last_toast(&mut ui), "Created ledger emma.pldb");
    assert!(created.is_file());
    assert_eq!(
        ui.unit_codes(),
        ["usd"],
        "the Locale's Currency is the only Unit"
    );
    assert!(tag_names(&mut ui).is_empty(), "none of the sample rows");
}

#[gpui::test]
fn in_new_a_file_row_click_selects_nothing_and_create_waits_for_a_free_name(
    app: &mut TestAppContext,
) {
    let folder = Folder::new();
    let entries = folder.entries();
    let mut ui = Harness::without_ledger(app);
    ui.browse_from(folder.path());

    ui.click("no-ledger-new");
    ui.click("explorer-row-0");
    ui.click("explorer-open");
    assert_eq!(
        ui.read(|shell| shell.nav().mode()),
        InputMode::Command,
        "no name, so nothing to create"
    );

    ui.press("a - h o u s e h o l d");
    assert_eq!(
        ui.read(|shell| shell.explorer_file_name_snapshot()),
        Some((
            "a-household".to_string(),
            Some("a-household.pldb".to_string())
        ))
    );
    assert!(ui.is_drawn("explorer-name-taken"));
    ui.click("explorer-open");
    assert!(!ui.read(|shell| shell.ledger_open()));
    assert_eq!(folder.entries(), entries, "nothing written");
}

#[gpui::test]
fn in_new_clicking_a_folder_creates_the_ledger_in_it(app: &mut TestAppContext) {
    let folder = Folder::new();
    let ledgers = folder.path().join("ledgers");
    std::fs::create_dir(&ledgers).expect("a folder");
    let mut ui = Harness::without_ledger(app);
    ui.browse_from(folder.path());

    ui.click("no-ledger-new");
    ui.click("explorer-row-0");
    assert_eq!(
        ui.read(|shell| shell.explorer_rows_snapshot()),
        Some(Vec::new()),
        "browsing the empty folder"
    );
    ui.press("t e d a");
    ui.click("explorer-open");
    ui.click("create-ledger-confirm");

    assert_eq!(ui.ledger_path(), Some(ledgers.join("teda.pldb")));
}

#[gpui::test]
fn cancel_on_the_create_ledger_dialog_writes_nothing_and_leaves_the_open_ledger(
    app: &mut TestAppContext,
) {
    let folder = Folder::new();
    let mut ui = started_on(app, &folder, &folder.household());
    let entries = folder.entries();

    ui.press(": n e w enter");
    ui.press("e m m a");
    ui.click("explorer-open");
    assert_eq!(ui.dialog(), Some("CreateLedger"));
    ui.click("create-ledger-cancel");

    assert_eq!(ui.dialog(), None);
    assert_eq!(ui.ledger_path(), Some(folder.household()));
    assert_eq!(folder.entries(), entries, "nothing written");
    assert!(ui.toasts().visible.is_empty());
}

#[gpui::test]
fn clicking_a_changed_base_unit_through_the_dialog_makes_it_the_new_ledgers_base(
    app: &mut TestAppContext,
) {
    let folder = Folder::new();
    let mut ui = Harness::without_ledger(app);
    ui.browse_from(folder.path());

    ui.click("no-ledger-new");
    ui.press("e m m a");
    ui.click("explorer-open");
    ui.click("create-ledger-code");
    ui.press("backspace backspace backspace e t h");
    ui.click("create-ledger-type");
    assert_eq!(ui.dialog_value("select_open").as_deref(), Some("true"));
    ui.click("create-ledger-type-option-1");
    assert_eq!(ui.dialog_value("select_open").as_deref(), Some("false"));
    assert_eq!(ui.dialog_value("kind").as_deref(), Some("Crypto"));
    ui.click("create-ledger-name");
    ui.press(&"backspace ".repeat("US Dollar".len()));
    ui.press("E t h e r");
    ui.click("create-ledger-confirm");

    assert_eq!(ui.dialog(), None);
    assert_eq!(ui.base_unit_code().as_deref(), Some("eth"));
    assert_eq!(
        ui.unit_rows(),
        [("eth".to_string(), "Ether".to_string(), "Crypto".to_string())]
    );
}

#[gpui::test]
fn create_does_nothing_while_a_field_is_invalid(app: &mut TestAppContext) {
    let folder = Folder::new();
    let entries = folder.entries();
    let mut ui = Harness::without_ledger(app);
    ui.browse_from(folder.path());

    ui.click("no-ledger-new");
    ui.press("e m m a");
    ui.click("explorer-open");
    ui.click("create-ledger-precision");
    ui.press("backspace x");
    assert_eq!(ui.dialog_value("can_confirm").as_deref(), Some("false"));
    ui.click("create-ledger-confirm");

    assert_eq!(ui.dialog(), Some("CreateLedger"));
    assert_eq!(folder.entries(), entries, "nothing written");
}

#[gpui::test]
fn after_close_the_dashboard_offers_open_and_the_rail_mutes_the_ledger_nouns(
    app: &mut TestAppContext,
) {
    let folder = Folder::new();
    let mut ui = started_on(app, &folder, &folder.household());
    ui.press(": c l o s e enter");
    assert!(!ui.read(|shell| shell.ledger_open()));

    for noun in ["Transactions", "Bills", "Budgets", "Documents"] {
        ui.click(&format!("primary-rail-{noun}"));
        assert_eq!(ui.noun(), Noun::Dashboard, "{noun} stays muted");
        assert!(ui.refused_without_ledger(), "{noun} says why");
    }
    ui.click("no-ledger-open");
    assert_eq!(
        ui.read(|shell| shell.nav().mode()),
        InputMode::Command,
        "the file explorer is open"
    );
}

#[gpui::test]
fn a_ledger_opened_by_click_after_close_is_remembered_again(app: &mut TestAppContext) {
    let folder = Folder::new();
    let mut ui = started_on(app, &folder, &folder.household());
    ui.press(": c l o s e enter");
    assert!(ui.read(|shell| shell.last_ledger_forgotten()));

    ui.click("no-ledger-open");
    ui.click("explorer-row-1");
    ui.click("explorer-open");

    assert_eq!(ui.ledger_path(), Some(folder.business()));
    assert!(!ui.read(|shell| shell.last_ledger_forgotten()));
}

#[gpui::test]
fn after_close_back_has_nowhere_to_go(app: &mut TestAppContext) {
    let folder = Folder::new();
    let mut ui = started_on(app, &folder, &folder.household());
    ui.press("g w");
    ui.press(": c l o s e enter");

    ui.click("topbar-back");

    assert_eq!(
        ui.noun(),
        Noun::Dashboard,
        "Bills was on the closed Ledger, so Back doesn't reach it"
    );
    assert!(!ui.read(|shell| shell.nav().can_go_back()));
}

#[gpui::test]
fn a_ledger_opened_by_click_gets_the_remembered_documents_view_back(app: &mut TestAppContext) {
    let folder = Folder::new();
    let persisted = PersistedState {
        documents_mode: Some("inbox".to_string()),
        documents_sort: Some("expiring".to_string()),
        ..Default::default()
    };
    let mut ui = Harness::starting(app, Started::NoLedger, persisted);
    ui.browse_from(folder.path());

    ui.click("no-ledger-open");
    ui.click("explorer-row-0");
    ui.click("explorer-open");

    let (mode, _, sort) = ui
        .read_app(|shell, cx| shell.documents_persisted(cx))
        .expect("a Ledger is open");
    assert_eq!(mode.id(), "inbox");
    assert_eq!(sort.id(), "expiring");
}

#[gpui::test]
fn a_write_that_failed_just_before_a_swap_is_reported(app: &mut TestAppContext) {
    let folder = Folder::new();
    let mut ui = Harness::with_failing_documents(app);
    ui.browse_from(folder.path());
    ui.press("g s");
    ui.press("j j j j j j j j j j j");
    ui.press("l");
    ui.press("n l e a s e s enter");

    open_row(&mut ui, 0);

    assert_eq!(ui.ledger_path(), Some(folder.household()));
    let toasts = ui.toasts().visible;
    assert!(
        toasts
            .iter()
            .any(|toast| toast.kind == "Error" && toast.text.contains("wasn't saved")),
        "{toasts:?}"
    );
}
