# Research: capturing tracing events live for the Desktop

**Question** ([#498](https://github.com/IanTeda/Personal-Ledger/issues/498), on the [Desktop Settings Tracing (Logs)](https://github.com/IanTeda/Personal-Ledger/issues/497) map). How do we capture the running Desktop's `tracing` events at `debug` into a bounded in-memory ring buffer (last 1000), while console and file output stay at the configured `log` level, and push new entries live into gpui?

**Versions checked** (from `Cargo.lock` on `concept` at `f201a4d`): `tracing` 0.1.44, `tracing-core` 0.1.36, `tracing-subscriber` 0.3.23 (default features plus `env-filter`, as `crates/libs/lib-tracing/Cargo.toml` asks), `tracing-log` 0.2.0, `tracing-appender` 0.2.5, `gpui` 0.2.2, `gpui-component` 0.5.1, `tokio` 1.53.1 (workspace `features = ["full"]`), `crossbeam-queue` 0.3.14 (transitive). Every claim below was read from the crate source in `~/.cargo/registry/src/index.crates.io-*/<crate>-<version>/`, cited by file and line, unless it says otherwise.

## Summary

- **Filtering:** drop the global `EnvFilter` layer and give every layer its own filter with `Layer::with_filter`: console and file share the existing `EnvFilter` (configured level, `RUST_LOG`, `calloop=warn`) via `console.and_then(file).with_filter(env_filter)`, and the capture layer gets its own fixed `Targets` filter (our crates at `DEBUG`, everything else at `WARN`). Keeping the global `EnvFilter` would gate the capture layer too. Every layer in the stack must be filtered, or the global max level hint collapses to `TRACE`.
- **Extraction:** `on_event` gets level, target, file/line and fields from `Metadata`; the message is the `message` field, read through a small `Visit` impl; parent span names come from `ctx.event_scope(event)`; there is no timestamp on an event, so take `SystemTime::now()` in `on_event`. Events bridged from the `log` crate need `tracing_log::NormalizeEvent` for their real target, and their `log.*` fields skipped.
- **Ring buffer:** `std::sync::Mutex<VecDeque<Arc<LogEntry>>>` capped at 1000, with a monotonically increasing sequence number per entry. Format the entry before taking the lock; the lock covers only a `push_back`/`pop_front`. Lock-free queues buy nothing at this event rate and lose non-destructive iteration.
- **Waking gpui:** the layer calls a registered `Fn() + Send + Sync` hook after each push; the Desktop's hook is `tokio::sync::Notify::notify_one` (runtime-agnostic, already a dependency); a `cx.spawn` loop awaits `notified()`, then waits a short coalescing window on gpui's timer, then `this.update(cx, …)` pulls entries newer than its last sequence and calls `cx.notify()` once. That caps redraws at about 10 per second under a burst.
- **Existing crates:** nothing worth adopting. The in-app viewer crates are tied to egui or ratatui, and the one generic ring-buffer layer (`tracing-flight-recorder`) is new and barely used. About 150 lines of our own in `lib-tracing`, opt-in so `bin-tui` and `sync-server` don't pay for it.

## 1. Per-layer filtering

### What exists today

`crates/libs/lib-tracing/src/init.rs` builds one `EnvFilter` (configured level as the default directive, replaced wholesale by `RUST_LOG` when set, plus a `calloop=warn` directive) and installs it as the first layer: `registry().with(env_filter).with(console_collector).with(file_collector)`. A filter installed as a plain layer is a **global** filter: the `tracing-subscriber` layer docs say `Layer::register_callsite` and `Layer::enabled` "determine whether a span or event is enabled *globally*", and that filter evaluation short-circuits when any layer returns `false`/`Interest::never()` (`tracing-subscriber-0.3.23/src/layer/mod.rs:397-416`). So a capture layer added beside it would never see a `debug` event while `log` is `info`.

### What `tracing-subscriber` 0.3.23 offers

**Per-layer filters.** `Layer::with_filter(filter)` returns a `Filtered` layer; the `Filter` trait controls "what spans and events are observed by an individual `Layer`, while still allowing other `Layer`s to potentially record them" (`layer/mod.rs:440-470`). "A span or event will be recorded if it is enabled by _any_ per-layer filter, but it will be skipped by the layers whose filters did not enable it" (`layer/mod.rs:527-529`). Two layers can share one filter by combining them first with `and_then` (`layer/mod.rs:564-597`). It needs the `registry` feature (`layer/mod.rs:442`), which `tracing-subscriber`'s default `fmt` feature enables (`Cargo.toml` `[features]`: `default` includes `fmt`, `fmt = ["registry", "std"]`), and `lib-tracing` doesn't turn defaults off. Only `Registry` supports per-layer filters as a root subscriber (`layer/mod.rs:489-495`), which is what `init` already uses.

**`EnvFilter` is a `Filter`.** `impl<S> Filter<S> for EnvFilter` exists under `registry` + `std` (`filter/env/mod.rs:704-757`), so the existing filter, `RUST_LOG` handling and `calloop=warn` directive move over unchanged. Other ready-made filters: `LevelFilter`, `Targets`, `FilterFn`/`DynFilterFn` (`layer/mod.rs:463-469`).

**Reload.** `reload::Layer` wraps either a `Layer` or a `Filter`; the docs advise wrapping the `Filter` directly when only the filter changes (`reload.rs:1-64`). `Handle::modify` takes a write lock, then calls `callsite::rebuild_interest_cache()` and, with the `tracing-log` feature, resets `log::set_max_level` to the new `LevelFilter::current()` (`reload.rs:305-331`). Not needed here: the map settles that the radios only filter the view and the capture always records `debug`, so no filter changes at run time. Worth knowing if the `log` Configuration ever becomes live-editable.

### Recommended composition

```rust
let console_and_file = console_layer
    .and_then(file_layer)          // Option<fmt::Layer<..>> is a Layer
    .with_filter(env_filter);      // configured level, RUST_LOG, calloop=warn

let capture_filter = Targets::new()
    .with_target("bin_desktop", LevelFilter::DEBUG)
    .with_target("lib_", ...)      // each workspace crate by name; Targets matches by prefix
    .with_default(LevelFilter::WARN);

tracing_subscriber::registry()
    .with(console_and_file)
    .with(capture.map(|c| c.with_filter(capture_filter)))   // None for bin-tui / sync-server
```

The exact crate list for the capture filter is a decision for the seam ticket. A crate allowlist matters because `LogTracer` bridges every `log` record from gpui's dependency tree (wgpu/blade, cosmic-text, zbus, …) into `tracing`; a bare `LevelFilter::DEBUG` on the capture layer would fill a 1000-entry buffer with dependency chatter in seconds and push out the Desktop's own few events.

### Caveats

**Every layer must carry a filter.** With per-layer filters, the global hint is computed by `Layered::pick_level_hint` (`layer/layered.rs:478-526`): when both sides have layer filters it is `max(outer?, inner?)`, and when one side has a filter and the other gives no hint the result is `None` (no limit, so effectively `TRACE`). An unfiltered capture layer, or one wrapped in a filter that doesn't implement `max_level_hint`, would therefore turn on every `trace!` callsite in the process, including the `calloop` per-frame `TRACE` noise that `init.rs` caps. `Targets`, `LevelFilter` and `EnvFilter` all report a hint; `FilterFn` doesn't unless you add one with `with_max_level_hint`.

**The global max level rises to `DEBUG`.** `LevelFilter::current()` becomes the maximum across the filters, so `debug!` callsites that the fast path used to skip outright now get registered and evaluated per event through each `Filtered`'s `enabled`. It is cheap (one metadata check per filter) but no longer free. `LogTracer::enabled` first compares a record's level with `tracing_core::LevelFilter::current()` (`tracing-log-0.2.0/src/log_tracer.rs:170-176`), so `log::debug!` records from dependencies also start reaching `dispatch.enabled`, where the capture filter's `Targets` rejects them by prefix. `LogTracer::init()` sets `log::set_max_level` from its builder's filter (`log_tracer.rs:286-294`), so the `log` side never gates anything.

**Callsite interest caching.** Interest is decided once per callsite in `register_callsite` and cached (`layer/mod.rs:418-438`). With per-layer filters, a callsite that one filter wants and another doesn't is cached as "sometimes", so `enabled` runs per event and each `Filtered` records its own answer. That is what makes per-layer filtering work; the only caveat is that anything changing a filter at run time must go through `reload` (which rebuilds the cache), never interior mutability inside a custom `Filter`.

**`RUST_LOG`.** It stays on the console/file `EnvFilter` only. Recommendation: the capture filter ignores `RUST_LOG` so the page behaves the same however the app was launched. If a developer sets `RUST_LOG=trace`, the global hint rises to `TRACE` for console/file; the capture filter still keeps its own `DEBUG`/`WARN` cut. Side note found while reading: `init.rs` builds `default_env_filter` with `.from_env_lossy()` (which already reads `RUST_LOG`) and then replaces it with `EnvFilter::try_from_default_env()` when `RUST_LOG` parses; the second step is redundant, and the build ticket can collapse it.

**Compile-time level caps.** `tracing`'s `max_level_*`/`release_max_level_*` features would compile `debug!` out of release builds. The workspace's `tracing = { version = "0.1.43" }` sets none, so release builds keep `debug!` callsites.

## 2. What `Layer::on_event` can extract

`fn on_event(&self, event: &Event<'_>, ctx: Context<'_, S>)` with `S: Subscriber + for<'a> LookupSpan<'a>`:

| Data | Source | Cost |
| --- | --- | --- |
| Level, target, module path, file, line | `event.metadata()` (`&'static Metadata`) | Free: `&'static str` and `Copy` values |
| Message | the field named `message`, through `event.record(&mut visitor)` | One `String` format |
| Other fields | the same visitor, every non-`message` field | One `String` per field, or one joined `String` |
| Parent span names | `ctx.event_scope(event)` → `Scope` iterator of `SpanRef`, leaf to root; `.from_root()` for root first (`layer/context.rs:343-367`) | Registry lookups; `span.name()` is `&'static str` |
| Timestamp | **not on the event**; take `SystemTime::now()` in `on_event` | One clock read |

**`Visit` pattern.** `tracing_core::field::Visit` requires only `record_debug(&mut self, field: &Field, value: &dyn fmt::Debug)`; `record_str`, `record_i64`, `record_u64`, `record_bool`, `record_f64`, `record_error` and friends all default to it (`tracing-core-0.1.36/src/field.rs:281-340`). Override `record_str` too, so a `&str` message isn't Debug-quoted:

```rust
#[derive(Default)]
struct EntryVisitor { message: String, fields: String }

impl Visit for EntryVisitor {
    fn record_str(&mut self, field: &Field, value: &str) {
        if field.name() == "message" { self.message.push_str(value) } else { self.push(field, &value) }
    }
    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        if field.name() == "message" { let _ = write!(self.message, "{value:?}"); }
        else if !field.name().starts_with("log.") { self.push(field, value) }
    }
}
```

`format_args!` messages arrive through `record_debug` with an `Arguments` value whose `Debug` writes the plain text, so `{value:?}` doesn't add quotes.

**Events bridged from `log`.** `tracing-log` dispatches every `log` record through one static callsite per level whose metadata target is the literal `"log"` (`tracing-log-0.2.0/src/lib.rs:278-290`), carrying the real target, module, file and line as `log.target`/`log.module_path`/`log.file`/`log.line` fields (`lib.rs:166-194`). Filters see the real target (dispatch checks `record.as_trace()`, whose target is `record.target()`, `lib.rs:168` and `lib.rs:229-240`), but `on_event` would show `target = "log"` unless it calls `tracing_log::NormalizeEvent::normalized_metadata()` (`lib.rs:455-464`), which returns the original metadata or `None` for native events. The visitor should skip the `log.*` fields. `fmt::Layer` already does both internally, so this only affects our layer.

**Spans.** `event_scope` inside a `Filtered` layer only yields spans that layer's filter enabled (the `Context` carries the layer's `FilterId`, `layer/context.rs:369-375`), so span names only appear when the capture filter also admits the span. Capturing span names is optional for the page; recommend recording the innermost-to-root names joined with `:` as fmt does, or leaving them out until the format ticket asks.

**Recommended entry type** (lives in `lib-tracing`, no gpui types):

```rust
pub struct LogEntry {
    pub seq: u64,                 // monotonic, survives Clear
    pub at: SystemTime,           // formatted by the Desktop through lib_locale::format
    pub level: tracing::Level,
    pub target: Cow<'static, str>,// &'static for native events, owned for log-bridged ones
    pub message: String,
    pub fields: String,           // "key=value key=value", may be empty
    pub spans: String,            // optional, see above
}
```

`SystemTime` keeps `lib-tracing` free of `chrono`; the Desktop already formats dates through `lib_locale`.

**Re-entrancy.** Nothing in `on_event` (or the wake hook it calls) may emit a `tracing` event or a `log` record while holding the buffer lock; with a non-reentrant `Mutex` that is a self-deadlock. Keep the lock region to the push itself.

## 3. Ring buffer and locking

**Shape:** `Mutex<Inner>` where `Inner { entries: VecDeque<Arc<LogEntry>>, next_seq: u64 }`, `VecDeque::with_capacity(1000)`; on push, `pop_front` when full, then `push_back`. Readers:

- `since(seq) -> Vec<Arc<LogEntry>>`: entries newer than the caller's last seen sequence (a short scan from the back), so the live page appends deltas instead of re-copying 1000 entries per update.
- `snapshot() -> Vec<Arc<LogEntry>>`: all current entries, when the page opens. With `Arc` entries this is 1000 refcount bumps, not 1000 string copies.
- `clear()`: empties the deque, keeps `next_seq` so a reader's cursor stays valid.

**Cost on the event path.** All formatting (visitor, `SystemTime::now()`, span names) happens before the lock; the critical section is a `pop_front` + `push_back` of a pointer, O(1) and allocation-free once the deque is at capacity. An uncontended `std::sync::Mutex` on Linux is a single atomic compare-and-swap (futex-based since Rust 1.62); contention only happens when two threads log at the same instant or the UI is mid-`since`, both rare at the Desktop's event rate (today: three `error!` and one `warn!`). `parking_lot::Mutex` (0.12.5 is already in the tree through gpui) is a fine drop-in but not needed; with `std`, handle poisoning with `.lock().unwrap_or_else(PoisonError::into_inner)` since the workspace denies `unwrap`/`expect`.

**Lock-free alternative, not recommended.** `crossbeam_queue::ArrayQueue::force_push` (0.3.14, `src/array_queue.rs:293`) gives exactly the "overwrite the oldest when full" semantics without a lock, but the only way to read it is `pop`, which is destructive: the UI would have to drain it into its own copy and that copy would then need its own synchronisation for `Clear` and for reopening the page. It moves the lock rather than removing it, for no measurable gain at this rate.

## 4. Waking gpui from other threads

**Constraints.** `tracing` events arrive on any thread (the main thread, `tokio` worker threads running `sqlx`, `tracing-appender`'s worker). gpui entities can only be touched on the foreground thread: `AsyncApp` is `!Send`, and the only `Send` entry point is `background_spawn` (`gpui-0.2.2/src/app/async_context.rs:105`). So the layer needs a thread-safe signal that a foreground task awaits.

**What gpui 0.2.2 offers.**

- `Context::spawn(async move |this: WeakEntity<T>, cx: &mut AsyncApp| …)` returns a `Task` to hold or `detach()` (`src/app/context.rs:233-245`). The Desktop already uses exactly this for the Toast clock (`crates/bins/bin-desktop/src/shell.rs:1018-1040`: a `loop` of `Timer::after(TICK).await` then `this.update(cx, |shell, cx| … cx.notify())`, breaking when `update` errors because the entity has gone) and the tooltip delay (`shell.rs:8316`).
- Timers: `gpui::Timer` is a re-export of `smol::Timer` (`src/gpui.rs:94`), real wall-clock time. `cx.background_executor().timer(duration)` (`src/executor.rs:357-367`) dispatches through the platform dispatcher, which the test dispatcher simulates (`src/platform/test/dispatcher.rs:302-309`, driven by `advance_clock`, `src/executor.rs:395`). Prefer the executor timer in new code so `/ui-tests` can drive it deterministically.
- No cross-thread channel of its own is re-exported (gpui depends on `futures` 0.3 and `smol` 2.0 internally, `Cargo.toml`, but only `smol::Timer` is public). `bin-desktop` already depends on `tokio` with `full`, and `tokio::sync` is documented as runtime-agnostic: "All synchronization primitives provided in this module are runtime agnostic" (`tokio-1.53.1/src/sync/mod.rs:440`), so `tokio::sync::Notify` works when awaited on gpui's executor.
- `cx.notify()` already coalesces: `App::notify` either invalidates the view's window once or queues a single pending `Notify` effect per entity (`src/app.rs:2033-2050`), so several notifies before the next frame cost one repaint. What it doesn't coalesce is the foreground wake-ups themselves, hence the throttle below.

**Recommended pattern.**

1. `lib-tracing` stays runtime-free: the capture handle accepts a set-once wake hook, `Box<dyn Fn() + Send + Sync>`, called after each push (outside the lock).
2. The Desktop registers `move || notify.notify_one()` with an `Arc<tokio::sync::Notify>`. `notify_one` stores at most one permit, so a burst of 500 events between polls is one wake.
3. The Tracing page (or the Shell, if the buffer's cursor should advance while the page is closed; the page only needs it while open) spawns:

```rust
cx.spawn(async move |this, cx| loop {
    notify.notified().await;
    // Coalescing window: let a burst finish, cap redraws at ~10/s.
    cx.background_executor().timer(Duration::from_millis(100)).await;
    if this.update(cx, |page, cx| {
        if page.pull_new_entries() { cx.notify(); }   // capture.since(page.last_seq)
    }).is_err() { break; }
})
.detach();
```

A simpler fallback, matching the Toast clock exactly, is a fixed 250 ms poll comparing an `AtomicU64` "latest sequence" against the page's cursor; it wakes four times a second even when idle, so only use it if `Notify` causes trouble. Either way, start the task when the page is shown and let it end when the page entity drops, so a closed page costs nothing.

**Test seam.** `/ui-tests` can push synthetic entries straight into a capture handle passed to `build_shell` (no global subscriber in tests, since `set_global_default` is once per process), call the hook, and `advance_clock` past the coalescing window.

## 5. Existing crates

Searched the crates.io API (`/api/v1/crates?q=…`, 2026-10-04) for "tracing ring buffer", "tracing memory", "tracing log viewer", "tracing capture layer", plus the known viewer crates. Figures are crates.io's own `max_version`, last update and 90-day downloads; internals were not audited.

| Crate | Version / updated / recent downloads | Fit |
| --- | --- | --- |
| `tracing-flight-recorder` | 0.3.0 / 2026-08-12 / 57 | Generic in-memory ring-buffer layer, closest in intent; very new, almost no users, would still need our entry type, wake hook and filtering. |
| `egui_tracing` | 0.3.0 / 2026-04-10 / 3,036 | Collector plus viewer, tied to egui. |
| `tui-logger` | 0.18.3 / 2026-07-04 / 408,331 | Ratatui logger widget with a circular buffer; `log`-centric, tied to ratatui. Possibly relevant to a later `bin-tui` page, not the Desktop. |
| `tui-tracing` | 0.2.5 / 2026-06-27 / 90 | Ratatui-only. |
| `tracing-capture` | 0.2.0-beta.1 / 2024-03-03 / 71,288 | Captures spans and events for testing assertions; unbounded, not a live log. |

**Recommendation:** write our own in `lib-tracing`: the layer, visitor, `LogEntry`, ring buffer and wake hook come to roughly 150 lines plus tests, with no new dependencies (`tracing-log`'s `NormalizeEvent` is already a dependency; `tokio::sync::Notify` lives on the Desktop side). Make it opt-in through `init` (e.g. `init(level, log_file, capture: Option<&LogCapture>)` or a builder), so `bin-tui` can adopt it later and `sync-server` never installs it.

## Sources

- `tracing-subscriber` 0.3.23 source: `src/layer/mod.rs` (global vs per-layer filtering, interest), `src/layer/layered.rs` (`pick_level_hint`), `src/layer/context.rs` (`event_scope`), `src/filter/env/mod.rs` (`Filter for EnvFilter`), `src/filter/layer_filters/mod.rs`, `src/reload.rs`, `Cargo.toml` features. Rendered docs: <https://docs.rs/tracing-subscriber/0.3.23/tracing_subscriber/layer/index.html#per-layer-filtering>.
- `tracing-core` 0.1.36 source: `src/field.rs` (`Visit`).
- `tracing-log` 0.2.0 source: `src/lib.rs` (`dispatch_record`, `log_cs!`, `NormalizeEvent`), `src/log_tracer.rs` (`init`, `enabled`).
- `gpui` 0.2.2 source: `src/gpui.rs`, `src/app/context.rs`, `src/app/async_context.rs`, `src/app.rs`, `src/executor.rs`, `src/platform/test/dispatcher.rs`, `Cargo.toml`.
- `tokio` 1.53.1 source: `src/sync/mod.rs`.
- `crossbeam-queue` 0.3.14 source: `src/array_queue.rs`.
- This repo: `crates/libs/lib-tracing/src/init.rs`, `crates/bins/bin-desktop/src/shell.rs`, root and crate `Cargo.toml`, `Cargo.lock`.
- crates.io API search results, 2026-10-04.
