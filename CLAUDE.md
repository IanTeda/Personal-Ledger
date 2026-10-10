# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project

Personal Ledger is a Rust Cargo workspace for a personal finance/accounting application (expenses, investments, assets). It is still in concept: the Desktop and TUI Clients are built UI-first, with most screens rendering in-memory stub data and persistence wiring a later phase, and the Sync Server serves a feasibility-cycle surface (`Push`/`Pull` behind an auth interceptor, plus `/authorize` and `/token`) rather than a finished sync protocol.

## Setup

Toolchain and dev tools (Rust, protoc, mdBook, sqlx-cli, cargo-watch, cargo-deny) are pinned in `mise.toml`. Run `mise install` once per checkout before `cargo`, or `protoc` and `sqlx-cli` won't be on `PATH`. Run `mise run install-hooks` once so `.githooks/pre-push` runs the lint.

`lib-database` uses `sqlx` compile-time query macros, so the workspace won't build without a `DATABASE_URL` or `SQLX_OFFLINE=true`. For local dev (rust-analyzer included), create a gitignored `.env` at the repo root with `DATABASE_URL=sqlite:<absolute-path-to-repo>/.personal-ledger-dev.db`; `sqlx` and `sqlx-cli` auto-load it. Then:

- `mise run db` resets the dev DB (drop, create, migrate) **and regenerates the `.sqlx` cache**, so commit any `.sqlx` changes it leaves.
- `mise run db-migrate` applies new migrations and keeps existing data.

Migrations come in two sets: `migrations/client/` (the Client Ledger) and `migrations/sync-server/` (`change_sets`, `sync_users`). While the schema is in concept there is one `create_<table>_table.sql` per table: edit that file rather than adding an `ALTER` migration, then `mise run db`.

## Commands

Most work is plain `cargo`; `mise tasks` lists the rest.

```sh
cargo build                                    # whole workspace
cargo run --package bin_sync_server            # Sync Server
mise run watch-tui                             # TUI, rebuilt on change
mise run watch-desktop                         # Desktop, rebuilt on change

cargo test --package lib_config                # one crate
cargo test --package lib_config parse_with_explicit_config_file  # one test
mise run end-to-end-ui <view>                  # Desktop UI tests for one View, e.g. settings_inventory

mise run lint           # exactly what CI runs: fmt --check + SQLX_OFFLINE clippy -D warnings
mise run lint-fix       # cargo fmt + clippy's machine-applicable fixes
mise run deny           # cargo-deny (deny.toml)
mise run docs-build     # rustdoc + mdBook; docs-serve serves on :8001
```

Package names and binary names differ: `bin_sync_server` → `sync-server`, `bin_tui` → `tui`, `bin_desktop` → `desktop` (the `bin_` prefix is a codebase convention end users shouldn't see).

## Workspace layout

Bins live in `crates/bins/bin-*`, libraries in `crates/libs/lib-*`. Members are listed by hand in the root `Cargo.toml`, so add a new crate there; the exception is `lib-core`, which joins only as a path dependency.

- **`bin-sync-server`** — serves `UtilitiesService` (open `Ping`) and `SyncService` (behind `auth::interceptor::AuthInterceptor`) merged with the `auth` HTTP routes on one listener (ADR-0010). Config comes via `Config::parse_for_sync_server`. The JWT key and bootstrap `sync_users` credentials are feasibility placeholders. See the [Sync Server feasibility](https://github.com/IanTeda/Personal-Ledger/issues/40) map.
- **`bin-desktop`** — `gpui`/`gpui-component` Client. Lib plus thin bin: modules live in `src/lib.rs`, and the app and the tests both build `Shell` through `build_shell`. `app.rs` defines `Shell`; `app/` holds `impl Shell` blocks per concern (`render.rs`, `focus.rs`, `key_dispatch.rs` are `impl` blocks, not modules of that name; `_ui` marks per-domain wiring). `chrome/` is the Chrome (ADR-0016), `view/` the screens, `navigation/` the keyboard grammar (`docs/navigation-design.md`). Headless UI tests live in `tests/` (ADR-0030).
- **`bin-tui`** — Ratatui Client: `shell.rs`, a screen per domain in `view/`, `popup/` for dialogs. Transactions, budgets, balance checks, reports and CSV import are wireframe placeholders; their old logic is in Git history (pre-#254 `screen/`), not in a dead module. `db.rs` is unreferenced plumbing kept for the first View that needs real data.
- **Domain crates** — `lib-accounts`, `lib-bills`, `lib-budgets`, `lib-categories`, `lib-documents`, `lib-institutions`, `lib-inventory`, `lib-payees`, `lib-tags`, `lib-transactions`, `lib-units`: one pure, `gpui`-free, I/O-free crate per domain (model, rules, stub seed) shared by both Clients. The Desktop wraps each in a store Entity and keeps forms and dialogs in the bin. Follow this split for a new domain.
- **`lib-core`** — shared domain value types with no I/O (`RowID`, `Money`, `CategoryTypes`, periods, …). SQLite-only assumptions, even though `sqlx`'s Postgres feature is on workspace-wide.
- **`lib-database`** — SQLx persistence. Each entity has its own directory split into `find.rs`/`insert.rs`/`update.rs`/`delete.rs`/`builder.rs`/`model.rs`; follow that split for new entities. The Sync Server's auth table is `sync_users` so it never collides with the domain `accounts`.
- **`lib-config`** — layered INI config, `lib_config::Config`. Sections `[Personal-Ledger]`, `[sync-server]`, `[keybindings]`; precedence and env-var naming in `docs/settings.md`.
- **`lib-tracing`** (imported as `telemetry`) — `init` returns a guard that must live for all of `main`. See `docs/tracing.md`.
- **`lib-rpc`** — protos in `proto/personal-ledger/v001/`, packages `personal_ledger.<service>.v001`. `src/generated/*.rs` is regenerated by `tonic_prost_build` on every build: checked in, but build output, never hand-edited.
- **`lib-locale`** / **`lib-locale-build`** — Fluent Catalogues, generated `msg::` accessors, ICU4X formatting and date parsing for both Clients. The Locale is Configuration, not a Preference (ADR-0021). See `docs/localisation-design.md`.
- **`lib-toast`** — pure Toast model shared by both Clients (`docs/toasts-design.md`). **`lib-colour-theme`** — Colour Themes (ADR-0025).
- **`lib-mcp`**, **`lib-omarchy`** — empty `cargo new` scaffolds; not implemented.

Crates in `docs/directories-files.md` or `README.md` that aren't listed here (e.g. the web/Leptos frontend) don't exist yet.

## Conventions

- **New UI text is a Message:** add it to the `en-US` Catalogue (`crates/libs/lib-locale/i18n/` if shared, else the bin's `i18n/`) and read it through a `msg::` accessor; no hardcoded display literals. What is deliberately not a Message is listed in `docs/localisation-design.md`.
- **Lint clean before every push:** run `mise run lint`. CI's Lint workflow runs only on `main`, so a break on `concept` otherwise surfaces late. Fix at the source; where a lint is deliberately left, use `#[expect(clippy::<lint>, reason = "...")]`, never relax `[workspace.lints]` or drop `-D warnings`. The `SQLX_OFFLINE=true` in the lint matters: a local build reads the live dev DB and hides a stale `.sqlx` cache. On "no cached data for this query", run `mise run db-prepare-generate` and commit `.sqlx`. See `/lint-fix`.
- **Desktop Views need UI tests:** every View under `bin-desktop/src/view/` and every Settings page has `<view>_keyboard.rs` and `<view>_mouse.rs` in `bin-desktop/tests/`. When adding or changing Desktop UI, add the missing ones without being asked (`/ui-tests`) and run `mise run end-to-end-ui <view>`. A View without tests is incomplete; report the gap in reviews, and say why in the commit or PR if one is deliberately left untested.
- **Full suites at parent-ticket close only:** the full UI suite (bare `mise run end-to-end-ui`) and the full workspace `cargo test` are required when closing a parent ticket (a Wayfinder map or any issue with sub-issues), not for each child ticket or push. Run the targeted tests for what you changed.
- **Tests:** unit tests alongside the code in `#[cfg(test)] mod tests`; DB tests use `sqlx::test`; generated data uses `fake` with deterministic seeds. No doctests: every library sets `[lib] doctest = false`, and doc comments explain why rather than carrying `# Examples`.
- **Commits:** `<area>: <short description>`, e.g. `desktop: move the Inventory into a store Entity behind lib_inventory's service`.
- **Issue titles:** prefix every GitHub issue title with its bin crate, `TUI: `, `DESKTOP: ` or `SYNC: `, or `CORE: ` for none or several; an issue for a lib crate inherits the prefix of the bin being worked on when it was created (`docs/agents/issue-tracker.md`).
- **Signing key check before committing:** commits are SSH-signed with a key held by the Bitwarden ssh-agent (`SSH_AUTH_SOCK`), which is only reachable while Bitwarden is open and unlocked. Before `git commit`, run `ssh-add -l | grep -qF "$(ssh-keygen -lf "$(git config user.signingkey)" | cut -d' ' -f2)"`. If it fails, don't commit (and never bypass signing): ask the user to open or unlock Bitwarden, then check again.

## Skills and docs

- `/rust-style` — error handling, persistence, secrets, comments (Australian English, why not what), `expect` messages. `/tracing` — spans and log levels. `/unit-tests` — `fake` and `sqlx::test`. `/ui-tests` — Desktop headless tests. `/end-user-docs` — `docs/<domain>.md` and its `docs/development/<domain>.md`.
- Issues are GitHub issues on `IanTeda/Personal-Ledger` via `gh` (`docs/agents/issue-tracker.md`); triage labels in `docs/agents/triage-labels.md`.
- Domain language is in `GLOSSARY.md`, decisions in `docs/adr/` (`docs/agents/domain.md`).
- Markdown: blank line around headings; paragraphs and list items are single unwrapped lines (`docs/agents/markdown-style.md`).

## Tickets and model choice

A self-contained ticket body is the spec: the user usually `/clear`s between independent tickets, so don't rely on earlier conversation. If the body lacks what you need (acceptance criteria, files, decisions), stop and ask.

Use Opus medium for research and decisions: Wayfinder charting and its `research`/`prototype`/`grilling` tickets, domain modelling, ADRs, design, code review, unclear requirements. Use Haiku low for implementation against a fixed scope: explicit acceptance criteria, tests, lint fixes, routine refactors, extending an established pattern. Raise to Opus high for hard bugs and cross-cutting design, or Sonnet medium for `/end-user-docs` pages. If Haiku low hits ambiguity or a decision point, stop and escalate rather than guess. The same split applies when picking a subagent's model.

## graphify

A knowledge graph lives in `graphify-out/`. For codebase questions, start with `graphify query "<question>"`, `graphify path "<A>" "<B>"` or `graphify explain "<concept>"`; they return a scoped subgraph, smaller than `GRAPH_REPORT.md` or raw grep. Read `GRAPH_REPORT.md` only for broad architecture review. After modifying code, run `graphify update .` (AST-only, no API cost).
