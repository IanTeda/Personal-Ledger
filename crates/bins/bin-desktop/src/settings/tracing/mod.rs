//! The **Tracing (Logs)** Settings page (ADR-0037, and its amendment's one folder per page): the
//! live log capture. `lib-settings`' Tracing Presenter is the [`PageView`] the generic `ModulePage`
//! hosts (ADR-0056), with the virtualised list that shows its rows as the page's widget state
//! ([`LogList`]). The Presenter tells its Router (`super::router::TracingRouter`) how the rows
//! changed and where to scroll, queued in the page's [`TracingPageSetup`]; [`PageView::after`]
//! applies that to the list. The capture is read
//! through the `TracingLog` port (`log.rs`); the page subscribes to the capture feed's
//! [`LogFeed`] (`PageView::subscribe`), and each signal is a [`TracingEvent::LogChanged`] input.
//! Clear logs is a module Dialog (`dialogs/`) the Router hosts. The page has no form: the level
//! radios filter the mirror at once.

pub mod dialogs;
pub mod log;
mod render;

use std::cell::RefCell;
use std::rc::Rc;

use gpui::{
    AnyElement, App, Context, Entity, ListAlignment, ListOffset, ListState, Pixels, Subscription,
    Window, px,
};
use lib_database::Domain;
use lib_settings::keymap::tracing_keymap;
use lib_settings::{
    LogChange, LogListChange, LogScroll, TracingEvent, TracingLevel, TracingLog, TracingPresenter,
    TracingSetup, TracingViewModel,
};
use lib_viper::dialog::DialogKeys;
use lib_viper::keymap::KeyBindingConfig;

use crate::{
    settings::{SettingsPage, router::TracingRouter, snapshot::PageSnapshot},
    shell::view::module_page::{ModulePage, PageDraw, PageView, SettingsPageView},
    shell::view::statusline::PageStatus,
    view::event::ViewEvent,
};
use log::{LogAppended, LogFeed};

/// One `j`/`k` step in the log box: a line of its 11px/1.6 monospace.
pub const LOG_LINE_STEP: Pixels = px(17.6);

/// Rows measured beyond the log box's visible edge, so scrolling doesn't pop rows in.
const LOG_LIST_OVERDRAW: Pixels = px(200.0);

/// The Tracing page: the Presenter in the generic page Entity, following the capture through
/// [`LogFeed`].
pub type TracingPage = ModulePage<TracingPresenter>;

/// Tracing reading `log`, opening on `level`, under `config`'s keys, with the Shell's `back` on
/// `back_chord`, told of new entries by `feed`. Tracing reads no Ledger, so it stands before one
/// is open.
pub fn tracing_page(
    log: Rc<dyn TracingLog>,
    level: TracingLevel,
    config: &KeyBindingConfig,
    back_chord: String,
    feed: Entity<LogFeed>,
    cx: &mut Context<'_, TracingPage>,
) -> TracingPage {
    let presenter = TracingPresenter::new(TracingSetup {
        log,
        keymap: tracing_keymap(config),
        back_chord,
        dialog_keys: DialogKeys::of(&lib_shell::dialog_keymap(config)),
        level,
    });
    let setup = TracingPageSetup {
        feed,
        list_effects: ListEffects::default(),
    };
    ModulePage::detached(presenter, setup, cx)
}

/// The log box's virtualised list, the page's widget state: the Presenter never sees it.
pub struct LogList(ListState);

impl Default for LogList {
    fn default() -> Self {
        Self(ListState::new(0, ListAlignment::Top, LOG_LIST_OVERDRAW))
    }
}

/// What the Presenter asked of the list during one handle, in order.
#[derive(Debug, Clone, Copy)]
pub(super) enum LogListEffect {
    Scroll(LogScroll),
    Changed(LogListChange),
}

/// The Router's half of the hand-over to [`PageView::after`]: the Router is built over the Setup
/// and never sees the page's widget state, so it queues what the Presenter asked of the list here
/// and `after` applies it once the handle returns. Shared, since the Router borrows the Setup only
/// for the handle.
pub(super) type ListEffects = Rc<RefCell<Vec<LogListEffect>>>;

/// What Tracing's Router is built from: the capture feed the page subscribes to, and the list
/// effects the Router queues for [`PageView::after`]. Owned by the page, so each Tracing page has
/// its own queue.
pub struct TracingPageSetup {
    feed: Entity<LogFeed>,
    list_effects: ListEffects,
}

impl PageView for TracingPresenter {
    /// The capture feed the page subscribes to, and the list effects its Router queues.
    type Setup = TracingPageSetup;
    type Widgets = LogList;
    type Before = ();

    /// Tracing reads the capture, not the Ledger.
    const REPAINTS: &'static [Domain] = &[];

    /// A movement the keymap doesn't bind does nothing: the log box fills the page and scrolls
    /// itself, so the frame's body has nothing to scroll.
    const FRAME_SCROLLS: bool = false;

    /// The log box fills the page and scrolls itself.
    const FILLS_PAGE: bool = true;

    fn with_router<R>(
        presenter: &mut Self,
        setup: &TracingPageSetup,
        cx: &mut Context<'_, TracingPage>,
        handle: impl FnOnce(&mut Self, &mut Self::Router<'_>) -> R,
    ) -> R {
        let mut router = TracingRouter {
            cx,
            list_effects: &setup.list_effects,
        };
        handle(presenter, &mut router)
    }

    /// Applies what the Presenter asked of the list, in the order it asked.
    fn after(
        &self,
        _before: (),
        setup: &TracingPageSetup,
        widgets: &mut LogList,
    ) -> Option<ViewEvent> {
        for effect in setup.list_effects.take() {
            match effect {
                LogListEffect::Scroll(to) => scroll(&widgets.0, to),
                LogListEffect::Changed(change) => apply_change(&widgets.0, change),
            }
        }
        None
    }

    /// Each signal of the capture feed pulls the new entries in, as one input.
    fn subscribe(setup: &TracingPageSetup, cx: &mut Context<'_, TracingPage>) -> Vec<Subscription> {
        vec![cx.subscribe(&setup.feed, |page, _, _: &LogAppended, cx| {
            page.send(TracingEvent::LogChanged, cx);
        })]
    }

    fn render(
        view: &TracingViewModel,
        page: PageDraw<'_, Self>,
        _window: &mut Window,
        cx: &mut Context<'_, TracingPage>,
    ) -> AnyElement {
        render::page(view, page.widgets.0.clone(), page.clicks, cx)
    }

    /// The Presenter's legend, rebound keys included, which the `?` cheat sheet reads.
    fn status(view: &TracingViewModel) -> PageStatus {
        PageStatus {
            hints: view.hints.clone(),
            right: crate::settings::scope_note(SettingsPage::Tracing),
        }
    }
}

impl SettingsPageView for TracingPresenter {
    const PAGE: SettingsPage = SettingsPage::Tracing;
}

/// Reads `log` from now on, opening on `level`, with the list restarted at its top over what it
/// shows: the level is chosen afresh, which is how the Presenter restarts the list.
pub fn configure(
    page: &mut TracingPage,
    log: Rc<dyn TracingLog>,
    level: TracingLevel,
    cx: &mut Context<'_, TracingPage>,
) {
    page.update_presenter(cx, |presenter| presenter.configure(log, level));
    page.send(TracingEvent::ChooseLevel(level), cx);
}

/// Keeps `list` in step with the rows the Presenter shows: a level or clear restarts it, new
/// entries splice in.
fn apply_change(list: &ListState, change: LogListChange) {
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
fn scroll(list: &ListState, scroll: LogScroll) {
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
    type Presenter = TracingPresenter;

    /// The Tracing page's state.
    fn read(page: &TracingPage, _cx: &App) -> Self {
        let view = page.view_model();
        SettingsTracingSnapshot {
            tracing_level: format!("{:?}", view.level),
            log_line_count: view.entries.len(),
            log_lines: view.entries.iter().map(|entry| entry.body()).collect(),
            log_hidden_count: view.hidden,
            log_scroll_top: page.widgets().0.logical_scroll_top().item_ix,
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
