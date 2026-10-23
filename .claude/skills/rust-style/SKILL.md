---
name: rust-style
description: Write or review Rust code in this repo, enforcing the conventions not covered by automated lints. Use when implementing code, reviewing for style, or writing a function that falls under the patterns below.
---

# Rust Style

This codebase follows Rust conventions and patterns enforced by clippy lints (see `Cargo.toml` [workspace.lints.clippy]). This skill covers the hand-checked rules that lints cannot catch.

## Error handling

Every crate has an `error.rs` module exporting `Error` and `Result<T>`. Use it for every fallible function rather than `Box<dyn Error>` or `panic!`. A lookup that may find nothing returns `Option`, since absence isn't a failure. Keep `mod error` private and re-export `Error` and `Result` from `lib.rs`.

**Infrastructure crates** (`lib-config`, `lib-database`, `lib-rpc`, …): start the enum with a single `Generic(String)` catch-all variant:

```rust
#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("{0}")]
    Generic(String),
    // ... specific variants added later
}
```

**Promoting a variant:** When the same kind of failure appears three or more times, promote it to its own specific variant (or a dedicated error type for complex cases). Example:

```rust
#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("{0}")]
    Generic(String),
    #[error("date parsing failed: {0}")]
    DateParse(String),
    #[error(transparent)]
    Sqlx(#[from] sqlx::Error),
}
```

**External crate errors:** Map them with `#[from]` so `?` converts them automatically. Never flatten into `Generic` via `to_string()`.

**Feature crates** (the VIPER domain crates: `lib-units`, `lib-bills`, …): `Error` is the crate's Refusals (`GLOSSARY.md`). It is one flat enum with a variant per rule, named for what the rule forbids, and each variant carries the data the Presenter needs for its Message (counts, ids). It never holds a pre-formatted string, which would be English the person can't have translated and would hide the counts that plural Messages need. The `#[error]` text is for logs and `Display`; the person sees the Presenter's `msg::` Message. Every fallible Interactor `run` returns `Result<T>`. A Presenter maps each variant it can receive to a Message, and its `_ =>` arm shows a generic Message with a `debug_assert!`, never `panic!` or `unreachable!`.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    #[error("unit not found")]
    UnitGone,
    #[error("a unit in use can only be deactivated")]
    UnitInUse(UnitUse),
    #[error("{0} other units are quoted in this unit")]
    QuotedBy(usize),
}
```

Feature crates never depend on `sqlx` or `lib-database`. Ports are infallible (ADR-0046): a failed write reaches the person as an error Toast through the writer channel, not through the Interactor. If a port ever has to fail, it returns a type the feature crate owns, and the `lib-database` adapter converts into it.

**Invariants in expect:** Any remaining `expect` outside tests must state the invariant in the message:

```rust
date.format("%Y-%m-%d")
    .expect("day 1, 1 Jan — ymd_opt cannot fail")
```

## Database persistence

A migrated domain's adapter is one file, `crates/libs/lib-database/src/adapter/<domain>.rs` (ADR-0041): it implements the module's ports, holds the domain's rows in memory and sends writes to the one shared writer, `writer.rs` (ADR-0046). `adapter/units.rs` is the model. Don't add per-entity `find.rs`/`insert.rs`/`update.rs`/`delete.rs` directories; the remaining ones predate the migration. No `SELECT *` — list columns explicitly.

How commits, reloads and write failures work is in the Persistence section of `docs/development/viper-architecture.md`; follow it there rather than restating it here.

## Secrets and sensitive data

Wrap secrets in `secrecy::SecretString` (or `SecretBox<T>` for other types). Never let them escape into logs, panics, or display traits.

```rust
use secrecy::SecretString;

let token: SecretString = api_response.token.into();
// Dereferencing requires explicit .expose_secret()
```

## Dependencies

**Workspace vs. local:** A dependency used by two or more crates goes in `[workspace.dependencies]`; single-use dependencies stay local. When a second crate needs a local dependency, move it to the workspace list.

## Comments and docs

**Australian English:** "colour", "favour", "organisation", not US spelling.

**What to comment:** Only when the WHY is non-obvious (hidden constraints, surprising invariants, workarounds for specific bugs). If the code's intent is clear from names, skip it.

**Never comment the WHAT:** Well-named identifiers already say what the code does. Don't repeat it.

**No doctests:** Don't put Rust code examples (`# Examples` sections, ` ```rust ` or untagged fences) in `///` or `//!` comments. Usage is specified by unit tests in the module's `#[cfg(test)] mod tests`, and uncompiled examples go stale silently. Every library crate sets `[lib] doctest = false`, so a stray example wouldn't even run. Non-Rust fences (` ```ini `, ` ```bash `) that illustrate config or shell usage are fine.

## Patterns

**Builder pattern:** For complex construction (especially with optional fields).

**Mock helpers in tests:** Gate behind `#[cfg(test)]` or document invariants in the `expect` message. For test fixtures of literal dates, a small helper (returning `Result` or built via `const`) is preferred over dozens of repeated `expect("valid date")`.

**Integration test files (`tests/*.rs`):** Open with a file-level `#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, reason = "integration test crate: a failed setup should fail the test")]` after the `//!` doc comment. `allow-*-in-tests` doesn't reach helper functions there, and the file is test-only by construction. This is the only sanctioned blanket allow; everywhere else a proven invariant gets a targeted `#[expect(clippy::expect_used, reason = "...")]`. See `/unit-tests`.

**Workspace lints:** All members opt in with `[lints] workspace = true`. Workspace lints are in `[workspace.lints.rust]` and `[workspace.lints.clippy]`; individual crate overrides are rare and must be documented.

## Checklist before finishing

- [ ] No `unwrap()`, `expect()`, or `panic!()` in non-test code (except documented invariant-checking `expect` with reason in the message).
- [ ] Secrets are wrapped in `secrecy::SecretString` or `SecretBox`.
- [ ] Comments explain the WHY, not the WHAT.
- [ ] No Rust code examples in doc comments; usage is covered by unit tests.
- [ ] Australian English in comments and docs.
- [ ] No `unsafe` code.
- [ ] All fallible functions return the crate's `Result` (or a compatible error type); a lookup that may find nothing returns `Option`.
- [ ] Infrastructure crates map external errors with `#[from]`; feature crates hold typed Refusal variants, never strings, and don't depend on `sqlx`.

## Further reading

- Error handling patterns: `crates/libs/lib-database/src/error.rs` (infrastructure) and `crates/libs/lib-units/src/error.rs` (feature crate, once #818 lands).
- Tracing conventions: `/tracing` skill (instrumentation, log levels).
- Tests and mocking: `/unit-tests` skill (fixtures, fakes, sqlx::test).
