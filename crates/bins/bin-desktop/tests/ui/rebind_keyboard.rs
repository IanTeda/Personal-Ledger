//! A `[keybindings]` rebind and a Display change reach every page `Root` hosts through the one
//! hosting trait (#1167): every main View's and every Settings page's keys show a rebound chord
//! (an info page's in its legend), iterated rather than named, and each Display reader draws with
//! a new date style, after a Settings › Display change and as a Ledger opens. State is read back
//! from `Root`.

use super::common;
use bin_desktop::shell::router::{DisplayDatesSnapshot, PageKeysSnapshot};
use common::Harness;
use gpui::TestAppContext;
use lib_core::DateStyle;
use lib_settings::SettingsPage;
use lib_viper::keymap::MOVE_DOWN;

/// The main Views `Root` hosts with a Ledger open: Dashboard, Transactions, the Import step,
/// Documents, Bills and Budgets.
const MAIN_VIEWS: usize = 6;

/// How many pages `Root` hosts with the Sample Ledger open: each main View, the Settings View, then
/// every Settings page.
fn hosted_pages() -> usize {
    MAIN_VIEWS + 1 + SettingsPage::ALL.len()
}

/// The chords `page` binds `command` to; empty for a page that doesn't bind it.
fn chords_of<'a>(page: &'a PageKeysSnapshot, command: &str) -> Vec<&'a str> {
    page.keys
        .iter()
        .filter(|(bound, _)| *bound == command)
        .map(|(_, chord)| chord.as_str())
        .collect()
}

/// Whether `page` shows `chord` as `MOVE_DOWN`'s: bound to it, or, for a page binding no key of
/// its own (an info page), in the scroll hint its legend reads from the Settings frame.
fn shows_move_down(page: &PageKeysSnapshot, chord: &str) -> bool {
    let bound = chords_of(page, MOVE_DOWN);
    if bound.is_empty() {
        let scroll = format!("{chord}/k");
        page.legend.contains(&scroll)
    } else {
        bound.iter().all(|bound| *bound == chord)
    }
}

#[gpui::test]
fn a_rebind_reaches_every_main_view_and_settings_page(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);
    let before = ui.read_app(|shell, cx| shell.page_keys_snapshot(cx));
    assert_eq!(
        before.len(),
        hosted_pages(),
        "every page is hosted: {before:?}"
    );
    for page in &before {
        assert!(shows_move_down(page, "j"), "{page:?}");
    }
    let commits = ui.commits();

    ui.rebind(&common::overriding(MOVE_DOWN, "n"));

    assert_eq!(
        ui.commits(),
        commits + 1,
        "every page hears the rebind in one input"
    );
    let after = ui.read_app(|shell, cx| shell.page_keys_snapshot(cx));
    assert_eq!(
        after.iter().map(|page| &page.page).collect::<Vec<_>>(),
        before.iter().map(|page| &page.page).collect::<Vec<_>>(),
        "the same pages are hosted"
    );
    for page in &after {
        assert!(
            shows_move_down(page, "n") && !shows_move_down(page, "j"),
            "{} shows the new chord and not the old: {page:?}",
            page.page
        );
    }
}

/// Settings › Display's date style stepped to ISO: the last of its choices.
fn choose_iso_dates(ui: &mut Harness<'_>) {
    ui.press("g s");
    ui.press("j");
    assert_eq!(ui.settings_frame().page, "Display");
    ui.press("l");
    ui.press("l l l l l");
}

fn dates(ui: &mut Harness<'_>) -> DisplayDatesSnapshot {
    ui.read_app(|shell, cx| shell.display_dates_snapshot(cx))
}

/// Whether `text` is an ISO date, `2026-09-18`.
fn is_iso(text: Option<&str>) -> bool {
    text.is_some_and(|text| {
        text.len() == 10
            && text.chars().enumerate().all(|(index, ch)| match index {
                4 | 7 => ch == '-',
                _ => ch.is_ascii_digit(),
            })
    })
}

fn assert_iso(dates: &DisplayDatesSnapshot) {
    assert!(is_iso(dates.dashboard_due.as_deref()), "{dates:?}");
    assert!(is_iso(dates.transactions_date.as_deref()), "{dates:?}");
    assert_eq!(dates.bills, Some(DateStyle::Iso));
    assert_eq!(dates.documents, Some(DateStyle::Iso));
    assert_eq!(dates.budgets, Some(DateStyle::Iso));
}

#[gpui::test]
fn a_display_change_reaches_every_display_reader(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);
    let before = dates(&mut ui);
    assert!(!is_iso(before.dashboard_due.as_deref()), "{before:?}");
    assert!(!is_iso(before.transactions_date.as_deref()), "{before:?}");

    choose_iso_dates(&mut ui);

    assert_iso(&dates(&mut ui));
}

#[gpui::test]
fn a_ledger_opens_under_the_display_preferences(app: &mut TestAppContext) {
    let mut config = lib_config::Config::default();
    config.display.date_style = Some(DateStyle::Iso);

    let mut ui = Harness::with_configuration(app, &config, &[]);

    assert_iso(&dates(&mut ui));
}
