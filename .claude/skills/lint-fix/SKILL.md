---
name: lint-fix
description: Get the branch lint-clean so the GitHub Lint workflow passes. Use before any `git push`, when the Lint workflow or pre-push hook fails, or when clippy/rustfmt report errors.
---

# Lint fix

The Lint workflow (`.github/workflows/lint.yaml`) and `mise run lint` run the same two checks: `cargo fmt --check`, then `cargo clippy --workspace --all-targets -- -D warnings` with `SQLX_OFFLINE=true`. The target state is **green**: `mise run lint` exits 0. Every step below serves that one observable.

## Steps

1. **Baseline.** Run `mise run lint`. Green → skip to step 5. Otherwise note every diagnostic: clippy stops at the first failing crates, so add `--keep-going` when running clippy by hand to see the whole workspace at once.
2. **Stale query cache.** Any `no cached data for this query` error means `lib-database`'s `.sqlx` cache is behind its `sqlx::query!` calls (test modules included). Run `mise run db` (rebuilds the dev DB and runs `db-prepare-generate`), then stage the new `crates/libs/lib-database/.sqlx/*.json` files.
3. **Mechanical fixes.** Run `mise run lint-fix`, then `mise run lint` again. Review the diff it made before moving on: `--fix` can break a build where a lint only fires under `cfg(test)` (see *Gotchas*).
4. **Hand fixes.** Work through each remaining diagnostic using *Fixing by lint* below until `mise run lint` is green.
5. **Prove it.** Run `cargo test` for every crate you touched, since refactors made for a lint change behaviour. A test that also fails on the base commit is pre-existing: note it rather than fixing it in this change.
6. **Report.** List what was fixed mechanically, what was fixed by hand, and every `#[expect]` added with its reason, so the human can challenge any suppression.

## Fixing by lint

Fix at the source. The workspace config (`Cargo.toml` `[workspace.lints]`, `clippy.toml`) and CI's `-D warnings` are fixed points; a lint you disagree with is a conversation with the human, never a config edit.

- **`unwrap_used` / `expect_used` / `panic` / `unwrap_in_result`**: restructure so the value can't be absent. Pair the data earlier instead of looking it up again (`filter_map` to `(row, item)`), use an infallible form (`today - Duration::days(today.day0().into())` instead of `with_day(1)`), or `map_err` into the crate's `Error` and `?`. For a genuine invariant, use `#[expect(clippy::expect_used, reason = "<the invariant>")]` with the `expect` message stating it too (see `/rust-style`).
- **`too_many_arguments`**: group what callers already hold together into a struct, following the existing pattern: `CategoriesPageProps` (page data and handlers passed down as `&props`), `AccountOptions`, `DialogHandlers` (handlers shared by sibling dialogs, destructured at the top of `render`).
- **`type_complexity`**: name the type with a `type` alias next to its siblings (the `On*Click` aliases in each view).
- **`elided_lifetimes_in_paths`** (via `rust_2018_idioms`): `--fix` won't apply these. Write `'_` where the compiler's `help:` line shows it (`Frame<'_>`, `Formatter<'_>`, `Arg<'_>`). For dozens, apply the suggestion spans from `cargo clippy --message-format=json` with a short script.
- **`if_same_then_else`** and other logic lints: treat as a probable bug and find the intended branch (the Categories progress bar had `ACCENT` on both arms; `theme.rs` reserved it for over-budget).
- **`allow_attributes_without_reason`**: turn `#[allow(x)]` into `#[expect(x, reason = "...")]`, which warns once the lint stops firing. Keep `#[allow(x, reason = "...")]` only where the lint may or may not fire (a module-level `unused`, generated code).
- **`unfulfilled_lint_expectations`**: the suppressed lint no longer fires; delete the `#[expect]`.

## Gotchas

- **Stale cache hidden locally.** A plain local `cargo clippy` reads the live dev DB through `.env` and passes; CI and `mise run lint` use the `.sqlx` cache. Only trust a run with `SQLX_OFFLINE=true`.
- **`cfg(test)`-only imports.** When a name is imported only under `#[cfg(test)]`, `unused_qualifications` fires in the test build and `--fix` strips the qualifier from non-test code, which then fails to compile. Make the import unconditional (as `bin-tui`'s `pub use commands::DOMAINS` is) or keep the qualified path.
- **Generated code** (`lib-rpc/src/generated/`, `lib-locale`'s `msg`) is exempted at its `mod` declaration. A new lint that fires there goes into that `#[allow(..., reason)]` list; the files themselves are rewritten each build.
- **`mise run` refusing to start** because a pinned tool is missing (e.g. a sandbox with a read-only mise install dir): run the task's commands from `mise.toml` directly, with the same `env`.
- **Build-script warnings** (`unused Message accessor ...`) come from `lib-locale-build` and don't fail `-D warnings`; they are not lint errors.
