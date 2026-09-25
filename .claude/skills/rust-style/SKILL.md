---
name: rust-style
description: Write or review Rust code in this repo, enforcing the conventions not covered by automated lints. Use when implementing code, reviewing for style, or writing a function that falls under the patterns below.
---

# Rust Style

This codebase follows Rust conventions and patterns enforced by clippy lints (see `Cargo.toml` [workspace.lints.clippy]). This skill covers the hand-checked rules that lints cannot catch.

## Error handling

Every crate has an `error.rs` module exporting `Error` and `Result<T>`. Use it for all fallible functions (not `Option`, `Box<dyn Error>`, or `panic!`).

**Error enum shape:** Start with a single `Generic(String)` catch-all variant:

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
    #[from]
    Sqlx(#[from] sqlx::Error),
}
```

**External crate errors:** Map them with `#[from]` so `?` converts them automatically. Never flatten into `Generic` via `to_string()`.

**Invariants in expect:** Any remaining `expect` outside tests must state the invariant in the message:

```rust
date.format("%Y-%m-%d")
    .expect("day 1, 1 Jan — ymd_opt cannot fail")
```

## Database persistence

Split CRUD by operation: each entity gets `find.rs`, `insert.rs`, `update.rs`, `delete.rs`, `builder.rs` (for complex construction), and `model.rs` (the domain type). No `SELECT *` — list columns explicitly.

Example structure:
```
crates/libs/lib-database/src/accounts/
  ├── find.rs       (find_by_id, find_all)
  ├── insert.rs     (insert)
  ├── update.rs     (update)
  ├── delete.rs     (delete)
  ├── builder.rs    (AccountBuilder)
  └── model.rs      (Account type)
```

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

## Patterns

**Builder pattern:** For complex construction (especially with optional fields). Use `builder.rs` for the builder struct.

**Mock helpers in tests:** Gate behind `#[cfg(test)]` or document invariants in the `expect` message. For test fixtures of literal dates, a small helper (returning `Result` or built via `const`) is preferred over dozens of repeated `expect("valid date")`.

**Integration test files (`tests/*.rs`):** Open with a file-level `#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, reason = "integration test crate: a failed setup should fail the test")]` after the `//!` doc comment. `allow-*-in-tests` doesn't reach helper functions there, and the file is test-only by construction. This is the only sanctioned blanket allow; everywhere else a proven invariant gets a targeted `#[expect(clippy::expect_used, reason = "...")]`. See `/unit-tests`.

**Workspace lints:** All members opt in with `[lints] workspace = true`. Workspace lints are in `[workspace.lints.rust]` and `[workspace.lints.clippy]`; individual crate overrides are rare and must be documented.

## Checklist before finishing

- [ ] No `unwrap()`, `expect()`, or `panic!()` in non-test code (except documented invariant-checking `expect` with reason in the message).
- [ ] All fallible functions return the crate's `Result` (or a compatible error type).
- [ ] Secrets are wrapped in `secrecy::SecretString` or `SecretBox`.
- [ ] Comments explain the WHY, not the WHAT.
- [ ] Australian English in comments and docs.
- [ ] No `unsafe` code.
- [ ] External crate errors map with `#[from]`.

## Further reading

- Error handling patterns: `crates/libs/lib-database/src/error.rs` (reference implementation).
- Tracing conventions: `/tracing` skill (instrumentation, log levels).
- Tests and mocking: `/unit-tests` skill (fixtures, fakes, sqlx::test).
