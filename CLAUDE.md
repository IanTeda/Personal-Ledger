# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project

Personal Ledger is a Rust Cargo workspace for a personal finance/accounting application (expenses, investments, assets). It is still in concept: the Desktop and TUI Clients are built UI-first, with most screens rendering in-memory mock data and the persistence wiring a later phase, and the Sync Server serves its feasibility-cycle surface (a `Push`/`Pull` `SyncService` behind an auth interceptor, plus the `/authorize` and `/token` HTTP endpoints) rather than a finished sync protocol.

## Commands

Toolchain and dev-tool versions (Rust, protoc, mdBook, sqlx-cli, cargo-watch, cargo-deny) are pinned in `mise.toml` at the workspace root and managed by [mise](https://mise.jdx.dev/). Run `mise install` once per checkout (or let mise's shell/dir activation do it) before using `cargo` — without it, tools like `protoc` or `sqlx-cli` won't be on `PATH`.

Build tooling uses `mise` tasks (defined in `mise.toml`; run `mise tasks` to list them) for a few tasks, but most day-to-day work is plain `cargo` run against the workspace or a specific package.

```sh
# Build / check
cargo build                                   # whole workspace
cargo build --package bin_sync_server --bin sync-server   # just the Sync Server binary

# Run the Sync Server (reads config, initialises tracing, migrates its DB, serves gRPC + auth HTTP)
cargo run --package bin_sync_server

# Run the TUI or the Desktop GUI, rebuilding and rerunning on file changes
mise run watch-tui
mise run watch-desktop

# Test
cargo test                                    # whole workspace
cargo test --package lib_config                # single crate
cargo test --package lib_config parse_with_explicit_config_file  # single test

# Lint / format
mise run lint           # exactly what the Lint workflow runs: fmt --check + clippy -D warnings
mise run lint-fix       # cargo fmt + clippy's machine-applicable fixes
mise run deny           # cargo-deny: advisories, licences, bans, sources (deny.toml)
mise run install-hooks  # once per checkout: .githooks/pre-push runs `mise run lint`

# Docs (mdBook + rustdoc), via mise tasks
mise run docs-build     # docs-rustdoc + docs-mdbook
mise run docs-serve     # serves mdBook on :8001
```

Building `lib_rpc` requires a system `protoc` (protobuf compiler) — provided via `mise.toml`. `tonic_prost_build` regenerates `crates/libs/lib-rpc/src/generated/*.rs` from the `.proto` files on every build; the generated files are checked in but should be treated as build output, not hand-edited, and `#[rustfmt::skip]` on their `mod` declarations in `generated/mod.rs` keeps `cargo fmt` off them so a build never leaves a formatting diff.

## Workspace layout

Binary crates live under `crates/bins/`, library crates under `crates/libs/`. Bin crates should be prefixed with `bin-` and library crates should be prefixed with `lib-`. Workspace members are listed one per line in the root `Cargo.toml`, not globbed, so a new crate has to be added there by hand — the exception is `lib-core`, which is not in the list and joins the workspace implicitly as a path dependency of the members that use it.

**`crates/libs/lib-database` is an active workspace member but currently fails `cargo build`/`cargo check` at the workspace root** without a live `DATABASE_URL` (or `SQLX_OFFLINE=true` against its checked-in `.sqlx` cache) for `sqlx`'s compile-time query macros — see the `db-prepare-generate`/`db-prepare-check` tasks in the root `mise.toml` for the expected `DATABASE_URL`. For local dev (including rust-analyzer/IDE checks, which shell out to `cargo check` and can't be handed `SQLX_OFFLINE`), create a gitignored `.env` at the repo root with `DATABASE_URL=sqlite:<absolute-path-to-repo>/.personal-ledger-dev.db` — `sqlx`'s macros and `sqlx-cli` auto-load `.env` via `dotenvy`, so plain `cargo build`/`check` and the `db-*` mise tasks all pick it up. The dev DB itself must exist first: run `mise run db` (or `db-create` + `db-migrate` if it already exists). Migrations live in two separate sets — `migrations/client/` for the Client Ledger and `migrations/sync-server/` for the Sync Server's own `change_sets`/`sync_users` tables — applied by the `db-migrate-client`/`db-migrate-sync-server` tasks and both together by `db-migrate`. They are kept to one `create_<table>_table.sql` file per table while the schema is still in concept (edit the table's file rather than adding an `ALTER`/rebuild migration, then rebuild the dev DB with `mise run db`); the Sync Server's auth table is named `sync_users` so it never collides with the Client-Ledger's domain `accounts` table.

- **`crates/bins/bin-sync-server`** — the Sync Server binary (package `bin_sync_server`, compiled binary `sync-server`, matching the `bin-tui`/`bin-desktop` naming convention). `main.rs` parses config via `lib_config::Config::parse_for_sync_server`, initialises `lib_tracing` (imported as `telemetry`), opens its own database and runs the `sync-server` migration set, bootstraps the single `sync_users` account, then serves `lib_rpc`'s `UtilitiesService` (open `Ping` liveness check) and `SyncService` (`Push`/`Pull`, behind `auth::interceptor::AuthInterceptor`) merged with the `auth` HTTP routes into one axum router on a single listener — ADR-0010's "one listener, not two". The JWT signing key is generated fresh in memory each start, and the bootstrap credentials are fixed feasibility-cycle placeholders, not real credential provisioning. See the [Sync Server feasibility](https://github.com/IanTeda/Personal-Ledger/issues/40) Wayfinder map.
- **`crates/bins/bin-tui`** — package name `bin_tui`, but the compiled binary is `tui` (see its `[[bin]]` override) — end users shouldn't see the `bin_` prefix, it's a codebase-navigation convention only. Ratatui Client: `shell.rs` plus a screen per domain in `view/` (accounts, transactions, categories, payees, tags, units, budgets, balance checks, reports, dashboard, help), with `popup/` for dialogs and per-domain `account/`, `category/`, `payee/`, `tag/` modules. The older pre-ADR-0013 `app.rs`/`screen/` stack is deleted (#254): `Shell`/`view/` is the only screen architecture, and the domains whose real CRUD and report logic only ever lived in `screen/` (transactions, budgets, balance checks, reports, CSV import) are wireframe `view/` placeholders until each gets its own redesign pass — recover the old code from Git history, not from a dead module. The workspace still allows `dead_code`, now for the generated `msg` accessors (one per Catalogue entry, unused until a call site reads it) and UI-first code whose wiring is a later phase; `db.rs` is likewise unreferenced but kept as neutral plumbing for the first `View` that needs real data. Packaged non-root for all three OSes (Linux AppImage, macOS dmg, Windows portable zip) via `.github/workflows/build-publish-tui.yaml`; see `crates/bins/bin-tui/README.md`.
- **`crates/bins/bin-desktop`** — package `bin_desktop`, compiled binary `desktop`. The `gpui`/`gpui-component` Client: `shell.rs`, `topbar.rs`, `statusline.rs` and `rail/` make up the chrome (ADR-0016), with `view/` holding the screens (accounts, transactions, settings, dashboard, help) and `palette.rs`/`command.rs`/`key_router.rs` the keyboard grammar (see `docs/navigation-design.md`).
- **`crates/libs/lib-config`** — layered configuration loader, re-exported as `lib_config::Config` (the type is `ledger::LedgerConfig`). INI format via the `config` crate. Clients call `Config::parse`, whose precedence runs lowest to highest: built-in defaults → system config (`/etc/personal-ledger/...`) → user config (XDG/platform config dir) → executable-directory config → CWD `./config/personal-ledger.conf` → explicit path passed to `parse()` → environment variables (`PERSONAL_LEDGER_*`, double-underscore nesting, e.g. `PERSONAL_LEDGER_PERSONAL_LEDGER__LOCALE`). `bin-sync-server` calls `Config::parse_for_sync_server` instead, which keeps only defaults → explicit path → environment (ADR-0014) because the other tiers mean nothing inside a container. Sections are `[Personal-Ledger]` (data dir, database file, log level/file, `locale` — this superseded the old `[telemetry]`/`[tracing]` section), `[sync-server]` (bind address, database URI; Clients don't read it) and `[keybindings]`. Section headers are lower-cased before parsing, so `[Personal-Ledger]`/`[personal-ledger]` are equivalent. See `docs/settings.md`.
- **`crates/libs/lib-tracing`** — package `lib_tracing`, usually imported as `telemetry`. `tracing`-based setup: `init(level, log_file_path)` returns a guard that must be held for the life of `main` (dropping it stops the background log-file writer), plus the `Levels` type consumed by `lib_config`'s `[Personal-Ledger]` section.
- **`crates/libs/lib-rpc`** — proto definitions (`proto/personal-ledger/v001/{utilities,sync}.proto`) and tonic-generated gRPC client/server code (`src/generated/`), re-exported through the `utilities/` and `sync/` modules as a flat API. Proto package versioning is `personal_ledger.<service>.v001`.
- **`crates/libs/lib-core`** — pure business/domain types with no I/O: `RowID` (UUIDv7-based), `CategoryTypes` (accounting categories: assets/liabilities/income/expenses/equity), `UrlSlug`, `HexColor`, `DateStyle`. Designed for SQLite-backed persistence specifically (no Postgres assumptions in the domain types), despite `sqlx`'s Postgres feature being enabled at the workspace level.
- **`crates/libs/lib-database`** — SQLx-based persistence layer. `DatabaseConnection::new(uri)` opens the pool (`.into_pool()` hands out the `SqlitePool`); each entity gets a directory splitting CRUD into separate `find.rs`/`insert.rs`/`update.rs`/`delete.rs`/`builder.rs`/`model.rs` files — follow this split (rather than one big repository file) when adding new persisted entities. Current entities: `accounts`, `transactions`, `categories`, `payees`, `payee_aliases`, `units`, `budgets`, `balance_checks`, `preferences`, plus the Sync-Server-only `change_sets` and `sync_users`.
- **`crates/libs/lib-mcp`** and **`crates/libs/lib-omarchy`** — empty `cargo new` scaffolds (still the placeholder `add` function). They are workspace members but hold no real code yet; don't treat them as implemented.

**`crates/libs/lib-locale`** (package `lib_locale`) is the shared localisation crate for `bin-desktop` and `bin-tui`, and it is built and wired into both bins. It owns the `Locale` set and negotiation, the embedded Fluent Catalogues, the set-once `init` loader, the `with_locale` test override and the generated `msg::` accessors ([#231](https://github.com/IanTeda/Personal-Ledger/issues/231)); `format::` (ICU4X number, date, currency and `upper` casing for the Locale in effect, [#232](https://github.com/IanTeda/Personal-Ledger/issues/232)); typed date input (`parse_date`, [#233](https://github.com/IanTeda/Personal-Ledger/issues/233)); the `Label` trait for domain-value Messages ([#234](https://github.com/IanTeda/Personal-Ledger/issues/234)); and rich Messages (key-token sentinels and `<tag>` spans returning `Segment`s, [#235](https://github.com/IanTeda/Personal-Ledger/issues/235)). **`crates/libs/lib-locale-build`** is its `fluent-syntax`-only generator, used from `lib-locale`'s `build.rs` and each bin's. Design in `docs/localisation-design.md` (user guide `docs/localisation.md`), on the [Localisation of the Desktop and TUI UX](https://github.com/IanTeda/Personal-Ledger/issues/211) Wayfinder map, and [ADR-0021](docs/adr/0021-locale-owns-formatting-and-replaces-number-and-date-preferences.md). The Locale is Configuration (`locale` in `lib-config`'s `[Personal-Ledger]` section, resolved default → system → config), not a Preference. **New UI text is a Message:** add it to the `en-US` Catalogue under `crates/libs/lib-locale/i18n/` (shared) or the bin's own `i18n/` and read it through a generated `msg::` accessor — don't write a hardcoded display literal. What is deliberately *not* a Message (command names and ids, element ids, paths, config keys, tracing and `expect` text, `sync-server` output, CLI `--help`) is listed in the design doc.

Planned-but-not-yet-present crates/binaries mentioned in `docs/directories-files.md` and `README.md` (notably the web/Leptos frontend) don't exist yet — don't assume they're there. `bin-desktop` does now exist, and `lib-mcp`/`lib-omarchy` exist only as empty scaffolds.

## Conventions

**Rust code style and safety:** See `/rust-style` skill for error handling (error.rs shapes, promoting variants, #[from] for external errors), database persistence (CRUD file split), secrets (secrecy::SecretString), dependencies, comments (Australian English, WHY not WHAT), and validated invariants in expect messages. Workspace lints in `Cargo.toml` [workspace.lints] enforce `unsafe_code = "forbid"`, `rust_2018_idioms` and `unused_qualifications` (Rust), `clippy::{unwrap_used, expect_used, panic, todo, unimplemented, dbg_macro} = "deny"` (allowed in tests via clippy.toml) and `clippy::{allow_attributes_without_reason, unwrap_in_result}` (clippy). Every crate opts in with `[lints] workspace = true`; tonic's generated code in `lib-rpc` is exempted at its `mod` declarations. CI runs `cargo clippy --workspace --all-targets -- -D warnings` and `cargo fmt --check`.

**Resolve lints before pushing:** the Lint workflow (`.github/workflows/lint.yaml`) fails on any rustfmt diff or clippy warning, but it runs only for `main` (pushes and PRs), so on `concept` a lint break otherwise surfaces late, at the PR into `main`. Run `mise run lint` (exactly what it runs: `cargo fmt --check` then `SQLX_OFFLINE=true cargo clippy --workspace --all-targets -- -D warnings`) and get it clean before every `git push`. The `.githooks/pre-push` hook enforces this once `mise run install-hooks` has been run. `SQLX_OFFLINE=true` matters: CI has no `DATABASE_URL`, so it checks `lib-database`'s query macros against the checked-in `.sqlx` cache, while a local build silently uses the live dev DB from `.env` and hides a stale cache. If clippy reports "no cached data for this query", regenerate the cache with `mise run db-prepare-generate` and commit the `.sqlx` changes. Fix lints at the source (`mise run lint-fix` for the mechanical ones, then by hand); where a lint is deliberately left, silence it locally with `#[expect(clippy::<lint>, reason = "...")]`, never by relaxing the workspace config or dropping `-D warnings`.

- Commit style: `<area>: <short description>` (e.g. `email-verification: add updated_at to model and migration`).
- Tests: unit tests live alongside the code (`#[cfg(test)] mod tests`); integration/DB tests use `sqlx::test`; use the `fake` crate with deterministic seeds for generated test data. No doctests: usage is specified by unit tests, doc comments explain why rather than carrying `# Examples`, and every library crate sets `[lib] doctest = false`.

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

## Model selection strategy

Use higher-capacity models (Opus/Sonnet) for ambiguous or architectural work that requires broad context and deep reasoning. Use lower-capacity models (Haiku) for well-scoped, actionable work with clear direction. This optimises for both quality and token efficiency.

**Use Opus/Sonnet for:**
- Planning and architecture (Wayfinder maps, design decisions, multi-ticket coordination)
- Writing or reviewing documentation (design docs, ADRs, user guides)
- Open-ended exploration or code review
- Situations where unclear requirements or unexpected findings need synthesis

**Use Haiku for:**
- Tickets with explicit acceptance criteria and defined scope
- Implementation against a fixed design/spec
- Routine refactors, test additions, or maintenance
- Adding a feature to an established pattern (e.g. a new CRUD entity following existing structure)

When starting a ticket:
- If the ticket body is clear, explicit, and self-contained → Haiku
- If the ticket requires understanding prior changes, weighing tradeoffs, or designing an approach → Opus/Sonnet

If Haiku encounters ambiguity, unexpected findings, or a decision point, it should pause and escalate to the user rather than guess.

## Agent skills

### Issue tracker

Issues live as GitHub issues on `IanTeda/Personal-Ledger`; use the `gh` CLI. See `docs/agents/issue-tracker.md`.

### Triage labels

Default five canonical labels (`needs-triage`, `needs-info`, `ready-for-agent`, `ready-for-human`, `wontfix`), used 1:1. See `docs/agents/triage-labels.md`.

### Domain docs

Single-context: `CONTEXT.md` + `docs/adr/` at the repo root. See `docs/agents/domain.md`.

### Markdown formatting

Headings get a blank line before and after; paragraphs and list items are written as a single unwrapped line relying on word wrap, not hard-wrapped. See `docs/agents/markdown-style.md`.

### End-user documentation

End-user domain pages (`docs/<domain>.md`) follow a fixed template with a matching `docs/development/<domain>.md`. Invoke `/end-user-docs` (or let it auto-trigger) when writing or reviewing them.

### Rust style, tracing and tests

Project skills under `.claude/skills/` capture this repo's conventions in more depth than fits here — invoke them (or let them auto-trigger) when doing the matching work: `/rust-style` for error handling, persistence patterns, secrets and comments (hand-checked rules beyond lints), `/tracing` for `tracing::instrument`/log-level conventions, `/unit-tests` for `fake`-crate mock data and `sqlx::test` patterns.
