//! The **Tracing (Logs)** Settings page (ADR-0037, and its amendment's one folder per page): the
//! live log capture. [`TracingPage`] is `lib-settings`' Tracing Presenter in a gpui Entity, with
//! the virtualised list that shows its rows, which the Presenter's Router keeps in step
//! (`super::router::TracingRouter`). The Settings View hosts it as any other page: it looks up each
//! key the Shell passes on in the Tracing keymap, turns clicks into events through `PageClicks`
//! and draws the view model (`render.rs`). The capture is read through the `TracingLog` port
//! (`log.rs`); the capture feed (`shell/router/log_feed.rs`) signals it when entries arrive. Clear
//! logs is a module Dialog (`dialogs/`) the Presenter's Router hosts. The page has no form: the
//! level radios filter the mirror at once.

pub mod dialogs;
pub mod log;
mod render;

use std::rc::Rc;

use gpui::{App, Context, EventEmitter, ListOffset, ListState, Pixels, px};
use lib_settings::keymap::tracing_keymap;
use lib_settings::{
    LogChange, LogListChange, LogScroll, TracingEvent, TracingLevel, TracingLog, TracingPresenter,
    TracingSetup, TracingViewModel,
};
use lib_shell::Hint;
use lib_viper::dialog::DialogKeys;
use lib_viper::keymap::KeyBindingConfig;

use crate::{
    settings::{
        SettingsPage,
        pages::{Page, PageRowClicked},
        router::TracingRouter,
        snapshot::PageSnapshot,
    },
    shell::view::page_clicks::PresenterPage,
    view::event::ViewEvent,
};

/// One `j`/`k` step in the log box: a line of its 11px/1.6 monospace.
pub const LOG_LINE_STEP: Pixels = px(17.6);

impl EventEmitter<ViewEvent> for TracingPage {}

/// The page has no rows, so it never emits a row click.
impl EventEmitter<PageRowClicked> for TracingPage {}

/// The Tracing page. `log_list` is the virtualised list that shows the Presenter's visible
/// entries, and is kept in step with them by the Router the Presenter's changes go through.
pub struct TracingPage {
    presenter: TracingPresenter,
    log_list: ListState,
}

impl TracingPage {
    /// Tracing reading `log`, opening on `level`, under `config`'s keys, with the Shell's `back`
    /// on `back_chord`.
    pub fn new(
        log: Rc<dyn TracingLog>,
        level: TracingLevel,
        config: &KeyBindingConfig,
        back_chord: String,
        log_list: ListState,
    ) -> Self {
        let presenter = TracingPresenter::new(TracingSetup {
            log,
            keymap: tracing_keymap(config),
            back_chord,
            dialog_keys: DialogKeys::of(&lib_shell::dialog_keymap(config)),
            level,
        });
        log_list.reset(presenter.visible_count());
        Self {
            presenter,
            log_list,
        }
    }

    /// The page's keys under `config`'s overrides.
    pub fn set_keymaps(&mut self, config: &KeyBindingConfig, back_chord: String) {
        self.presenter.set_keymap(
            tracing_keymap(config),
            back_chord,
            DialogKeys::of(&lib_shell::dialog_keymap(config)),
        );
    }

    pub fn view_model(&self) -> TracingViewModel {
        self.presenter.view_model()
    }

    pub fn log_list(&self) -> &ListState {
        &self.log_list
    }

    /// Reads `log` from now on, opening on `level` and restarting the list at its top.
    pub fn configure(
        &mut self,
        log: Rc<dyn TracingLog>,
        level: TracingLevel,
        cx: &mut Context<'_, Self>,
    ) {
        self.presenter.configure(log, level);
        self.log_list.reset(self.presenter.visible_count());
        cx.notify();
    }

    /// The capture gained entries: pulls them in. Not an input, since nothing here writes, and it
    /// redraws only when the rows changed, or while none show, since the empty note counts the
    /// hidden ones; entries the level hides redraw nothing else.
    pub fn appended(&mut self, cx: &mut Context<'_, Self>) {
        if self.route(TracingEvent::Appended, cx) || self.presenter.visible_count() == 0 {
            cx.notify();
        }
    }

    /// Clear logs was confirmed, so the capture is empty.
    pub fn cleared(&mut self, cx: &mut Context<'_, Self>) {
        self.send(TracingEvent::Cleared, cx);
    }

    /// A key, click or command as one input, committed once it is handled; one sent inside another
    /// input (a Dialog's) runs in it.
    fn send(&mut self, event: TracingEvent, cx: &mut Context<'_, Self>) {
        crate::input::join_or_send(cx, |cx| self.handle(event, cx));
    }

    fn handle(&mut self, event: TracingEvent, cx: &mut Context<'_, Self>) {
        self.route(event, cx);
        cx.notify();
    }

    /// Hands `event` to the Presenter; whether it changed the log list.
    fn route(&mut self, event: TracingEvent, cx: &mut Context<'_, Self>) -> bool {
        let mut router = TracingRouter {
            list: &self.log_list,
            cx,
            list_changed: false,
        };
        self.presenter.handle(event, &mut router);
        router.list_changed
    }
}

/// The page's clicks on its level radios and Clear logs.
impl PresenterPage<TracingEvent> for TracingPage {
    fn send(&mut self, event: TracingEvent, cx: &mut Context<'_, Self>) {
        TracingPage::send(self, event, cx);
    }
}

impl Page for TracingPage {
    const PAGE: SettingsPage = SettingsPage::Tracing;
    /// The log box fills the page and scrolls itself.
    const FILLS_PAGE: bool = true;

    /// The keymap's chords ahead of the frame's, so `h` steps the level rather than leaving
    /// (Display's radio grammar; `esc` leaves). Focus never enters the box itself.
    fn module_key(&mut self, chord: &str, cx: &mut Context<'_, Self>) -> bool {
        let Some(event) = self.presenter.keymap().event(chord) else {
            return false;
        };
        self.send(event, cx);
        true
    }

    /// The Presenter's legend, rebound keys included, which the `?` cheat sheet reads.
    fn hints(&self, _cx: &App) -> Vec<Hint> {
        self.presenter.view_model().hints
    }
}

/// Keeps `list` in step with the rows the Presenter shows: a level or clear restarts it, new
/// entries splice in.
pub(super) fn apply_change(list: &ListState, change: LogListChange) {
    match change {
        LogListChange::Reset(count) => list.reset(count),
        LogListChange::Spliced(change) => mirror_log_change(list, change),
    }
}

/// Mirrors a pull onto the list: evicted rows leave the bottom, new ones arrive at the top.
/// A reader at the very top keeps seeing the newest; one scrolled down stays where they are.
fn mirror_log_change(list: &ListState, change: LogChange) {
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

/// Scrolls the log box: a line, a page of its height, or to the oldest entry.
pub(super) fn scroll(list: &ListState, scroll: LogScroll) {
    use lib_list_cursor::Step;
    let page = list.viewport_bounds().size.height.max(LOG_LINE_STEP);
    match scroll {
        LogScroll::Line(Step::Next) => list.scroll_by(LOG_LINE_STEP),
        LogScroll::Line(_) => list.scroll_by(-LOG_LINE_STEP),
        LogScroll::Page(Step::Next) => list.scroll_by(page),
        LogScroll::Page(_) => list.scroll_by(-page),
        LogScroll::Oldest => {
            if let Some(last) = list.item_count().checked_sub(1) {
                list.scroll_to_reveal_item(last);
            }
        }
    }
}

impl PageSnapshot for SettingsTracingSnapshot {
    type Page = TracingPage;

    /// The Tracing page's state.
    fn read(page: &TracingPage, _cx: &App) -> Self {
        let view = page.presenter.view_model();
        SettingsTracingSnapshot {
            tracing_level: format!("{:?}", view.level),
            log_line_count: view.entries.len(),
            log_lines: view.entries.iter().map(|entry| entry.body()).collect(),
            log_hidden_count: view.hidden,
            log_scroll_top: page.log_list.logical_scroll_top().item_ix,
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
