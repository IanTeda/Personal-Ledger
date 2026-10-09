# The TUI is tested in process through ratatui's TestBackend

We want Claude, Ian and later CI to integration-test the TUI's keyboard behaviour with `cargo test`, as ADR-0030 does for the Desktop. The TUI has no window and no GPU, so nothing like the Desktop's gpui harness is needed. Driving the real `Shell` with crossterm key events and reading the drawn screen from ratatui's `TestBackend` is enough, and it runs headless on Claude's sandbox and in GitHub Actions. Scripting a real terminal (a pty smoke tier against the compiled `tui`) is deferred: its feasibility in both places is unknown, and the in-process tier covers the keyboard grammar without it.

To let tests build `Shell` the way the app does, `bin_tui` becomes a lib plus a thin bin in one package, as `bin_desktop` already is. Modules move into `src/lib.rs` with `[lib] doctest = false`, and `main.rs` keeps only the CLI, config, tracing, locale initialisation and terminal colour detection before handing off to the lib's entry point. The generated `msg` module moves with them. `build_shell` takes the resolved `Config` and a `Colours` value, and never touches the locale or the terminal, so tests pass a fixed colour set and call `locale::init_for_tests` themselves. `locale::init` is set-once, which is why it stays in `main`.

No `test-support` Cargo feature is added: `TestBackend` ships in ratatui's normal build, so the harness needs no extra dependency and no self dev-dependency.

Tests assert on what the frame shows, because unlike gpui the TUI has a readable frame. Drawn screen text is the default, checked with whole-frame `contains` plus harness helpers for a row or the line holding some text when a label could appear in more than one place. Behaviour that lives only in a style (the selected row, a focused pane, an error tint) is checked by Colour Role: the cell's style equals the named style from the same `Colours` the test built the `Shell` with, never a raw colour or modifier. Whole-frame snapshots (`insta`) are not used: with every View being restructured they would churn on each layout or Catalogue tweak and say nothing about what a test is for.

A `#[doc(hidden)]` accessor on `Shell` is allowed only when the behaviour under test has no visible effect in the frame, as text or as a Colour Role (a store change to a field the View doesn't show, a changed Preference). It is never a shortcut for state the screen does show. Each is read-only, returns plain data and is one narrow function per fact, promoted to a per-View snapshot struct only once a View needs three or more readbacks.

## Consequences

- Modules are `pub(crate)` by default. `Shell`, `build_shell` and the types a test must name are `pub`, and a wider surface is added only when a test needs it. `Shell` fields stay private.
- Tests are hermetic: a fixed `today`, the locale pinned with `locale::init_for_tests`, the deterministic fixture seed, default keybindings, a fixed colour depth, no database and no terminal probe.
- Existing in-module tests in `shell.rs` are not backfilled. Each View gains `tests/<view>_keyboard.rs` as it is restructured.
- The harness gains a role-based style check and row/line helpers when the first test needs them; none exist yet.
- The pty smoke tier, shell-level chrome tests and mouse tests (the TUI has no mouse support) are out of scope for this decision.
- ADR-0030 still governs the Desktop. Its out-of-scope line for the TUI now points here.
