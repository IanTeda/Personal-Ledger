---
name: unit-tests
description: Write unit tests for this repo using the fake crate for deterministic mock data, mock() constructors, and sqlx::test for DB-backed tests. Use when adding tests for a new or changed type/function.
---

# Unit tests

Write tests using this project's `fake`-crate mock-data conventions ([[CLAUDE.md]] Conventions: "unit tests live alongside the code... use the `fake` crate with deterministic seeds").

## Mock constructors

Give types a `#[cfg(test)] pub fn mock() -> Self` that builds realistic data via `fake`, rather than hand-writing fixture literals in every test:

```rust
impl Category {
    #[cfg(test)]
    pub fn mock() -> Self {
        Self {
            id: lib_domain::RowID::mock(),
            name: Self::generate_mock_name(),
            ..
        }
    }

    #[cfg(test)]
    fn generate_mock_name() -> String {
        use fake::Fake;
        use fake::faker::name::en::Name;
        Name().fake()
    }
}
```

- Optional fields: randomise `Some`/`None` (e.g. `Boolean(60).fake()` for 60% `Some`) so both paths get exercised across the suite.
- Enum fields: bias the distribution toward realistic proportions rather than uniform random, when that matters to the test.

## Determinism

Prefer to keep faked values incidental to what's under test (assert on shape/invariants, not exact faked strings). When a test genuinely needs reproducibility, seed explicitly:

```rust
use fake::rand::{SeedableRng, rngs::StdRng};
let mut rng = StdRng::from_seed([42u8; 32]);
let value: String = fake::faker::lorem::en::Word().fake_with_rng(&mut rng);
```

## Database tests

Use `#[sqlx::test]` to get a stubbed test database per test:

```rust
#[sqlx::test]
async fn create_category_with_mock_data(pool: sqlx::SqlitePool) {
    let category = Category::mock();
    let result = category.create(&pool).await;
    assert!(result.is_ok());
}
```

## Structure

- Unit tests live in `#[cfg(test)] mod tests` alongside the code they test, not in a separate `tests/` file, unless it's a true cross-crate integration test.
- Group related tests into nested modules (`mod validation`, `mod edge_cases`) when a file's test count grows large enough that flat organisation stops helping.
- Cover: happy path, edge cases (empty/max-length/unicode/whitespace input), and error conditions — not just the happy path.

## Before finishing

- New/changed public behaviour has a test.
- Mock data goes through `fake`/`mock()`, not hand-typed fixtures, where a mock constructor already exists or is worth adding.
- DB-backed tests use `#[sqlx::test]`.
