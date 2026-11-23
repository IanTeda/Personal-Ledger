//! The Settings Data & backup page driven by keys: paging to it and the focus behaviour. State is
//! read back from `Root`.

use super::common;

use bin_desktop::WindowSize;
use bin_desktop::shell::router::{SettingsDataBackupSnapshot, SettingsUnitsSnapshot};
use common::Harness;
use gpui::TestAppContext;

fn on_page(app: &mut TestAppContext) -> Harness<'_> {
    to_page(Harness::new(app))
}

fn to_page(mut ui: Harness<'_>) -> Harness<'_> {
    ui.press("g s");
    ui.press(&["j"; 14].join(" "));
    assert_eq!(ui.settings_frame().page, "DataBackup");
    ui
}

#[gpui::test]
fn data_backup_is_the_last_page_after_sync_server(app: &mut TestAppContext) {
    let mut ui = on_page(app);

    ui.press("k");
    assert_eq!(ui.settings_frame().page, "SyncServer");
    ui.press("j j");
    assert_eq!(ui.settings_frame().page, "DataBackup");
}

#[gpui::test]
fn data_backup_takes_focus_with_l_and_escape_returns(app: &mut TestAppContext) {
    let mut ui = on_page(app);

    ui.press("l");
    assert!(ui.settings_frame().page_focused);
    ui.press("escape");
    assert!(!ui.settings_frame().page_focused);
}

#[gpui::test]
fn keys_alone_do_not_trigger_the_buttons(app: &mut TestAppContext) {
    let mut ui = on_page(app);
    ui.press("l");

    ui.press("j k enter");
    assert_eq!(ui.settings_frame().status_message, None);
}

fn encryption_key(ui: &mut Harness<'_>) -> Option<String> {
    ui.settings::<SettingsDataBackupSnapshot>()
        .value("Encryption key")
        .map(str::to_string)
}

#[gpui::test]
fn the_encryption_key_fact_reads_not_set_without_a_secret(app: &mut TestAppContext) {
    let mut ui = on_page(app);

    assert_eq!(encryption_key(&mut ui).as_deref(), Some("Not set"));
}

/// Saves CoinGecko's Demo key from the Units page, which makes this device's API key secret, and
/// leaves `ui` on the Units page, its pane docked at X-Large.
fn save_coingeckos_key(ui: &mut Harness<'_>) {
    ui.resize_to(WindowSize::XLarge);
    ui.press("g s j j j j j l tab");
    for _ in 0..5 {
        if coingecko_selected(ui) {
            break;
        }
        ui.press("j");
    }
    assert!(coingecko_selected(ui));
    ui.press("e tab tab");
    ui.press("C G");
    ui.press("enter");
    assert_eq!(ui.dialog(), None);
    assert_eq!(api_key_fact(ui).as_deref(), Some("Saved"));
}

fn coingecko_selected(ui: &mut Harness<'_>) -> bool {
    ui.settings::<SettingsUnitsSnapshot>()
        .price_sources
        .iter()
        .any(|source| source.selected && source.name == "CoinGecko")
}

fn api_key_fact(ui: &mut Harness<'_>) -> Option<String> {
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
        .find_map(|fact| (fact.label == "API key").then_some(fact.value))
}

#[gpui::test]
fn the_encryption_key_fact_shows_the_secrets_fingerprint_never_the_secret(
    app: &mut TestAppContext,
) {
    let mut ui = Harness::new(app);
    save_coingeckos_key(&mut ui);

    ui.press("escape");
    ui.press(&["j"; 9].join(" "));
    assert_eq!(ui.settings_frame().page, "DataBackup");

    let fingerprint = encryption_key(&mut ui).expect("the fact is drawn");
    assert_eq!(fingerprint.len(), 9, "{fingerprint}");
    assert_eq!(fingerprint.as_bytes()[4], b'-', "{fingerprint}");
    assert!(!ui.conf().contains(&fingerprint));
}

#[gpui::test]
fn removing_all_api_keys_deletes_every_key_so_sources_read_not_set(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);
    save_coingeckos_key(&mut ui);

    ui.press("escape");
    ui.press(&["j"; 9].join(" "));
    assert_eq!(ui.settings_frame().page, "DataBackup");
    ui.press("l x");
    assert_eq!(ui.dialog(), Some("RemoveApiKeys"));
    ui.press("enter");
    assert_eq!(ui.dialog(), None);
    assert_eq!(
        ui.toast_texts().first().map(String::as_str),
        Some("API keys removed")
    );

    ui.press("escape");
    ui.press(&["k"; 9].join(" "));
    ui.press("l");
    assert_eq!(api_key_fact(&mut ui).as_deref(), Some("Not set"));
}

#[gpui::test]
fn x_asks_first_and_escape_keeps_the_keys(app: &mut TestAppContext) {
    let mut ui = on_page(app);
    ui.press("l");

    ui.press("x");
    assert_eq!(ui.dialog(), Some("RemoveApiKeys"));
    ui.press("escape");

    assert_eq!(ui.dialog(), None);
    assert!(ui.toast_texts().is_empty());
}

#[gpui::test]
fn x_then_enter_removes_every_key_and_says_so(app: &mut TestAppContext) {
    let mut ui = on_page(app);
    ui.press("l");

    ui.press("x");
    assert_eq!(ui.dialog(), Some("RemoveApiKeys"));
    ui.press("enter");

    assert_eq!(ui.dialog(), None);
    assert_eq!(
        ui.toast_texts().first().map(String::as_str),
        Some("API keys removed")
    );
}
