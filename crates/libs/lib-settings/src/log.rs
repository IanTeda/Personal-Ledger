//! What Settings › Tracing shows of the log capture, `gpui`-free: a captured [`LogLine`], the
//! [`TracingLevel`] view filter over it and the one-line entry format (issue #499). The capture
//! itself is `lib-tracing`'s, which this crate may not depend on, so the Client hands lines in
//! through [`TracingLog`](crate::TracingLog).
//!
//! The format (`[hh:mm:ss] LEVEL subsystem: message key=value`) is diagnostic output and so not a
//! Message, per `docs/localisation-design.md`; only the empty states are.

use std::time::SystemTime;

use chrono::{DateTime, Local};

use crate::msg;

/// How severe a captured event was, least verbose first, so `a <= b` reads "a is at least as
/// severe as b".
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

/// One captured event, as the capture recorded it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogLine {
    /// The capture's own running number, so a reader's cursor stays valid when the ring evicts.
    pub seq: u64,
    pub time: SystemTime,
    pub level: LogLevel,
    /// The module path, e.g. `bin_desktop::shell`.
    pub target: String,
    pub message: String,
    /// Every structured field other than the message, in recorded order.
    pub fields: Vec<(String, String)>,
}

impl LogLine {
    /// The 5-wide upper-case level tag, padded so messages line up.
    pub fn tag(&self) -> &'static str {
        match self.level {
            LogLevel::Error => "ERROR",
            LogLevel::Warn => "WARN ",
            LogLevel::Info => "INFO ",
            LogLevel::Debug => "DEBUG",
            LogLevel::Trace => "TRACE",
        }
    }

    /// Everything after the level tag: `subsystem: message key=value key=value`.
    pub fn body(&self) -> String {
        let mut text = format!("{}: {}", subsystem(&self.target), self.message);
        for (name, value) in &self.fields {
            text.push(' ');
            text.push_str(name);
            text.push('=');
            text.push_str(value);
        }
        text
    }

    /// `[hh:mm:ss]` in local time.
    pub fn timestamp(&self) -> String {
        DateTime::<Local>::from(self.time)
            .format("[%H:%M:%S]")
            .to_string()
    }
}

/// The short "subsystem" a target is shown as. For our own crates it is the first module after
/// the crate (`bin_desktop::shell::router::toasts` is `shell`), or the crate without its `bin_`/`lib_`
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
            Self::Error => msg::settings_tracing_level_error(),
            Self::Warn => msg::settings_tracing_level_warn(),
            Self::Info => msg::settings_tracing_level_info(),
            Self::Debug => msg::settings_tracing_level_debug(),
        }
    }

    /// Whether an entry at `level` shows under this filter.
    pub fn admits(self, level: LogLevel) -> bool {
        let most_verbose = match self {
            Self::Error => LogLevel::Error,
            Self::Warn => LogLevel::Warn,
            Self::Info => LogLevel::Info,
            Self::Debug => LogLevel::Debug,
        };
        level <= most_verbose
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(target: &str) -> LogLine {
        LogLine {
            seq: 0,
            time: SystemTime::UNIX_EPOCH,
            level: LogLevel::Warn,
            target: target.to_string(),
            message: "retrying push".to_string(),
            fields: Vec::new(),
        }
    }

    #[test]
    fn subsystem_is_the_first_module_after_our_crate() {
        assert_eq!(subsystem("bin_desktop::shell::router::toasts"), "shell");
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
            LogLevel::Error,
            LogLevel::Warn,
            LogLevel::Info,
            LogLevel::Debug,
            LogLevel::Trace,
        ] {
            let line = LogLine {
                level,
                ..line("bin_desktop")
            };
            assert_eq!(line.tag().len(), 5);
        }
    }

    #[test]
    fn body_puts_fields_after_the_message() {
        let mut line = line("bin_desktop::sync");
        line.fields = vec![
            ("attempt".to_string(), "2".to_string()),
            ("path".to_string(), "a/b".to_string()),
        ];
        assert_eq!(line.body(), "sync: retrying push attempt=2 path=a/b");
    }

    #[test]
    fn timestamp_is_bracketed_hours_minutes_seconds() {
        let shown = LogLine {
            time: SystemTime::now(),
            ..line("bin_desktop")
        }
        .timestamp();
        assert_eq!(shown.len(), 10);
        assert!(shown.starts_with('[') && shown.ends_with(']'));
        assert_eq!(shown.matches(':').count(), 2);
    }

    #[test]
    fn tracing_level_admits_its_own_level_and_anything_more_severe() {
        assert!(TracingLevel::Warn.admits(LogLevel::Error));
        assert!(TracingLevel::Warn.admits(LogLevel::Warn));
        assert!(!TracingLevel::Warn.admits(LogLevel::Info));
        assert!(TracingLevel::Debug.admits(LogLevel::Debug));
        assert!(!TracingLevel::Debug.admits(LogLevel::Trace));
        assert!(!TracingLevel::Error.admits(LogLevel::Warn));
    }
}
