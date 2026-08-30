---
name: tracing
description: Add or review tracing::instrument spans and log statements in this repo. Use when instrumenting a new function or deciding what level/fields a trace event should have.
---

# Tracing

Add consistent, useful `tracing` telemetry using the levels and patterns this codebase follows (`lib_telemetry`'s `TelemetryLevels`, consumed by `server`'s `telemetry::init`).

## Levels, highest to lowest priority

- **ERROR** — requires immediate attention: DB connection failures, external service unavailability, data corruption, security violations, resource exhaustion.
- **WARN** — potentially harmful but recovered: rate-limit triggers, repeated failures, deprecated API usage, config issues, degraded performance.
- **INFO** — normal but noteworthy: successful create/update/delete, auth events, state changes, startup/shutdown.
- **DEBUG** (default for `#[instrument]`) — troubleshooting detail: function entry/exit, query execution, cache hits/misses, timings.
- **TRACE** — reserve for high-frequency or expensive-to-compute detail.

## Instrumenting a function

```rust
#[tracing::instrument(
    name = "Descriptive operation name",
    level = "debug",           // "info" for entry points/handlers, "trace" for hot/internal detail
    skip(pool),                 // skip large or sensitive parameters
    fields(
        category_id = %id,      // %  = Display
        query_params = ?params, // ?  = Debug
    ),
    err                          // record error results in the span
)]
pub async fn find_category_by_id(
    pool: &sqlx::Pool<sqlx::Sqlite>,
    id: i64,
) -> Result<Option<Category>, DatabaseError> {
    // ...
}
```

- `skip(...)` anything large or sensitive (passwords, tokens, PII, full request bodies) — never let it reach a span field.
- Prefer `%` for user-facing values, `?` for structured/debug detail.
- Emit `tracing::debug!`/`info!`/`warn!`/`error!` inside the body for interesting sub-steps, not just the span itself.

## What to instrument

Functions doing database operations, business logic, external calls, or user input handling — not trivial getters/pure helpers.

## Avoid

- DEBUG/TRACE logging inside tight loops or hot paths.
- Logging secrets, tokens, or PII in any field, at any level.
- Bloating spans with fields that don't aid debugging.

## Before finishing

- Instrumented functions use `err` so failures show up in the span.
- No sensitive data appears in any field or log message.
- Level matches the guidance above (don't default everything to `info`).
