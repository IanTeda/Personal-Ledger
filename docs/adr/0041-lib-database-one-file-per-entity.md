# lib-database keeps one file per entity, with executor-taking queries

`lib-database` splits each entity into `model`/`builder`/`find`/`insert`/`update`/`delete` files (`payees/` is nine files and 1,094 lines; `delete.rs` is 73 lines, mostly doc comment). Its queries take `&Pool`, so several tables cannot be written in one transaction, which ADR-0039's `apply` needs. Decided in #634 on the [Deepen the domain libs and finish the View Entities](https://github.com/IanTeda/Personal-Ledger/issues/588) map.

## Decision

- Each entity is one file holding its row struct and queries, split into a module only when it grows past a few hundred lines.
- The builders go: the SQLite adapter maps a domain value to a row directly.
- Query functions take an executor or `&mut SqliteConnection`, not `&Pool`, so they compose inside one transaction.
- This is applied in [Wire Ledger persistence to SQLite](https://github.com/IanTeda/Personal-Ledger/issues/636). Until then the current split stands, and CLAUDE.md, `/rust-style` and ADR-0009's file-split sentence change when it lands.

## Consequences

- Fewer, deeper files: one place to read an entity's persistence.
- Multi-table writes become atomic.
- The Sync Server's `change_sets` and `sync_users` code follows the same layout when it is next touched.

## Considered Options

- **Keep the per-entity file split.** Rejected: each file is a thin `impl` block, so the split adds navigation without hiding anything, and the pool-taking signatures block transactions.
