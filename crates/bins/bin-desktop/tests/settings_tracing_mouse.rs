//! The Settings Tracing page driven by clicks: the level radios, Clear logs and its confirm, and
//! the empty states. Only elements tagged with `debug_selector` are clicked.

mod common;

use common::{Harness, log_entry};
use gpui::TestAppContext;
use lib_tracing::{LOG_CAPACITY, Levels, LogBuffer};
use tracing::Level;

fn on_page(mut ui: Harness<'_>) -> Harness<'_> {
    ui.press("g s");
    ui.click("settings-index-Tracing");
    assert_eq!(ui.settings().page, "Tracing");
    ui
}

fn captured() -> LogBuffer {
    let logs = LogBuffer::new(LOG_CAPACITY);
    logs.push(log_entry(Level::DEBUG, "lib_locale", "negotiated en-AU"));
    logs.push(log_entry(Level::WARN, "bin_desktop::sync", "retrying push"));
    logs
}

#[gpui::test]
fn clicking_a_level_radio_filters_the_box(app: &mut TestAppContext) {
    let mut ui = on_page(Harness::with_logs(app, captured(), Some(Levels::INFO)));
    assert_eq!(ui.settings().log_line_count, 1);

    ui.click("tracing-level-debug");
    assert_eq!(ui.settings().tracing_level, "Debug");
    assert_eq!(ui.settings().log_line_count, 2);
    ui.click("tracing-level-error");
    assert_eq!(ui.settings().tracing_level, "Error");
    assert_eq!(ui.settings().log_line_count, 0);
    ui.click("tracing-level-warn");
    assert_eq!(ui.settings().log_line_count, 1);
    ui.click("tracing-level-info");
    assert_eq!(ui.settings().tracing_level, "Info");
}

#[gpui::test]
fn the_box_draws_a_row_per_shown_entry(app: &mut TestAppContext) {
    let mut ui = on_page(Harness::with_logs(app, captured(), Some(Levels::DEBUG)));
    assert!(ui.is_drawn("tracing-log-box"));
    assert!(ui.is_drawn("tracing-log-row-0"));
    assert!(ui.is_drawn("tracing-log-row-1"));
    assert!(!ui.is_drawn("tracing-log-empty"));
}

#[gpui::test]
fn an_empty_capture_shows_the_empty_state(app: &mut TestAppContext) {
    let mut ui = on_page(Harness::new(app));
    assert!(ui.is_drawn("tracing-log-empty"));
    assert!(!ui.is_drawn("tracing-log-row-0"));
}

#[gpui::test]
fn a_filter_hiding_everything_shows_the_empty_state(app: &mut TestAppContext) {
    let mut ui = on_page(Harness::with_logs(app, captured(), Some(Levels::ERROR)));
    assert!(ui.is_drawn("tracing-log-empty"));
    assert_eq!(ui.settings().log_hidden_count, 2);
}

#[gpui::test]
fn clear_logs_asks_and_cancel_keeps_the_entries(app: &mut TestAppContext) {
    let mut ui = on_page(Harness::with_logs(app, captured(), Some(Levels::DEBUG)));

    ui.click("settings-clear-logs");
    assert_eq!(ui.settings().dialog.as_deref(), Some("ClearLogs"));
    ui.click("clear-logs-cancel");

    let page = ui.settings();
    assert_eq!(page.dialog, None);
    assert_eq!(page.log_line_count, 2);
}

#[gpui::test]
fn confirming_clear_logs_empties_the_box(app: &mut TestAppContext) {
    let mut ui = on_page(Harness::with_logs(app, captured(), Some(Levels::DEBUG)));

    ui.click("settings-clear-logs");
    ui.click("clear-logs-confirm");

    let page = ui.settings();
    assert_eq!(page.dialog, None);
    assert_eq!(page.log_line_count, 0);
    assert_eq!(page.tracing_level, "Debug", "clearing keeps the level");
    assert!(ui.is_drawn("tracing-log-empty"));
}
