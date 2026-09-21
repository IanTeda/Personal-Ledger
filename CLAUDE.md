# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project

Personal Ledger is a Rust Cargo workspace for a personal finance/accounting application (expenses, investments, assets). It is early-stage: the gRPC server currently only wires up a `Ping` utility RPC, and several planned crates/binaries do not exist yet.

## Commands

Toolchain and dev-tool versions (Rust, protoc, mdBook, sqlx-cli, cargo-watch, cargo-audit) are pinned in `mise.toml` at the workspace root and managed by [mise](https://mise.jdx.dev/). Run `mise install` once per checkout (or let mise's shell/dir activation do it) before using `cargo` — without it, tools like `protoc` or `sqlx-cli` won't be on `PATH`.

Build tooling uses `mise` tasks (defined in `mise.toml`; run `mise tasks` to list them) for a few tasks, but most day-to-day work is plain `cargo` run against the workspace or a specific package.

```sh
# Build / check
cargo build                                   # whole workspace
cargo build --package bin_sync_server --bin sync-server   # just the Sync Server binary

# Run the Sync Server (reads config, initialises telemetry, serves the stub Ping RPC)
cargo run --package bin_sync_server

# Run the TUI, rebuilding and rerunning on file changes
mise run watch-tui

# Test
cargo test                                    # whole workspace
cargo test --package lib_config                # single crate
cargo test --package lib_config parse_with_explicit_config_file  # single test

# Lint / format
cargo clippy
cargo fmt

# Docs (mdBook + rustdoc), via mise tasks
mise run docs-build     # docs-rustdoc + docs-mdbook
mise run docs-serve     # serves mdBook on :8001
```

Building `lib_rpc` requires a system `protoc` (protobuf compiler) — provided via `mise.toml`. `tonic_prost_build` regenerates `crates/libs/lib-rpc/src/generated/*.rs` from the `.proto` files on every build; the generated files are checked in but should be treated as build output, not hand-edited.

## Workspace layout

Binary crates live under `crates/bins/`, library crates under `crates/libs/`. Bin crates should be prefixed with `bin-` and library crates should be prefixed with `lib-`. Cargo workspace members (`Cargo.toml`): `crates/bins/*` and `crates/libs/*`.

**`crates/bins/bin-sync-server`** (package `bin_sync_server`, binary `sync-server`) is the Sync Server — an active workspace member as of the [Sync Server feasibility](https://github.com/IanTeda/Personal-Ledger/issues/40) Wayfinder map. It wires a minimal `tonic` gRPC serve loop around `lib_rpc`'s stub `Ping` RPC; the real sync protocol, auth, and Docker packaging are still in progress (see that map's open tickets) — treat anything beyond the `Ping` RPC as not yet built.

**`crates/libs/lib-database` is an active workspace member but currently fails `cargo build`/`cargo check` at the workspace root** without a live `DATABASE_URL` (or `SQLX_OFFLINE=true` against its checked-in `.sqlx` cache) for `sqlx`'s compile-time query macros — see the `db-prepare-generate`/`db-prepare-check` tasks in the root `mise.toml` for the expected `DATABASE_URL`. For local dev (including rust-analyzer/IDE checks, which shell out to `cargo check` and can't be handed `SQLX_OFFLINE`), create a gitignored `.env` at the repo root with `DATABASE_URL=sqlite:<absolute-path-to-repo>/.personal-ledger-dev.db` — `sqlx`'s macros and `sqlx-cli` auto-load `.env` via `dotenvy`, so plain `cargo build`/`check` and the `db-*` mise tasks all pick it up. The dev DB itself must exist first: run `mise run db` (or `db-create` + `db-migrate` if it already exists). Migrations are kept to one `create_<table>_table.sql` file per table while the schema is still in concept (edit the table's file rather than adding an `ALTER`/rebuild migration, then rebuild the dev DB with `mise run db`); the Sync Server's auth table is named `sync_users` so it never collides with the Client-Ledger's domain `accounts` table.

- **`crates/bins/bin-sync-server`** — the Sync Server binary (package `bin_sync_server`, compiled binary `sync-server`, matching the `bin-tui`/`bin-desktop` naming convention). Thin `main.rs`: parses config via `lib_config`, initialises `lib_telemetry`, and serves the `lib_rpc` gRPC services (currently just the stub `Ping` RPC) over `tonic` on a hardcoded placeholder address. The real sync protocol, durable Change Set log, and auth are not yet built — see the [Sync Server feasibility](https://github.com/IanTeda/Personal-Ledger/issues/40) Wayfinder map.
- **`crates/bins/bin-tui`** — package name `bin_tui`, but the compiled binary is `tui` (see its `[[bin]]` override) — end users shouldn't see the `bin_` prefix, it's a codebase-navigation convention only. Six screens from the "TUI App feasibility" Wayfinder map (GitHub issue #7, closed): line/doughnut/candlestick/divergent chart demos, a table demo (all dummy data), and a live screen against real `lib-database` data. Packaged non-root for all three OSes (Linux AppImage, macOS dmg, Windows portable zip) via `.github/workflows/build-publish-tui.yaml`; see `crates/bins/bin-tui/README.md`.
- **`crates/libs/lib-config`** — layered configuration loader (`LedgerConfig::parse`). INI format via the `config` crate. Precedence, lowest to highest: built-in defaults → system config (`/etc/personal-ledger/...`) → user config (XDG/platform config dir) → executable-directory config → CWD `./config/personal-ledger.conf` → explicit path passed to `parse()` → environment variables (`PERSONAL_LEDGER_*`, double-underscore nesting, e.g. `PERSONAL_LEDGER_TELEMETRY__TELEMETRY_LEVEL`). Section headers are lower-cased before parsing so `[Telemetry]`/`[telemetry]` are equivalent. See `docs/configuration.md`.
- **`crates/libs/lib-telemetry`** — `tracing`-based telemetry setup and `TelemetryConfig`/`TelemetryLevels`, consumed by both `lib_config` (for the `[telemetry]` config section) and `bin_sync_server` (for `telemetry::init`).
- **`crates/libs/lib-rpc`** — proto definitions (`proto/personal-ledger/v001/*.proto`) and tonic-generated gRPC client/server code (`src/generated/`), re-exported through `categories.rs` / `utilities.rs` as a flat API. Proto package versioning is `personal_ledger.<service>.v001`.
- **`crates/libs/lib-core`** — pure business/domain types with no I/O: `RowID` (UUIDv7-based), `CategoryTypes` (accounting categories: assets/liabilities/income/expenses/equity), `UrlSlug`, `HexColor`. Designed for SQLite-backed persistence specifically (no Postgres assumptions in the domain types), despite `sqlx`'s Postgres feature being enabled at the workspace level.
- **`crates/libs/lib-database`** — SQLx-based persistence layer. `DatabasePool` wraps connection pooling; `categories/` splits CRUD into separate `find.rs`/`insert.rs`/`update.rs`/`delete.rs`/`builder.rs`/`model.rs` files per entity — follow this split (rather than one big repository file) when adding new persisted entities.

**`crates/libs/lib-locale`** (package `lib_locale`) is the shared localisation crate for `bin-desktop` and `bin-tui`, scaffolded by [#231](https://github.com/IanTeda/Personal-Ledger/issues/231): the `Locale` set and negotiation, embedded Fluent Catalogues, the set-once `init` loader, `with_locale` test override and generated `msg::` accessors. `format::` (ICU4X number, date, currency and `upper` casing for the Locale in effect, per [#232](https://github.com/IanTeda/Personal-Ledger/issues/232)) and typed date input (`parse_date`, [#233](https://github.com/IanTeda/Personal-Ledger/issues/233)) and the `Label` trait for domain-value Messages ([#234](https://github.com/IanTeda/Personal-Ledger/issues/234)) and rich Messages (key-token sentinels and `<tag>` spans returning `Segment`s, [#235](https://github.com/IanTeda/Personal-Ledger/issues/235)) are built on their own tickets. **`crates/libs/lib-locale-build`** is its `fluent-syntax`-only generator, used from `lib-locale`'s `build.rs` and later each bin's. Design in `docs/localisation-design.md` (user guide `docs/localisation.md`), on the [Localisation of the Desktop and TUI UX](https://github.com/IanTeda/Personal-Ledger/issues/211) Wayfinder map, and [ADR-0021](docs/adr/0021-locale-owns-formatting-and-replaces-number-and-date-preferences.md). The Locale is Configuration (`locale` in `lib-config`'s `[Personal-Ledger]` section, resolved default → system → config), not a Preference. Don't add hardcoded UI strings expecting it to be wired into the bins yet; leave literals until its migration tickets land.

Planned-but-not-yet-present binaries/crates mentioned in `docs/directories-files.md` and `README.md` (desktop, web/Leptos frontend) don't exist yet — don't assume they're there.

## Conventions

- Workspace-wide lint: `unsafe_code = "forbid"` (see `Cargo.toml`) — don't introduce `unsafe`.
- A dependency used by two or more workspace crates goes in `[workspace.dependencies]`, and member crates reference it as `dep = { workspace = true }` rather than pinning versions locally. A dependency used by only one crate is declared in that crate's own `Cargo.toml` with its version, not in the workspace `Cargo.toml`; when a second crate needs it, move it up to `[workspace.dependencies]` and switch both crates to `workspace = true`.
- Use `thiserror::Error` for domain/crate error enums (see `lib-database/src/error.rs`, `lib-config/src/error.rs`), and map lower-level errors (e.g. `sqlx::Error`) into structured variants rather than propagating them directly.
- Wrap secrets/tokens in `secrecy::Secret` so they can't leak into logs/traces.
- Avoid `SELECT *` in SQL queries — list explicit columns.
- Comments and rustdoc use Australian English.
- Avoid `unwrap()`/`expect()`/`panic!()` outside tests — propagate with `?` or map into a `thiserror` variant. Every crate has an `error.rs` module exporting its own `Error` enum and `Result<T>` alias (see `lib-database/src/error.rs`); if the crate you are working in lacks one, create it (add `thiserror` per the dependency rule below) before adding fallible code, and have fallible functions return that `Result` rather than `Option`, `Box<dyn Error>` or a panic. Start a new `Error` enum with a single `Generic(String)` catch-all variant (as in `lib-database`) and use it for one-off failures; do not pre-design a variant per failure. Once the same kind of failure has appeared three or more times, promote it to its own specific variant (or a dedicated error type when it carries structure) and convert the call sites. Any error inherited from an external crate gets its own derived variant with `#[from]` (e.g. `Sqlx(#[from] sqlx::Error)`, `Migration(#[from] sqlx::migrate::MigrateError)`) so `?` converts it, rather than being flattened into `Generic` via `to_string()`. The one accepted exception is an invariant proven by the surrounding code (e.g. a literal date) — prefer restructuring so the type system carries the proof, and where an `expect` remains, its message must state the invariant. Binary `main` returns the bin's own `error.rs` `Result`.
- Commit style: `<area>: <short description>` (e.g. `email-verification: add updated_at to model and migration`).
- Tests: unit tests live alongside the code (`#[cfg(test)] mod tests`); integration/DB tests use `sqlx::test`; use the `fake` crate with deterministic seeds for generated test data.

## Agent work on independent tickets

When starting work on a GitHub ticket as an agent (especially on Wayfinder maps where tickets are independent and stand-alone):
- Run `/clear` at the start of your session if the ticket is fully self-contained and the Wayfinder map/GitHub ticket body provides all necessary context
- This keeps context fresh and prevents accumulated state from prior tickets from creating noise
- Do NOT clear context if your task depends on understanding prior changes or multi-ticket coordination within the same conversation
- After clearing, the ticket description will be your primary reference — ensure it has everything needed (acceptance criteria, file lists, specifications)

Wayfinder tickets (like localization migration tickets #247-#252) typically benefit from clearing because:
- Each ticket is independent in scope
- All specifications are in the ticket body
- File paths are explicit
- Prior ticket's test output/build state is not needed

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
