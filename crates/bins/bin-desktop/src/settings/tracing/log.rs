//! The Desktop's half of Settings › Tracing's capture: [`CaptureLog`] answers `lib-settings`'
//! `TracingLog` port over `lib_tracing`'s [`LogBuffer`], the ring the app's tracing subscriber
//! feeds from any thread, [`LogFeed`] is what the page subscribes to for word that it gained
//! entries, and [`opening_level`] picks the radio the page opens on.

use std::rc::Rc;

use gpui::{Context, EventEmitter};

use lib_settings::log::{LogLevel, LogLine};
use lib_settings::{TracingLevel, TracingLog};
use lib_tracing::{LogBuffer, LogEntry};
use tracing::Level;

/// `lib-tracing`'s capture as the Tracing page reads it.
pub struct CaptureLog {
    buffer: LogBuffer,
}

impl CaptureLog {
    pub fn new(buffer: LogBuffer) -> Self {
        Self { buffer }
    }
}

impl TracingLog for CaptureLog {
    fn capacity(&self) -> usize {
        self.buffer.capacity()
    }

    fn since(&self, from: u64) -> Vec<Rc<LogLine>> {
        self.buffer
            .since(from)
            .iter()
            .map(|entry| Rc::new(line(entry)))
            .collect()
    }

    fn next_seq(&self) -> u64 {
        self.buffer.next_seq()
    }

    fn clear(&self) {
        self.buffer.clear();
    }
}

/// The capture gained entries.
#[derive(Debug, Clone, Copy)]
pub struct LogAppended;

/// Where the capture's wake reaches the gpui side: the capture feed (`shell/router/log_feed.rs`)
/// signals it on the main thread once a burst settles, and the Tracing page subscribes to it as it
/// is built (`PageView::subscribe`), before the real capture is handed over.
pub struct LogFeed;

impl EventEmitter<LogAppended> for LogFeed {}

impl LogFeed {
    /// Tells the page the capture gained entries.
    pub fn signal(&mut self, cx: &mut Context<'_, Self>) {
        cx.emit(LogAppended);
    }
}

fn line(entry: &LogEntry) -> LogLine {
    LogLine {
        seq: entry.seq,
        time: entry.time,
        level: level(entry.level),
        target: entry.target.to_string(),
        message: entry.message.clone(),
        fields: entry.fields.clone(),
    }
}

fn level(level: Level) -> LogLevel {
    match level {
        Level::ERROR => LogLevel::Error,
        Level::WARN => LogLevel::Warn,
        Level::INFO => LogLevel::Info,
        Level::DEBUG => LogLevel::Debug,
        Level::TRACE => LogLevel::Trace,
    }
}

/// The radio the page opens on: the configured `log` level, clamped to the four offered, so the
/// page starts by showing what the console and log file show.
pub fn opening_level(configured: Option<lib_tracing::Levels>) -> TracingLevel {
    use lib_tracing::Levels;
    match configured {
        Some(Levels::OFF | Levels::ERROR) => TracingLevel::Error,
        Some(Levels::WARN) => TracingLevel::Warn,
        Some(Levels::INFO) | None => TracingLevel::Info,
        Some(Levels::DEBUG | Levels::TRACE) => TracingLevel::Debug,
    }
}

#[cfg(test)]
mod tests {
    use std::borrow::Cow;
    use std::time::SystemTime;

    use lib_settings::contract::{self, TracingLogFactory};

    use super::*;

    struct Captures;

    impl TracingLogFactory for Captures {
        type Log = CaptureLog;

        fn with_capacity(&mut self, capacity: usize) -> CaptureLog {
            CaptureLog::new(LogBuffer::new(capacity))
        }

        fn capture(&mut self, log: &CaptureLog, level: LogLevel, message: &str) {
            log.buffer.push(LogEntry {
                seq: 0,
                time: SystemTime::UNIX_EPOCH,
                level: match level {
                    LogLevel::Error => Level::ERROR,
                    LogLevel::Warn => Level::WARN,
                    LogLevel::Info => Level::INFO,
                    LogLevel::Debug => Level::DEBUG,
                    LogLevel::Trace => Level::TRACE,
                },
                target: Cow::Borrowed("bin_desktop::shell"),
                message: message.to_string(),
                fields: Vec::new(),
            });
        }
    }

    #[test]
    fn the_log_buffer_keeps_the_tracing_log_contract() {
        contract::run_all_tracing_log(&mut Captures);
    }

    #[test]
    fn the_page_opens_on_the_configured_level_clamped() {
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
            assert_eq!(opening_level(configured), expected);
        }
    }

    #[test]
    fn a_captured_entry_carries_its_fields_across() {
        let buffer = LogBuffer::new(4);
        buffer.push(LogEntry {
            seq: 0,
            time: SystemTime::UNIX_EPOCH,
            level: Level::WARN,
            target: Cow::Borrowed("bin_desktop::sync"),
            message: "retrying push".to_string(),
            fields: vec![("attempt".to_string(), "2".to_string())],
        });

        let lines = CaptureLog::new(buffer).since(0);

        assert_eq!(lines[0].level, LogLevel::Warn);
        assert_eq!(lines[0].body(), "sync: retrying push attempt=2");
    }
}
