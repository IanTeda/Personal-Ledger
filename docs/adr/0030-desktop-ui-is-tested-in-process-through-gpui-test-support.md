# The Desktop UI is tested in process through gpui's test support

We want Claude, Ian and later CI to integration-test the Desktop's keyboard and mouse behaviour with `cargo test`. The alternative is scripting the compositor (`wtype`, `hyprctl`, `grim`) against a real window, but Claude's sandbox cannot initialise the GPU, the result is slow and flaky, and it would not run in CI without a virtual display. So we test in process through gpui's `test-support` (`TestAppContext`, `VisualTestContext`, `simulate_keystrokes`, `simulate_click`), which drives the real `Shell`, key router and mouse handlers with no window. Compositor scripting stays a manual visual spot-check against the `.dc.html` frames.

To let tests build `Shell` the way the app does, `bin_desktop` is a lib plus a thin bin in one package (no new crate): modules live in `src/lib.rs` with `[lib] doctest = false`, `main.rs` is config, tracing and a call to `run`, and both go through `build_shell`. The `test-support` Cargo feature turns on `gpui/test-support`, and only a self dev-dependency enables it, so shipped builds never compile it. Tests click elements tagged with `debug_selector` and found with `debug_bounds`, never coordinates, and assert on state read back from `Shell` through narrow `#[doc(hidden)]` accessors, never on pixels.

## Consequences

- Modules become `pub` only as a test needs them, and `Shell` fields stay private.
- Tests are hermetic: a fixed `today`, the locale pinned to en-US with `locale::init_for_tests`, the deterministic stub seed, no `persistence` calls and no real OS calls. A seam for `Open`, `Show in folder` and CSV export is added when the first test needs it.
- `mise run test-ui` runs the suite, and `.github/workflows/test-ui-main-push.yaml` runs it in CI on `main` pushes and PRs, headless with no virtual display.
- Pixel or screenshot comparison and testing the TUI are out of scope.
