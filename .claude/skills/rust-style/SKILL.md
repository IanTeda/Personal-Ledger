---
name: rust-style
description: Write or review Rust code in this repo, enforcing the conventions not covered by automated lints. Use when implementing code, reviewing for style, or writing a function that falls under the patterns below.
---

# Rust Style

This codebase follows Rust conventions and patterns enforced by clippy lints (see `Cargo.toml` [workspace.lints.clippy]). This skill covers the hand-checked rules that lints cannot catch.

## Error handling

Every crate, library or bin, has a private `error.rs` module exporting `Error` and `pub type Result<T, E = Error> = std::result::Result<T, E>;`, re-exported from a library's `lib.rs` as `pub use error::{Error, Result};`. This has no exemption: a crate with nothing fallible yet (`lib-list-cursor`, `lib-toast`, `lib-viper`) still has one, so a developer never has to decide whether it is needed. Every fallible function, public or private, returns the crate's `Result` (ADR-0060), never `std::result::Result`, `io::Result`, `Box<dyn Error>` or `panic!`. A lookup that may find nothing returns `Option`, since absence isn't a failure.

A crate never returns another crate's `Result` or `Error`, and never imports another crate's `Result`. Where a workflow carries a library error up with `?`, wrap it in a `#[from]` variant of your own `Error`, as `lib-bill-payments` wraps `lib_bills::Error`. Importing another crate's `Error` to match on it, map it or implement its port is fine.

No function returns `Result<_, String>`: a `String` error is English the person can't have translated and text a caller can't match on. Make the failure a typed variant instead.

**Value errors** are the one exception (ADR-0060): an enum declared beside one value type, naming only the ways a text or number fails to become that value, reading no port and doing no I/O. Its parse or validate function returns `Result<T, ItsValueError>` through the crate's alias, so a caller matches exactly those failures. Any crate may name another crate's value error in a signature. `Error` wraps a value error with `#[from]` only where one of the crate's workflows carries it up with `?`. A refusal that reads ports or the disk is not a value error and belongs in `Error`. Every value error is named on the `VALUE_ERRORS` list in `crates/libs/lib-database/tests/error_convention.rs`; adding one is a reviewed edit.

**Signatures fixed from outside** are exempt, and the convention test recognises them by syntax with no marker: a method inside `impl Trait for Type` (`FromStr`, `TryFrom`, serde, sqlx, tonic), an error that is a generic parameter's associated type (`D::Error`), `fmt::Result`, and an error that is the function's or impl's own generic parameter.

**Infrastructure crates** (`lib-config`, `lib-database`, `lib-locale`, `lib-locale-build`, `lib-rpc`, `lib-tracing`): start the enum with a single `Generic(String)` catch-all variant. `Generic` is for infrastructure crates only: a domain or feature crate has no `Generic` and is typed only, like `lib-core`, so a domain failure can't hide in a string:

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

**External crate errors:** Map them with `#[from]` so `?` converts them automatically. Never flatten into `Generic` via `to_string()` or `format!`: `Generic` is for the crate's own failures, never another error's text.

**Matching on an `Error`** (ADR-0060): a `match` on the crate's own `Error`, or on a feature crate's, that picks a Message, an outcome or a mapped error per variant names every variant, with no `_` or catch-all binding, so a new variant fails to compile until someone places it. The variants the workflow can't receive share one named dead arm, which fires only on a bug: it fails a `debug_assert!` ending in `{error:?}`, logs `tracing::error!(error = ?error, "<what> was refused with an error it never returns")` (ADR-0047 lets it log in any layer), and falls back to a Message, never `panic!` or `unreachable!`. A match that picks out one variant is `if let` or `matches!`. An infrastructure crate's `Error` (the list above) or a foreign crate's (`sqlx`, `io`) is open-ended and may keep a wildcard.

**Bin crates** (`bin-desktop`, `bin-tui`, `bin-sync-server`): `Error` is typed only, with no `Generic`. It holds the bin's fatal failures (start-up and shutdown) and the typed failures its own functions return, wrapping a library error with `#[from]` only where one of its functions carries it up. Before the UI runs and after it quits, a library error goes up with `?` to `main`, which returns `Result<()>`. While the UI runs, handle it where it's received: log its text with `tracing` and show a Message (an Error Toast, or the status line for a Refusal) chosen by matching on its variant. An error's `Display` text never goes into a Message. The Sync Server maps its `Error` to `tonic::Status` and `IntoResponse` at the boundary.

**Feature crates** (the VIPER domain crates: `lib-units`, `lib-bills`, …): `Error` is the crate's Refusals (`GLOSSARY.md`). Workflow refusals are flat variants of this one `Error`, never a per-function or per-entity error type; an old one is removed when its variants move in. It is one flat enum with a variant per rule, named for what the rule forbids, and each variant carries the data the Presenter needs for its Message (counts, ids). It never holds a pre-formatted string, which would be English the person can't have translated and would hide the counts that plural Messages need. The `#[error]` text is for logs and `Display`; the person sees the Presenter's `msg::` Message. Interactors log that text as the refusal, so `/tracing`'s rule holds for it: ids, counts, dates and codes, never a name, an amount or anything the person typed; the data a Message needs stays in the variant's fields. Every fallible Interactor `run` returns `Result<T>`. A Presenter maps each variant it can receive to a Message, and the rest share the dead arm in **Matching on an `Error`** above.

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

Feature crates never depend on `sqlx` or `lib-database`. Ports are infallible (ADR-0046): a failed write reaches the person as an error Toast through the writer channel, not through the Interactor. If a port ever has to fail, as `LedgerCreation` does, it returns the feature crate's `Result<T>`, and the `lib-database` adapter converts its own errors into the feature crate's `Error`.

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
- [ ] Every fallible function, private ones included, returns the crate's `Result<T>`, or `Result<T, ItsValueError>` for a value error on `VALUE_ERRORS` (ADR-0060); a lookup that may find nothing returns `Option`.
- [ ] No function returns another crate's `Result` or `Error`, and no `use` imports another crate's `Result`; a carried library error is a `#[from]` variant.
- [ ] Every crate, bins included, has a private `error.rs` exporting `Error` and `Result<T, E = Error>`, re-exported from a library's `lib.rs`, including crates with nothing fallible.
- [ ] No function returns `Result<_, String>`.
- [ ] Infrastructure libraries map external errors with `#[from]` and may use `Generic(String)` for their own failures only; feature crates and bins are typed only, never `Generic` or strings, and feature crates don't depend on `sqlx`.
- [ ] No error's `Display` text goes into a Message; a bin picks the Message by matching the variant.

## Further reading

- Error handling patterns: `crates/libs/lib-database/src/error.rs` (infrastructure), `crates/libs/lib-units/src/error.rs` (feature crate) and `crates/libs/lib-core/src/error.rs` (domain value types).
- Tracing conventions: `/tracing` skill (instrumentation, log levels).
- Tests and mocking: `/unit-tests` skill (fixtures, fakes, sqlx::test).
