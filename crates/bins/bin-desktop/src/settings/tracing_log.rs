//! The Settings › Tracing page's model, `gpui`-free: a mirror of the captured [`LogEntry`]s, the
//! level filter over it, and the one-line entry format (issue #499). `Shell` pulls new entries in
//! and applies the returned [`LogChange`] to the virtualised list, so the list only remeasures
//! what moved.
//!
//! The format (`[hh:mm:ss] LEVEL subsystem: message key=value`) is diagnostic output and so not a
//! Message, per `docs/localisation-design.md`; only the empty states are.

use std::{collections::VecDeque, rc::Rc, sync::Arc, time::SystemTime};

use chrono::{DateTime, Local};
use lib_tracing::{LogBuffer, LogEntry};
use tracing::Level;

/// The 5-wide upper-case level tag, padded so messages line up.
pub fn level_tag(level: Level) -> &'static str {
    match level {
        Level::ERROR => "ERROR",
        Level::WARN => "WARN ",
        Level::INFO => "INFO ",
        Level::DEBUG => "DEBUG",
        Level::TRACE => "TRACE",
    }
}

/// The short "subsystem" a target is shown as. For our own crates it is the first module after
/// the crate (`bin_desktop::shell::toast` is `shell`), or the crate without its `bin_`/`lib_`
/// prefix at the crate root (`lib_locale` is `locale`). A dependency shows as its crate, since
/// its module names mean nothing on their own.
pub fn subsystem(target: &str) -> &str {
    let mut segments = target.split("::");
    let krate = segments.next().unwrap_or(target);
    let ours = krate
        .strip_prefix("bin_")
        .or_else(|| krate.strip_prefix("lib_"));
    match (ours, segments.next()) {
        (Some(_), Some(module)) => module,
        (Some(bare), None) => bare,
        (None, _) => krate,
    }
}

/// Everything after the level tag: `subsystem: message key=value key=value`.
pub fn body(entry: &LogEntry) -> String {
    let mut text = format!("{}: {}", subsystem(&entry.target), entry.message);
    for (name, value) in &entry.fields {
        text.push(' ');
        text.push_str(name);
        text.push('=');
        text.push_str(value);
    }
    text
}

/// `[hh:mm:ss]` in local time.
pub fn timestamp(time: SystemTime) -> String {
    DateTime::<Local>::from(time)
        .format("[%H:%M:%S]")
        .to_string()
}

/// How the visible list changed on a pull, newest first: `evicted` entries left the bottom (the
/// capture's ring overflowed) and `added` arrived at the top.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct LogChange {
    pub evicted: usize,
    pub added: usize,
}

/// The entries the page knows about and the filter over them.
pub struct LogView {
    buffer: LogBuffer,
    capacity: usize,
    /// Oldest first, as in the buffer.
    entries: VecDeque<Arc<LogEntry>>,
    /// The next `seq` to pull from the buffer.
    cursor: u64,
    level: TracingLevel,
    /// The filtered entries, newest first: what the list renders. `Rc` so a render closure can
    /// hold it without copying.
    visible: Rc<Vec<Arc<LogEntry>>>,
}

impl LogView {
    pub fn new(buffer: LogBuffer, level: TracingLevel) -> Self {
        let mut view = Self {
            capacity: buffer.capacity(),
            buffer,
            entries: VecDeque::new(),
            cursor: 0,
            level,
            visible: Rc::default(),
        };
        view.pull();
        view
    }

    pub fn level(&self) -> TracingLevel {
        self.level
    }

    pub fn visible(&self) -> Rc<Vec<Arc<LogEntry>>> {
        self.visible.clone()
    }

    /// Captured entries the filter is hiding.
    pub fn hidden_count(&self) -> usize {
        self.entries.len() - self.visible.len()
    }

    /// Takes in whatever the capture has gained since the last pull.
    pub fn pull(&mut self) -> LogChange {
        let new = self.buffer.since(self.cursor);
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

    pub fn set_level(&mut self, level: TracingLevel) {
        self.level = level;
        self.rebuild_visible();
    }

    /// Clear logs: empties the capture itself, so every reader sees it gone.
    pub fn clear(&mut self) {
        self.buffer.clear();
        self.cursor = self.buffer.next_seq();
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

/// The **Tracing (Logs)** page's level radios: the most verbose level the log box shows. A view
/// filter over what the capture already holds (it records at `debug`), so changing it is instant
/// and retroactive. Session only, not a Preference (issue #501). There is no `trace` or `off`:
/// the capture never records `trace`, and `off` would only ever show the empty state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TracingLevel {
    Error,
    Warn,
    /// `lib_tracing::init`'s own default when no `log` level is configured.
    #[default]
    Info,
    Debug,
}

impl TracingLevel {
    pub const ALL: [TracingLevel; 4] = [Self::Error, Self::Warn, Self::Info, Self::Debug];

    pub fn label(self) -> String {
        match self {
            Self::Error => crate::msg::desktop_settings_tracing_level_error(),
            Self::Warn => crate::msg::desktop_settings_tracing_level_warn(),
            Self::Info => crate::msg::desktop_settings_tracing_level_info(),
            Self::Debug => crate::msg::desktop_settings_tracing_level_debug(),
        }
    }

    /// The radio the page opens on: the configured `log` level, clamped to the four offered, so
    /// the page starts by showing what the console and log file show.
    pub fn from_configured(level: Option<lib_tracing::Levels>) -> Self {
        use lib_tracing::Levels;
        match level {
            Some(Levels::OFF | Levels::ERROR) => Self::Error,
            Some(Levels::WARN) => Self::Warn,
            Some(Levels::INFO) | None => Self::Info,
            Some(Levels::DEBUG | Levels::TRACE) => Self::Debug,
        }
    }

    /// Whether an entry at `level` shows under this filter.
    pub fn admits(self, level: Level) -> bool {
        let most_verbose = match self {
            Self::Error => Level::ERROR,
            Self::Warn => Level::WARN,
            Self::Info => Level::INFO,
            Self::Debug => Level::DEBUG,
        };
        // `tracing` orders levels by verbosity: `ERROR` is the least.
        level <= most_verbose
    }
}

#[cfg(test)]
mod tests {
    use std::borrow::Cow;

    use fake::{Fake, faker::lorem::en::Sentence};

    use super::*;

    fn entry(level: Level, message: &str) -> LogEntry {
        LogEntry {
            seq: 0,
            time: SystemTime::UNIX_EPOCH,
            level,
            target: Cow::Borrowed("bin_desktop::shell"),
            message: message.to_string(),
            fields: Vec::new(),
        }
    }

    fn messages(view: &LogView) -> Vec<String> {
        view.visible()
            .iter()
            .map(|entry| entry.message.clone())
            .collect()
    }

    #[test]
    fn subsystem_is_the_first_module_after_our_crate() {
        assert_eq!(subsystem("bin_desktop::shell::toast"), "shell");
        assert_eq!(subsystem("lib_locale::format"), "format");
    }

    #[test]
    fn subsystem_at_our_crate_root_drops_the_prefix() {
        assert_eq!(subsystem("lib_locale"), "locale");
        assert_eq!(subsystem("bin_desktop"), "desktop");
    }

    #[test]
    fn subsystem_of_a_dependency_is_its_crate() {
        assert_eq!(subsystem("zbus::connection"), "zbus");
        assert_eq!(subsystem("wgpu_core"), "wgpu_core");
    }

    #[test]
    fn level_tags_are_five_wide() {
        for level in [
            Level::ERROR,
            Level::WARN,
            Level::INFO,
            Level::DEBUG,
            Level::TRACE,
        ] {
            assert_eq!(level_tag(level).len(), 5);
        }
    }

    #[test]
    fn body_puts_fields_after_the_message() {
        let mut entry = entry(Level::WARN, "retrying push");
        entry.target = Cow::Borrowed("bin_desktop::sync");
        entry.fields = vec![
            ("attempt".to_string(), "2".to_string()),
            ("path".to_string(), "a/b".to_string()),
        ];
        assert_eq!(body(&entry), "sync: retrying push attempt=2 path=a/b");
    }

    #[test]
    fn timestamp_is_bracketed_hours_minutes_seconds() {
        let shown = timestamp(SystemTime::now());
        assert_eq!(shown.len(), 10);
        assert!(shown.starts_with('[') && shown.ends_with(']'));
        assert_eq!(shown.matches(':').count(), 2);
    }

    #[test]
    fn a_new_view_shows_what_is_already_captured_newest_first() {
        let buffer = LogBuffer::new(8);
        let first: String = Sentence(2..4).fake();
        buffer.push(entry(Level::INFO, &first));
        buffer.push(entry(Level::WARN, "second"));

        let view = LogView::new(buffer, TracingLevel::Info);
        assert_eq!(messages(&view), ["second".to_string(), first]);
    }

    #[test]
    fn the_filter_hides_more_verbose_entries_and_counts_them() {
        let buffer = LogBuffer::new(8);
        buffer.push(entry(Level::DEBUG, "noise"));
        buffer.push(entry(Level::ERROR, "failed"));

        let mut view = LogView::new(buffer, TracingLevel::Warn);
        assert_eq!(messages(&view), ["failed"]);
        assert_eq!(view.hidden_count(), 1);

        view.set_level(TracingLevel::Debug);
        assert_eq!(messages(&view), ["failed", "noise"]);
        assert_eq!(view.hidden_count(), 0);
    }

    #[test]
    fn pull_reports_only_visible_additions() {
        let buffer = LogBuffer::new(8);
        let mut view = LogView::new(buffer.clone(), TracingLevel::Warn);
        buffer.push(entry(Level::WARN, "a"));
        buffer.push(entry(Level::DEBUG, "b"));

        assert_eq!(
            view.pull(),
            LogChange {
                evicted: 0,
                added: 1
            }
        );
        assert_eq!(view.pull(), LogChange::default());
        assert_eq!(messages(&view), ["a"]);
    }

    #[test]
    fn pull_drops_the_oldest_past_capacity() {
        let buffer = LogBuffer::new(2);
        let mut view = LogView::new(buffer.clone(), TracingLevel::Info);
        buffer.push(entry(Level::INFO, "a"));
        view.pull();
        buffer.push(entry(Level::INFO, "b"));
        buffer.push(entry(Level::INFO, "c"));

        assert_eq!(
            view.pull(),
            LogChange {
                evicted: 1,
                added: 2
            }
        );
        assert_eq!(messages(&view), ["c", "b"]);
    }

    #[test]
    fn clear_empties_the_capture_and_later_entries_still_arrive() {
        let buffer = LogBuffer::new(8);
        buffer.push(entry(Level::INFO, "old"));
        let mut view = LogView::new(buffer.clone(), TracingLevel::Info);

        view.clear();
        assert!(view.visible().is_empty());
        assert!(buffer.snapshot().is_empty());

        buffer.push(entry(Level::INFO, "new"));
        view.pull();
        assert_eq!(messages(&view), ["new"]);
    }

    #[test]
    fn tracing_level_starts_from_the_configured_level_clamped() {
        use lib_tracing::Levels;
        let cases = [
            (Some(Levels::OFF), TracingLevel::Error),
            (Some(Levels::ERROR), TracingLevel::Error),
            (Some(Levels::WARN), TracingLevel::Warn),
            (Some(Levels::INFO), TracingLevel::Info),
            (None, TracingLevel::Info),
            (Some(Levels::DEBUG), TracingLevel::Debug),
            (Some(Levels::TRACE), TracingLevel::Debug),
        ];
        for (configured, expected) in cases {
            assert_eq!(TracingLevel::from_configured(configured), expected);
        }
    }

    #[test]
    fn tracing_level_admits_its_own_level_and_anything_more_severe() {
        use tracing::Level;
        assert!(TracingLevel::Warn.admits(Level::ERROR));
        assert!(TracingLevel::Warn.admits(Level::WARN));
        assert!(!TracingLevel::Warn.admits(Level::INFO));
        assert!(TracingLevel::Debug.admits(Level::DEBUG));
        assert!(!TracingLevel::Debug.admits(Level::TRACE));
        assert!(!TracingLevel::Error.admits(Level::WARN));
    }

    #[test]
    fn tracing_level_label_comes_from_the_catalogue() {
        crate::locale::init_for_tests();
        assert_eq!(TracingLevel::Warn.label(), "warn");
    }
}
