# History starts in memory, Client-local and session-only

The Settings deck's History page (16u, `docs/ux/desktop-mockups/16-settings/README.md`) draws History as one record across devices: "all devices · last 30 days", a DEVICE column, and "synced changes undo on every device". The [History map](https://github.com/IanTeda/Personal-Ledger/issues/1237) needed Undo and Redo on the Desktop now, from Settings › General first, then Display and every Ledger domain, while the Sync Server is still a feasibility surface ([ADR-0009](0009-lww-sqlite-change-set-log.md)) with no Change Set written by a Client.

We're starting History in memory, on the one Client, for the one session:

- **Held by the Shell's Presenter.** `lib-shell`'s generic `History<R>` lives in the Shell's Presenter, which owns Undo and Redo as global commands (`u`, `ctrl r`) and the header's dropdown ([#1238](https://github.com/IanTeda/Personal-Ledger/issues/1238), [#1241](https://github.com/IanTeda/Personal-Ledger/issues/1241), [#1242](https://github.com/IanTeda/Personal-Ledger/issues/1242)). The Client's root Router reverses and replays an entry; the Shell never names what a Ledger change is.
- **50 entries deep, fixed.** One entry is one input's change, a commit or one Configuration key written, and a new entry past the depth drops the oldest. The depth isn't configurable.
- **Client-local.** An entry records the before and after values this Client wrote: raw values in `personal-ledger.conf` ([#1239](https://github.com/IanTeda/Personal-Ledger/issues/1239)), whole-row images from each store's recording wrapper, cascades included ([#1240](https://github.com/IanTeda/Personal-Ledger/issues/1240)). Another Client, the TUI or another device, neither sees nor undoes them, and the TUI keeps no History.
- **Session-only.** Nothing is written to disk. History is lost on quitting, and a Ledger's entries leave it when that Ledger closes or another opens, so an entry is never applied to a file it didn't come from.
- **Linear, so no conflict check.** Undo writes the earlier value back through the same door as any other write, overrides still winning, and redo is cleared by a new change. Within one session on one Client nothing else writes between an entry and its Undo, so the before image is always the right one to restore.

This departs from 16u on purpose: the DEVICE column, "last 30 days", "synced changes undo on every device" and the History page itself (its marker band, Restore to here and the area filter) are out of the first stage, not rejected.

## Considered options

- **16u's synced History now.** Rejected for this stage: it needs Change Sets a Client doesn't yet write, and conflict rules no one has decided, before the first Undo could ship.
- **Persisting History in the Ledger file, unsynced.** Rejected for now: an entry would outlive the code that wrote it, so its before and after images would need a stable, migrated form, and a Configuration entry has no Ledger to live in.

## Consequences

A persisted or synced History needs what this one leaves out, and the in-memory one is the seam to grow from rather than a dead end:

- **Entries that outlive the process:** a stored form of each entry and its reversal, versioned across schema changes, and a home for Configuration entries, which belong to the device, not a Ledger.
- **A Change Set per entry:** an entry that undoes on every device must reach them as the Change Sets the Sync Server already moves ([ADR-0009](0009-lww-sqlite-change-set-log.md)), naming its origin device for 16u's DEVICE column.
- **Conflict rules once History isn't linear:** when another device or Client may have written the same field since, restoring a before image blindly can lose that write. A synced Undo needs a rule for it, under or beside ADR-0009's per-field last-write-wins, and the "no conflict check" above stops holding.
