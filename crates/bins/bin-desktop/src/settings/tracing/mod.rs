//! The **Tracing (Logs)** Settings page (ADR-0037, and its amendment's one folder per page): the
//! live log capture. [`TracingPage`] owns the log mirror and the virtualised list that shows it,
//! renders itself (`render.rs`), builds the Clear logs Dialog (`dialogs/`) and reaches `Shell`
//! only through [`ViewEvent`]. Its Dialog draws and confirms itself through the Dialog host, and
//! confirms through `apply.rs`; the page takes the outcome back in [`TracingPage::land`]. The
//! capture feed (`app/log_feed.rs`) pushes new entries in. The Settings View
//! forwards the page's keys and status to it while the page has focus. The page has no form: the
//! level radios filter the mirror at once.

pub mod apply;
pub mod dialogs;
pub mod log;
mod render;

use crate::settings::pages::Page;
use gpui::{App, Context, EventEmitter, Keystroke, ListOffset, ListState, Pixels, px};

use crate::{
    chrome::dialog_host::open_dialog,
    settings::{SettingsPage, hints::settings_tracing_hints, step_choice},
    view::event::ViewEvent,
};
use apply::TracingOutcome;
use dialogs::TracingDialog;
use log::{LogChange, LogView, TracingLevel};

/// One `j`/`k` step in the log box: a line of its 11px/1.6 monospace.
pub const LOG_LINE_STEP: Pixels = px(17.6);

impl EventEmitter<ViewEvent> for TracingPage {}

/// The Tracing page. `log_list` is the virtualised list that shows `log`, and must be kept in
/// step with it by [`Self::pull_log`], [`Self::replace_log`] and the edits below.
pub struct TracingPage {
    log: LogView,
    log_list: ListState,
}

impl TracingPage {
    pub fn new(log: LogView, log_list: ListState) -> Self {
        Self { log, log_list }
    }

    pub fn log(&self) -> &LogView {
        &self.log
    }

    pub fn log_list(&self) -> &ListState {
        &self.log_list
    }

    // -----------------------------------------------------------------------------------------
    // The capture feed
    // -----------------------------------------------------------------------------------------

    /// Hands the page a new capture, opening on `log`'s level, and restarts the list at its top.
    pub fn replace_log(&mut self, cx: &mut Context<'_, Self>, log: LogView) {
        self.log = log;
        self.log_list.reset(self.log.visible().len());
        cx.notify();
    }

    /// Pulls new entries from the capture and mirrors them onto the list. Notifies only when
    /// something changed, so an idle capture doesn't redraw the page.
    pub fn pull_log(&mut self, cx: &mut Context<'_, Self>) -> LogChange {
        let change = self.log.pull();
        if change != LogChange::default() {
            self.mirror_log_change(change);
            cx.notify();
        }
        change
    }

    /// Mirrors a pull onto the list: evicted rows leave the bottom, new ones arrive at the top.
    /// A reader at the very top keeps seeing the newest; one scrolled down stays where they are.
    fn mirror_log_change(&mut self, change: LogChange) {
        let list = &self.log_list;
        let top = list.logical_scroll_top();
        let at_top = top.item_ix == 0 && top.offset_in_item <= px(0.0);
        let count = list.item_count();
        let evicted = change.evicted.min(count);
        list.splice(count - evicted..count, 0);
        list.splice(0..0, change.added);
        if at_top {
            list.scroll_to(ListOffset::default());
        }
    }

    /// Edits the log mirror in place (its level and clear), then redraws. The list is reset to
    /// the mirror's visible rows, as a level or clear changes which rows show.
    fn update_log(&mut self, cx: &mut Context<'_, Self>, edit: impl FnOnce(&mut LogView)) {
        edit(&mut self.log);
        self.log_list.reset(self.log.visible().len());
        cx.notify();
    }

    /// Empties the capture, once Clear logs is confirmed.
    fn clear_log(&mut self, cx: &mut Context<'_, Self>) {
        self.update_log(cx, LogView::clear);
    }

    /// Takes a landed Dialog's outcome back: Clear logs empties the capture.
    pub fn land(&mut self, outcome: TracingOutcome, cx: &mut Context<'_, Self>) {
        match outcome {
            TracingOutcome::LogsConfirmed => self.clear_log(cx),
        }
    }
}

impl Page for TracingPage {
    const PAGE: SettingsPage = SettingsPage::Tracing;

    /// The page's keys, ahead of the focus keys so `h` steps the level rather than leaving
    /// (Display's radio grammar; `esc` leaves): `h`/`l` the level, `j`/`k` a line, `J`/`K` a
    /// page, `G` the oldest entry, `c` Clear logs. Focus never enters the box itself.
    fn key_ahead(&mut self, keystroke: &Keystroke, cx: &mut Context<'_, Self>) -> bool {
        let list = self.log_list.clone();
        let page = list.viewport_bounds().size.height.max(LOG_LINE_STEP);
        match (keystroke.modifiers.shift, keystroke.key.as_str()) {
            (false, "h" | "left") => self.step_level(-1, cx),
            (false, "l" | "right") => self.step_level(1, cx),
            (false, "j" | "down") => list.scroll_by(LOG_LINE_STEP),
            (false, "k" | "up") => list.scroll_by(-LOG_LINE_STEP),
            (true, "j") => list.scroll_by(page),
            (true, "k") => list.scroll_by(-page),
            (true, "g") => {
                if let Some(last) = list.item_count().checked_sub(1) {
                    list.scroll_to_reveal_item(last);
                }
            }
            (false, "c") => self.click_clear_logs(cx),
            _ => return false,
        }
        true
    }

    fn hints(&self, _cx: &App) -> Vec<(&'static str, String)> {
        settings_tracing_hints()
    }
}

impl TracingPage {
    // -----------------------------------------------------------------------------------------
    // Clicks
    // -----------------------------------------------------------------------------------------

    /// The level radios filter the log box at once, including what is already there, and start
    /// it back at the newest entry.
    fn set_level(&mut self, level: TracingLevel, cx: &mut Context<'_, Self>) {
        self.update_log(cx, |log| log.set_level(level));
    }

    fn step_level(&mut self, delta: isize, cx: &mut Context<'_, Self>) {
        let level = step_choice(&TracingLevel::ALL, self.log.level(), delta);
        self.set_level(level, cx);
    }

    /// **Clear logs** (and `c`) asks first, since the capture is emptied for good (#502).
    fn click_clear_logs(&mut self, cx: &mut Context<'_, Self>) {
        cx.emit(ViewEvent::SetStatus(None));
        open_dialog(TracingDialog::ClearLogs, Self::land, cx);
    }

    // -----------------------------------------------------------------------------------------
    // Tests
    // -----------------------------------------------------------------------------------------

    /// The Tracing page's state.
    #[doc(hidden)]
    pub fn snapshot(&self) -> SettingsTracingSnapshot {
        SettingsTracingSnapshot {
            tracing_level: format!("{:?}", self.log.level()),
            log_line_count: self.log.visible().len(),
            log_lines: self
                .log
                .visible()
                .iter()
                .map(|entry| log::body(entry))
                .collect(),
            log_hidden_count: self.log.hidden_count(),
            log_scroll_top: self.log_list.logical_scroll_top().item_ix,
        }
    }
}

/// The Tracing page: the chosen level (`TracingLevel` variant name), how many entries the filter
/// shows, their bodies newest first (`subsystem: message key=value`), how many it hides, and the
/// list's top row.
#[doc(hidden)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingsTracingSnapshot {
    pub tracing_level: String,
    pub log_line_count: usize,
    pub log_lines: Vec<String>,
    pub log_hidden_count: usize,
    pub log_scroll_top: usize,
}
