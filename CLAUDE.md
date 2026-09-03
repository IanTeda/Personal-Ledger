# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project

Personal Ledger is a Rust Cargo workspace for a personal finance/accounting application (expenses, investments, assets). It is early-stage: the gRPC server currently only wires up a `Ping` utility RPC, and several planned crates/binaries do not exist yet.

## Commands

Toolchain and dev-tool versions (Rust, protoc, mdBook, cargo-make, sqlx-cli, cargo-watch, cargo-audit) are pinned in `mise.toml` at the workspace root and managed by [mise](https://mise.jdx.dev/). Run `mise install` once per checkout (or let mise's shell/dir activation do it) before using `cargo`/`cargo make` — without it, tools like `protoc` or `cargo-make` won't be on `PATH`.

Build tooling uses `cargo-make` (`Makefile.toml`) for a few tasks, but most day-to-day work is plain `cargo` run against the workspace or a specific package.

```sh
# Build / check
cargo build                                   # whole workspace
cargo build --package bin_sync_server --bin sync-server   # just the Sync Server binary

# Run the Sync Server (reads config, initialises telemetry, serves the stub Ping RPC)
cargo run --package bin_sync_server

# Test
cargo test                                    # whole workspace
cargo test --package lib_config                # single crate
cargo test --package lib_config parse_with_explicit_config_file  # single test

# Lint / format
cargo clippy
cargo fmt

# Docs (mdBook + rustdoc), via cargo-make
cargo make docs-build     # docs-rustdoc + docs-mdbook
cargo make docs-serve     # serves mdBook on :8001
```

Building `lib_rpc` requires a system `protoc` (protobuf compiler) — provided via `mise.toml`. `tonic_prost_build` regenerates `crates/libs/lib-rpc/src/generated/*.rs` from the `.proto` files on every build; the generated files are checked in but should be treated as build output, not hand-edited.

## Workspace layout

Binary crates live under `crates/bins/`, library crates under `crates/libs/`. Bin crates should be prefixed with `bin-` and library crates should be prefixed with `lib-`. Cargo workspace members (`Cargo.toml`): `crates/bins/*` and `crates/libs/*`.

**`crates/bins/bin-sync-server`** (package `bin_sync_server`, binary `sync-server`) is the Sync Server — an active workspace member as of the [Sync Server feasibility](https://github.com/IanTeda/Personal-Ledger/issues/40) Wayfinder map. It wires a minimal `tonic` gRPC serve loop around `lib_rpc`'s stub `Ping` RPC; the real sync protocol, auth, and Docker packaging are still in progress (see that map's open tickets) — treat anything beyond the `Ping` RPC as not yet built.

**`crates/libs/lib-database` is an active workspace member but currently fails `cargo build`/`cargo check` at the workspace root** without a live `DATABASE_URL` (or `SQLX_OFFLINE=true` against its checked-in `.sqlx` cache) for `sqlx`'s compile-time query macros — see the `sqlx-prepare`/`sqlx-prepare-check` tasks in the root `Makefile.toml` for the expected `DATABASE_URL`.

- **`crates/bins/bin-sync-server`** — the Sync Server binary (package `bin_sync_server`, compiled binary `sync-server`, matching the `bin-tui`/`bin-desktop` naming convention). Thin `main.rs`: parses config via `lib_config`, initialises `lib_telemetry`, and serves the `lib_rpc` gRPC services (currently just the stub `Ping` RPC) over `tonic` on a hardcoded placeholder address. The real sync protocol, durable Change Set log, and auth are not yet built — see the [Sync Server feasibility](https://github.com/IanTeda/Personal-Ledger/issues/40) Wayfinder map.
- **`crates/bins/bin-tui`** — package name `bin_tui`, but the compiled binary is `tui` (see its `[[bin]]` override) — end users shouldn't see the `bin_` prefix, it's a codebase-navigation convention only. Six screens from the "TUI App feasibility" Wayfinder map (GitHub issue #7, closed): line/doughnut/candlestick/divergent chart demos, a table demo (all dummy data), and a live screen against real `lib-database` data. Packaged non-root for all three OSes (Linux AppImage, macOS dmg, Windows portable zip) via `.github/workflows/build-publish-tui.yaml`; see `crates/bins/bin-tui/README.md`.
- **`crates/libs/lib-config`** — layered configuration loader (`LedgerConfig::parse`). INI format via the `config` crate. Precedence, lowest to highest: built-in defaults → system config (`/etc/personal-ledger/...`) → user config (XDG/platform config dir) → executable-directory config → CWD `./config/personal-ledger.conf` → explicit path passed to `parse()` → environment variables (`PERSONAL_LEDGER_*`, double-underscore nesting, e.g. `PERSONAL_LEDGER_TELEMETRY__TELEMETRY_LEVEL`). Section headers are lower-cased before parsing so `[Telemetry]`/`[telemetry]` are equivalent. See `docs/configuration.md`.
- **`crates/libs/lib-telemetry`** — `tracing`-based telemetry setup and `TelemetryConfig`/`TelemetryLevels`, consumed by both `lib_config` (for the `[telemetry]` config section) and `bin_sync_server` (for `telemetry::init`).
- **`crates/libs/lib-rpc`** — proto definitions (`proto/personal-ledger/v001/*.proto`) and tonic-generated gRPC client/server code (`src/generated/`), re-exported through `categories.rs` / `utilities.rs` as a flat API. Proto package versioning is `personal_ledger.<service>.v001`.
- **`crates/libs/lib-domain`** — pure business/domain types with no I/O: `RowID` (UUIDv7-based), `CategoryTypes` (accounting categories: assets/liabilities/income/expenses/equity), `UrlSlug`, `HexColor`. Designed for SQLite-backed persistence specifically (no Postgres assumptions in the domain types), despite `sqlx`'s Postgres feature being enabled at the workspace level.
- **`crates/libs/lib-database`** — SQLx-based persistence layer. `DatabasePool` wraps connection pooling; `categories/` splits CRUD into separate `find.rs`/`insert.rs`/`update.rs`/`delete.rs`/`builder.rs`/`model.rs` files per entity — follow this split (rather than one big repository file) when adding new persisted entities.

Planned-but-not-yet-present binaries/crates mentioned in `docs/directories-files.md` and `README.md` (desktop, web/Leptos frontend) don't exist yet — don't assume they're there.

## Conventions

- Workspace-wide lint: `unsafe_code = "forbid"` (see `Cargo.toml`) — don't introduce `unsafe`.
- Shared dependency versions live in `[workspace.dependencies]`; reference them from member crates as `dep = { workspace = true }` rather than pinning versions locally.
- Use `thiserror::Error` for domain/crate error enums (see `lib-database/src/error.rs`, `lib-config/src/error.rs`), and map lower-level errors (e.g. `sqlx::Error`) into structured variants rather than propagating them directly.
- Wrap secrets/tokens in `secrecy::Secret` so they can't leak into logs/traces.
- Avoid `SELECT *` in SQL queries — list explicit columns.
- Comments and rustdoc use Australian English.
- Avoid `unwrap()`/`expect()` outside tests — propagate with `?` or map into a `thiserror` variant.
- Commit style: `<area>: <short description>` (e.g. `email-verification: add updated_at to model and migration`).
- Tests: unit tests live alongside the code (`#[cfg(test)] mod tests`); integration/DB tests use `sqlx::test`; use the `fake` crate with deterministic seeds for generated test data.

## Agent skills

### Issue tracker

Issues live as GitHub issues on `IanTeda/Personal-Ledger`; use the `gh` CLI. See `docs/agents/issue-tracker.md`.

### Triage labels

Default five canonical labels (`needs-triage`, `needs-info`, `ready-for-agent`, `ready-for-human`, `wontfix`), used 1:1. See `docs/agents/triage-labels.md`.

### Domain docs

Single-context: `CONTEXT.md` + `docs/adr/` at the repo root. See `docs/agents/domain.md`.

### Markdown formatting

Headings get a blank line before and after; paragraphs and list items are written as a single unwrapped line relying on word wrap, not hard-wrapped. See `docs/agents/markdown-style.md`.

### Writing docs, tracing, and tests

Project skills under `.claude/skills/` capture this repo's conventions in more depth than fits here — invoke them (or let them auto-trigger) when doing the matching work: `/rustdocs` for rustdoc comments, `/tracing` for `tracing::instrument`/log-level conventions, `/unit-tests` for `fake`-crate mock data and `sqlx::test` patterns.
