//! The Settings Units page driven by keys, on in-memory Units ports. Wiring and drawing only: a key
//! reaches the `lib-units` Presenter through the Units keymap or the Dialog host, and the page or
//! Dialog draws the view model it builds. The rules behind each key (refusals, the base Unit move,
//! Deactivate for a Unit in use, validation) are the Presenter tests' in `lib-units`.

use super::common;
use bin_desktop::shell::router::SettingsUnitsSnapshot;
use lib_config::KeyBindingConfig;
use lib_database::Domain;

use bin_desktop::{
    WindowSize, WindowSizeExt,
    navigation::nav::{FocusZone, InputMode, Noun},
};
use common::{Harness, no_unit_users};
use gpui::TestAppContext;
use lib_core::{InstitutionId, UnitId};
use lib_database::FakeStores;
use lib_toast::{Notice, ToastKind};

/// The page at Large with keyboard focus on it.
fn on_units(app: &mut TestAppContext) -> Harness<'_> {
    on_units_at(app, WindowSize::Large)
}

fn on_units_at(app: &mut TestAppContext, size: WindowSize) -> Harness<'_> {
    to_units_at(Harness::new(app), size)
}

/// As [`on_units`], on stores no Unit is in use in, so a seeded Unit can be deleted.
fn on_units_unused(app: &mut TestAppContext) -> Harness<'_> {
    on_units_unused_at(app, WindowSize::Large)
}

fn on_units_unused_at(app: &mut TestAppContext, size: WindowSize) -> Harness<'_> {
    to_units_at(Harness::arrange(app, no_unit_users), size)
}

fn on_units_with(app: &mut TestAppContext, arrange: impl FnOnce(&mut FakeStores)) -> Harness<'_> {
    to_units_at(
        Harness::arrange(app, |fakes| {
            no_unit_users(fakes);
            arrange(fakes);
        }),
        WindowSize::Large,
    )
}

fn to_units_at(mut ui: Harness<'_>, size: WindowSize) -> Harness<'_> {
    ui.resize_to(size);
    to_units(&mut ui);
    ui
}

/// Takes `ui` to the Units page, with keyboard focus on it.
fn to_units(ui: &mut Harness<'_>) {
    ui.press("g s");
    assert_eq!(ui.noun(), Noun::Settings);
    ui.press("j j j j j");
    assert_eq!(ui.settings_frame().page, "Units");
    ui.press("l");
    assert!(ui.settings_frame().page_focused);
}

/// The selected row: a Unit's code or a Price Source's name.
fn selected(ui: &mut Harness<'_>) -> String {
    let page = ui.settings::<SettingsUnitsSnapshot>();
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
    ui.press("g s j j j j j");
    assert_eq!(ui.settings_frame().page, "Units");
    assert!(!ui.settings_frame().page_focused);

    ui.press("l");
    assert!(ui.settings_frame().page_focused);
    assert_eq!(ui.dialog(), None);
    assert_eq!(ui.read(|shell| shell.nav().mode()), InputMode::Normal);
}

#[gpui::test]
fn the_movement_keys_reach_the_presenter_and_walk_both_tables(app: &mut TestAppContext) {
    let mut ui = on_units(app);
    assert_eq!(selected(&mut ui), "AUD");

    ui.press("j");
    assert_eq!(selected(&mut ui), "BTC");
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
        "AUD",
        "g g passes the Shell's `g` on to the keymap's move_first"
    );
    ui.press("down");
    assert_eq!(selected(&mut ui), "BTC", "the arrows too");
}

#[gpui::test]
fn home_end_and_the_page_keys_move_through_the_units_keymap(app: &mut TestAppContext) {
    let mut ui = on_units(app);

    ui.press("end");
    assert_eq!(selected(&mut ui), "Yahoo Finance");
    ui.press("home");
    assert_eq!(selected(&mut ui), "AUD");
    ui.press("pagedown");
    assert_ne!(selected(&mut ui), "AUD");
    ui.press("pageup");
    assert_eq!(selected(&mut ui), "AUD");
}

#[gpui::test]
fn an_override_of_arrow_down_moves_it_and_frees_down(app: &mut TestAppContext) {
    let mut ui =
        Harness::with_keybindings(app, KeyBindingConfig::overriding("arrow_down", "ctrl+n"));
    ui.resize_to(WindowSize::Large);
    to_units(&mut ui);

    ui.press("ctrl-n");
    assert_eq!(selected(&mut ui), "BTC");
    ui.press("down");
    assert_eq!(selected(&mut ui), "BTC", "down is no longer bound");
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
    assert_eq!(selected(&mut ui), "AUD");
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
    assert_eq!(ui.dialog_value("code").as_deref(), Some("BTC"));
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
        ["AUD", "BTC", "VAS"],
        "nothing was written"
    );
}

#[gpui::test]
fn typing_tab_and_enter_fill_and_save_an_add_unit_dialog(app: &mut TestAppContext) {
    let mut ui = on_units(app);
    ui.press("n");
    assert_eq!(ui.dialog_value("field").as_deref(), Some("Code"));
    assert_eq!(ui.dialog_value("can_confirm").as_deref(), Some("false"));

    type_word(&mut ui, "ETH");
    ui.press("backspace h");
    ui.press("tab");
    assert_eq!(ui.dialog_value("field").as_deref(), Some("Type"));
    ui.press("down");
    assert_eq!(ui.dialog_value("kind").as_deref(), Some("Crypto"));
    ui.press("tab");
    type_word(&mut ui, "Ether");
    assert_eq!(ui.dialog_value("code").as_deref(), Some("ETH"));
    ui.press("enter");

    assert_eq!(ui.dialog(), None);
    assert_eq!(
        ui.unit_rows().iter().find(|row| row.0 == "ETH"),
        Some(&("ETH".to_string(), "Ether".to_string(), "Crypto".to_string()))
    );
}

#[gpui::test]
fn precision_follows_type_on_add_takes_typed_digits_and_reopens_in_edit(app: &mut TestAppContext) {
    let mut ui = on_units(app);
    ui.press("n");
    assert_eq!(ui.dialog_value("precision").as_deref(), Some("2"));
    type_word(&mut ui, "ETH");
    ui.press("tab down");
    assert_eq!(ui.dialog_value("precision").as_deref(), Some("8"));

    ui.press("tab");
    type_word(&mut ui, "Ether");
    ui.press("tab");
    assert_eq!(ui.dialog_value("field").as_deref(), Some("Precision"));
    ui.press("backspace 1 9");
    assert_eq!(ui.dialog_value("can_confirm").as_deref(), Some("false"));
    ui.press("backspace");
    ui.press("enter");
    assert_eq!(ui.dialog(), None);

    // A new Unit lands in its place by code, between btc and vas.
    ui.press("g g j j");
    assert_eq!(selected(&mut ui), "ETH");
    ui.press("e");
    assert_eq!(ui.dialog_value("precision").as_deref(), Some("1"));
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
            .starts_with("BTC is used by "),
    );
    assert!(!ui.is_drawn("delete-unit-base-note"));

    ui.press("enter");
    assert_eq!(ui.dialog(), None);
    assert_eq!(ui.inactive_unit_codes(), ["BTC"]);
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
    let mut ui = on_units_unused(app);
    ui.press("j j d");
    type_word(&mut ui, "VAS");
    ui.press("enter");

    assert_eq!(ui.dialog(), None);
    assert_eq!(ui.unit_codes(), ["AUD", "BTC"]);
    let toasts: Vec<String> = ui
        .toasts()
        .visible
        .into_iter()
        .map(|toast| toast.text)
        .collect();
    assert_eq!(toasts, ["Deleted unit VAS"]);
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
    let mut ui = on_units_with(app, common::unquote_units);
    ui.press("d");
    assert_eq!(
        ui.dialog_value("new_base").as_deref(),
        Some("BTC — Bitcoin")
    );
    ui.press("down");
    assert_eq!(
        ui.dialog_value("new_base").as_deref(),
        Some("VAS — Vanguard Aus Shares")
    );
    type_word(&mut ui, "AUD");
    ui.press("enter");

    assert_eq!(ui.dialog(), None);
    assert_eq!(ui.base_unit_code().as_deref(), Some("VAS"));
}

#[gpui::test]
fn r_reactivates_the_selected_deactivated_unit(app: &mut TestAppContext) {
    let mut ui = on_units(app);
    ui.press("j d");
    type_word(&mut ui, "BTC");
    ui.press("enter");
    assert_eq!(ui.inactive_unit_codes(), ["BTC"]);

    ui.press("r");
    assert!(ui.inactive_unit_codes().is_empty());
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
    ui.press("tab");
    assert_eq!(ui.dialog_value("field").as_deref(), Some("Headers"));
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
        .settings::<SettingsUnitsSnapshot>()
        .price_sources
        .into_iter()
        .map(|source| source.name)
        .collect();
    assert!(names.contains(&"broker".to_string()));
}

/// The X-Large pane's PRICE SOURCE facts for the selected row, as `(label, value)`.
#[expect(
    clippy::expect_used,
    reason = "test helper: a pane missing at X-Large should fail the test loudly"
)]
fn source_facts(ui: &mut Harness<'_>) -> Vec<(String, String)> {
    use lib_units::presenter::PaneGroup;
    ui.settings::<SettingsUnitsSnapshot>()
        .pane
        .expect("docked")
        .groups
        .into_iter()
        .find_map(|group| match group {
            PaneGroup::Source(source) => Some(source.facts),
            _ => None,
        })
        .expect("a PRICE SOURCE group")
        .into_iter()
        .map(|fact| (fact.label, fact.value))
        .collect()
}

fn api_key_fact(ui: &mut Harness<'_>) -> Option<String> {
    source_facts(ui)
        .into_iter()
        .find_map(|(label, value)| (label == "API key").then_some(value))
}

/// Adds a Custom source named `name` taking an API key in a parameter, its key `key`, and selects
/// it.
fn add_keyed_source(ui: &mut Harness<'_>, name: &str, key: &str) {
    ui.press("tab n");
    type_word(ui, name);
    ui.press("tab tab");
    type_word(ui, "https://b.example");
    ui.press("tab");
    type_word(ui, "/p");
    ui.press("tab");
    type_word(ui, "/v");
    ui.press("tab tab right tab");
    type_word(ui, "apikey");
    ui.press("tab");
    type_word(ui, key);
    ui.press("enter");
    assert_eq!(ui.dialog(), None);
    ui.press("G");
    for _ in 0..5 {
        if selected(ui) == name {
            break;
        }
        ui.press("k");
    }
    assert_eq!(selected(ui), name);
}

#[gpui::test]
fn a_saved_key_shows_only_as_the_placeholder_and_remove_key_clears_it_on_save(
    app: &mut TestAppContext,
) {
    let mut ui = on_units_at(app, WindowSize::XLarge);
    add_keyed_source(&mut ui, "broker", "s3cret");
    assert_eq!(api_key_fact(&mut ui).as_deref(), Some("Saved"));
    assert!(
        ui.conf().contains("[Secrets]\napi_key_secret = "),
        "the first key generates this device's secret: {}",
        ui.conf()
    );
    assert!(!ui.conf().contains("s3cret"));

    ui.press("e");
    assert_eq!(ui.dialog(), Some("EditPriceSource"));
    assert_eq!(
        ui.dialog_value("api_key").as_deref(),
        Some(""),
        "never shown"
    );
    assert_eq!(
        ui.dialog_value("api_key_placeholder").as_deref(),
        Some("Key saved · leave blank to keep it")
    );
    assert_eq!(ui.dialog_value("remove_key").as_deref(), Some("Remove key"));
    ui.press("tab tab tab tab tab tab tab tab tab");
    assert_eq!(ui.dialog_value("field").as_deref(), Some("RemoveKey"));
    ui.press("enter");
    assert_eq!(
        ui.dialog(),
        Some("EditPriceSource"),
        "Remove key waits for Save"
    );
    assert_eq!(ui.dialog_value("remove_key").as_deref(), Some(""));
    assert_eq!(ui.dialog_value("api_key_placeholder").as_deref(), Some(""));
    assert_eq!(ui.dialog_value("field").as_deref(), Some("ApiKey"));
    ui.press("enter");

    assert_eq!(ui.dialog(), None);
    assert_eq!(api_key_fact(&mut ui).as_deref(), Some("Not set"));
}

#[gpui::test]
fn blank_keeps_a_saved_key_and_typed_text_replaces_it_masked(app: &mut TestAppContext) {
    let mut ui = on_units_at(app, WindowSize::XLarge);
    add_keyed_source(&mut ui, "broker", "s3cret");
    let secret = ui.conf();

    ui.press("e enter");
    assert_eq!(ui.dialog(), None);
    assert_eq!(
        api_key_fact(&mut ui).as_deref(),
        Some("Saved"),
        "blank keeps it"
    );

    ui.press("e tab tab tab tab tab tab tab tab");
    assert_eq!(ui.dialog_value("field").as_deref(), Some("ApiKey"));
    type_word(&mut ui, "n3w");
    assert_eq!(
        ui.dialog_value("api_key").as_deref(),
        Some("\u{2022}\u{2022}\u{2022}")
    );
    ui.press("enter");
    assert_eq!(ui.dialog(), None);
    assert_eq!(api_key_fact(&mut ui).as_deref(), Some("Saved"));
    assert_eq!(ui.conf(), secret, "the secret is generated once");
}

#[gpui::test]
fn coingeckos_edit_dialog_takes_its_optional_demo_key(app: &mut TestAppContext) {
    let mut ui = on_units_at(app, WindowSize::XLarge);
    ui.press("tab");
    for _ in 0..5 {
        if selected(&mut ui) == "CoinGecko" {
            break;
        }
        ui.press("j");
    }
    assert_eq!(selected(&mut ui), "CoinGecko");
    assert_eq!(api_key_fact(&mut ui).as_deref(), Some("Not set"));

    ui.press("e");
    assert_eq!(ui.dialog(), Some("EditBuiltInPriceSource"));
    assert_eq!(
        ui.dialog_value("key_note").as_deref(),
        Some("Optional. Without a key CoinGecko is asked keyless, at a lower call limit.")
    );
    ui.press("tab tab");
    assert_eq!(ui.dialog_value("field").as_deref(), Some("ApiKey"));
    type_word(&mut ui, "CG-demo");
    ui.press("enter");

    assert_eq!(ui.dialog(), None);
    assert_eq!(api_key_fact(&mut ui).as_deref(), Some("Saved"));
}

#[gpui::test]
fn a_key_another_devices_secret_saved_reads_unreadable_and_its_field_is_off(
    app: &mut TestAppContext,
) {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let path = dir.path().join("ledger.pldb");
    lib_database::seed::sample_ledger(&path, common::today()).expect("the Ledger is created");
    let storage = lib_database::open_storage(&path).expect("the Ledger opens");
    let writer = storage.writer.clone();
    let (elsewhere, _) = lib_database::LedgerPorts::new(
        lib_database::Stores::from(storage),
        lib_core::Clock::fixed(common::today()),
        lib_database::ApiKeySecret::in_memory(Some("another device's secret".into())),
    );
    elsewhere
        .units()
        .borrow_mut()
        .keys_mut()
        .store_key(lib_units::COINGECKO, "CG-demo".into())
        .expect("the key is sealed");
    let _ = elsewhere.commit().commit();
    writer.flush().expect("the writer is running");
    drop(elsewhere);

    let storage = lib_database::open_storage(&path).expect("the Ledger opens again");
    let mut ui = to_units_at(Harness::with_storage(app, storage), WindowSize::XLarge);
    ui.press("tab");
    for _ in 0..5 {
        if selected(&mut ui) == "CoinGecko" {
            break;
        }
        ui.press("j");
    }
    assert_eq!(
        api_key_fact(&mut ui).as_deref(),
        Some("Saved, but can't be read on this device")
    );

    ui.press("e");
    assert_eq!(ui.dialog_value("api_key_enabled").as_deref(), Some("false"));
    assert_eq!(ui.dialog_value("remove_key").as_deref(), Some(""));
    assert_eq!(
        ui.dialog_value("key_note").as_deref(),
        Some(
            "This Ledger's API keys were saved with another device's API key secret. Copy that \
             secret into this device's settings to change keys here."
        )
    );
    ui.press("tab tab");
    assert_ne!(
        ui.dialog_value("field").as_deref(),
        Some("ApiKey"),
        "it takes no focus"
    );
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
    assert!(ui.settings::<SettingsUnitsSnapshot>().menu.is_some());

    ui.press("escape");
    assert_eq!(ui.settings::<SettingsUnitsSnapshot>().menu, None);
    assert!(ui.settings_frame().page_focused);
}

#[gpui::test]
fn the_units_table_draws_each_units_last_price_and_its_age_under_last(app: &mut TestAppContext) {
    let mut ui = on_units(app);
    let last: Vec<_> = ui
        .settings::<SettingsUnitsSnapshot>()
        .units
        .into_iter()
        .map(|unit| (unit.last_price, unit.last_age))
        .collect();
    assert_eq!(
        last,
        [
            ("—".to_string(), None),
            (
                "148,212.40 AUD".to_string(),
                Some("5 minutes ago".to_string())
            ),
            ("—".to_string(), None),
        ]
    );
    for row in 0..last.len() {
        assert!(ui.is_drawn(&format!("unit-last-{row}")), "row {row}");
    }
}

#[gpui::test]
fn at_x_large_the_pane_is_drawn_and_follows_the_selection(app: &mut TestAppContext) {
    let mut ui = on_units_at(app, WindowSize::XLarge);
    assert!(ui.is_drawn("units-detail-pane"));
    assert!(
        ui.is_drawn("units-pane-add-source"),
        "AUD is priced by hand"
    );

    ui.press("j");
    assert!(ui.is_drawn("units-pane-edit-source"), "BTC's CoinGecko");
    ui.press("tab");
    assert!(ui.is_drawn("units-pane-values"));
}

#[gpui::test]
fn at_x_large_seeded_btc_docks_unit_with_its_precision_then_price_source_price_and_accounts(
    app: &mut TestAppContext,
) {
    use lib_units::presenter::PaneGroup;
    let mut ui = on_units_at(app, WindowSize::XLarge);
    ui.press("j");
    let groups = ui
        .settings::<SettingsUnitsSnapshot>()
        .pane
        .expect("docked")
        .groups;

    let slugs: Vec<_> = groups.iter().map(PaneGroup::slug).collect();
    assert_eq!(
        slugs,
        ["unit", "history", "source", "price", "accounts", "activity"]
    );
    let PaneGroup::Unit(unit) = &groups[0] else {
        panic!("UNIT first")
    };
    assert!(
        unit.facts
            .iter()
            .any(|fact| fact.label == "Precision" && fact.value == "8"),
        "{:?}",
        unit.facts
    );
    let PaneGroup::Price(price) = &groups[3] else {
        panic!("PRICE fourth")
    };
    let labels: Vec<_> = price.facts.iter().map(|fact| fact.label.as_str()).collect();
    assert_eq!(labels, ["Last price", "Week change", "52w range"]);
    assert!(ui.is_drawn("units-pane-price"));
}

#[gpui::test]
fn at_x_large_an_outcome_lands_in_activity_without_a_toast(app: &mut TestAppContext) {
    let mut ui = on_units_unused_at(app, WindowSize::XLarge);
    ui.press("j j d");
    type_word(&mut ui, "VAS");
    ui.press("enter");

    let activity: Vec<String> = ui
        .settings::<SettingsUnitsSnapshot>()
        .activity
        .into_iter()
        .map(|line| line.text)
        .collect();
    assert_eq!(activity, ["Deleted unit VAS"]);
    assert!(ui.toasts().visible.is_empty());
}

#[gpui::test]
fn a_renamed_unit_reaches_the_bill_plan_form_without_a_restart(app: &mut TestAppContext) {
    let mut ui = on_units(app);
    ui.press("e");
    ui.press("backspace backspace backspace");
    type_word(&mut ui, "aux");
    ui.press("enter");
    assert_eq!(ui.unit_codes()[0], "AUX");

    ui.press("g w");
    assert_eq!(ui.noun(), Noun::Bills);
    ui.press("n");
    assert_eq!(ui.bills().plan_unit.as_deref(), Some("AUX"));
}

/// Both tables fill the page from Medium's floor to X-Large, past the 840px other pages cap at,
/// and never draw narrower than the page at the floor.
#[gpui::test]
fn both_tables_fill_the_page_from_the_medium_floor_to_x_large(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);
    ui.resize_to(WindowSize::Medium);
    ui.press("g s j j j j j enter");
    assert_eq!(ui.settings_frame().page, "Units");
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
    assert_eq!(ui.unit_codes()[0], "AUX", "memory changes at once");
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
    let mut ui = Harness::arrange(app, |fakes| fakes.fails.writes = vec![Domain::Units]);
    ui.resize_to(WindowSize::Large);
    ui.press("g s j j j j j l");
    rename_aud_to_aux(&mut ui);

    ui.settle_writes();

    assert_eq!(ui.unit_codes(), ["AUD", "BTC", "VAS"]);
    assert!(visible_toasts(&mut ui).contains(&(
        "Error".to_string(),
        "Couldn't save the change to units, so they are back as last saved".to_string()
    )));
    ui.press("g w n");
    assert_eq!(
        ui.bills().plan_unit.as_deref(),
        Some("AUD"),
        "the Bill Plan form's Unit picker reads the stored rows"
    );
}

#[gpui::test]
fn a_failed_write_draws_its_error_toast_with_no_further_key(app: &mut TestAppContext) {
    let mut ui = Harness::arrange(app, |fakes| fakes.fails.writes = vec![Domain::Units]);
    ui.resize_to(WindowSize::Large);
    ui.press("g s j j j j j l");
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
    assert_eq!(ui.unit_codes()[0], "AUX", "the confirm saved the rename");
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
    let mut ui = Harness::arrange(app, |fakes| {
        fakes.fails.writes = vec![Domain::Units];
        fakes.fails.reloads = vec![Domain::Units];
    });
    ui.resize_to(WindowSize::Large);
    ui.press("g s j j j j j l");
    rename_aud_to_aux(&mut ui);

    ui.settle_writes();

    assert_eq!(ui.unit_codes()[0], "AUX");
    assert!(visible_toasts(&mut ui).contains(&(
        "Error".to_string(),
        "Couldn't save the change to units or reload them, so what you see isn't saved".to_string()
    )));
}

#[gpui::test]
fn deleting_a_unit_other_units_quote_shows_the_refusal_and_keeps_it(app: &mut TestAppContext) {
    let mut ui = on_units_unused(app);
    ui.press("d");
    type_word(&mut ui, "AUD");
    ui.press("enter");

    assert_eq!(ui.dialog(), Some("DeleteUnit"));
    assert!(
        ui.dialog_value("refusal")
            .is_some_and(|text| text.starts_with("AUD is the quote unit of 2 other units")),
        "the refusal says how many units quote it"
    );
    assert_eq!(ui.unit_codes(), ["AUD", "BTC", "VAS"]);
}

#[gpui::test]
fn a_unit_a_budget_uses_opens_delete_as_deactivate_naming_the_budget(app: &mut TestAppContext) {
    let mut ui = on_units_with(app, |fakes| common::add_budget_in(fakes, UnitId::SEED_VAS));
    ui.press("j j d");
    assert_eq!(ui.dialog(), Some("DeleteUnit"));
    assert_eq!(ui.dialog_value("in_use").as_deref(), Some("true"));
    assert_eq!(
        ui.dialog_value("in_use_text").as_deref(),
        Some("VAS is used by 1 budget, so it can't be deleted."),
    );
    assert_eq!(ui.unit_codes(), ["AUD", "BTC", "VAS"]);
}

#[gpui::test]
fn a_unit_a_location_names_opens_delete_as_deactivate_naming_the_location(
    app: &mut TestAppContext,
) {
    let mut ui = on_units_with(app, |fakes| {
        common::add_location_in(fakes, UnitId::SEED_VAS)
    });
    ui.press("j j d");
    assert_eq!(ui.dialog(), Some("DeleteUnit"));
    assert_eq!(ui.dialog_value("in_use").as_deref(), Some("true"));
    assert_eq!(
        ui.dialog_value("in_use_text").as_deref(),
        Some("VAS is used by 1 location, so it can't be deleted."),
    );
    assert_eq!(ui.unit_codes(), ["AUD", "BTC", "VAS"]);
}

#[gpui::test]
fn a_unit_an_institution_names_asks_for_its_usual_unit_on_none_and_deletes_leaving_none(
    app: &mut TestAppContext,
) {
    let mut ui = on_units_unused(app);
    ui.press("j d");
    assert_eq!(ui.dialog(), Some("DeleteUnit"));
    assert_eq!(ui.dialog_value("in_use").as_deref(), Some("false"));
    assert_eq!(ui.dialog_value("new_base").as_deref(), Some(""));
    assert!(ui.is_drawn("delete-unit-usual-units-help"));
    assert_eq!(
        ui.dialog_value("usual_units_help").as_deref(),
        Some(
            "1 institution uses BTC as its usual unit. Choose the unit it uses instead, or leave \
             it on None."
        ),
    );
    assert_eq!(
        ui.dialog_value("usual_units").as_deref(),
        Some("Cryptocurrency Exchange=None"),
        "never silently the base Unit"
    );
    assert_eq!(
        ui.dialog_value("usual_unit_options").as_deref(),
        Some("None|AUD — Australian Dollar|VAS — Vanguard Aus Shares"),
    );

    type_word(&mut ui, "BTC");
    ui.press("enter");

    assert_eq!(ui.dialog(), None);
    assert_eq!(ui.unit_codes(), ["AUD", "VAS"]);
    assert_eq!(
        ui.usual_unit_code(InstitutionId::SEED_CRYPTO_EXCHANGE),
        None
    );
}

#[gpui::test]
fn down_on_an_institutions_usual_unit_reassigns_it_as_the_unit_goes(app: &mut TestAppContext) {
    let mut ui = on_units_unused(app);
    ui.press("j d");
    ui.press("down");
    assert_eq!(
        ui.dialog_value("usual_units").as_deref(),
        Some("Cryptocurrency Exchange=AUD — Australian Dollar"),
    );

    type_word(&mut ui, "BTC");
    ui.press("enter");

    assert_eq!(ui.dialog(), None);
    assert_eq!(ui.unit_codes(), ["AUD", "VAS"]);
    assert_eq!(
        ui.usual_unit_code(InstitutionId::SEED_CRYPTO_EXCHANGE)
            .as_deref(),
        Some("AUD")
    );
}

#[gpui::test]
fn the_named_base_unit_asks_for_both_and_tab_moves_between_the_selects(app: &mut TestAppContext) {
    let mut ui = on_units_with(app, |fakes| {
        common::unquote_units(fakes);
        common::name_usual_unit(fakes, InstitutionId::SEED_WESTPAC, UnitId::SEED_AUD);
    });
    ui.press("d");
    assert_eq!(
        ui.dialog_value("new_base").as_deref(),
        Some("BTC — Bitcoin")
    );
    assert_eq!(
        ui.dialog_value("usual_units").as_deref(),
        Some("Westpac Banking=None")
    );
    assert_eq!(ui.dialog_value("focused").as_deref(), Some("New base unit"));

    ui.press("tab");
    assert_eq!(
        ui.dialog_value("focused").as_deref(),
        Some("Westpac Banking")
    );
    ui.press("down");
    assert_eq!(
        ui.dialog_value("usual_units").as_deref(),
        Some("Westpac Banking=BTC — Bitcoin")
    );
    ui.press("shift-tab down");
    assert_eq!(
        ui.dialog_value("new_base").as_deref(),
        Some("VAS — Vanguard Aus Shares")
    );
    type_word(&mut ui, "AUD");
    ui.press("enter");

    assert_eq!(ui.dialog(), None);
    assert_eq!(ui.base_unit_code().as_deref(), Some("VAS"));
    assert_eq!(
        ui.usual_unit_code(InstitutionId::SEED_WESTPAC).as_deref(),
        Some("BTC")
    );
}

#[gpui::test]
fn escape_cancels_a_named_units_delete_having_reassigned_nothing(app: &mut TestAppContext) {
    let mut ui = on_units_unused(app);
    ui.press("j d down");
    type_word(&mut ui, "BTC");

    ui.press("escape");

    assert_eq!(ui.dialog(), None);
    assert_eq!(ui.unit_codes(), ["AUD", "BTC", "VAS"]);
    assert_eq!(
        ui.usual_unit_code(InstitutionId::SEED_CRYPTO_EXCHANGE)
            .as_deref(),
        Some("BTC")
    );
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

    type_word(&mut ui, "ETH");
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
    type_word(&mut ui, "ETH");
    ui.press("tab tab");
    type_word(&mut ui, "Ether");
    ui.press("shift-tab enter");
    assert_eq!(ui.dialog_value("select_open").as_deref(), Some("true"));

    ui.press("ctrl-enter");

    assert_eq!(ui.dialog(), None);
    assert!(ui.unit_codes().contains(&"ETH".to_string()));
}

#[gpui::test]
fn an_override_of_next_field_moves_the_field_and_frees_tab(app: &mut TestAppContext) {
    let mut ui =
        Harness::with_keybindings(app, KeyBindingConfig::overriding("next_field", "ctrl+n"));
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

#[gpui::test]
fn question_mark_lists_the_keys_the_status_line_shows(app: &mut TestAppContext) {
    let mut ui = on_units(app);

    ui.assert_cheat_sheet_matches_status("Settings keys");
}

#[gpui::test]
fn the_cheat_sheet_shows_a_rebound_key(app: &mut TestAppContext) {
    let mut ui = Harness::with_keybindings(app, KeyBindingConfig::overriding("new", "ctrl+n"));
    ui.resize_to(WindowSize::Large);
    to_units(&mut ui);
    assert!(
        ui.status_as_shortcuts()
            .iter()
            .any(|(_, keys)| keys == "ctrl+n"),
        "the status line shows the rebound key"
    );

    ui.assert_cheat_sheet_matches_status("Settings keys");
}

/// The selected Unit's Last price in the docked pane's PRICE SOURCE group.
fn pane_last_price(ui: &mut Harness<'_>) -> Option<String> {
    use lib_units::presenter::PaneGroup;
    ui.settings::<SettingsUnitsSnapshot>()
        .pane?
        .groups
        .into_iter()
        .find_map(|group| match group {
            PaneGroup::Source(source) => Some(source.facts),
            _ => None,
        })?
        .into_iter()
        .find(|fact| fact.label == "Last price")
        .map(|fact| fact.value)
}

/// Empties the Set price Dialog's focused text field with Backspace.
fn clear(ui: &mut Harness<'_>, field: &str) {
    let len = ui.dialog_value(field).unwrap_or_default().chars().count();
    for _ in 0..len {
        ui.press("backspace");
    }
}

#[gpui::test]
fn p_on_a_manual_entry_unit_sets_this_weeks_price_and_the_pane_shows_it(app: &mut TestAppContext) {
    let mut ui = on_units_at(app, WindowSize::XLarge);
    ui.press("j j");
    assert_eq!(pane_last_price(&mut ui).as_deref(), Some("—"));

    ui.press("p");
    assert_eq!(ui.dialog(), Some("SetPrice"));
    assert_eq!(ui.dialog_value("title").as_deref(), Some("Set price — VAS"));
    assert_eq!(ui.dialog_value("field").as_deref(), Some("Close"));
    assert_eq!(ui.dialog_value("can_confirm").as_deref(), Some("false"));
    type_word(&mut ui, "42");
    ui.press("enter");

    assert_eq!(ui.dialog(), None);
    assert_eq!(pane_last_price(&mut ui).as_deref(), Some("42.00 AUD"));
    let activity = ui.settings::<SettingsUnitsSnapshot>().activity;
    assert!(
        activity[0].text.starts_with("Set the price of VAS"),
        "{activity:?}"
    );
}

#[gpui::test]
fn a_future_week_is_refused_with_its_message_and_a_past_week_is_set(app: &mut TestAppContext) {
    let mut ui = on_units(app);
    ui.press("j j p tab");
    assert_eq!(ui.dialog_value("field").as_deref(), Some("Week"));
    clear(&mut ui, "week");
    type_word(&mut ui, "2999-01-09");
    assert_eq!(
        ui.dialog_value("week_help").as_deref(),
        Some("Week of Jan 7, 2999"),
        "the date snaps to its Monday"
    );
    ui.press("tab");
    type_word(&mut ui, "10");
    ui.press("enter");

    assert_eq!(ui.dialog(), Some("SetPrice"), "the Dialog stays open");
    assert_eq!(
        ui.dialog_value("refusal").as_deref(),
        Some("The week of Jan 7, 2999 hasn't started, so it can't be priced yet.")
    );

    ui.press("tab");
    clear(&mut ui, "week");
    type_word(&mut ui, "2020-01-08");
    ui.press("enter");

    assert_eq!(ui.dialog(), None);
    assert_eq!(
        ui.toast_texts(),
        ["Set the price of VAS for the week of Jan 6, 2020"]
    );
}

#[gpui::test]
fn p_on_a_unit_its_source_prices_opens_nothing(app: &mut TestAppContext) {
    let mut ui = on_units(app);

    ui.press("j p");

    assert_eq!(ui.dialog(), None, "BTC is priced by CoinGecko");
}

/// Every Unit as the stores hold it, by code: what an Undo must put back under its own id.
fn stored_units(ui: &mut Harness<'_>) -> Vec<lib_units::UnitRow> {
    let mut units = ui.stores(|stores| stores.units.borrow().units().units());
    units.sort_by(|a, b| a.code.cmp(&b.code));
    units
}

/// The labels History would undo, newest first.
fn undo_labels(ui: &mut Harness<'_>) -> Vec<String> {
    ui.read(|shell| shell.history())
        .undo
        .into_iter()
        .map(|row| row.label)
        .collect()
}

#[gpui::test]
fn u_undoes_an_added_unit_and_ctrl_r_redoes_it_under_its_own_id(app: &mut TestAppContext) {
    let mut ui = on_units(app);
    let before = stored_units(&mut ui);
    ui.press("n");
    type_word(&mut ui, "ETH");
    ui.press("tab down tab");
    type_word(&mut ui, "Ether");
    ui.press("enter");
    let added = stored_units(&mut ui);

    ui.press("u");
    assert_eq!(stored_units(&mut ui), before);
    assert_eq!(ui.unit_codes(), ["AUD", "BTC", "VAS"], "the page repaints");
    assert_eq!(
        status_message(&mut ui).as_deref(),
        Some("Undid Add unit ETH")
    );

    ui.press("ctrl-r");
    assert_eq!(stored_units(&mut ui), added, "back under its own id");
    assert_eq!(ui.unit_codes(), ["AUD", "BTC", "ETH", "VAS"]);
    assert_eq!(
        status_message(&mut ui).as_deref(),
        Some("Redid Add unit ETH")
    );
}

#[gpui::test]
fn u_undoes_an_edited_unit_and_ctrl_r_redoes_it(app: &mut TestAppContext) {
    let mut ui = on_units(app);
    rename_aud_to_aux(&mut ui);

    ui.press("u");
    assert_eq!(ui.unit_codes()[0], "AUD");
    assert_eq!(
        status_message(&mut ui).as_deref(),
        Some("Undid Edit unit AUX")
    );

    ui.press("ctrl-r");
    assert_eq!(ui.unit_codes()[0], "AUX");
}

#[gpui::test]
fn u_undoes_a_unit_delete_with_the_institutions_it_reassigned(app: &mut TestAppContext) {
    let mut ui = on_units_unused(app);
    let before = stored_units(&mut ui);
    ui.press("j d down");
    type_word(&mut ui, "BTC");
    ui.press("enter");
    assert_eq!(
        ui.usual_unit_code(InstitutionId::SEED_CRYPTO_EXCHANGE)
            .as_deref(),
        Some("AUD")
    );

    ui.press("u");
    assert_eq!(stored_units(&mut ui), before, "BTC back under its own id");
    assert_eq!(
        ui.usual_unit_code(InstitutionId::SEED_CRYPTO_EXCHANGE)
            .as_deref(),
        Some("BTC"),
        "the Institution names BTC again"
    );
    assert_eq!(
        status_message(&mut ui).as_deref(),
        Some("Undid Delete unit BTC")
    );

    ui.press("ctrl-r");
    assert_eq!(ui.unit_codes(), ["AUD", "VAS"]);
    assert_eq!(
        ui.usual_unit_code(InstitutionId::SEED_CRYPTO_EXCHANGE)
            .as_deref(),
        Some("AUD")
    );
}

#[gpui::test]
fn u_undoes_a_deleted_base_unit_and_the_base_it_moved(app: &mut TestAppContext) {
    let mut ui = on_units_with(app, common::unquote_units);
    ui.press("d down");
    type_word(&mut ui, "AUD");
    ui.press("enter");
    assert_eq!(ui.base_unit_code().as_deref(), Some("VAS"));

    ui.press("u");
    assert_eq!(
        ui.unit_codes(),
        ["AUD", "BTC", "VAS"],
        "restored in its place by code"
    );
    assert_eq!(ui.base_unit_code().as_deref(), Some("AUD"));

    ui.press("ctrl-r");
    assert_eq!(ui.unit_codes(), ["BTC", "VAS"]);
    assert_eq!(ui.base_unit_code().as_deref(), Some("VAS"));
}

#[gpui::test]
fn a_moved_base_unit_is_undone_and_redone(app: &mut TestAppContext) {
    let mut ui = on_units(app);
    ui.press("B down enter");
    assert_eq!(ui.dialog(), None);
    let moved = ui.base_unit_code();
    assert_ne!(moved.as_deref(), Some("AUD"), "the base moved");

    ui.press("u");
    assert_eq!(ui.base_unit_code().as_deref(), Some("AUD"));

    ui.press("ctrl-r");
    assert_eq!(ui.base_unit_code(), moved);
}

#[gpui::test]
fn a_failed_undo_reloads_the_units_keeps_the_entry_and_says_why(app: &mut TestAppContext) {
    let arm = lib_database::FailArm::disarmed();
    let armed = arm.clone();
    let mut ui = Harness::arrange(app, move |fakes| {
        fakes.fails.writes = vec![Domain::Units];
        fakes.fails.armed = armed;
    });
    ui.resize_to(WindowSize::Large);
    to_units(&mut ui);
    rename_aud_to_aux(&mut ui);
    ui.settle_writes();
    assert_eq!(ui.unit_codes()[0], "AUX", "the rename landed");

    arm.arm();
    ui.press("u");

    assert_eq!(ui.unit_codes()[0], "AUX", "read back as stored");
    assert_eq!(undo_labels(&mut ui), ["Edit unit AUX"], "still to undo");
    assert_eq!(
        status_message(&mut ui).as_deref(),
        Some("Couldn't save that, so the Ledger is back as last saved")
    );
    assert!(visible_toasts(&mut ui).contains(&(
        "Error".to_string(),
        "Couldn't save the change to units, so they are back as last saved".to_string()
    )));
}

#[gpui::test]
fn closing_the_ledger_leaves_only_configuration_on_history(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);
    ui.resize_to(WindowSize::Large);
    ui.press("g s l l");
    let configuration = undo_labels(&mut ui);
    assert_eq!(configuration.len(), 1, "a General change");
    ui.press("escape j j j j j l");
    assert_eq!(ui.settings_frame().page, "Units");
    rename_aud_to_aux(&mut ui);
    ui.press("B down enter");
    ui.press("u");
    assert_eq!(undo_labels(&mut ui).len(), 2);
    assert_eq!(ui.read(|shell| shell.history()).redo.len(), 1);

    ui.press(": c l o s e enter");

    let history = ui.read(|shell| shell.history());
    assert_eq!(
        undo_labels(&mut ui),
        configuration,
        "only the Configuration is left"
    );
    assert!(history.redo.is_empty(), "the undone Ledger change went too");
}

#[gpui::test]
fn opening_another_ledger_leaves_only_configuration_on_history(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);
    ui.resize_to(WindowSize::Large);
    ui.press("g s l l");
    let configuration = undo_labels(&mut ui);
    assert_eq!(configuration.len(), 1, "a General change");
    ui.press("escape j j j j j l");
    assert_eq!(ui.settings_frame().page, "Units");
    rename_aud_to_aux(&mut ui);
    ui.press("B down enter");
    ui.press("u");
    assert_eq!(undo_labels(&mut ui).len(), 2);
    assert_eq!(ui.read(|shell| shell.history()).redo.len(), 1);
    let folder = tempfile::tempdir().expect("a temporary directory");
    let other = folder.path().join("other.pldb");
    common::new_ledger(&other);
    ui.browse_from(folder.path());

    ui.press(": o p e n enter");
    ui.click("explorer-row-0");
    ui.click("explorer-open");

    assert_eq!(ui.ledger_path(), Some(other), "the other Ledger opened");
    let history = ui.read(|shell| shell.history());
    assert_eq!(
        undo_labels(&mut ui),
        configuration,
        "only the Configuration is left"
    );
    assert!(history.redo.is_empty(), "the undone Ledger change went too");
}

/// A Unit deleted from the middle of the list and restored by Undo returns to its place by code
/// (#1261).
#[gpui::test]
fn u_restores_a_deleted_unit_to_its_place_by_code(app: &mut TestAppContext) {
    let mut ui = on_units_unused(app);
    let before = ui.unit_codes();
    ui.press("j d");
    type_word(&mut ui, "BTC");
    ui.press("enter");
    assert_eq!(ui.unit_codes(), ["AUD", "VAS"]);

    ui.press("u");

    assert_eq!(ui.unit_codes(), before);
}

/// Adds EUR, quoted in aud, on Frankfurter, priced as the Ledger opens, so the check on open
/// leaves it alone. For [`Harness::fetching`].
#[expect(
    clippy::expect_used,
    reason = "eur was just added and its price is a literal decimal"
)]
fn eur_on_frankfurter(fakes: &mut FakeStores) {
    eur_unpriced(fakes);
    let opened = lib_core::Clock::fixed(common::today()).now();
    let eur = fakes.units.units.units.last_mut().expect("eur was added");
    eur.record_price("1.6".parse().expect("a decimal"), common::today(), opened);
}

/// Adds EUR, quoted in aud, on Frankfurter, with no price, so it is due as the Ledger opens.
fn eur_unpriced(fakes: &mut FakeStores) {
    fakes.units.units.units.push(lib_units::UnitRow::new(
        "EUR".into(),
        "Euro".into(),
        lib_core::UnitKind::Currency,
        lib_units::Precision::of_kind(lib_core::UnitKind::Currency),
        lib_units::PriceLink {
            source_id: lib_units::FRANKFURTER,
            symbol: "EUR".into(),
            quote: UnitId::SEED_AUD,
        },
    ));
}

#[gpui::test]
fn f_on_a_frankfurter_unit_asks_the_feed_and_its_answer_writes_the_price_with_a_toast(
    app: &mut TestAppContext,
) {
    let (transport, responses) = lib_units::fake::FakeTransport::channel();
    let mut ui = Harness::fetching(app, (transport.clone(), responses), eur_on_frankfurter);
    ui.resize_to(WindowSize::Large);
    to_units(&mut ui);
    ui.press("j j");
    assert_eq!(selected(&mut ui), "EUR");
    assert!(
        ui.status_as_shortcuts()
            .contains(&("fetch price".into(), "f".into()))
    );

    ui.press("f");
    assert_eq!(
        transport.urls(),
        ["https://api.frankfurter.dev/v2/rate/EUR/AUD"],
        "f reached the Presenter, which sent Frankfurter v2's request"
    );

    transport.answer_last(200, r#"{"base":"EUR","quote":"AUD","rate":1.6512}"#);
    ui.advance_clock(bin_desktop::shell::router::PRICE_RESPONSE_POLL);

    let eur = ui
        .settings::<SettingsUnitsSnapshot>()
        .units
        .into_iter()
        .find(|unit| unit.code == "EUR")
        .expect("EUR is listed");
    assert_eq!(eur.last_price, "1.65 AUD");
    assert!(
        visible_toasts(&mut ui)
            .contains(&("Success".to_string(), "Fetched EUR at 1.65 AUD".to_string()))
    );
    assert!(
        undo_labels(&mut ui).is_empty(),
        "a fetched price is never undoable"
    );
}

/// A Custom source sending one fixed header, for [`one_unit_on_each_fetching_source`].
fn broker() -> lib_units::CustomSource {
    lib_units::CustomSource {
        id: lib_core::PriceSourceId::new(),
        name: "Broker".into(),
        endpoint: lib_units::Endpoint {
            address: "https://broker.example".to_string().into(),
            request: "/quote/{symbol}".to_string().into(),
            price_path: "/last".to_string().into(),
            auth: lib_units::PriceSourceAuth::None,
            headers: vec![lib_units::RequestHeader {
                name: "X-Client".to_string().into(),
                value: "ledger; v1".to_string().into(),
            }]
            .into(),
            batch: None,
            history: None,
        },
        updates: lib_units::Updates::Hourly,
    }
}

/// Adds ETH on CoinGecko, BHP on Yahoo Finance and ABC on a Custom source with a header, each
/// quoted in aud and, with BTC, priced as the Ledger opens, so the check on open leaves them
/// alone. For [`Harness::fetching`].
#[expect(clippy::expect_used, reason = "the prices are literal decimals")]
fn one_unit_on_each_fetching_source(fakes: &mut FakeStores) {
    let broker = broker();
    let opened = lib_core::Clock::fixed(common::today()).now();
    for (code, kind, source_id, symbol) in [
        (
            "ETH",
            lib_core::UnitKind::Crypto,
            lib_units::COINGECKO,
            "ethereum",
        ),
        (
            "BHP",
            lib_core::UnitKind::Equity,
            lib_units::YAHOO_FINANCE,
            "BHP.AX",
        ),
        ("ABC", lib_core::UnitKind::Equity, broker.id, "ABC"),
    ] {
        fakes.units.units.units.push(lib_units::UnitRow::new(
            code.into(),
            code.into(),
            kind,
            lib_units::Precision::of_kind(kind),
            lib_units::PriceLink {
                source_id,
                symbol: symbol.into(),
                quote: UnitId::SEED_AUD,
            },
        ));
    }
    fakes
        .units
        .price_sources
        .records
        .push(lib_units::PriceSourceRecord::Custom(broker));
    for unit in &mut fakes.units.units.units {
        unit.record_price("1".parse().expect("a decimal"), common::today(), opened);
    }
}

/// Moves the selection down to the Unit `code`.
#[expect(
    clippy::panic,
    reason = "a Unit the test needs that isn't listed should fail it"
)]
fn select_unit(ui: &mut Harness<'_>, code: &str) {
    for _ in 0..10 {
        if selected(ui) == code {
            return;
        }
        ui.press("j");
    }
    panic!("{code} is not listed");
}

#[gpui::test]
fn f_on_a_coingecko_a_yahoo_and_a_custom_sources_unit_sends_their_requests_and_writes_prices(
    app: &mut TestAppContext,
) {
    let (transport, responses) = lib_units::fake::FakeTransport::channel();
    let mut ui = Harness::fetching(
        app,
        (transport.clone(), responses),
        one_unit_on_each_fetching_source,
    );
    ui.resize_to(WindowSize::Large);
    to_units(&mut ui);
    assert!(transport.requests().is_empty(), "nothing is due on open");
    let user_agent = (
        "User-Agent".to_string(),
        format!("Personal-Ledger/{}", env!("CARGO_PKG_VERSION")),
    );
    let cases = [
        (
            "ABC",
            "https://broker.example/quote/ABC",
            vec![("X-Client".to_string(), "ledger; v1".to_string())],
            r#"{"last":12.5}"#,
            "12.50 AUD",
        ),
        (
            "BHP",
            "https://query1.finance.yahoo.com/v8/finance/chart/BHP.AX?interval=1d&range=1d",
            vec![user_agent],
            r#"{"chart":{"result":[{"meta":{"regularMarketPrice":52.52}}],"error":null}}"#,
            "52.52 AUD",
        ),
        (
            "ETH",
            "https://api.coingecko.com/api/v3/simple/price?ids=ethereum&vs_currencies=aud",
            vec![],
            r#"{"ethereum":{"aud":4321.5}}"#,
            "4,321.50 AUD",
        ),
    ];
    for (code, url, headers, body, price) in cases {
        select_unit(&mut ui, code);
        ui.press("f");
        let requests = transport.requests();
        let (_, request) = requests.last().expect("f sent a request");
        assert_eq!((request.url.as_str(), &request.headers), (url, &headers));

        transport.answer_last(200, body);
        ui.advance_clock(bin_desktop::shell::router::PRICE_RESPONSE_POLL);

        assert_eq!(unit_line(&mut ui, code).1.last_price, price);
    }
}

#[gpui::test]
fn f_after_a_429_waits_out_its_retry_after_with_one_info_toast_then_the_poll_sends_it(
    app: &mut TestAppContext,
) {
    let (transport, responses) = lib_units::fake::FakeTransport::channel();
    let mut ui = Harness::fetching(
        app,
        (transport.clone(), responses),
        one_unit_on_each_fetching_source,
    );
    ui.resize_to(WindowSize::Large);
    to_units(&mut ui);
    select_unit(&mut ui, "ETH");
    ui.press("f");
    transport.answer_last_with(429, "", Some("30"));
    ui.advance_clock(bin_desktop::shell::router::PRICE_RESPONSE_POLL);

    ui.press("f");
    assert_eq!(
        transport.requests().len(),
        1,
        "held by CoinGecko's Retry-After"
    );
    let waiting = (
        "Info".to_string(),
        "Waiting for CoinGecko, 30 s\u{2026}".to_string(),
    );
    let waits = |ui: &mut Harness<'_>| {
        visible_toasts(ui)
            .into_iter()
            .filter(|(_, text)| text.starts_with("Waiting for"))
            .count()
    };
    assert!(visible_toasts(&mut ui).contains(&waiting));
    assert_eq!(
        status_message(&mut ui),
        None,
        "fetching leaves the status line"
    );

    ui.advance_now(chrono::Duration::seconds(20));
    ui.advance_clock(bin_desktop::shell::router::PRICE_RESPONSE_POLL);
    assert_eq!(waits(&mut ui), 1, "the poll never counts it down");
    assert_eq!(status_message(&mut ui), None);
    assert_eq!(transport.requests().len(), 1);

    ui.advance_now(chrono::Duration::seconds(10));
    ui.advance_clock(bin_desktop::shell::router::PRICE_RESPONSE_POLL);
    assert_eq!(
        transport.requests().len(),
        2,
        "the poll sent the paced call"
    );
    assert_eq!(waits(&mut ui), 1, "and said nothing more");

    transport.answer_last(200, r#"{"ethereum":{"aud":4321.5}}"#);
    ui.advance_clock(bin_desktop::shell::router::PRICE_RESPONSE_POLL);
    assert_eq!(unit_line(&mut ui, "ETH").1.last_price, "4,321.50 AUD");
    assert!(
        visible_toasts(&mut ui).contains(&(
            "Success".to_string(),
            "Fetched ETH at 4,321.50 AUD".to_string()
        )),
        "{:?}",
        visible_toasts(&mut ui)
    );
    assert_eq!(status_message(&mut ui), None);
}

#[gpui::test]
fn coingeckos_credit_and_yahoos_unofficial_note_show_in_the_pane(app: &mut TestAppContext) {
    let mut ui = on_units_at(app, WindowSize::XLarge);
    select_unit(&mut ui, "BTC");
    assert!(ui.is_drawn("units-pane-attribution"), "BTC is on CoinGecko");

    select_unit(&mut ui, "VAS");
    assert!(
        !ui.is_drawn("units-pane-attribution"),
        "VAS is on Manual entry"
    );
}

#[gpui::test]
fn f_on_a_manual_entry_unit_sends_nothing(app: &mut TestAppContext) {
    let (transport, responses) = lib_units::fake::FakeTransport::channel();
    let mut ui = Harness::fetching(app, (transport.clone(), responses), eur_on_frankfurter);
    ui.resize_to(WindowSize::Large);
    to_units(&mut ui);
    assert_eq!(selected(&mut ui), "AUD");
    assert!(
        !ui.status_as_shortcuts()
            .contains(&("fetch price".into(), "f".into()))
    );

    ui.press("f");

    assert!(transport.requests().is_empty());
}

#[gpui::test]
fn opening_the_ledger_fetches_a_due_unit_silently_and_the_minute_check_fetches_what_falls_due(
    app: &mut TestAppContext,
) {
    let (transport, responses) = lib_units::fake::FakeTransport::channel();
    let mut ui = Harness::fetching(app, (transport.clone(), responses), eur_unpriced);
    ui.resize_to(WindowSize::Large);
    assert_eq!(
        transport.urls(),
        ["https://api.frankfurter.dev/v2/rate/EUR/AUD"],
        "the check on open asked for eur, with no price this week; btc was priced minutes ago"
    );
    to_units(&mut ui);
    let status = ui.status();

    ui.advance_clock(bin_desktop::shell::router::PRICE_CHECK);
    assert_eq!(
        transport.requests().len(),
        1,
        "Frankfurter's round is still in flight, so the minute check skips it"
    );

    transport.answer_last(200, r#"{"base":"EUR","quote":"AUD","rate":1.6512}"#);
    ui.advance_clock(bin_desktop::shell::router::PRICE_RESPONSE_POLL);
    let eur = ui
        .settings::<SettingsUnitsSnapshot>()
        .units
        .into_iter()
        .find(|unit| unit.code == "EUR")
        .expect("EUR is listed");
    assert_eq!(eur.last_price, "1.65 AUD");
    assert!(
        visible_toasts(&mut ui).is_empty(),
        "a scheduled round raises no Toast"
    );
    assert_eq!(ui.status(), status, "nor a status-line message");

    // A day on, eur's Daily price and btc's 15-minute one have both aged past their intervals.
    ui.turn_date_to(common::date(2026, 6, 16));
    ui.advance_clock(bin_desktop::shell::router::PRICE_CHECK);

    assert_eq!(
        transport.urls()[1..],
        [
            "https://api.coingecko.com/api/v3/simple/price?ids=bitcoin&vs_currencies=aud",
            "https://api.frankfurter.dev/v2/rate/EUR/AUD",
        ],
        "the minute check reached the Presenter, one round per source"
    );
}

/// `code`'s row in the Units table, and its index.
#[expect(
    clippy::panic,
    reason = "test helper: a Unit that isn't listed should fail the test loudly"
)]
fn unit_line(ui: &mut Harness<'_>, code: &str) -> (usize, lib_units::presenter::UnitLine) {
    let Some(found) = ui
        .settings::<SettingsUnitsSnapshot>()
        .units
        .into_iter()
        .enumerate()
        .find(|(_, unit)| unit.code == code)
    else {
        panic!("{code} is not listed")
    };
    found
}

/// `name`'s row in the Price Sources table, and its index.
#[expect(
    clippy::panic,
    reason = "test helper: a source that isn't listed should fail the test loudly"
)]
fn source_line(ui: &mut Harness<'_>, name: &str) -> (usize, lib_units::presenter::SourceLine) {
    let Some(found) = ui
        .settings::<SettingsUnitsSnapshot>()
        .price_sources
        .into_iter()
        .enumerate()
        .find(|(_, source)| source.name == name)
    else {
        panic!("{name} is not listed")
    };
    found
}

/// The Last fetch fact in the docked pane's PRICE SOURCE group, if it shows one.
fn last_fetch(ui: &mut Harness<'_>) -> Option<String> {
    ui.settings::<SettingsUnitsSnapshot>()
        .pane?
        .groups
        .into_iter()
        .find_map(|group| match group {
            lib_units::presenter::PaneGroup::Source(source) => Some(source),
            _ => None,
        })?
        .facts
        .into_iter()
        .find(|fact| fact.label == "Last fetch")
        .map(|fact| fact.value)
}

#[gpui::test]
fn f_answered_not_found_warns_and_flags_the_row_the_history_and_last_fetch(
    app: &mut TestAppContext,
) {
    let (transport, responses) = lib_units::fake::FakeTransport::channel();
    let mut ui = Harness::fetching(app, (transport.clone(), responses), eur_on_frankfurter);
    ui.resize_to(WindowSize::Large);
    to_units(&mut ui);
    ui.press("j j");
    assert_eq!(selected(&mut ui), "EUR");

    ui.press("f");
    transport.answer_last(404, r#"{"message":"not found"}"#);
    ui.advance_clock(bin_desktop::shell::router::PRICE_RESPONSE_POLL);

    assert!(visible_toasts(&mut ui).contains(&(
        "Warning".to_string(),
        "Couldn't fetch EUR: Frankfurter has no EUR in AUD".to_string()
    )));
    let (index, eur) = unit_line(&mut ui, "EUR");
    assert_eq!(eur.fetch_failed_tag.as_deref(), Some("fetch failed"));
    assert_eq!(eur.last_price, "1.60 AUD", "the last price stays");
    assert!(ui.is_drawn(&format!("unit-fetch-failed-{index}")));
    assert_eq!(history(&mut ui).warning.as_deref(), Some("⚠ fetch failed"));
    assert!(ui.is_drawn("units-history-warning"));

    ui.resize_to(WindowSize::XLarge);
    assert_eq!(
        last_fetch(&mut ui).as_deref(),
        Some(
            "Failed just now · Not found: Frankfurter has no EUR in AUD · tries again in 24 hours"
        )
    );
}

#[gpui::test]
fn a_scheduled_round_that_fails_raises_nothing_and_shows_its_reason_in_last_updated(
    app: &mut TestAppContext,
) {
    let (transport, responses) = lib_units::fake::FakeTransport::channel();
    let mut ui = Harness::fetching(app, (transport.clone(), responses), eur_unpriced);
    ui.resize_to(WindowSize::Large);
    to_units(&mut ui);

    let (request, _) = transport.requests()[0].clone();
    transport.answer(
        request,
        lib_units::TransportOutcome::Failed(lib_units::TransportError::Offline),
    );
    ui.advance_clock(bin_desktop::shell::router::PRICE_RESPONSE_POLL);

    assert!(
        visible_toasts(&mut ui).is_empty(),
        "a scheduled failure raises no Toast"
    );
    let (index, frankfurter) = source_line(&mut ui, "Frankfurter");
    assert_eq!(frankfurter.last_updated, "Offline");
    assert!(frankfurter.failed);
    assert!(ui.is_drawn(&format!("price-source-failed-{index}")));
    assert_eq!(
        unit_line(&mut ui, "EUR").1.fetch_failed_tag.as_deref(),
        Some("fetch failed")
    );

    ui.advance_clock(bin_desktop::shell::router::PRICE_CHECK);
    assert_eq!(
        transport.requests().len(),
        1,
        "Frankfurter backs off, so the minute check leaves eur alone"
    );
}

#[gpui::test]
fn a_price_past_twice_its_interval_is_out_of_date_in_the_row_and_the_history(
    app: &mut TestAppContext,
) {
    let mut ui = on_units(app);
    ui.press("j");
    assert_eq!(selected(&mut ui), "BTC");
    assert!(!unit_line(&mut ui, "BTC").1.out_of_date);

    // A day on, btc's 15-minute price is long past its 30 minutes.
    ui.turn_date_to(common::date(2026, 6, 16));

    let (index, btc) = unit_line(&mut ui, "BTC");
    assert!(btc.out_of_date);
    assert!(ui.is_drawn(&format!("unit-last-age-{index}")));
    assert!(!unit_line(&mut ui, "VAS").1.out_of_date, "Manual entry");
    assert_eq!(history(&mut ui).warning.as_deref(), Some("⚠ Out of date"));
    assert!(ui.is_drawn("units-history-warning"));
}

/// The PRICE HISTORY the page shows.
#[expect(
    clippy::panic,
    reason = "test helper: a page with no PRICE HISTORY should fail the test loudly"
)]
fn history(ui: &mut Harness<'_>) -> lib_units::presenter::PriceHistory {
    let Some(history) = ui.settings::<SettingsUnitsSnapshot>().history else {
        panic!("the page shows no PRICE HISTORY")
    };
    history
}

/// The PRICE HISTORY the page shows, by its Unit's code.
fn history_unit(ui: &mut Harness<'_>) -> String {
    let history = history(ui);
    ui.settings::<SettingsUnitsSnapshot>()
        .units
        .iter()
        .find(|unit| unit.id == history.unit)
        .map(|unit| unit.code.clone())
        .unwrap_or_default()
}

#[gpui::test]
fn at_large_price_history_draws_the_selected_units_weeks_scrolling_within_four_rows(
    app: &mut TestAppContext,
) {
    let mut ui = on_units(app);
    ui.press("j");
    assert_eq!(history_unit(&mut ui), "BTC");

    let history = history(&mut ui);
    assert_eq!(history.subtitle, "BTC · CoinGecko");
    assert_eq!(history.rows.len(), 12);
    assert!(ui.is_drawn("units-history"));
    assert!(ui.is_drawn("units-history-table"));
    assert!(ui.is_drawn("units-history-flag"));
    assert!(ui.is_drawn("units-history-provisional"));
    let viewport = ui.bounds("units-history-rows").size.height;
    let row = ui.bounds("units-history-row-0").size.height;
    assert_eq!(
        viewport,
        row * 4.0,
        "Large shows 4 rows and scrolls the rest"
    );
    assert!(!ui.is_drawn("units-history-empty"));
}

#[gpui::test]
fn at_medium_price_history_shows_ten_rows(app: &mut TestAppContext) {
    let mut ui = on_units_at(app, WindowSize::Medium);
    ui.press("j");

    let viewport = ui.bounds("units-history-rows").size.height;
    let row = ui.bounds("units-history-row-0").size.height;
    assert_eq!(viewport, row * 10.0);
}

#[gpui::test]
fn price_history_follows_the_units_and_keeps_the_last_while_a_price_source_is_selected(
    app: &mut TestAppContext,
) {
    let mut ui = on_units(app);
    assert_eq!(history_unit(&mut ui), "AUD");
    ui.press("j");
    assert_eq!(history_unit(&mut ui), "BTC");

    // `tab` and `j/k` move between the tables' rows, never into the history.
    ui.press("tab");
    assert_eq!(selected(&mut ui), "CoinGecko");
    assert_eq!(history_unit(&mut ui), "BTC");
    ui.press("j");
    assert_eq!(history_unit(&mut ui), "BTC");

    ui.press("tab enter");
    assert_eq!(selected(&mut ui), "BTC");
    assert!(
        ui.settings::<Option<bin_desktop::shell::router::UnitPricesSnapshot>>()
            .is_some(),
        "enter on a Unit still opens its prices view"
    );
}

#[gpui::test]
fn price_history_says_why_the_base_and_an_unpriced_unit_have_none(app: &mut TestAppContext) {
    let mut ui = on_units(app);
    let base = history(&mut ui);
    assert_eq!(
        base.empty.as_deref(),
        Some("AUD is the base unit: everything is valued in it, so it has no price.")
    );
    assert!(ui.is_drawn("units-history-empty"));
    assert!(!ui.is_drawn("units-history-table"));

    ui.press("j j");
    let vas = history(&mut ui);
    assert_eq!(
        vas.empty.as_deref(),
        Some("VAS has no prices yet. A fetch or backfill will fill this.")
    );
    assert!(ui.is_drawn("units-history-empty"));
}

#[gpui::test]
fn at_x_large_price_history_moves_into_the_pane_with_a_sparkline(app: &mut TestAppContext) {
    let mut ui = on_units_at(app, WindowSize::XLarge);
    ui.press("j");

    assert!(!ui.is_drawn("units-history"), "no section on the page");
    assert!(ui.is_drawn("units-pane-history"));
    assert!(ui.is_drawn("units-history-sparkline"));
    assert!(ui.is_drawn("units-history-pane-row-0"));
    let history = history(&mut ui);
    assert_eq!(
        history.pane_hint.as_deref(),
        Some("enter opens all 52 weeks")
    );
}

#[gpui::test]
fn f_on_a_price_source_row_fetches_its_units_with_a_start_toast_and_fetching_rows(
    app: &mut TestAppContext,
) {
    let (transport, responses) = lib_units::fake::FakeTransport::channel();
    let mut ui = Harness::fetching(app, (transport.clone(), responses), eur_on_frankfurter);
    ui.resize_to(WindowSize::Large);
    to_units(&mut ui);
    ui.press("tab j");
    assert_eq!(selected(&mut ui), "Frankfurter");
    assert!(
        ui.status_as_shortcuts()
            .contains(&("fetch price".into(), "f".into()))
    );

    ui.press("f");

    assert_eq!(
        transport.urls(),
        ["https://api.frankfurter.dev/v2/rate/EUR/AUD"],
        "a fresh price and Manual only are no reason to skip"
    );
    assert!(visible_toasts(&mut ui).contains(&(
        "Info".to_string(),
        "Fetching prices from Frankfurter\u{2026}".to_string()
    )));
    assert_eq!(
        ui.read(bin_desktop::shell::router::Root::status_message),
        None,
        "the start Toast says it, not the status line"
    );
    assert_eq!(
        unit_line(&mut ui, "EUR").1.last_age.as_deref(),
        Some("Fetching\u{2026}")
    );
    assert_eq!(
        source_line(&mut ui, "Frankfurter").1.last_updated,
        "Fetching\u{2026}"
    );

    ui.press("f");
    assert_eq!(
        transport.requests().len(),
        1,
        "a second press sends nothing"
    );
    assert!(visible_toasts(&mut ui).contains(&(
        "Info".to_string(),
        "Already fetching prices from Frankfurter".to_string()
    )));

    transport.answer_last(200, r#"{"base":"EUR","quote":"AUD","rate":1.6512}"#);
    ui.advance_clock(bin_desktop::shell::router::PRICE_RESPONSE_POLL);

    assert!(
        visible_toasts(&mut ui)
            .contains(&("Success".to_string(), "Fetched EUR at 1.65 AUD".to_string()))
    );
    assert_eq!(
        ui.read(bin_desktop::shell::router::Root::status_message),
        None
    );
    assert_ne!(
        source_line(&mut ui, "Frankfurter").1.last_updated,
        "Fetching\u{2026}"
    );
}

#[gpui::test]
fn t_on_a_price_source_row_tests_its_first_unit_with_one_toast_writing_nothing(
    app: &mut TestAppContext,
) {
    let (transport, responses) = lib_units::fake::FakeTransport::channel();
    let mut ui = Harness::fetching(app, (transport.clone(), responses), eur_on_frankfurter);
    ui.resize_to(WindowSize::Large);
    to_units(&mut ui);
    ui.press("t");
    assert!(transport.requests().is_empty(), "AUD is priced by hand");
    ui.press("tab j");
    assert_eq!(selected(&mut ui), "Frankfurter");

    ui.press("t");
    assert_eq!(
        transport.urls(),
        ["https://api.frankfurter.dev/v2/rate/EUR/AUD"]
    );
    ui.press("t");
    assert_eq!(
        transport.requests().len(),
        1,
        "a second press sends nothing"
    );
    assert!(visible_toasts(&mut ui).contains(&(
        "Info".to_string(),
        "Already testing Frankfurter".to_string()
    )));

    transport.answer_last(200, r#"{"base":"EUR","quote":"AUD","rate":1.6512}"#);
    ui.advance_clock(bin_desktop::shell::router::PRICE_RESPONSE_POLL);

    assert!(visible_toasts(&mut ui).contains(&(
        "Info".to_string(),
        "Frankfurter works: EUR \u{b7} 1.65 AUD \u{b7} 0 ms".to_string()
    )));
    assert_eq!(
        unit_line(&mut ui, "EUR").1.last_price,
        "1.60 AUD",
        "a test writes no price"
    );
}

#[gpui::test]
fn test_with_symbol_tests_an_unsaved_custom_source_and_opens_its_response_on_no_price(
    app: &mut TestAppContext,
) {
    let (transport, responses) = lib_units::fake::FakeTransport::channel();
    let mut ui = Harness::fetching(app, (transport.clone(), responses), eur_on_frankfurter);
    ui.resize_to(WindowSize::Large);
    to_units(&mut ui);
    ui.press("tab n");
    type_word(&mut ui, "broker");
    ui.press("tab tab");
    type_word(&mut ui, "https://b.example");
    ui.press("tab");
    type_word(&mut ui, "/q");
    ui.press("tab");
    type_word(&mut ui, "/last");
    assert_eq!(
        ui.dialog_value("test_help").as_deref(),
        Some("Type a symbol to test.")
    );
    assert_eq!(ui.dialog_value("test_enabled").as_deref(), Some("false"));
    assert_eq!(
        ui.dialog_value("test_note").as_deref(),
        Some("A test writes nothing.")
    );

    // Past Headers, Authentication and the History section.
    ui.press("tab tab tab tab tab tab tab tab");
    assert_eq!(ui.dialog_value("field").as_deref(), Some("Symbol"));
    type_word(&mut ui, "abc");
    assert_eq!(
        ui.dialog_value("test_help").as_deref(),
        Some("Quoted in AUD, the base unit.")
    );
    ui.press("tab enter");
    assert_eq!(transport.urls(), ["https://b.example/q"]);
    assert_eq!(ui.dialog_value("test").as_deref(), Some("Testing\u{2026}"));

    transport.answer_last(200, r#"{"price":12.5}"#);
    ui.advance_clock(bin_desktop::shell::router::PRICE_RESPONSE_POLL);

    assert_eq!(ui.dialog(), Some("AddPriceSource"), "the form stays open");
    assert_eq!(ui.dialog_value("test_heading").as_deref(), Some("No price"));
    assert_eq!(
        ui.dialog_value("response").as_deref(),
        Some("Response \u{b7} HTTP 200")
    );
    assert_eq!(ui.dialog_value("response_open").as_deref(), Some("true"));
    ui.press("tab tab");
    assert_eq!(ui.dialog_value("field").as_deref(), Some("Response"));
    ui.press("enter");
    assert_eq!(ui.dialog_value("response_open").as_deref(), Some("false"));
    let names: Vec<String> = ui
        .settings::<SettingsUnitsSnapshot>()
        .price_sources
        .into_iter()
        .map(|source| source.name)
        .collect();
    assert!(
        !names.contains(&"broker".to_string()),
        "a test saves nothing"
    );
}

#[gpui::test]
fn a_built_ins_edit_dialog_tests_the_first_unit_it_values(app: &mut TestAppContext) {
    let (transport, responses) = lib_units::fake::FakeTransport::channel();
    let mut ui = Harness::fetching(app, (transport.clone(), responses), eur_on_frankfurter);
    ui.resize_to(WindowSize::Large);
    to_units(&mut ui);
    ui.press("tab j e");
    assert_eq!(ui.dialog(), Some("EditBuiltInPriceSource"));
    assert_eq!(
        ui.dialog_value("test").as_deref(),
        Some("Test price source")
    );
    assert_eq!(
        ui.dialog_value("test_help").as_deref(),
        Some("Prices EUR, the first unit it values.")
    );

    ui.press("tab tab enter");
    assert_eq!(
        transport.urls(),
        ["https://api.frankfurter.dev/v2/rate/EUR/AUD"]
    );
    transport.answer_last(429, "");
    ui.advance_clock(bin_desktop::shell::router::PRICE_RESPONSE_POLL);

    assert_eq!(
        ui.dialog_value("test_heading").as_deref(),
        Some("Rate limited")
    );
    assert_eq!(
        ui.dialog_value("test_detail").as_deref(),
        Some("Frankfurter asked to be called less often")
    );
    assert_eq!(
        ui.dialog_value("response").as_deref(),
        Some(""),
        "a built-in shows none"
    );
    ui.press("escape");
    assert!(
        !source_line(&mut ui, "Frankfurter")
            .1
            .last_updated
            .contains("Rate limited"),
        "a test holds no failure"
    );
}

/// As [`eur_on_frankfurter`], with an Account in EUR opened in June 2019, so every week since
/// then wants a Weekly Price.
#[expect(
    clippy::expect_used,
    reason = "eur was just added beside the sample's Accounts"
)]
fn eur_held_since_2019(fakes: &mut FakeStores) {
    eur_on_frankfurter(fakes);
    let eur = fakes.units.units.units.last().expect("eur was added").id;
    let mut account = fakes
        .accounts
        .accounts
        .accounts
        .first()
        .cloned()
        .expect("the sample has Accounts");
    account.id = lib_core::AccountId::new();
    account.name = "Euro savings".into();
    account.unit = eur;
    account.opened_at = common::date(2019, 6, 5);
    fakes.accounts.accounts.accounts.push(account);
}

/// The history requests sent, by URL.
fn history_requests(
    transport: &lib_units::fake::FakeTransport,
) -> Vec<(lib_units::RequestId, String)> {
    transport
        .requests()
        .into_iter()
        .filter(|(_, request)| request.url.contains("/v2/rates?"))
        .map(|(id, request)| (id, request.url))
        .collect()
}

/// The weeks EUR has a Weekly Price for, as stored.
fn eur_weeks(ui: &Harness<'_>) -> Vec<chrono::NaiveDate> {
    ui.stores(|stores| {
        stores
            .units
            .borrow()
            .units()
            .units()
            .into_iter()
            .find(|unit| unit.code == "EUR")
            .map(|unit| unit.prices.iter().map(|price| price.week).collect())
            .unwrap_or_default()
    })
}

#[gpui::test]
fn a_unit_with_accounts_opened_in_2019_fills_its_empty_weeks_back_to_then_on_open(
    app: &mut TestAppContext,
) {
    let (transport, responses) = lib_units::fake::FakeTransport::channel();
    let mut ui = Harness::fetching(app, (transport.clone(), responses), eur_held_since_2019);
    ui.resize_to(WindowSize::Large);
    to_units(&mut ui);
    select_unit(&mut ui, "EUR");

    let asked = history_requests(&transport);
    assert_eq!(
        asked
            .iter()
            .map(|(_, url)| url.as_str())
            .collect::<Vec<_>>(),
        ["https://api.frankfurter.dev/v2/rates?base=EUR&quotes=AUD&from=2019-06-03&to=2026-06-14"],
        "the open asked for every week from June 2019 to last week"
    );
    assert_eq!(
        history(&mut ui).backfill.as_deref(),
        Some("Fetching history\u{2026}")
    );
    assert!(ui.is_drawn("units-history-backfill"));

    let rates: Vec<String> = (1..=12)
        .map(|day| {
            format!(
                r#"{{"date":"2026-06-{day:02}","base":"EUR","quote":"AUD","rate":1.6{day:02}}}"#
            )
        })
        .collect();
    transport.answer(
        asked[0].0,
        lib_units::TransportOutcome::Answered {
            status: 200,
            body: format!("[{}]", rates.join(",")),
            retry_after: None,
        },
    );
    ui.advance_clock(bin_desktop::shell::router::PRICE_RESPONSE_POLL);

    assert_eq!(
        eur_weeks(&ui),
        [
            common::date(2026, 6, 1),
            common::date(2026, 6, 8),
            common::date(2026, 6, 15)
        ],
        "the weeks the source had, beside this week's fetched price"
    );
    assert_eq!(history(&mut ui).backfill, None);
    assert!(
        visible_toasts(&mut ui).is_empty(),
        "a backfill never toasts"
    );
}

#[gpui::test]
fn a_failed_backfill_says_so_in_place_and_raises_no_toast(app: &mut TestAppContext) {
    let (transport, responses) = lib_units::fake::FakeTransport::channel();
    let mut ui = Harness::fetching(app, (transport.clone(), responses), eur_held_since_2019);
    ui.resize_to(WindowSize::Large);
    to_units(&mut ui);
    select_unit(&mut ui, "EUR");

    let (id, _) = history_requests(&transport)[0].clone();
    transport.answer(
        id,
        lib_units::TransportOutcome::Failed(lib_units::TransportError::Offline),
    );
    ui.advance_clock(bin_desktop::shell::router::PRICE_RESPONSE_POLL);

    assert_eq!(
        history(&mut ui).backfill.as_deref(),
        Some("History failed \u{b7} Offline \u{b7} tries again in 10 minutes")
    );
    assert!(ui.is_drawn("units-history-backfill"));
    assert!(visible_toasts(&mut ui).is_empty(), "never a Toast");
    let (_, eur) = unit_line(&mut ui, "EUR");
    assert_eq!(eur.fetch_failed_tag, None, "never fetch failed");
}

#[gpui::test]
fn a_custom_sources_history_request_is_typed_and_tested_after_its_price(app: &mut TestAppContext) {
    let (transport, responses) = lib_units::fake::FakeTransport::channel();
    let mut ui = Harness::fetching(app, (transport.clone(), responses), eur_on_frankfurter);
    ui.resize_to(WindowSize::Large);
    to_units(&mut ui);
    ui.press("tab n");
    type_word(&mut ui, "broker");
    ui.press("tab tab");
    type_word(&mut ui, "https://b.example");
    ui.press("tab");
    type_word(&mut ui, "/q");
    ui.press("tab");
    type_word(&mut ui, "/last");
    ui.press("tab tab tab");
    assert_eq!(ui.dialog_value("field").as_deref(), Some("HistoryRequest"));
    type_word(&mut ui, "/h?f={from}");
    assert_eq!(
        ui.dialog_value("test_help").as_deref(),
        Some("Fill in Date path to test.")
    );
    assert!(ui.is_drawn("price-source-history"));
    ui.press("tab right");
    assert_eq!(
        ui.dialog_value("series_shape").as_deref(),
        Some("Keyed by date")
    );
    ui.press("left tab");
    type_word(&mut ui, "/data");
    ui.press("tab");
    type_word(&mut ui, "/d");
    ui.press("tab");
    type_word(&mut ui, "/c");
    ui.press("tab");
    type_word(&mut ui, "abc");
    ui.press("tab enter");
    transport.answer_last(200, r#"{"last":12.5}"#);
    ui.advance_clock(bin_desktop::shell::router::PRICE_RESPONSE_POLL);

    assert_eq!(
        transport.urls().last().map(String::as_str),
        Some("https://b.example/h?f=2026-06-02"),
        "the last 14 days, once the price passed"
    );
    assert_eq!(ui.dialog_value("test").as_deref(), Some("Testing\u{2026}"));
    transport.answer_last(
        200,
        r#"{"data":[{"d":1780876800,"c":1.2},{"d":"2026-06-09T10:00:00Z","c":"1.2345"}]}"#,
    );
    ui.advance_clock(bin_desktop::shell::router::PRICE_RESPONSE_POLL);

    assert_eq!(
        ui.dialog_value("test_heading").as_deref(),
        Some("Price found \u{b7} 0 ms")
    );
    assert_eq!(
        ui.dialog_value("test_history").as_deref(),
        Some("History \u{b7} 2 points \u{b7} newest 2026-06-09 \u{b7} 1.2345")
    );
    assert_eq!(
        ui.dialog_value("history_response").as_deref(),
        Some("History response \u{b7} HTTP 200")
    );
    assert!(ui.is_drawn("price-source-test-history"));
    assert_eq!(ui.dialog_value("response_open").as_deref(), Some("false"));

    // The test left focus on Symbol; back one is the Close path, where Enter saves.
    ui.press("shift-tab");
    assert_eq!(
        ui.dialog_value("field").as_deref(),
        Some("HistoryPricePath")
    );
    ui.press("enter");
    assert_eq!(ui.dialog(), None, "saved");
    let names: Vec<String> = ui
        .settings::<SettingsUnitsSnapshot>()
        .price_sources
        .into_iter()
        .map(|source| source.name)
        .collect();
    assert!(names.contains(&"broker".to_string()));
}

#[gpui::test]
fn an_empty_history_opens_the_test_response_by_itself(app: &mut TestAppContext) {
    let (transport, responses) = lib_units::fake::FakeTransport::channel();
    let mut ui = Harness::fetching(app, (transport.clone(), responses), eur_on_frankfurter);
    ui.resize_to(WindowSize::Large);
    to_units(&mut ui);
    ui.press("tab n");
    type_word(&mut ui, "broker");
    ui.press("tab tab");
    type_word(&mut ui, "https://b.example");
    ui.press("tab");
    type_word(&mut ui, "/q");
    ui.press("tab");
    type_word(&mut ui, "/last");
    ui.press("tab tab tab");
    type_word(&mut ui, "/h");
    ui.press("tab right tab tab tab");
    type_word(&mut ui, "/c");
    ui.press("tab");
    type_word(&mut ui, "abc");
    ui.press("tab enter");
    transport.answer_last(200, r#"{"last":12.5}"#);
    ui.advance_clock(bin_desktop::shell::router::PRICE_RESPONSE_POLL);
    transport.answer_last(200, "{}");
    ui.advance_clock(bin_desktop::shell::router::PRICE_RESPONSE_POLL);

    assert_eq!(
        ui.dialog_value("test_history").as_deref(),
        Some("History \u{b7} No price \u{b7} broker gave no price for abc in AUD")
    );
    assert_eq!(ui.dialog_value("response_open").as_deref(), Some("true"));
    assert!(ui.is_drawn("price-source-history-response-body"));
}

/// Adds AAA on a Custom source whose history is a list of `[seconds, close]` pairs, AAB on one
/// keyed by date and AAC on one with no history request, each created three weeks before today.
fn custom_units(fakes: &mut FakeStores) {
    let mut source = |name: &str, history: Option<lib_units::HistoryEndpoint>| {
        lib_units::add_custom_source(
            &mut fakes.units.price_sources.records,
            name.into(),
            lib_units::Endpoint {
                address: "https://rates.example".to_string().into(),
                request: "/{symbol}".to_string().into(),
                price_path: "/price".to_string().into(),
                auth: lib_units::PriceSourceAuth::None,
                headers: Vec::new().into(),
                batch: None,
                history: history.map(lib_units::HistoryRef::from),
            },
            lib_units::Updates::Daily,
        )
        .id
    };
    let points = source(
        "Pairs",
        Some(lib_units::HistoryEndpoint {
            request: "/pairs/{symbol}?from={from_unix}&to={to_unix}"
                .to_string()
                .into(),
            series_path: "/prices".to_string().into(),
            date_path: "/0".to_string().into(),
            price_path: "/1".to_string().into(),
            shape: lib_units::SeriesShape::Points,
        }),
    );
    let keyed = source(
        "Keyed",
        Some(lib_units::HistoryEndpoint {
            request: "/keyed/{symbol}?from={from}&to={to}".to_string().into(),
            series_path: "/rates".to_string().into(),
            date_path: "".into(),
            price_path: "/{quote}".to_string().into(),
            shape: lib_units::SeriesShape::Keyed,
        }),
    );
    let plain = source("Plain", None);
    for (code, source_id) in [("AAA", points), ("AAB", keyed), ("AAC", plain)] {
        let mut unit = lib_units::UnitRow::new(
            code.into(),
            code.into(),
            lib_core::UnitKind::Custom,
            lib_units::Precision::of_kind(lib_core::UnitKind::Custom),
            lib_units::PriceLink {
                source_id,
                symbol: code.into(),
                quote: UnitId::SEED_AUD,
            },
        );
        unit.created_on = Some(common::today() - chrono::Duration::weeks(3));
        fakes.units.units.units.push(unit);
    }
}

/// The weeks Unit `code` has a Weekly Price for, as stored.
fn weeks_of(ui: &Harness<'_>, code: &str) -> Vec<chrono::NaiveDate> {
    ui.stores(|stores| {
        stores
            .units
            .borrow()
            .units()
            .units()
            .into_iter()
            .find(|unit| unit.code == code)
            .map(|unit| unit.prices.iter().map(|price| price.week).collect())
            .unwrap_or_default()
    })
}

#[gpui::test]
fn a_custom_source_backfills_from_either_shape_and_one_without_a_history_request_says_so(
    app: &mut TestAppContext,
) {
    let (transport, responses) = lib_units::fake::FakeTransport::channel();
    let mut ui = Harness::fetching(app, (transport.clone(), responses), custom_units);
    ui.resize_to(WindowSize::Large);
    to_units(&mut ui);
    let urls = transport.urls();
    assert!(
        urls.contains(&"https://rates.example/keyed/AAB?from=2026-05-25&to=2026-06-14".to_string()),
        "{urls:?}"
    );
    assert!(
        !urls
            .iter()
            .any(|url| url.contains("AAC") && url != "https://rates.example/AAC"),
        "no history request, no backfill: {urls:?}"
    );
    for (id, request) in transport.requests() {
        let body = if request.url.contains("/pairs/AAA") {
            // 8 June 2026, in Unix seconds.
            r#"{"prices":[[1780876800,3.4567]]}"#
        } else if request.url.contains("/keyed/AAB") {
            r#"{"rates":{"2026-06-09":{"AUD":"5.6789"}}}"#
        } else {
            continue;
        };
        transport.answer(
            id,
            lib_units::TransportOutcome::Answered {
                status: 200,
                body: body.into(),
                retry_after: None,
            },
        );
    }
    ui.advance_clock(bin_desktop::shell::router::PRICE_RESPONSE_POLL);

    assert_eq!(weeks_of(&ui, "AAA"), [common::date(2026, 6, 8)]);
    assert_eq!(weeks_of(&ui, "AAB"), [common::date(2026, 6, 8)]);
    select_unit(&mut ui, "AAC");
    assert_eq!(
        history(&mut ui).backfill.as_deref(),
        Some("Plain has no history request, so past weeks fill only by hand")
    );
    assert!(ui.is_drawn("units-history-backfill"));
}
