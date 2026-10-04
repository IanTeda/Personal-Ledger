//! The Settings Tracing page driven by keys: the opening level, `h`/`l` on the level radios,
//! scrolling the log box, live entries and Clear logs with its confirm. Entries are pushed into
//! the Shell's own capture, so no global subscriber is involved.

mod common;

use common::{Harness, log_entry};
use gpui::TestAppContext;
use lib_tracing::{LOG_CAPACITY, Levels, LogBuffer};
use tracing::Level;

/// Settings with the Tracing page focused.
fn focused(mut ui: Harness<'_>) -> Harness<'_> {
    ui.press("g s");
    ui.press(&["j"; 12].join(" "));
    assert_eq!(ui.settings().page, "Tracing");
    ui.press("l");
    assert!(ui.settings().page_focused);
    ui
}

/// A capture already holding one entry per level, oldest (`debug`) first.
fn one_of_each() -> LogBuffer {
    let logs = LogBuffer::new(LOG_CAPACITY);
    logs.push(log_entry(Level::DEBUG, "lib_locale", "negotiated en-AU"));
    logs.push(log_entry(
        Level::INFO,
        "bin_desktop::shell",
        "settings opened",
    ));
    logs.push(log_entry(Level::WARN, "bin_desktop::sync", "retrying push"));
    logs.push(log_entry(Level::ERROR, "lib_database", "open failed"));
    logs
}

#[gpui::test]
fn the_page_opens_on_the_configured_level(app: &mut TestAppContext) {
    let mut ui = Harness::with_logs(app, LogBuffer::new(8), Some(Levels::WARN));
    assert_eq!(ui.settings().tracing_level, "Warn");
}

#[gpui::test]
fn with_no_configured_level_the_page_opens_on_info(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);
    assert_eq!(ui.settings().tracing_level, "Info");
}

#[gpui::test]
fn the_box_lists_entries_newest_first_down_to_the_level(app: &mut TestAppContext) {
    let mut ui = Harness::with_logs(app, one_of_each(), Some(Levels::INFO));
    let page = ui.settings();
    assert_eq!(
        page.log_lines,
        [
            "database: open failed",
            "sync: retrying push",
            "shell: settings opened"
        ]
    );
    assert_eq!(page.log_hidden_count, 1);
}

#[gpui::test]
fn h_and_l_step_the_level_and_filter_at_once(app: &mut TestAppContext) {
    let mut ui = focused(Harness::with_logs(app, one_of_each(), Some(Levels::INFO)));

    ui.press("l");
    assert_eq!(ui.settings().tracing_level, "Debug");
    assert_eq!(ui.settings().log_line_count, 4);
    ui.press("l");
    assert_eq!(ui.settings().tracing_level, "Debug", "stops at the end");

    ui.press("h h h");
    let page = ui.settings();
    assert_eq!(page.tracing_level, "Error");
    assert_eq!(page.log_lines, ["database: open failed"]);
    assert!(page.page_focused, "h steps the level, not out of the page");
}

#[gpui::test]
fn escape_leaves_the_page(app: &mut TestAppContext) {
    let mut ui = focused(Harness::new(app));
    ui.press("escape");
    assert!(!ui.settings().page_focused);
}

#[gpui::test]
fn the_level_keys_are_inert_on_the_index(app: &mut TestAppContext) {
    let mut ui = focused(Harness::new(app));
    ui.press("escape");
    ui.press("l");
    assert!(ui.settings().page_focused, "l on the index opens the page");
    assert_eq!(ui.settings().tracing_level, "Info");
}

#[gpui::test]
fn new_entries_arrive_at_the_top(app: &mut TestAppContext) {
    let mut ui = focused(Harness::with_logs(app, one_of_each(), Some(Levels::INFO)));

    ui.logs
        .push(log_entry(Level::WARN, "bin_desktop::shell", "late arrival"));
    ui.settle_logs();

    let page = ui.settings();
    assert_eq!(page.log_lines[0], "shell: late arrival");
    assert_eq!(page.log_scroll_top, 0);
}

#[gpui::test]
fn entries_below_the_level_arrive_hidden(app: &mut TestAppContext) {
    let mut ui = focused(Harness::with_logs(
        app,
        LogBuffer::new(8),
        Some(Levels::WARN),
    ));

    ui.logs
        .push(log_entry(Level::DEBUG, "bin_desktop::shell", "chatter"));
    ui.settle_logs();

    let page = ui.settings();
    assert_eq!(page.log_line_count, 0);
    assert_eq!(page.log_hidden_count, 1);
}

/// A capture with more entries than the box can show at once.
fn many() -> LogBuffer {
    let logs = LogBuffer::new(LOG_CAPACITY);
    for index in 0..120 {
        logs.push(log_entry(
            Level::INFO,
            "bin_desktop::shell",
            &format!("entry {index}"),
        ));
    }
    logs
}

#[gpui::test]
fn j_and_k_scroll_the_box(app: &mut TestAppContext) {
    let mut ui = focused(Harness::with_logs(app, many(), Some(Levels::INFO)));

    ui.press(&["j"; 10].join(" "));
    let scrolled = ui.settings().log_scroll_top;
    assert!(scrolled > 0, "j scrolls down into older entries");

    ui.press(&["k"; 10].join(" "));
    assert_eq!(ui.settings().log_scroll_top, 0);
}

#[gpui::test]
fn shift_j_pages_and_shift_g_reaches_the_oldest(app: &mut TestAppContext) {
    let mut ui = focused(Harness::with_logs(app, many(), Some(Levels::INFO)));

    ui.press("shift-j");
    let paged = ui.settings().log_scroll_top;
    assert!(paged > 1, "a page is more than one line");

    ui.press("shift-g");
    assert!(ui.settings().log_scroll_top > paged);
    ui.press("shift-k");
    assert!(ui.settings().log_scroll_top < 119);
}

#[gpui::test]
fn a_scrolled_reader_stays_put_as_entries_arrive(app: &mut TestAppContext) {
    let mut ui = focused(Harness::with_logs(app, many(), Some(Levels::INFO)));
    ui.press("j j j j j j");
    let before = ui.settings().log_scroll_top;
    assert!(before > 0);

    for _ in 0..3 {
        ui.logs
            .push(log_entry(Level::INFO, "bin_desktop::shell", "newer"));
    }
    ui.settle_logs();

    assert_eq!(
        ui.settings().log_scroll_top,
        before + 3,
        "anchored on the same entry"
    );
}

#[gpui::test]
fn c_asks_before_clearing_and_escape_keeps_the_logs(app: &mut TestAppContext) {
    let mut ui = focused(Harness::with_logs(app, one_of_each(), Some(Levels::INFO)));

    ui.press("c");
    assert_eq!(ui.settings().dialog.as_deref(), Some("ClearLogs"));
    ui.press("escape");

    let page = ui.settings();
    assert_eq!(page.dialog, None);
    assert_eq!(page.log_line_count, 3);
}

#[gpui::test]
fn enter_in_the_confirm_clears_every_entry(app: &mut TestAppContext) {
    let mut ui = focused(Harness::with_logs(app, one_of_each(), Some(Levels::INFO)));

    ui.press("c enter");

    let page = ui.settings();
    assert_eq!(page.dialog, None);
    assert_eq!(page.log_line_count, 0);
    assert_eq!(page.log_hidden_count, 0);
    assert!(
        ui.logs.snapshot().is_empty(),
        "the capture itself is emptied"
    );
    let toasts = ui.toasts();
    assert_eq!(
        toasts.visible.last().map(|t| t.text.as_str()),
        Some("Logs cleared")
    );
}

#[gpui::test]
fn entries_after_a_clear_still_arrive(app: &mut TestAppContext) {
    let mut ui = focused(Harness::with_logs(app, one_of_each(), Some(Levels::INFO)));
    ui.press("c enter");

    ui.logs
        .push(log_entry(Level::ERROR, "bin_desktop::shell", "after"));
    ui.settle_logs();

    assert_eq!(ui.settings().log_lines, ["shell: after"]);
}
