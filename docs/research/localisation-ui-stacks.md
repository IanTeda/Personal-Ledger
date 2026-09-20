# Research: gpui, gpui-component and ratatui text and locale behaviour

**Question.** Issue #214 (child of map #211, "Localisation of the Desktop and TUI UX") asks how the two UI stacks interact with localised text: (1) whether `gpui-component` ships its own i18n and how that coexists with a Fluent-based `lib-localisation`; (2) how a runtime Locale change would propagate in `gpui`; (3) how `ratatui` measures, truncates and wraps text, so the `en-XA` pseudo-Locale is a meaningful test. This note reports findings and the constraints they impose. It does not decide anything. Every claim is cited to primary source: the crate sources in the local cargo registry (`~/.cargo/registry/src/index.crates.io-*/`) at the versions the workspace pins (`gpui` 0.2.2, `gpui-component` 0.5.1, `ratatui` 0.30.2, resolved through `ratatui-core` 0.1.2 and `ratatui-widgets` 0.3.2, plus `rust-i18n` 3.1.5 and `unicode-width` 0.2.2). Nothing was executed; behaviour described is read from source. Research date 2026-09-20.

## 1. gpui-component ships its own i18n

**It uses `rust-i18n`.** `gpui-component-0.5.1/Cargo.toml` depends on `rust-i18n` version 3, and `src/lib.rs` line 92 invokes `rust_i18n::i18n!("locales", fallback = "en");`. Strings are looked up with `rust_i18n::t!` at the widget call sites (for example `src/dialog.rs`, `src/select.rs`, `src/time/calendar.rs`).

**Public API.** `src/lib.rs` (lines 118 to 126) exposes exactly two functions: `gpui_component::locale()` returning a `Deref<Target = str>`, and `gpui_component::set_locale(&str)`. Both are thin wrappers over `rust_i18n::locale()` and `rust_i18n::set_locale()`. There is no per-window, per-`App` or per-entity locale; nothing takes a `cx`.

**The current Locale is a process-wide static.** `rust-i18n-3.1.5/src/lib.rs` line 16 declares `static CURRENT_LOCALE: Lazy<AtomicStr>` initialised to `"en"`, and `set_locale` replaces it. It is not a gpui `Global` and setting it does not notify anything.

**Locales shipped.** A single file, `gpui-component-0.5.1/locales/ui.yml` (`_version: 2`, key-per-locale layout), carries `en`, `zh-CN` and `zh-HK` for 40 keys each, and `it` for 32 keys. `it` is missing 8 keys, so those fall back to `en`. There is no Australian English, `en-XA`, or other locale.

**Fallback behaviour.** The `i18n!` macro (`rust-i18n-macro-3.1.5/src/lib.rs`, `_rust_i18n_lookup_fallback` at line 351 and `_rust_i18n_try_translate` at line 373) tries the exact locale, then trims the last `-` subtag repeatedly (`en-XA` becomes `en`, `zh-CN` becomes `zh`), then the configured fallback `en`. If no locale has the key, `_rust_i18n_translate` returns the string `"<locale>.<key>"` rather than an error. So setting `gpui_component::set_locale("en-XA")` would show plain English for every widget string, not pseudo-localised text, unless `en-XA` entries are supplied to the crate's backend (which is compiled from its own `ui.yml`; the workspace cannot extend it without forking or wrapping).

**Which strings are built in.** Grepping `t!("...")` in `src/` gives these key groups (all in `locales/ui.yml`):

- `Calendar`: `week.0` to `week.6` (two-letter weekday headers, Sunday first) and `month.January` to `month.December` (`src/time/calendar.rs`, lines 469 to 480 and 498 onwards).
- `DatePicker.placeholder` (`src/time/date_picker.rs` line 364) and `Select.placeholder` (`src/select.rs` line 740).
- `Dialog.ok` and `Dialog.cancel` (`src/dialog.rs` lines 304 and 334).
- `List.search_placeholder` (`src/list/list.rs` line 97), `Settings.search_placeholder` and `Settings.Reset All`.
- `Input.Cut`, `Copy`, `Paste`, `Select All`, `Replace`, `Replace All`, `Go to Definition`, `Show Code Actions` (context menu and search, `src/input/popovers/context_menu.rs`, `src/input/search.rs`).
- `Dock.Close`, `Collapse`, `Expand`, `Unnamed`, `Zoom In`, `Zoom Out`.

**Which are overridable without touching the locale.** Most of the widget strings are supplied by the caller when the caller wants. Confirmed in source: `Dialog` `button_props.ok_text` / `cancel_text` (`src/dialog.rs` lines 302 to 304, `unwrap_or_else` to the `t!` default); the `Select` and `DatePicker` placeholders (`unwrap_or_else(|| t!(...))`); `InputState::placeholder`. The `Calendar` weekday and month names have no override in what was read: `month_name` (line 462) and `months` (line 495) call `t!` directly. The context-menu strings (`Input.*`) and `Dock.*` were not found to take an override either.

**Date formatting is not locale-aware.** `DatePicker` stores `date_format` defaulting to `"%Y/%m/%d"` (`src/time/date_picker.rs` line 129), settable by `date_format()`, and formats through `chrono`'s `NaiveDate::format` (`src/time/calendar.rs` line 96). `chrono`'s plain `format` does not localise. The calendar's first weekday is not driven by locale (no week-start setting found in `src/time/`; the header order comes from the `week.0` to `week.6` keys). Number and currency display in the workspace's own views is `format!("{:.2}", ...)` (`crates/bins/bin-desktop/src/main.rs` lines 329 and 603), so nothing in either stack is locale-aware today.

### Coexistence with a Fluent `lib-localisation`

- Two independent translation systems will exist: `rust-i18n` inside `gpui-component` (English, Chinese, Italian only) and Fluent in `lib-localisation`. They share nothing but the Locale identifier the application chooses.
- The application must call `gpui_component::set_locale` itself whenever its own current Locale changes, mapping the workspace Locale to one of `en`, `zh-CN`, `zh-HK`, `it`. For any Locale outside those (including `en-AU` if that is a shipped Locale, and `en-XA`), the mapping lands on `en` through the fallback rule above.
- We are forced to leave to `gpui-component` (or to fork): calendar month and weekday names, the `Input.*` context menu, `Dock.*`, `Settings.Reset All`. We can and should override at the call site: `Dialog` button labels, placeholders on `Select`, `DatePicker`, `List` and `Input`.
- Consequence for testing: `en-XA` cannot pseudo-localise the forced strings. That is acceptable so long as the widgets in question are either avoided or their strings are overridden from `lib-localisation` Messages at the call site.

## 2. Runtime Locale change in gpui

**gpui has no Locale concept.** A case-insensitive search of `gpui-0.2.2/src` for "locale" hits only platform code (`platform/linux/platform.rs`, `platform/mac/open_type.rs`, `platform/windows/direct_write.rs`), i.e. font and platform plumbing. There is no locale `Global`, no text-direction handling in the element layer and no translation hook.

**Views are immediate-mode; they re-run `render` on redraw.** `Render::render` is called each time an entity is redrawn (see the workspace's `impl Render for DesktopApp` at `crates/bins/bin-desktop/src/main.rs` line 478, which builds its element tree from state on every call). A `t!`-style lookup placed inside `render` therefore picks up a new Locale the next time that view redraws. Text is not cached by gpui between renders: `LineLayoutCache` (`gpui-0.2.2/src/text_system/line_layout.rs` line 392) holds shaped layouts keyed by the text content and style for the previous and current frame, so changed text just misses the cache and is re-shaped.

**Ways to force a redraw after a Locale change.** All exist in gpui 0.2.2:

- `App::refresh_windows()` (`src/app.rs` line 755) queues an `Effect::RefreshWindows`, redrawing every window once even if called repeatedly in one update. This is the blunt full re-render option.
- `Window::refresh()` (`src/window.rs` line 1367) marks one window dirty.
- `Context::notify()` (`src/app/context.rs` line 229) marks one entity, and its observers, dirty.
- A `Global` (`pub trait Global: 'static`, `src/global.rs` line 22) set with `App::set_global` (`src/app.rs` line 1497) plus `observe_global` (`App` line 1522, `Context` line 176, `Window` line 4093, `observe_global_in` line 671) lets views subscribe to a Locale change and call `cx.notify()`. gpui docs for `Global` recommend a newtype with accessor methods.

**Where views do cache text.** Text captured at construction time will not follow a Locale change:

- Anything stored in a `SharedString` field on view state (for example a title computed once in `new`).
- `gpui-component` builders that resolve their default at build time: `Dialog` reads `t!("Dialog.ok")` inside `render`-time builder code (`src/dialog.rs` lines 302 to 304, evaluated in the `Dialog` render path when the dialog is built), so an already open dialog keeps its old label until reopened; `List` creates its search `InputState` with `placeholder(t!("List.search_placeholder"))` in the constructor (`src/list/list.rs` line 97), so an existing list keeps the old placeholder until recreated. Widgets that call `t!` inside their own `render` (calendar month names are computed by `month_name` when drawing) follow the change on the next redraw. Which category each widget is in has to be checked per widget; the constructor-time ones are the risk.
- The workspace's `bin-desktop` today renders literals directly (`format!("Failed to load: {message}")`, `Day {day}` at `main.rs` lines 136 and 621), so there is no lookup layer yet to cache anything.

**Constraints for the lookup API and Locale location (gpui).**

- The lookup must be callable from `render` without a `cx` if views are to stay simple. Otherwise each call needs `cx.global::<...>()` access, which is fine but noisier.
- The current Locale should have one owner outside gpui (in `lib-localisation` or the app), because `gpui-component` keeps its own static that the app has to mirror. A gpui `Global` holding the Locale is a good notification vehicle, but it cannot be the only copy, since the TUI has no gpui and `gpui-component` reads the `rust-i18n` static.
- A Locale switch handler should do three things in order: update the shared Locale, call `gpui_component::set_locale` with the mapped tag, then `cx.refresh_windows()`. Long-lived widgets that cache placeholders need to be rebuilt or have their placeholder reset.

## 3. ratatui text width and expansion

**Width is measured in terminal cells via `unicode-width` and grapheme segmentation.**

- `ratatui-core-0.1.2/Cargo.toml` depends on `unicode-segmentation` and `unicode-truncate`; `ratatui-widgets-0.3.2` depends on `unicode-segmentation`. `unicode-width` 0.2.2 is in the workspace lockfile as a transitive dependency.
- `Span::width()` (`ratatui-core-0.1.2/src/text/span.rs` line 271) is `UnicodeWidthStr::width`. The buffer measures with a `CellWidth` trait (`ratatui-core-0.1.2/src/buffer/cell_width.rs`): a one-byte string is width 1, otherwise `str::width()` plus a correction for the halfwidth Japanese sound marks U+FF9E and U+FF9F, which `unicode-width` reports as zero.
- Wrapping and truncation in `Paragraph` and other widgets operate on grapheme clusters (`ratatui-widgets-0.3.2/src/reflow.rs`, using `UnicodeSegmentation::graphemes(src, true)` and `cell_width()`); the module has a `LineTruncator` (line 236 onwards) and word wrapping, selected with `Paragraph::wrap(Wrap { trim })` (`paragraph.rs` line 216). Without `wrap`, over-long lines are truncated at the widget width, not wrapped.

**How accented text behaves.** `unicode-width` 0.2.2 follows UAX #11 (its README states this). Precomposed accented Latin letters (for example U+00E9, `é`) are one cell wide, as is ASCII. A base letter followed by a combining mark (decomposed form) also measures one cell, because combining marks have width zero and ratatui iterates by grapheme cluster. So Latin accents do not break layout; the two forms only differ in byte length. The same README warns computed widths "may not match the actual rendered column width" for complex scripts (its example is Devanagari conjuncts), and the terminal, not the library, decides actual rendering. CJK characters are two cells wide by default; `width_cjk` exists but ratatui uses `width()`, so East Asian ambiguous-width characters are treated as narrow. This matters for any future `zh-CN` TUI Locale but not for `en-XA`, which is Latin.

**Expansion.** ratatui does no layout adaptation for longer text. `Constraint`-based layout allocates cells independent of content, and `Table` column widths are set by `.widths()` (per the existing research in `docs/research/tui-charting-libraries.md`, which notes columns render at width 0 if omitted). A Message longer than the English original is simply cut at the cell boundary by `Span`/`Line` rendering, or wrapped if a `Paragraph` has `wrap` enabled. Nothing truncates with an ellipsis automatically; that has to be done by the application (using `unicode-truncate` or grapheme iteration) if wanted. Fixed-width columns and one-line status hints are therefore where expansion will show up as clipped text.

**Why `en-XA` is a meaningful TUI test.** A pseudo-Locale that both accents Latin letters and pads each Message by roughly a third exercises the two things that matter here: multi-byte characters (proving nothing indexes strings by byte or `char` count rather than cell width) and expansion (proving truncation and wrapping paths are reached). Because `unicode-width` gives accented Latin letters width 1, the accents test byte and `char` indexing bugs in application code, not ratatui itself; the padding is what tests layout. Bracket delimiters typically used by pseudo-Locales (`[!! ... !!]`) make truncation visible, and so do they for missing-Message detection. This is a reasoning from the source above, not something demonstrated in a running TUI.

**Constraints for the lookup API (TUI).**

- Lookup must return owned or borrowed `str` usable for `Span::raw`, `Line::from` and `Cell::from`; nothing about ratatui requires `'static`.
- Any width arithmetic done in the TUI's own code (centring, padding, column sizing from Message lengths) must use `UnicodeWidthStr::width` or ratatui's helpers, never `str::len()` or `chars().count()`.
- The current Locale can live in the TUI's `App` state (or a `lib-localisation` handle held there) and be read when building each frame, since ratatui redraws from state each tick; no notification mechanism is needed, unlike gpui.
- Fixed-width UI (table headers, hint strips, tab titles) needs either generous constraints or explicit truncation with an ellipsis for `en-XA` to pass cleanly.

## Summary of constraints for the lookup API and where the Locale lives

- One owner for the current Locale, outside both UI crates (`lib-localisation`), read synchronously by both binaries. gpui-component's `rust-i18n` static must be kept in sync with it through `gpui_component::set_locale`, because it cannot be told otherwise.
- The lookup should work without a gpui `cx` in the call path, and return text at call time (not at construction) so `render`-time lookups pick up changes. A gpui-side `Global` is useful only as a change signal.
- After a change: `set_locale`, then `refresh_windows()`; rebuild or reset widgets that cached placeholders at construction.
- `gpui-component` covers only `en`, `zh-CN`, `zh-HK` and `it`; `en-AU` and `en-XA` resolve to English there. Calendar names, edit-menu and dock strings cannot be overridden from outside in 0.5.1 as far as read, so either avoid those widgets or accept English for them.
- Dates, numbers and currency are not locale-aware in either stack; that has to come from `lib-localisation`, with formats passed in (`DatePicker::date_format`, ratatui cell contents).
- TUI: measure by cell width, plan for clipping, use `en-XA` to find it.

## Not verified

- No code was run. Widget-by-widget timing of `t!` calls (constructor versus render) was checked for `Dialog`, `List`, `Select`, `DatePicker` and `Calendar` only.
- Whether `gpui-component` has an override for the calendar month and weekday names or the `Input.*` menu strings was inferred from the call sites, not from an exhaustive API audit.
- Actual terminal rendering of accented text depends on the terminal and font.
