# Ledger ids are lib-core newtypes over the UUIDv7 RowID

The domain crates use `u32` ids. `lib-database`, the schema and ADR-0009's Change Sets use the UUIDv7 `RowID` in `lib-core`. ADR-0038 moved the cross-domain ids into `lib-core` newtypes and left their type to #597. Decided in #634 on the [Deepen the domain libs and finish the View Entities](https://github.com/IanTeda/Personal-Ledger/issues/588) map.

## Decision

- Every Ledger id is a `lib-core` newtype over `RowID` (`PayeeId(RowID)`, `CategoryId(RowID)`, …), replacing `u32`.
- The stub seeds use fixed UUIDs, so the headless UI tests stay deterministic.
- `lib-core`'s sqlx impls for `RowID` sit behind a `sqlx` feature that only `lib-database` enables, so the domain crates don't pull in sqlx.
- The switch is its own ticket ([Switch Ledger ids to lib-core newtypes over RowID](https://github.com/IanTeda/Personal-Ledger/issues/635)), after `lib-ledger` exists and separate from the Ledger migration.

## Consequences

- Offline Clients create rows without colliding on sync. A per-Client `u32` sequence would collide on the first sync.
- This confirms ADR-0009 and extends it to Client rows.
- Ids are larger in memory and in test output, and a test can no longer write `PayeeId(3)`.

## Considered Options

- **Keep `u32` in memory and map to `RowID` at the storage boundary.** Rejected: it doubles the id space and needs a lookup on every write and sync.
