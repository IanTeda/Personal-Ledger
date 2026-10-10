---
name: ui-tests
description: Write or extend headless keyboard and mouse integration tests for the Desktop UI (bin-desktop) using the gpui test-support harness. Use when adding a test under crates/bins/bin-desktop/tests/ or when a Desktop UI change needs behaviour coverage.
---

# Desktop UI tests

Run them with `mise run end-to-end-ui <view>` while developing (headless; no window or GPU, so it works in Claude's sandbox), where `<view>` is a substring of the `tests/` file names, e.g. `settings_inventory` or `documents`. The bare `mise run end-to-end-ui` (the whole suite) is required only when closing a parent ticket (a Wayfinder map or any issue with sub-issues), not for each child ticket or push. Design and gating are in ADR-0030.

## Harness (`tests/common/mod.rs`)

Each test file starts with `mod common;` and takes a `&mut TestAppContext` under `#[gpui::test]`.

- `Harness::new(app)` boots the app-level globals, pins the Locale to en-US and builds `Shell` through the lib's `build_shell` with `today()` fixed at 2026-06-15.
- `ui.press("g f")` presses a space-separated keystroke sequence through the real key router, then settles.
- `ui.click("selector")` clicks the centre of the element tagged `.debug_selector(|| "selector".to_string())` and panics if it was not drawn.
- `ui.noun()`, `ui.documents()` and `ui.read(|shell| ...)` read state back from `Shell`.

## Model

Haiku is the minimum, per the model-selection policy in `CLAUDE.md`: use it to add a test that follows the existing harness and a file such as `documents_keyboard.rs` or `documents_mouse.rs`, with the flow and expected state spelled out (an explicit ticket, a new case for a surface already covered). Use Sonnet or Opus when the work needs design judgement: a new surface with no precedent, a new accessor or snapshot shape, the OS-call seam, a harness change, or diagnosing a flaky or hanging test. If Haiku hits any of these, or an unexpected result, it should stop and escalate to the user rather than guess.

## When to add a test

Add one for user-visible keyboard or mouse behaviour: a key-router scope, a click target, a mode or selection change. Pure logic belongs in a unit test beside the code (`/unit-tests`).

## Adding what a test needs

- **A click target:** tag only the element the test clicks with `debug_selector`. It needs no `cfg`; gpui makes it a no-op without `test-support`. Never hard-code coordinates. Rows below the fold have no bounds, so reach them with keys.
- **A readback:** add a narrow read-only `#[doc(hidden)]` accessor or snapshot (like `DocumentsSnapshot`) on `Shell`. Keep `Shell` fields private and make a module `pub` only when a test needs it. No facade up front.

## What not to assert

- Pixels, screenshots or colours, and comparison against the `.dc.html` frames; those are a manual spot-check.
- Anything needing the wall clock, the host locale, `persistence`, the database or a real OS call (`Open`, `Show in folder`, CSV export). Keep tests hermetic; if one needs an OS call, add an injectable seam rather than performing it.
- Private layout details. "Is it drawn" is fine, as `debug_bounds` existing.

Conventions: `#![expect(clippy::..., reason = "...")]` for the `expect`/`panic` a test file uses, and `mise run lint` clean before pushing.
