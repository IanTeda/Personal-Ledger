//! The Settings Units page driven by keys, on in-memory Units ports. Wiring and drawing only: a key
//! reaches the `lib-units` Presenter through the Units keymap or the Dialog host, and the page or
//! Dialog draws the view model it builds. The rules behind each key (refusals, the base Unit move,
//! Deactivate for a Unit in use, validation) are the Presenter tests' in `lib-units`.

use super::common;

use bin_desktop::{
    WindowSize,
    navigation::nav::{FocusZone, InputMode, Noun},
};
use common::Harness;
use gpui::TestAppContext;
use lib_toast::{Notice, ToastKind};

/// The page at Large with keyboard focus on it.
fn on_units(app: &mut TestAppContext) -> Harness<'_> {
    on_units_at(app, WindowSize::Large)
}

fn on_units_at(app: &mut TestAppContext, size: WindowSize) -> Harness<'_> {
    let mut ui = Harness::new(app);
    ui.resize_to(size);
    to_units(&mut ui);
    ui
}

/// Takes `ui` to the Units page, with keyboard focus on it.
fn to_units(ui: &mut Harness<'_>) {
    ui.press("g s");
    assert_eq!(ui.noun(), Noun::Settings);
    ui.press("j j");
    assert_eq!(ui.settings().page, "Units");
    ui.press("l");
    assert!(ui.settings().page_focused);
}

/// The selected row: a Unit's code or a Price Source's name.
fn selected(ui: &mut Harness<'_>) -> String {
    let page = ui.settings_units();
    page.units
        .iter()
        .find(|unit| unit.selected)
        .map(|unit| unit.code.clone())
        .or_else(|| {
            page.price_sources
                .iter()
                .find(|source| source.selected)
                .map(|source| source.name.clone())
        })
        .unwrap_or_default()
}

fn status_message(ui: &mut Harness<'_>) -> Option<String> {
    ui.read(|shell| shell.status_message())
}

fn type_word(ui: &mut Harness<'_>, word: &str) {
    let keys: Vec<String> = word.chars().map(|ch| ch.to_string()).collect();
    ui.press(&keys.join(" "));
}

#[gpui::test]
fn j_j_from_the_index_swaps_to_units_and_l_steps_into_it(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);
    ui.press("g s j j");
    assert_eq!(ui.settings().page, "Units");
    assert!(!ui.settings().page_focused);

    ui.press("l");
    assert!(ui.settings().page_focused);
    assert_eq!(ui.dialog(), None);
    assert_eq!(ui.read(|shell| shell.nav().mode()), InputMode::Normal);
}

#[gpui::test]
fn the_movement_keys_reach_the_presenter_and_walk_both_tables(app: &mut TestAppContext) {
    let mut ui = on_units(app);
    assert_eq!(selected(&mut ui), "aud");

    ui.press("j");
    assert_eq!(selected(&mut ui), "btc");
    ui.press("tab");
    assert_eq!(
        selected(&mut ui),
        "CoinGecko",
        "tab jumps to the other table"
    );
    ui.press("G");
    assert_eq!(selected(&mut ui), "Yahoo Finance");
    ui.press("g g");
    assert_eq!(
        selected(&mut ui),
        "aud",
        "g g passes the Shell's `g` on to the keymap's move_first"
    );
    ui.press("down");
    assert_eq!(selected(&mut ui), "btc", "the arrows too");
}

#[gpui::test]
fn home_end_and_the_page_keys_move_through_the_units_keymap(app: &mut TestAppContext) {
    let mut ui = on_units(app);

    ui.press("end");
    assert_eq!(selected(&mut ui), "Yahoo Finance");
    ui.press("home");
    assert_eq!(selected(&mut ui), "aud");
    ui.press("pagedown");
    assert_ne!(selected(&mut ui), "aud");
    ui.press("pageup");
    assert_eq!(selected(&mut ui), "aud");
}

#[gpui::test]
fn an_override_of_arrow_down_moves_it_and_frees_down(app: &mut TestAppContext) {
    let mut ui = Harness::with_keybindings(app, common::overriding("arrow_down", "ctrl+n"));
    ui.resize_to(WindowSize::Large);
    to_units(&mut ui);

    ui.press("ctrl-n");
    assert_eq!(selected(&mut ui), "btc");
    ui.press("down");
    assert_eq!(selected(&mut ui), "btc", "down is no longer bound");
}

#[gpui::test]
fn capital_b_opens_set_base_unit(app: &mut TestAppContext) {
    let mut ui = on_units(app);

    ui.press("B");

    assert_eq!(ui.dialog(), Some("SetBaseUnit"));
}

#[gpui::test]
fn shift_tab_and_b_stay_the_shells(app: &mut TestAppContext) {
    let mut ui = on_units(app);
    ui.press("b");
    assert_eq!(ui.dialog(), None, "b toggles the rail, not Set base unit");

    ui.press("shift-tab");
    assert_ne!(ui.read(|shell| shell.nav().focus()), FocusZone::View);
    assert_eq!(selected(&mut ui), "aud");
}

#[gpui::test]
fn the_status_line_draws_the_view_models_hints(app: &mut TestAppContext) {
    let mut ui = on_units(app);
    let hints = |ui: &mut Harness<'_>| -> Vec<(String, String)> {
        ui.status()
            .map(|status| status.hints)
            .unwrap_or_default()
            .into_iter()
            .take(2)
            .collect()
    };
    let pairs = |list: &[(&str, &str)]| -> Vec<(String, String)> {
        list.iter()
            .map(|(key, action)| (key.to_string(), action.to_string()))
            .collect()
    };

    assert_eq!(hints(&mut ui), pairs(&[("j/k", "unit"), ("e", "edit")]));
    ui.press("tab");
    assert_eq!(
        hints(&mut ui),
        pairs(&[("j/k", "price source"), ("e", "edit")])
    );
}

#[gpui::test]
fn n_e_and_d_open_the_selected_tables_dialogs_and_escape_closes_them(app: &mut TestAppContext) {
    let mut ui = on_units(app);

    ui.press("n");
    assert_eq!(ui.dialog(), Some("AddUnit"));
    ui.press("escape");
    ui.press("j e");
    assert_eq!(ui.dialog(), Some("EditUnit"));
    assert_eq!(ui.dialog_value("code").as_deref(), Some("btc"));
    ui.press("escape");
    ui.press("d");
    assert_eq!(ui.dialog(), Some("DeleteUnit"));
    ui.press("escape");

    ui.press("tab");
    ui.press("n");
    assert_eq!(ui.dialog(), Some("AddPriceSource"));
    ui.press("escape");
    ui.press("e");
    assert_eq!(ui.dialog(), Some("EditBuiltInPriceSource"));
    ui.press("escape");
    assert_eq!(ui.dialog(), None);
    assert_eq!(
        ui.unit_codes(),
        ["aud", "btc", "vas"],
        "nothing was written"
    );
}

#[gpui::test]
fn typing_tab_and_enter_fill_and_save_an_add_unit_dialog(app: &mut TestAppContext) {
    let mut ui = on_units(app);
    ui.press("n");
    assert_eq!(ui.dialog_value("field").as_deref(), Some("Code"));
    assert_eq!(ui.dialog_value("can_confirm").as_deref(), Some("false"));

    type_word(&mut ui, "eth");
    ui.press("backspace h");
    ui.press("tab");
    assert_eq!(ui.dialog_value("field").as_deref(), Some("Type"));
    ui.press("down");
    assert_eq!(ui.dialog_value("kind").as_deref(), Some("Crypto"));
    ui.press("tab");
    type_word(&mut ui, "Ether");
    assert_eq!(ui.dialog_value("code").as_deref(), Some("eth"));
    ui.press("enter");

    assert_eq!(ui.dialog(), None);
    assert_eq!(
        ui.unit_rows().last(),
        Some(&("eth".to_string(), "Ether".to_string(), "Crypto".to_string()))
    );
}

#[gpui::test]
fn a_select_opens_on_enter_moves_on_down_and_commits_on_enter(app: &mut TestAppContext) {
    let mut ui = on_units(app);
    ui.press("n tab");
    ui.press("enter");
    assert_eq!(ui.dialog_value("select_open").as_deref(), Some("true"));
    ui.press("down enter");
    assert_eq!(ui.dialog_value("select_open").as_deref(), Some("false"));
    assert_eq!(ui.dialog_value("kind").as_deref(), Some("Crypto"));

    ui.press("enter");
    ui.press("escape");
    assert_eq!(
        ui.dialog(),
        Some("AddUnit"),
        "the first escape closes the list only"
    );
}

#[gpui::test]
fn a_unit_in_use_opens_delete_as_deactivate_with_its_counts_and_enter_deactivates(
    app: &mut TestAppContext,
) {
    let mut ui = on_units(app);
    ui.press("j d");
    assert_eq!(ui.dialog(), Some("DeleteUnit"));
    assert_eq!(ui.dialog_value("in_use").as_deref(), Some("true"));
    assert!(ui.is_drawn("delete-unit-in-use"));
    assert!(
        ui.dialog_value("in_use_text")
            .unwrap_or_default()
            .starts_with("btc is used by "),
    );
    assert!(!ui.is_drawn("delete-unit-base-note"));

    ui.press("enter");
    assert_eq!(ui.dialog(), None);
    assert_eq!(ui.inactive_unit_codes(), ["btc"]);
}

#[gpui::test]
fn the_base_unit_in_use_opens_delete_with_deactivate_disabled_and_enter_keeps_it_open(
    app: &mut TestAppContext,
) {
    let mut ui = on_units(app);
    ui.press("d");
    assert_eq!(ui.dialog_value("in_use").as_deref(), Some("true"));
    assert!(ui.is_drawn("delete-unit-base-note"));

    ui.press("enter");
    assert_eq!(ui.dialog(), Some("DeleteUnit"));
    assert_eq!(ui.inactive_unit_codes(), Vec::<String>::new());
}

#[gpui::test]
fn edit_unit_draws_the_real_usage_line(app: &mut TestAppContext) {
    let mut ui = on_units(app);
    ui.press("j e");
    assert_eq!(ui.dialog(), Some("EditUnit"));
    assert!(ui.is_drawn("unit-form-usage"));
    assert!(
        ui.dialog_value("usage")
            .unwrap_or_default()
            .starts_with("Used by "),
    );
}

#[gpui::test]
fn a_confirmed_delete_below_x_large_raises_the_presenters_toast(app: &mut TestAppContext) {
    let mut ui = on_units(app);
    ui.drop_unit_users();
    ui.press("j j d");
    type_word(&mut ui, "vas");
    ui.press("enter");

    assert_eq!(ui.dialog(), None);
    assert_eq!(ui.unit_codes(), ["aud", "btc"]);
    let toasts: Vec<String> = ui
        .toasts()
        .visible
        .into_iter()
        .map(|toast| toast.text)
        .collect();
    assert_eq!(toasts, ["Deleted unit vas"]);
}

#[gpui::test]
fn a_notice_queued_while_a_dialog_handles_a_key_shows_once_the_key_returns(
    app: &mut TestAppContext,
) {
    let mut ui = on_units(app);
    ui.press("n");
    assert_eq!(ui.dialog(), Some("AddUnit"));

    ui.queue_notice(Notice::Toast(ToastKind::Warning, "Rate limited".into()));
    assert!(
        ui.toast_texts().is_empty(),
        "nothing shows before the drain"
    );
    ui.press("x");

    assert_eq!(ui.dialog(), Some("AddUnit"), "the Dialog stays open");
    assert_eq!(ui.toast_texts(), ["Rate limited"]);

    ui.queue_notice(Notice::Status("not yet built".into()));
    ui.press("y");
    assert_eq!(status_message(&mut ui).as_deref(), Some("not yet built"));
}

#[gpui::test]
fn deleting_the_base_unit_moves_the_base_with_down(app: &mut TestAppContext) {
    let mut ui = on_units(app);
    ui.drop_unit_users();
    ui.unquote_units();
    ui.press("d");
    assert_eq!(
        ui.dialog_value("new_base").as_deref(),
        Some("btc — Bitcoin")
    );
    ui.press("down");
    assert_eq!(
        ui.dialog_value("new_base").as_deref(),
        Some("vas — Vanguard Aus Shares")
    );
    type_word(&mut ui, "aud");
    ui.press("enter");

    assert_eq!(ui.dialog(), None);
    assert_eq!(ui.base_unit_code().as_deref(), Some("vas"));
}

#[gpui::test]
fn r_reactivates_the_selected_deactivated_unit(app: &mut TestAppContext) {
    let mut ui = on_units(app);
    ui.press("j d");
    type_word(&mut ui, "btc");
    ui.press("enter enter");
    assert_eq!(ui.inactive_unit_codes(), ["btc"]);

    ui.press("r");
    assert!(ui.inactive_unit_codes().is_empty());
}

#[gpui::test]
fn t_puts_the_presenters_not_built_on_the_status_line(app: &mut TestAppContext) {
    let mut ui = on_units(app);
    ui.press("t");
    assert_eq!(status_message(&mut ui), None, "aud is priced by hand");
    ui.press("j t");
    assert_eq!(
        status_message(&mut ui).as_deref(),
        Some("test price source — not yet built")
    );
}

#[gpui::test]
fn a_custom_price_source_is_added_by_typing_tab_and_left_right(app: &mut TestAppContext) {
    let mut ui = on_units(app);
    ui.press("tab n");
    type_word(&mut ui, "broker");
    ui.press("tab");
    assert_eq!(ui.dialog_value("field").as_deref(), Some("Updates"));
    ui.press("left");
    assert_eq!(ui.dialog_value("updates").as_deref(), Some("Hourly"));
    ui.press("tab");
    type_word(&mut ui, "https://b.example");
    ui.press("tab");
    type_word(&mut ui, "/p");
    ui.press("tab");
    type_word(&mut ui, "/v");
    ui.press("tab right");
    assert_eq!(ui.dialog_value("auth").as_deref(), Some("API key"));
    ui.press("tab");
    type_word(&mut ui, "key");
    ui.press("tab");
    type_word(&mut ui, "secret");
    assert_eq!(
        ui.dialog_value("api_key").as_deref(),
        Some("\u{2022}\u{2022}\u{2022}\u{2022}\u{2022}\u{2022}"),
        "drawn masked"
    );
    ui.press("enter");

    assert_eq!(ui.dialog(), None);
    let names: Vec<String> = ui
        .settings_units()
        .price_sources
        .into_iter()
        .map(|source| source.name)
        .collect();
    assert!(names.contains(&"broker".to_string()));
}

#[gpui::test]
fn a_custom_source_is_deleted_once_its_name_is_typed_back(app: &mut TestAppContext) {
    let mut ui = on_units(app);
    ui.press("tab n");
    type_word(&mut ui, "broker");
    ui.press("tab tab");
    type_word(&mut ui, "h");
    ui.press("tab");
    type_word(&mut ui, "r");
    ui.press("tab");
    type_word(&mut ui, "p");
    ui.press("enter");
    assert_eq!(ui.dialog(), None);
    ui.press("G");
    for _ in 0..5 {
        if selected(&mut ui) == "broker" {
            break;
        }
        ui.press("k");
    }
    assert_eq!(selected(&mut ui), "broker");

    ui.press("d");
    assert_eq!(ui.dialog(), Some("DeletePriceSource"));
    ui.press("enter");
    assert_eq!(ui.dialog(), Some("DeletePriceSource"), "not yet typed back");
    type_word(&mut ui, "broker");
    ui.press("enter");
    assert_eq!(ui.dialog(), None);
    assert_ne!(selected(&mut ui), "broker");
}

#[gpui::test]
fn escape_closes_an_open_row_menu_before_leaving_the_page(app: &mut TestAppContext) {
    let mut ui = on_units(app);
    ui.right_click("unit-row-0");
    assert!(ui.settings_units().menu.is_some());

    ui.press("escape");
    assert_eq!(ui.settings_units().menu, None);
    assert!(ui.settings().page_focused);
}

#[gpui::test]
fn at_x_large_the_pane_is_drawn_and_follows_the_selection(app: &mut TestAppContext) {
    let mut ui = on_units_at(app, WindowSize::XLarge);
    assert!(ui.is_drawn("units-detail-pane"));
    assert!(
        ui.is_drawn("units-pane-add-source"),
        "aud is priced by hand"
    );

    ui.press("j");
    assert!(ui.is_drawn("units-pane-edit-source"), "btc's CoinGecko");
    ui.press("tab");
    assert!(ui.is_drawn("units-pane-values"));
}

#[gpui::test]
fn at_x_large_an_outcome_lands_in_activity_without_a_toast(app: &mut TestAppContext) {
    let mut ui = on_units_at(app, WindowSize::XLarge);
    ui.drop_unit_users();
    ui.press("j j d");
    type_word(&mut ui, "vas");
    ui.press("enter");

    let activity: Vec<String> = ui
        .settings_units()
        .activity
        .into_iter()
        .map(|line| line.text)
        .collect();
    assert_eq!(activity, ["Deleted unit vas"]);
    assert!(ui.toasts().visible.is_empty());
}

#[gpui::test]
fn a_renamed_unit_reaches_the_bill_plan_form_without_a_restart(app: &mut TestAppContext) {
    let mut ui = on_units(app);
    ui.press("e");
    ui.press("backspace backspace backspace");
    type_word(&mut ui, "aux");
    ui.press("enter");
    assert_eq!(ui.unit_codes()[0], "aux");

    ui.press("g w");
    assert_eq!(ui.noun(), Noun::Bills);
    ui.press("n");
    assert_eq!(ui.bills().plan_unit.as_deref(), Some("aux"));
}

/// Both tables fill the page from Medium's floor to X-Large, past the 840px other pages cap at,
/// and never draw narrower than the page at the floor.
#[gpui::test]
fn both_tables_fill_the_page_from_the_medium_floor_to_x_large(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);
    ui.resize_to(WindowSize::Medium);
    ui.press("g s j j enter");
    assert_eq!(ui.settings().page, "Units");
    // The page's width in a window at Medium's 720px floor, where the tables stop shrinking.
    const MIN_WIDTH: gpui::Pixels = gpui::px(610.0);
    let floor = gpui::size(gpui::px(720.0), gpui::px(800.0));
    let mut widths = Vec::new();
    for resize in [
        Some(floor),
        None,
        Some(WindowSize::Large.reference_frame()),
        Some(WindowSize::XLarge.reference_frame()),
    ] {
        if let Some(size) = resize {
            ui.resize(size);
        } else {
            ui.resize_to(WindowSize::Medium);
        }
        let page = ui.bounds("settings-page").size.width;
        for table in ["settings-units-table", "settings-price-sources-table"] {
            let width = ui.bounds(table).size.width;
            assert_eq!(width, page, "{table} fills the page");
            assert!(width >= MIN_WIDTH, "{table} keeps its minimum");
        }
        widths.push(page);
    }
    assert_eq!(widths[0], MIN_WIDTH);
    assert!(
        widths[3] > gpui::px(840.0),
        "X-Large draws past the Settings cap"
    );
}

/// Renames `aud` to `aux` on the Units page of `ui`, which must have focus on it.
fn rename_aud_to_aux(ui: &mut Harness<'_>) {
    ui.press("e");
    ui.press("backspace backspace backspace");
    type_word(ui, "aux");
    ui.press("enter");
    assert_eq!(ui.unit_codes()[0], "aux", "memory changes at once");
}

fn visible_toasts(ui: &mut Harness<'_>) -> Vec<(String, String)> {
    ui.toasts()
        .visible
        .into_iter()
        .map(|toast| (toast.kind, toast.text))
        .collect()
}

#[gpui::test]
fn a_failed_write_puts_the_units_back_as_stored_with_an_error_toast(app: &mut TestAppContext) {
    let mut ui = Harness::with_failing_units(app, false);
    ui.resize_to(WindowSize::Large);
    ui.press("g s j j l");
    rename_aud_to_aux(&mut ui);

    ui.settle_writes();

    assert_eq!(ui.unit_codes(), ["aud", "btc", "vas"]);
    assert!(visible_toasts(&mut ui).contains(&(
        "Error".to_string(),
        "Couldn't save the change to units, so they are back as last saved".to_string()
    )));
    ui.press("g w n");
    assert_eq!(
        ui.bills().plan_unit.as_deref(),
        Some("aud"),
        "the Bill Plan form's Unit picker reads the stored rows"
    );
}

#[gpui::test]
fn a_failed_write_draws_its_error_toast_with_no_further_key(app: &mut TestAppContext) {
    let mut ui = Harness::with_failing_units(app, false);
    ui.resize_to(WindowSize::Large);
    ui.press("g s j j l");
    rename_aud_to_aux(&mut ui);
    let shown = visible_toasts(&mut ui).len();
    assert!(
        !ui.is_drawn(&format!("toast-dismiss-{shown}")),
        "nothing has failed yet"
    );

    ui.settle_writes();

    assert!(
        ui.is_drawn(&format!("toast-dismiss-{shown}")),
        "the Error Toast is drawn as the failure arrives"
    );
    assert_eq!(visible_toasts(&mut ui)[shown].0, "Error");
}

#[gpui::test]
fn each_units_key_commits_once_whether_or_not_it_writes(app: &mut TestAppContext) {
    let mut ui = on_units(app);

    ui.press("e");
    ui.press("backspace backspace backspace");
    type_word(&mut ui, "aux");
    let before = ui.commits();
    ui.press("enter");
    assert_eq!(ui.unit_codes()[0], "aux", "the confirm saved the rename");
    assert_eq!(
        ui.commits(),
        before + 1,
        "the Dialog's confirm commits once"
    );

    let before = ui.commits();
    ui.press("j");
    assert_eq!(
        ui.commits(),
        before + 1,
        "a move writes nothing and commits once"
    );
}

#[gpui::test]
fn a_failed_reload_leaves_the_units_in_memory_with_an_error_toast(app: &mut TestAppContext) {
    let mut ui = Harness::with_failing_units(app, true);
    ui.resize_to(WindowSize::Large);
    ui.press("g s j j l");
    rename_aud_to_aux(&mut ui);

    ui.settle_writes();

    assert_eq!(ui.unit_codes()[0], "aux");
    assert!(visible_toasts(&mut ui).contains(&(
        "Error".to_string(),
        "Couldn't save the change to units or reload them, so what you see isn't saved".to_string()
    )));
}

#[gpui::test]
fn deleting_a_unit_other_units_quote_shows_the_refusal_and_keeps_it(app: &mut TestAppContext) {
    let mut ui = on_units(app);
    ui.drop_unit_users();
    ui.press("d");
    type_word(&mut ui, "aud");
    ui.press("enter");

    assert_eq!(ui.dialog(), Some("DeleteUnit"));
    assert!(
        ui.dialog_value("refusal")
            .is_some_and(|text| text.starts_with("aud is the quote unit of 2 other units")),
        "the refusal says how many units quote it"
    );
    assert_eq!(ui.unit_codes(), ["aud", "btc", "vas"]);
}

#[gpui::test]
fn escape_closes_an_open_select_then_the_next_escape_cancels_the_dialog(app: &mut TestAppContext) {
    let mut ui = on_units(app);
    let before = ui.unit_rows();
    ui.press("n tab enter");
    assert_eq!(ui.dialog_value("select_open").as_deref(), Some("true"));

    ui.press("escape");
    assert_eq!(ui.dialog(), Some("AddUnit"));
    assert_eq!(ui.dialog_value("select_open").as_deref(), Some("false"));

    ui.press("escape");
    assert_eq!(ui.dialog(), None);
    assert_eq!(ui.read(|shell| shell.nav().mode()), InputMode::Normal);
    assert_eq!(ui.unit_rows(), before, "a cancelled Dialog writes nothing");
}

#[gpui::test]
fn enter_confirms_only_a_valid_form_and_on_a_select_opens_it(app: &mut TestAppContext) {
    let mut ui = on_units(app);
    ui.press("n enter");
    assert_eq!(ui.dialog(), Some("AddUnit"), "an empty Code can't confirm");

    type_word(&mut ui, "eth");
    ui.press("tab tab");
    type_word(&mut ui, "Ether");
    ui.press("shift-tab");
    assert_eq!(ui.dialog_value("field").as_deref(), Some("Type"));
    assert_eq!(ui.dialog_value("can_confirm").as_deref(), Some("true"));
    ui.press("enter");
    assert_eq!(ui.dialog(), Some("AddUnit"), "Enter on a select opens it");
    assert_eq!(ui.dialog_value("select_open").as_deref(), Some("true"));
}

#[gpui::test]
fn ctrl_enter_confirms_a_valid_form_from_a_select(app: &mut TestAppContext) {
    let mut ui = on_units(app);
    ui.press("n");
    type_word(&mut ui, "eth");
    ui.press("tab tab");
    type_word(&mut ui, "Ether");
    ui.press("shift-tab enter");
    assert_eq!(ui.dialog_value("select_open").as_deref(), Some("true"));

    ui.press("ctrl-enter");

    assert_eq!(ui.dialog(), None);
    assert!(ui.unit_codes().contains(&"eth".to_string()));
}

#[gpui::test]
fn an_override_of_next_field_moves_the_field_and_frees_tab(app: &mut TestAppContext) {
    let mut ui = Harness::with_keybindings(app, common::overriding("next_field", "ctrl+n"));
    ui.resize_to(WindowSize::Large);
    to_units(&mut ui);
    ui.press("n");
    assert_eq!(ui.dialog_value("field").as_deref(), Some("Code"));

    ui.press("ctrl-n");
    assert_eq!(ui.dialog_value("field").as_deref(), Some("Type"));
    ui.press("tab");
    assert_eq!(
        ui.dialog_value("field").as_deref(),
        Some("Type"),
        "tab is no longer bound"
    );
}
