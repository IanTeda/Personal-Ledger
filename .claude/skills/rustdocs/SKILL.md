---
name: rustdocs
description: Write rustdoc comments for this repo's Rust code — structure, sections, and Australian English conventions. Use when adding or updating /// or //! documentation on public items.
---

# Rustdocs

Write clear, accurate rustdoc comments that follow Rust community conventions and this project's standards.

## Conventions

- Australian/British spelling ("optimise", "behaviour", "colour") per [[CLAUDE.md]] Conventions.
- Start with a one-line summary, then elaborate if the item's purpose or behaviour isn't obvious from its name.
- Document parameters, return values, error conditions, panics, and safety requirements — but only the sections that apply; don't pad with empty ones.
- Include a runnable example (` ``` `) when it clarifies usage; use `rust,no_run` for examples needing external state (a DB pool, network) and `rust,ignore` for conceptual-only snippets.

## Function documentation

```rust
/// Brief description of what the function does.
///
/// More detail if the summary alone doesn't cover it.
///
/// # Arguments
/// * `param1` - What it means, valid ranges/formats
///
/// # Errors
/// When and why this returns an error
///
/// # Panics
/// Conditions that panic (omit section if none)
///
/// # Examples
/// ```
/// let result = my_function(42, "hello");
/// assert_eq!(result, expected_value);
/// ```
pub fn my_function(param1: Type1, param2: Type2) -> ReturnType { }
```

## Structs, enums, traits

- Struct docs: one-line summary, then what the fields collectively represent — don't restate each field's type, just its meaning where non-obvious.
- Enum variant docs: a short `///` line per variant explaining when it applies.
- Trait docs: describe the contract implementers must satisfy, not just what the trait "is".
- Module docs (`//!`) at the top of `mod.rs`/`lib.rs`: overview, then any cross-cutting concerns (security, invariants) implementers/callers need to know.

## Project-specific notes

- Error enums: document each `thiserror` variant and when it's produced (see `lib-database/src/error.rs`, `lib-config/src/error.rs` for the pattern).
- Database-backed functions: note the SQL operation's intent and any constraints/indexes that affect behaviour, not the raw query text (that's already in the code).
- Examples should use `?`, never `unwrap()`/`expect()`.

## Before finishing

- Every public item touched has a doc comment.
- Any doctest examples actually compile (`cargo test --doc` where practical).
- Error and panic conditions are documented where they exist.
