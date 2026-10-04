//! # Log Capture
//!
//! An in-memory ring of the most recent `tracing` events, so a Client can show its own live log
//! (the Desktop's Settings › Tracing page) without reading the log file back. Opt-in: a caller
//! builds a [`LogBuffer`] and hands it to [`crate::init`]; the Sync Server never does.
//!
//! The capture layer records at `debug` for our own crates whatever the `log` Configuration says
//! (the page filters by level itself), and only `warn` and above for dependencies, whose bridged
//! `log::debug!` chatter would otherwise push our few events out of the ring in seconds.
//!
//! See `docs/research/tracing-ring-buffer-capture.md` (issue #498) and the seam decision (#500).

use std::{
    borrow::Cow,
    collections::VecDeque,
    fmt::{self, Write as _},
    sync::{Arc, Mutex, MutexGuard, PoisonError},
    time::SystemTime,
};

use tracing::{
    Event, Level, Subscriber,
    field::{Field, Visit},
};
use tracing_log::NormalizeEvent;
use tracing_subscriber::{
    filter::{LevelFilter, Targets},
    layer::{Context, Layer},
};

/// How many entries the Desktop keeps: 16m's "last 1000 entries".
pub const LOG_CAPACITY: usize = 1000;

/// One captured event, structured so the viewer decides the format (issue #499).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogEntry {
    /// Monotonic across the buffer's life, including across [`LogBuffer::clear`], so a reader's
    /// cursor stays valid.
    pub seq: u64,
    /// Events carry no timestamp of their own, so this is when the layer saw it.
    pub time: SystemTime,
    pub level: Level,
    /// The module path, e.g. `bin_desktop::shell`. `log`-bridged events get their original
    /// target, not `tracing-log`'s literal `"log"`.
    pub target: Cow<'static, str>,
    pub message: String,
    /// Every structured field other than the message, in recorded order.
    pub fields: Vec<(String, String)>,
}

type Waker = Arc<dyn Fn() + Send + Sync>;

#[derive(Default)]
struct Inner {
    entries: VecDeque<Arc<LogEntry>>,
    next_seq: u64,
}

struct Shared {
    capacity: usize,
    inner: Mutex<Inner>,
    waker: Mutex<Option<Waker>>,
}

/// A bounded, shared ring of [`LogEntry`]s, newest at the back. Cloning shares the same ring.
#[derive(Clone)]
pub struct LogBuffer {
    shared: Arc<Shared>,
}

impl fmt::Debug for LogBuffer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LogBuffer")
            .field("capacity", &self.shared.capacity)
            .field("len", &self.inner().entries.len())
            .finish()
    }
}

impl LogBuffer {
    pub fn new(capacity: usize) -> Self {
        Self {
            shared: Arc::new(Shared {
                capacity,
                inner: Mutex::new(Inner {
                    entries: VecDeque::with_capacity(capacity),
                    next_seq: 0,
                }),
                waker: Mutex::new(None),
            }),
        }
    }

    /// How many entries the ring holds before it evicts the oldest.
    pub fn capacity(&self) -> usize {
        self.shared.capacity
    }

    /// A poisoned lock only means another thread panicked mid-push; the ring itself is still a
    /// valid `VecDeque`, so keep using it rather than losing the log.
    fn inner(&self) -> MutexGuard<'_, Inner> {
        self.shared
            .inner
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }

    /// Appends an entry, evicting the oldest when full, then calls the waker. `entry.seq` is
    /// overwritten with the buffer's own next sequence number.
    pub fn push(&self, mut entry: LogEntry) {
        if self.shared.capacity == 0 {
            return;
        }
        {
            let mut inner = self.inner();
            entry.seq = inner.next_seq;
            inner.next_seq += 1;
            if inner.entries.len() == self.shared.capacity {
                inner.entries.pop_front();
            }
            inner.entries.push_back(Arc::new(entry));
        }
        // Called outside the ring's lock, so a waker that logs can't deadlock on it.
        let waker = self
            .shared
            .waker
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone();
        if let Some(waker) = waker {
            waker();
        }
    }

    /// Every entry still held, oldest first.
    pub fn snapshot(&self) -> Vec<Arc<LogEntry>> {
        self.inner().entries.iter().cloned().collect()
    }

    /// Entries with `seq` greater than or equal to `from`, oldest first: what arrived since a
    /// reader last saw [`Self::next_seq`].
    pub fn since(&self, from: u64) -> Vec<Arc<LogEntry>> {
        let inner = self.inner();
        let newer = inner
            .entries
            .iter()
            .rev()
            .take_while(|entry| entry.seq >= from)
            .count();
        inner
            .entries
            .iter()
            .skip(inner.entries.len() - newer)
            .cloned()
            .collect()
    }

    /// The sequence number the next pushed entry will get.
    pub fn next_seq(&self) -> u64 {
        self.inner().next_seq
    }

    /// Empties the ring for every reader. Sequence numbers keep counting up.
    pub fn clear(&self) {
        self.inner().entries.clear();
    }

    /// Registers the hook called after each push (replacing any earlier one). It runs on whatever
    /// thread logged, so it should only signal, e.g. `Notify::notify_one`.
    pub fn set_waker(&self, waker: impl Fn() + Send + Sync + 'static) {
        *self
            .shared
            .waker
            .lock()
            .unwrap_or_else(PoisonError::into_inner) = Some(Arc::new(waker));
    }
}

/// The layer that feeds a [`LogBuffer`], wrapped in its own [`capture_filter`].
pub(crate) struct CaptureLayer {
    buffer: LogBuffer,
}

impl CaptureLayer {
    pub(crate) fn new(buffer: LogBuffer) -> Self {
        Self { buffer }
    }
}

/// Our crates at `debug`, everything else at `warn`. `Targets` matches by prefix, so `lib_`
/// covers every workspace library. It ignores `RUST_LOG` so the page shows the same whatever the
/// app was launched with.
pub(crate) fn capture_filter() -> Targets {
    Targets::new()
        .with_target("bin_", LevelFilter::DEBUG)
        .with_target("lib_", LevelFilter::DEBUG)
        .with_default(LevelFilter::WARN)
}

impl<S: Subscriber> Layer<S> for CaptureLayer {
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        let normalized = event.normalized_metadata();
        let metadata = normalized.as_ref().unwrap_or_else(|| event.metadata());
        let mut visitor = EntryVisitor::default();
        event.record(&mut visitor);
        let target = match normalized {
            // A bridged `log` record's metadata is borrowed from the event, not `'static`.
            Some(_) => Cow::Owned(metadata.target().to_string()),
            None => Cow::Borrowed(event.metadata().target()),
        };
        self.buffer.push(LogEntry {
            seq: 0,
            time: SystemTime::now(),
            level: *metadata.level(),
            target,
            message: visitor.message,
            fields: visitor.fields,
        });
    }
}

#[derive(Default)]
struct EntryVisitor {
    message: String,
    fields: Vec<(String, String)>,
}

impl Visit for EntryVisitor {
    // Overridden so a `&str` value isn't Debug-quoted.
    fn record_str(&mut self, field: &Field, value: &str) {
        if field.name() == "message" {
            self.message.push_str(value);
        } else if !field.name().starts_with("log.") {
            self.fields
                .push((field.name().to_string(), value.to_string()));
        }
    }

    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        if field.name() == "message" {
            // `format_args!` messages arrive here; their `Debug` is the plain text.
            let _ = write!(self.message, "{value:?}");
        } else if !field.name().starts_with("log.") {
            self.fields
                .push((field.name().to_string(), format!("{value:?}")));
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use fake::{Fake, faker::lorem::en::Sentence};
    use tracing_subscriber::prelude::*;

    use super::*;

    fn entry(message: &str) -> LogEntry {
        LogEntry {
            seq: 0,
            time: SystemTime::UNIX_EPOCH,
            level: Level::INFO,
            target: Cow::Borrowed("bin_desktop::shell"),
            message: message.to_string(),
            fields: Vec::new(),
        }
    }

    fn messages(entries: &[Arc<LogEntry>]) -> Vec<&str> {
        entries.iter().map(|entry| entry.message.as_str()).collect()
    }

    /// Runs `body` with only a capture layer installed, scoped to this thread.
    fn captured(body: impl FnOnce()) -> Vec<Arc<LogEntry>> {
        let buffer = LogBuffer::new(LOG_CAPACITY);
        let subscriber = tracing_subscriber::registry()
            .with(CaptureLayer::new(buffer.clone()).with_filter(capture_filter()));
        tracing::subscriber::with_default(subscriber, body);
        buffer.snapshot()
    }

    #[test]
    fn push_numbers_entries_in_order() {
        let buffer = LogBuffer::new(4);
        let first: String = Sentence(2..4).fake();
        buffer.push(entry(&first));
        buffer.push(entry("second"));

        let entries = buffer.snapshot();
        assert_eq!(messages(&entries), [first.as_str(), "second"]);
        assert_eq!(entries[0].seq, 0);
        assert_eq!(entries[1].seq, 1);
        assert_eq!(buffer.next_seq(), 2);
    }

    #[test]
    fn a_full_buffer_evicts_the_oldest() {
        let buffer = LogBuffer::new(2);
        for message in ["a", "b", "c"] {
            buffer.push(entry(message));
        }
        assert_eq!(messages(&buffer.snapshot()), ["b", "c"]);
    }

    #[test]
    fn a_zero_capacity_buffer_keeps_nothing() {
        let buffer = LogBuffer::new(0);
        buffer.push(entry("a"));
        assert!(buffer.snapshot().is_empty());
    }

    #[test]
    fn since_returns_only_newer_entries() {
        let buffer = LogBuffer::new(8);
        buffer.push(entry("a"));
        let cursor = buffer.next_seq();
        buffer.push(entry("b"));
        buffer.push(entry("c"));

        assert_eq!(messages(&buffer.since(cursor)), ["b", "c"]);
        assert!(buffer.since(buffer.next_seq()).is_empty());
        assert_eq!(messages(&buffer.since(0)), ["a", "b", "c"]);
    }

    #[test]
    fn clear_empties_every_clone_and_keeps_counting() {
        let buffer = LogBuffer::new(8);
        let reader = buffer.clone();
        buffer.push(entry("a"));
        buffer.push(entry("b"));

        reader.clear();
        assert!(buffer.snapshot().is_empty());

        buffer.push(entry("c"));
        assert_eq!(buffer.snapshot()[0].seq, 2);
    }

    #[test]
    fn the_waker_runs_after_each_push() {
        let buffer = LogBuffer::new(8);
        let calls = Arc::new(AtomicUsize::new(0));
        let counter = calls.clone();
        buffer.set_waker(move || {
            counter.fetch_add(1, Ordering::SeqCst);
        });

        buffer.push(entry("a"));
        buffer.push(entry("b"));
        assert_eq!(calls.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn a_waker_may_read_the_buffer_without_deadlocking() {
        let buffer = LogBuffer::new(8);
        let reader = buffer.clone();
        let seen = Arc::new(AtomicUsize::new(0));
        let counter = seen.clone();
        buffer.set_waker(move || {
            counter.store(reader.snapshot().len(), Ordering::SeqCst);
        });

        buffer.push(entry("a"));
        assert_eq!(seen.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn the_layer_records_level_target_message_and_fields() {
        let entries = captured(|| {
            tracing::warn!(target: "bin_desktop::sync", attempt = 2, path = "a/b", "retrying push");
        });

        assert_eq!(entries.len(), 1);
        let entry = &entries[0];
        assert_eq!(entry.level, Level::WARN);
        assert_eq!(entry.target, "bin_desktop::sync");
        assert_eq!(entry.message, "retrying push");
        assert_eq!(
            entry.fields,
            [
                ("attempt".to_string(), "2".to_string()),
                ("path".to_string(), "a/b".to_string())
            ]
        );
    }

    #[test]
    fn the_layer_formats_interpolated_messages_without_quotes() {
        let entries = captured(|| {
            let locale = "en-AU";
            tracing::debug!(target: "lib_locale", "negotiated {locale}");
        });
        assert_eq!(messages(&entries), ["negotiated en-AU"]);
    }

    #[test]
    fn the_filter_keeps_our_debug_and_drops_dependency_debug() {
        let entries = captured(|| {
            tracing::debug!(target: "bin_desktop::shell", "ours");
            tracing::trace!(target: "bin_desktop::shell", "too verbose");
            tracing::debug!(target: "wgpu_core::device", "theirs");
            tracing::warn!(target: "wgpu_core::device", "their warning");
        });
        assert_eq!(messages(&entries), ["ours", "their warning"]);
    }

    #[test]
    fn bridged_log_records_keep_their_own_target() {
        let entries = captured(|| {
            let record = log::Record::builder()
                .level(log::Level::Warn)
                .target("zbus::connection")
                .args(format_args!("socket closed"))
                .build();
            tracing_log::format_trace(&record).expect("bridging a record should succeed");
        });

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].target, "zbus::connection");
        assert_eq!(entries[0].message, "socket closed");
        assert!(entries[0].fields.is_empty(), "log.* fields are skipped");
    }
}
