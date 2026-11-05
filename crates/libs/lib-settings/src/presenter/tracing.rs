//! Settings › Tracing's Presenter (issues #499–#502): the live log capture, newest first, under
//! level radios that filter it. The captured entries come through [`TracingLog`]; the Presenter
//! keeps a mirror of them and the filter over it, and tells its Router how the rows of the list
//! changed, so the Client's virtualised list only remeasures what moved. The level radios are a
//! session-only view filter that opens on the configured `log` level and never changes what is
//! captured. Clear logs is a Dialog ([`ClearLogsPresenter`]).
//!
//! The capture fills from any thread, so the Client signals [`TracingEvent::Appended`] when
//! entries arrive; the page pulls them in then.

use std::collections::VecDeque;
use std::rc::Rc;

use lib_list_cursor::Step;
use lib_viper::keymap::{Keymap, MOVE_DOWN, MOVE_UP};
use lib_viper::view::{Choice, Hint};

use super::clear_logs::ClearLogsPresenter;
use super::device::{chord, chords, step_choice};
use crate::keymap::{CLEAR_LOGS, NEXT_CHOICE, PREVIOUS_CHOICE};
use crate::log::{LogLine, TracingLevel};
use crate::msg;
use crate::protocols::{TracingLog, TracingRouter};

/// How the log box scrolls.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogScroll {
    /// A line, down (`Next`, towards older entries) or up (`Prev`).
    Line(Step),
    /// A page of the box, down or up.
    Page(Step),
    /// The oldest entry.
    Oldest,
}

/// How the visible list changed on a pull, newest first: `evicted` entries left the bottom (the
/// capture's ring overflowed) and `added` arrived at the top.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct LogChange {
    pub evicted: usize,
    pub added: usize,
}

/// What the Client does to its list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogListChange {
    /// Rows left the bottom and arrived at the top.
    Spliced(LogChange),
    /// A different set of rows shows (a level or a clear), `count` of them, from the newest.
    Reset(usize),
}

/// What Tracing is told and the person asks of it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TracingEvent {
    /// `h`/`l`: the level before or after.
    Level(Step),
    /// A click on a level radio.
    ChooseLevel(TracingLevel),
    Scroll(LogScroll),
    /// `c` or the Clear logs button: asks first, since the capture is emptied for good (#502).
    ClearLogs,
    /// The capture gained entries.
    Appended,
    /// Clear logs was confirmed, so the capture is empty.
    Cleared,
}

/// What the Client hands Tracing as it builds it.
pub struct TracingSetup {
    pub log: Rc<dyn TracingLog>,
    pub keymap: Keymap<TracingEvent>,
    /// The chord of the Shell's `back`, for the legend.
    pub back_chord: String,
    /// The radio the page opens on: the configured `log` level, clamped to the four offered.
    pub level: TracingLevel,
}

/// Tracing as the Client draws it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TracingViewModel {
    pub levels: Vec<Choice<TracingLevel>>,
    pub level: TracingLevel,
    pub clear_label: String,
    /// The entries the filter shows, newest first.
    pub entries: Rc<Vec<Rc<LogLine>>>,
    /// Captured entries the filter hides.
    pub hidden: usize,
    /// The log box's empty state, when it shows no entry.
    pub empty: Option<String>,
    /// The page's keys, for the status line and the `?` cheat sheet.
    pub hints: Vec<Hint>,
}

/// The entries the page knows about and the filter over them.
struct Mirror {
    log: Rc<dyn TracingLog>,
    capacity: usize,
    /// Oldest first, as in the capture.
    entries: VecDeque<Rc<LogLine>>,
    /// The next `seq` to pull from the capture.
    cursor: u64,
    level: TracingLevel,
    /// The filtered entries, newest first.
    visible: Rc<Vec<Rc<LogLine>>>,
}

impl Mirror {
    fn new(log: Rc<dyn TracingLog>, level: TracingLevel) -> Self {
        let mut mirror = Self {
            capacity: log.capacity(),
            log,
            entries: VecDeque::new(),
            cursor: 0,
            level,
            visible: Rc::default(),
        };
        mirror.pull();
        mirror
    }

    fn hidden(&self) -> usize {
        self.entries.len() - self.visible.len()
    }

    /// Takes in whatever the capture has gained since the last pull.
    fn pull(&mut self) -> LogChange {
        let new = self.log.since(self.cursor);
        let Some(last) = new.last() else {
            return LogChange::default();
        };
        self.cursor = last.seq + 1;
        let added = new.iter().filter(|e| self.level.admits(e.level)).count();
        self.entries.extend(new);
        let mut evicted = 0;
        while self.entries.len() > self.capacity {
            if let Some(gone) = self.entries.pop_front()
                && self.level.admits(gone.level)
            {
                evicted += 1;
            }
        }
        self.rebuild_visible();
        LogChange { evicted, added }
    }

    fn set_level(&mut self, level: TracingLevel) {
        self.level = level;
        self.rebuild_visible();
    }

    /// The capture was emptied: forget every entry. The cursor stays, since the capture's `seq`s
    /// keep counting after a clear: an entry logged between the clear and this call is still
    /// read, where moving the cursor to `next_seq` would skip it.
    fn emptied(&mut self) {
        self.entries.clear();
        self.visible = Rc::default();
    }

    fn rebuild_visible(&mut self) {
        self.visible = Rc::new(
            self.entries
                .iter()
                .rev()
                .filter(|entry| self.level.admits(entry.level))
                .cloned()
                .collect(),
        );
    }
}

/// Settings › Tracing's Presenter.
pub struct TracingPresenter {
    log: Rc<dyn TracingLog>,
    mirror: Mirror,
    keymap: Keymap<TracingEvent>,
    back_chord: String,
}

impl TracingPresenter {
    pub fn new(setup: TracingSetup) -> Self {
        Self {
            mirror: Mirror::new(setup.log.clone(), setup.level),
            log: setup.log,
            keymap: setup.keymap,
            back_chord: setup.back_chord,
        }
    }

    pub fn keymap(&self) -> &Keymap<TracingEvent> {
        &self.keymap
    }

    /// Replaces the keymap and the `back` chord, for a `[keybindings]` change.
    pub fn set_keymap(&mut self, keymap: Keymap<TracingEvent>, back_chord: String) {
        self.keymap = keymap;
        self.back_chord = back_chord;
    }

    /// Reads `log` from now on, opening on `level`. The Client restarts its list at the top, at
    /// [`Self::visible_count`].
    pub fn configure(&mut self, log: Rc<dyn TracingLog>, level: TracingLevel) {
        self.mirror = Mirror::new(log.clone(), level);
        self.log = log;
    }

    pub fn level(&self) -> TracingLevel {
        self.mirror.level
    }

    /// How many entries the filter shows.
    pub fn visible_count(&self) -> usize {
        self.mirror.visible.len()
    }

    pub fn handle(&mut self, event: TracingEvent, router: &mut dyn TracingRouter) {
        match event {
            TracingEvent::Level(step) => {
                let level = self.mirror.level;
                if let Some(next) = step_choice(&TracingLevel::ALL, &level, &level, step) {
                    self.set_level(next, router);
                }
            }
            TracingEvent::ChooseLevel(level) => self.set_level(level, router),
            TracingEvent::Scroll(scroll) => router.scroll(scroll),
            TracingEvent::ClearLogs => {
                router.open_clear_logs(ClearLogsPresenter::new(self.log.clone()));
            }
            TracingEvent::Appended => {
                let change = self.mirror.pull();
                if change != LogChange::default() {
                    router.log_changed(LogListChange::Spliced(change));
                }
            }
            TracingEvent::Cleared => {
                self.mirror.emptied();
                router.log_changed(LogListChange::Reset(0));
            }
        }
    }

    pub fn view_model(&self) -> TracingViewModel {
        let mirror = &self.mirror;
        let empty = mirror.visible.is_empty().then(|| {
            if mirror.hidden() == 0 {
                msg::settings_tracing_empty()
            } else {
                msg::settings_tracing_all_hidden(
                    &mirror.level.label(),
                    i64::try_from(mirror.hidden()).unwrap_or(i64::MAX),
                )
            }
        });
        TracingViewModel {
            levels: TracingLevel::ALL
                .into_iter()
                .map(|value| Choice {
                    value,
                    label: value.label(),
                })
                .collect(),
            level: mirror.level,
            clear_label: msg::settings_tracing_clear(),
            entries: mirror.visible.clone(),
            hidden: mirror.hidden(),
            empty,
            hints: vec![
                Hint::new(
                    chords(&self.keymap, &[PREVIOUS_CHOICE, NEXT_CHOICE]),
                    msg::settings_tracing_hint_level(),
                ),
                Hint::new(
                    chords(&self.keymap, &[MOVE_DOWN, MOVE_UP]),
                    msg::settings_tracing_hint_scroll(),
                ),
                Hint::new(
                    chord(&self.keymap, CLEAR_LOGS),
                    msg::settings_tracing_hint_clear_logs(),
                ),
                Hint::new(self.back_chord.clone(), msg::settings_hint_index()),
            ],
        }
    }

    /// A level filters the log box at once, including what is already there, and starts it back at
    /// the newest entry.
    fn set_level(&mut self, level: TracingLevel, router: &mut dyn TracingRouter) {
        self.mirror.set_level(level);
        router.log_changed(LogListChange::Reset(self.mirror.visible.len()));
    }
}

#[cfg(test)]
mod tests {
    use lib_toast::{Notice, Notify};
    use lib_viper::keymap::KeyBindingConfig;

    use super::*;
    use crate::fake::InMemoryTracingLog;
    use crate::keymap::tracing_keymap;
    use crate::log::LogLevel;

    #[derive(Default)]
    struct Recording {
        scrolls: Vec<LogScroll>,
        changes: Vec<LogListChange>,
        dialogs: usize,
    }

    impl Notify for Recording {
        fn notify(&mut self, _notice: Notice) {}
    }

    impl TracingRouter for Recording {
        fn scroll(&mut self, scroll: LogScroll) {
            self.scrolls.push(scroll);
        }

        fn log_changed(&mut self, change: LogListChange) {
            self.changes.push(change);
        }

        fn open_clear_logs(&mut self, _dialog: ClearLogsPresenter) {
            self.dialogs += 1;
        }
    }

    fn presenter(log: &Rc<InMemoryTracingLog>, level: TracingLevel) -> TracingPresenter {
        TracingPresenter::new(TracingSetup {
            log: log.clone(),
            keymap: tracing_keymap(&KeyBindingConfig::default()),
            back_chord: "esc".to_string(),
            level,
        })
    }

    fn messages(presenter: &TracingPresenter) -> Vec<String> {
        presenter
            .view_model()
            .entries
            .iter()
            .map(|entry| entry.message.clone())
            .collect()
    }

    #[test]
    fn it_opens_on_the_level_with_what_is_captured_newest_first() {
        let log = Rc::new(InMemoryTracingLog::new(8));
        log.push(LogLevel::Info, "bin_desktop::shell", "first");
        log.push(LogLevel::Debug, "bin_desktop::shell", "noise");
        log.push(LogLevel::Warn, "bin_desktop::shell", "second");

        let presenter = presenter(&log, TracingLevel::Info);

        assert_eq!(presenter.level(), TracingLevel::Info);
        assert_eq!(messages(&presenter), ["second", "first"]);
        assert_eq!(presenter.view_model().hidden, 1);
    }

    #[test]
    fn a_level_filters_at_once_and_resets_the_list() {
        let log = Rc::new(InMemoryTracingLog::new(8));
        log.push(LogLevel::Debug, "bin_desktop::shell", "noise");
        log.push(LogLevel::Error, "bin_desktop::shell", "failed");
        let mut presenter = presenter(&log, TracingLevel::Warn);
        let mut router = Recording::default();
        assert_eq!(messages(&presenter), ["failed"]);

        presenter.handle(TracingEvent::ChooseLevel(TracingLevel::Debug), &mut router);

        assert_eq!(messages(&presenter), ["failed", "noise"]);
        assert_eq!(router.changes, [LogListChange::Reset(2)]);
        assert_eq!(log.since(0).len(), 2, "the capture is untouched");
    }

    #[test]
    fn the_level_keys_step_and_stop_at_either_end() {
        let log = Rc::new(InMemoryTracingLog::new(8));
        let mut presenter = presenter(&log, TracingLevel::Info);
        let mut router = Recording::default();

        for step in [Step::Next, Step::Next, Step::Next] {
            presenter.handle(TracingEvent::Level(step), &mut router);
        }
        assert_eq!(presenter.level(), TracingLevel::Debug);

        for _ in 0..5 {
            presenter.handle(TracingEvent::Level(Step::Prev), &mut router);
        }
        assert_eq!(presenter.level(), TracingLevel::Error);
    }

    #[test]
    fn appended_reports_only_visible_additions() {
        let log = Rc::new(InMemoryTracingLog::new(8));
        let mut presenter = presenter(&log, TracingLevel::Warn);
        let mut router = Recording::default();
        log.push(LogLevel::Warn, "bin_desktop::shell", "a");
        log.push(LogLevel::Debug, "bin_desktop::shell", "b");

        presenter.handle(TracingEvent::Appended, &mut router);
        presenter.handle(TracingEvent::Appended, &mut router);

        assert_eq!(
            router.changes,
            [LogListChange::Spliced(LogChange {
                evicted: 0,
                added: 1
            })]
        );
        assert_eq!(messages(&presenter), ["a"]);
    }

    #[test]
    fn appended_drops_the_oldest_past_capacity() {
        let log = Rc::new(InMemoryTracingLog::new(2));
        let mut presenter = presenter(&log, TracingLevel::Info);
        let mut router = Recording::default();
        log.push(LogLevel::Info, "bin_desktop::shell", "a");
        presenter.handle(TracingEvent::Appended, &mut router);
        log.push(LogLevel::Info, "bin_desktop::shell", "b");
        log.push(LogLevel::Info, "bin_desktop::shell", "c");
        presenter.handle(TracingEvent::Appended, &mut router);

        assert_eq!(
            router.changes.last(),
            Some(&LogListChange::Spliced(LogChange {
                evicted: 1,
                added: 2
            }))
        );
        assert_eq!(messages(&presenter), ["c", "b"]);
    }

    #[test]
    fn cleared_empties_the_mirror_and_later_entries_still_arrive() {
        let log = Rc::new(InMemoryTracingLog::new(8));
        log.push(LogLevel::Info, "bin_desktop::shell", "old");
        let mut presenter = presenter(&log, TracingLevel::Info);
        let mut router = Recording::default();

        log.clear();
        presenter.handle(TracingEvent::Cleared, &mut router);
        assert!(messages(&presenter).is_empty());
        assert_eq!(presenter.view_model().hidden, 0);
        assert_eq!(router.changes, [LogListChange::Reset(0)]);

        log.push(LogLevel::Info, "bin_desktop::shell", "new");
        presenter.handle(TracingEvent::Appended, &mut router);
        assert_eq!(messages(&presenter), ["new"]);
    }

    #[test]
    fn an_entry_logged_between_the_clear_and_cleared_still_arrives() {
        let log = Rc::new(InMemoryTracingLog::new(8));
        log.push(LogLevel::Info, "bin_desktop::shell", "old");
        let mut presenter = presenter(&log, TracingLevel::Info);
        let mut router = Recording::default();

        log.clear();
        log.push(LogLevel::Info, "lib_database::writer", "between");
        presenter.handle(TracingEvent::Cleared, &mut router);
        presenter.handle(TracingEvent::Appended, &mut router);

        assert_eq!(messages(&presenter), ["between"]);
    }

    #[test]
    fn clear_logs_asks_the_router_for_its_dialog() {
        let log = Rc::new(InMemoryTracingLog::new(8));
        let mut presenter = presenter(&log, TracingLevel::Info);
        let mut router = Recording::default();

        presenter.handle(TracingEvent::ClearLogs, &mut router);

        assert_eq!(router.dialogs, 1);
        assert_eq!(log.since(0).len(), 0);
    }

    #[test]
    fn scrolling_goes_to_the_router() {
        let log = Rc::new(InMemoryTracingLog::new(8));
        let mut presenter = presenter(&log, TracingLevel::Info);
        let mut router = Recording::default();

        presenter.handle(TracingEvent::Scroll(LogScroll::Oldest), &mut router);

        assert_eq!(router.scrolls, [LogScroll::Oldest]);
    }

    #[test]
    fn the_empty_states_say_why() {
        let log = Rc::new(InMemoryTracingLog::new(8));
        let mut presenter = presenter(&log, TracingLevel::Error);
        assert_eq!(
            presenter.view_model().empty.as_deref(),
            Some("No log entries yet.")
        );

        log.push(LogLevel::Debug, "bin_desktop::shell", "noise");
        presenter.handle(TracingEvent::Appended, &mut Recording::default());
        assert_eq!(
            presenter.view_model().empty.as_deref(),
            Some("No entries at error or above. Choose a lower level to see 1 hidden entry.")
        );
    }

    #[test]
    fn the_legend_follows_the_keymap() {
        let log = Rc::new(InMemoryTracingLog::new(8));
        let hints = presenter(&log, TracingLevel::Info).view_model().hints;
        let shown: Vec<(&str, &str)> = hints
            .iter()
            .map(|hint| (hint.chord.as_str(), hint.action.as_str()))
            .collect();
        assert_eq!(
            shown,
            [
                ("h/l", "level"),
                ("j/k", "scroll"),
                ("c", "clear logs"),
                ("esc", "index"),
            ]
        );
    }
}
