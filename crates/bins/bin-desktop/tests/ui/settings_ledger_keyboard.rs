//! The Settings View's own keys on the index and the Ledger page: the index's ends, `/` with no
//! Search, `Esc` on the index, and the page's fields: the walk, the Ledger name and Owner edited in
//! place, and the Financial year starts and Base unit selects. State is read back from `Root`.

use super::common;
use bin_desktop::shell::router::SettingsLedgerSnapshot;

use bin_desktop::navigation::nav::{InputMode, Noun};
use common::Harness;
use gpui::TestAppContext;

fn on_settings(app: &mut TestAppContext) -> Harness<'_> {
    let mut ui = Harness::new(app);
    ui.press("g s");
    assert_eq!(ui.noun(), Noun::Settings);
    ui
}

/// Settings on the Ledger page, the first of THIS LEDGER, with focus left on the index.
fn on_ledger(app: &mut TestAppContext) -> Harness<'_> {
    let mut ui = on_settings(app);
    ui.press("j j j j");
    assert_eq!(ui.settings_frame().page, "Ledger");
    ui
}

#[gpui::test]
fn shift_g_and_g_g_jump_to_the_index_ends(app: &mut TestAppContext) {
    let mut ui = on_settings(app);

    ui.press("shift-g");
    assert_eq!(ui.settings_frame().page, "DataBackup");
    ui.press("g g");
    assert_eq!(ui.settings_frame().page, "General");
}

#[gpui::test]
fn slash_is_inert_on_settings(app: &mut TestAppContext) {
    let mut ui = on_settings(app);

    ui.press("/");
    assert_eq!(ui.read(|shell| shell.nav().mode()), InputMode::Normal);
    assert_eq!(ui.settings_frame().page, "General");
}

#[gpui::test]
fn escape_on_the_index_stays_on_settings(app: &mut TestAppContext) {
    let mut ui = on_settings(app);

    ui.press("l");
    assert!(ui.settings_frame().page_focused);
    ui.press("escape escape");
    let page = ui.settings_frame();
    assert!(!page.page_focused);
    assert_eq!(page.page, "General");
    assert_eq!(ui.noun(), Noun::Settings);
}

#[gpui::test]
fn the_base_unit_select_starts_on_aud_and_lists_every_unit(app: &mut TestAppContext) {
    let mut ui = on_ledger(app);

    let general = ui.settings::<SettingsLedgerSnapshot>();
    assert_eq!(general.base_unit.as_deref(), Some("aud"));
    assert_eq!(general.options, ["aud", "btc", "vas"]);
}

#[gpui::test]
fn j_on_the_index_changes_the_page_and_not_the_base_unit(app: &mut TestAppContext) {
    let mut ui = on_ledger(app);

    ui.press("j");
    assert_eq!(ui.settings_frame().page, "Units");
    assert_eq!(
        ui.settings::<SettingsLedgerSnapshot>().base_unit.as_deref(),
        Some("aud")
    );
}

#[gpui::test]
fn a_base_unit_set_by_key_moves_the_units_pages_glyph(app: &mut TestAppContext) {
    let mut ui = on_ledger(app);

    ui.press("l j j j enter j enter");
    ui.press("escape");
    ui.press("j");
    assert_eq!(ui.settings_frame().page, "Units");
    assert_eq!(ui.base_unit_code().as_deref(), Some("btc"));
}

#[gpui::test]
fn a_deactivated_unit_is_not_offered_as_the_base_unit(app: &mut TestAppContext) {
    let mut ui = on_ledger(app);
    assert!(
        ui.settings::<SettingsLedgerSnapshot>()
            .options
            .contains(&"btc".to_string())
    );

    ui.press("j");
    ui.press("l");
    ui.press("j");
    ui.press("d");
    // A Unit in use opens its Delete dialog as Deactivate.
    ui.press("enter");
    ui.press("escape");
    ui.press("k");

    let general = ui.settings::<SettingsLedgerSnapshot>();
    assert!(!general.options.contains(&"btc".to_string()));
    assert_eq!(general.base_unit.as_deref(), Some("aud"));
}

#[gpui::test]
fn u_undoes_the_base_unit_and_ctrl_r_redoes_it_saying_so(app: &mut TestAppContext) {
    let mut ui = on_ledger(app);
    ui.press("l j j j enter j enter");
    let base = |ui: &mut Harness<'_>| ui.settings::<SettingsLedgerSnapshot>().base_unit;
    assert_eq!(base(&mut ui).as_deref(), Some("btc"));

    ui.press("u");
    assert_eq!(base(&mut ui).as_deref(), Some("aud"));
    assert_eq!(
        ui.read(|shell| shell.status_message()).as_deref(),
        Some("Undid Base unit: btc")
    );

    ui.press("ctrl-r");
    assert_eq!(base(&mut ui).as_deref(), Some("btc"));
}

fn ledger(ui: &mut Harness<'_>) -> SettingsLedgerSnapshot {
    ui.settings::<SettingsLedgerSnapshot>()
}

fn mode(ui: &mut Harness<'_>) -> InputMode {
    ui.read(|shell| shell.nav().mode())
}

fn status(ui: &mut Harness<'_>) -> Option<String> {
    ui.read(|shell| shell.status_message())
}

/// `text` as the keystrokes that type it, a space as `space`.
fn keys(text: &str) -> String {
    text.chars()
        .map(|ch| match ch {
            ' ' => "space".to_string(),
            ch if ch.is_ascii_uppercase() => format!("shift-{}", ch.to_ascii_lowercase()),
            ch => ch.to_string(),
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Settings › Ledger with the keyboard on the page, editing the Ledger name with its draft cleared.
fn editing_a_cleared_name(app: &mut TestAppContext) -> Harness<'_> {
    let mut ui = on_ledger(app);
    ui.press("l enter");
    let backspaces = vec!["backspace"; ledger(&mut ui).name.chars().count()].join(" ");
    ui.press(&backspaces);
    assert_eq!(ledger(&mut ui).name, "");
    ui
}

#[gpui::test]
fn arriving_focuses_the_name_and_j_k_walk_the_fields_stopping_at_the_ends(
    app: &mut TestAppContext,
) {
    let mut ui = on_ledger(app);

    ui.press("l");
    assert_eq!(ledger(&mut ui).focused, Some("name"));
    ui.press("k");
    assert_eq!(ledger(&mut ui).focused, Some("name"));
    ui.press("j");
    assert_eq!(ledger(&mut ui).focused, Some("owner"));
    ui.press("j");
    assert_eq!(ledger(&mut ui).focused, Some("financial_year"));
    ui.press("j j");
    assert_eq!(ledger(&mut ui).focused, Some("base_unit"));
    assert_eq!(
        ledger(&mut ui).base_unit.as_deref(),
        Some("aud"),
        "j/k no longer change the base unit"
    );

    ui.press("escape");
    assert!(
        !ui.settings_frame().page_focused,
        "esc goes back to the index"
    );
    assert_eq!(ui.settings_frame().page, "Ledger");
}

#[gpui::test]
fn enter_edits_the_name_in_insert_mode_and_enter_saves_it_as_one_entry(app: &mut TestAppContext) {
    let mut ui = editing_a_cleared_name(app);
    assert_eq!(mode(&mut ui), InputMode::Insert);
    assert!(ledger(&mut ui).editing);

    ui.press(&keys(" Teda household "));
    let before = ui.commits();
    ui.press("enter");

    let page = ledger(&mut ui);
    assert_eq!(page.name, "Teda household", "the draft is trimmed");
    assert!(!page.editing);
    assert_eq!(mode(&mut ui), InputMode::Normal);
    assert_eq!(ui.commits(), before + 1, "the save is one input");

    ui.press("u");
    assert_eq!(ledger(&mut ui).name, "Sample Ledger");
    assert_eq!(
        status(&mut ui).as_deref(),
        Some("Undid Ledger name changed to Teda household")
    );
    ui.press("ctrl-r");
    assert_eq!(ledger(&mut ui).name, "Teda household");
}

#[gpui::test]
fn a_typed_key_on_a_text_field_starts_its_edit_and_esc_discards_it(app: &mut TestAppContext) {
    let mut ui = on_ledger(app);
    ui.press("l j");

    ui.press("x");
    let page = ledger(&mut ui);
    assert!(page.editing);
    assert_eq!(
        page.owner, "Sam Samplex",
        "the key is the draft's first new character"
    );
    assert_eq!(mode(&mut ui), InputMode::Insert);

    ui.press("escape");
    let page = ledger(&mut ui);
    assert!(!page.editing);
    assert_eq!(page.owner, "Sam Sample");
    assert_eq!(mode(&mut ui), InputMode::Normal);
    assert!(
        ui.settings_frame().page_focused,
        "the first esc only discards"
    );
}

#[gpui::test]
fn while_editing_u_and_j_are_text(app: &mut TestAppContext) {
    let mut ui = on_ledger(app);
    ui.press("l enter u j k");
    assert_eq!(ledger(&mut ui).name, "Sample Ledgerujk");

    ui.press("enter");
    assert_eq!(ledger(&mut ui).name, "Sample Ledgerujk");
    assert_eq!(ledger(&mut ui).focused, Some("name"));
}

#[gpui::test]
fn tab_saves_and_moves_to_the_next_field(app: &mut TestAppContext) {
    let mut ui = on_ledger(app);
    ui.press("l enter s tab");

    let page = ledger(&mut ui);
    assert_eq!(page.name, "Sample Ledgers");
    assert_eq!(page.focused, Some("owner"));
    assert_eq!(mode(&mut ui), InputMode::Normal);
}

#[gpui::test]
fn saving_what_is_stored_writes_nothing(app: &mut TestAppContext) {
    let mut ui = on_ledger(app);

    ui.press("l enter space enter");
    assert_eq!(ledger(&mut ui).name, "Sample Ledger");

    ui.press("u");
    assert_ne!(
        status(&mut ui).as_deref(),
        Some("Undid Ledger name changed to Sample Ledger"),
        "no History entry"
    );
}

#[gpui::test]
fn a_blank_name_is_refused_and_leaving_keeps_the_stored_one(app: &mut TestAppContext) {
    let mut ui = editing_a_cleared_name(app);
    ui.press("space enter");

    let page = ledger(&mut ui);
    assert!(page.editing && page.refused, "the field stays open");
    assert_eq!(page.name, " ", "with the draft");
    assert_eq!(status(&mut ui).as_deref(), Some("A Ledger needs a name"));
    assert_eq!(mode(&mut ui), InputMode::Insert);

    ui.click("primary-rail-Dashboard");
    assert_eq!(ui.noun(), Noun::Dashboard);
    assert_eq!(
        status(&mut ui).as_deref(),
        Some("Ledger name kept: it can't be blank")
    );
    ui.press("g s");
    assert_eq!(ledger(&mut ui).name, "Sample Ledger");
}

#[gpui::test]
fn a_name_over_sixty_characters_is_refused_keeping_the_text(app: &mut TestAppContext) {
    let mut ui = editing_a_cleared_name(app);
    ui.press(&keys(&"x".repeat(61)));
    ui.press("enter");

    let page = ledger(&mut ui);
    assert!(page.editing && page.refused);
    assert_eq!(page.name, "x".repeat(61));
    assert_eq!(
        status(&mut ui).as_deref(),
        Some("A ledger name is at most 60 characters")
    );
}

#[gpui::test]
fn an_empty_owner_saves_as_owner_cleared(app: &mut TestAppContext) {
    let mut ui = on_ledger(app);
    ui.press("l j enter");
    let backspaces = vec!["backspace"; ledger(&mut ui).owner.chars().count()].join(" ");
    ui.press(&backspaces);
    ui.press("enter");
    assert_eq!(ledger(&mut ui).owner, "");

    ui.press("u");
    assert_eq!(ledger(&mut ui).owner, "Sam Sample");
    assert_eq!(status(&mut ui).as_deref(), Some("Undid Owner cleared"));
}

#[gpui::test]
fn a_g_jump_away_saves_an_open_list(app: &mut TestAppContext) {
    let mut ui = on_ledger(app);
    // While editing `g` is text, so the jump is made from an open list, whose highlight it saves.
    ui.press("l j j enter j");
    assert!(ledger(&mut ui).list_open);

    ui.press("g d");
    assert_eq!(ui.noun(), Noun::Dashboard);
    ui.press("g s");
    assert_eq!(ledger(&mut ui).financial_year.as_deref(), Some("Aug"));
}

#[gpui::test]
fn the_financial_year_list_moves_without_saving_and_enter_saves_it(app: &mut TestAppContext) {
    let mut ui = on_ledger(app);
    ui.press("l j j");
    assert_eq!(ledger(&mut ui).financial_year.as_deref(), Some("Jul"));

    ui.press("enter");
    assert!(ledger(&mut ui).list_open);
    ui.press("j j j");
    assert_eq!(
        ledger(&mut ui).financial_year.as_deref(),
        Some("Jul"),
        "moving writes nothing"
    );

    ui.press("enter");
    let page = ledger(&mut ui);
    assert!(!page.list_open);
    assert_eq!(page.financial_year.as_deref(), Some("Oct"));

    ui.press("u");
    assert_eq!(ledger(&mut ui).financial_year.as_deref(), Some("Jul"));
    assert_eq!(
        status(&mut ui).as_deref(),
        Some("Undid Financial year starts changed to Oct")
    );
    ui.press("ctrl-r");
    assert_eq!(ledger(&mut ui).financial_year.as_deref(), Some("Oct"));
}

#[gpui::test]
fn esc_collapses_the_list_without_saving_and_l_changes_nothing(app: &mut TestAppContext) {
    let mut ui = on_ledger(app);
    ui.press("l j j enter k escape");

    let page = ledger(&mut ui);
    assert!(!page.list_open);
    assert_eq!(page.financial_year.as_deref(), Some("Jul"));
    assert!(ui.settings_frame().page_focused);

    ui.press("l");
    assert_eq!(ledger(&mut ui).financial_year.as_deref(), Some("Jul"));
}

#[gpui::test]
fn this_ledger_counts_follow_an_added_account(app: &mut TestAppContext) {
    let mut ui = on_ledger(app);
    let figure = |ui: &mut Harness<'_>, label: &str| {
        ledger(ui)
            .summary
            .into_iter()
            .find(|(name, _)| name == label)
            .map(|(_, value)| value)
            .expect("the summary row")
    };
    let before: usize = figure(&mut ui, "accounts")
        .replace(',', "")
        .parse()
        .unwrap();
    assert!(before > 0, "the Sample Ledger has Accounts");

    ui.write_outside(|stores| {
        let mut store = stores.accounts.borrow_mut();
        let mut account = lib_database::AccountsStore::accounts(&*store)
            .accounts()
            .remove(0);
        account.id = lib_core::AccountId::new();
        lib_database::AccountsStore::accounts_mut(&mut *store).upsert_account(account);
    });

    assert_eq!(figure(&mut ui, "accounts"), (before + 1).to_string());
}
