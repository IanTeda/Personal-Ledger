# Desktop Views are gpui Entities that own their View state

The Desktop's per-destination state (selection, scroll, filter forms, expanded rows, the open Import) sits as roughly 80 flat fields on `Shell`, with prefixed groups such as `settings_*`, `documents_*` and `transactions_*`. Each group belongs to one View, but `Shell` owns all of them, so the Divergent Change in `shell.rs` (#531) will not shrink by moving code alone. [ADR-0016](0016-shell-replaces-feasibility-tab-bar-for-desktop-navigation.md) and its #538 update rejected a `View` trait and kept `Shell` dispatching on the exhaustive `ActiveView` enum, because a `dyn View` would need a borrow-split context bag around `Shell`'s shared state.

We're making each destination's **View** a gpui `Entity` that owns its View state. `Shell` holds one typed `Entity` per destination and dispatches on `ActiveView`, as it does now, so no trait object or `AnyView` erasure is needed. The Entity owns only what belongs to its destination; `Shell` keeps what is cross-cutting.

## Decision

- **Ownership.** A destination's View state moves into its View's Entity. There is no top-level `state/` module. The state stays in `view/<domain>/`, next to the rendering.
- **One Entity per destination.** A destination with several Views, such as Transactions with its Import mode, is one Entity. The modes are state inside it.
- **Events, not back-references.** A View reports to `Shell` by emitting typed events (`cx.emit`), and `Shell` subscribes to them. A View never holds a `WeakEntity<Shell>` or calls `Shell` directly.
- **Keyboard stays in one place.** `navigation/key_router.rs` remains the one router for the keyboard grammar in `docs/navigation-design.md`. It dispatches to the active View's Entity by `ActiveView`. A View owns its focus only for its own text input.
- **Title and hints are data.** Each View exposes its title and hints as typed values built from lib-locale Messages. Chrome renders them and holds no destination-specific text.
- **Chrome is not an Entity.** `Shell` owns a `ChromeState` struct defined in `chrome/`. Chrome is the frame around the active View, not a destination.
- **Each View loads its own data.** A View reads ledger data through a service in a lib crate, never through raw `sqlx`, so the TUI can share the same logic. Shell does not hold ledger `Vec`s.
- **Migration.** Help moves first, as it has no ledger data. Accounts moves second, to prove the data path. Each View moves in its own PR with headless UI tests ([ADR-0030](0030-desktop-ui-is-tested-in-process-through-gpui-test-support.md)). Until the last View moves, `ActiveView` dispatch stays.

## Consequences

- `Shell`'s flat per-destination fields disappear as each View moves. `Shell` shrinks toward cross-cutting state: navigation, Chrome, the Dialog host, Toasts, Preferences and the active destination.
- Each View's data is loaded asynchronously, so each View renders a loading state until its data arrives.
- Each View can be tested and reasoned about on its own. The cost is that every View's events and data flow must be wired through `Shell` consistently, and the test harness must drive Entities.
- Desktop and TUI can read the same data through the same lib service. Without that, the two clients would drift.
- Entities add a second source of truth risk: a View's copy of data can go stale when another View changes it. Views that write to shared data must emit an event that `Shell` uses to refresh the others. This applies, for example, to Bills and Documents, which write Transactions.

Update ([#594](https://github.com/IanTeda/Personal-Ledger/issues/594)): [ADR-0037](0037-desktop-view-entities-render-and-handle-their-own-input.md) finishes this move. Each View Entity renders itself and handles its own keys, Esc, search and movement, and `ViewEvent` replaces the per-View intent events.

## Considered Options

- **Shell owns each destination's state in a per-destination struct (`BudgetsState` style).** This was the recommended option. It keeps one event and data path and needs no Entity wiring. It was rejected because every per-destination struct would still be a field on `Shell`, so the Divergent Change would move rather than end.
- **Each View owns its own focus handle and gpui actions.** Rejected for now. The keyboard grammar is shared across destinations, so splitting its routing across Views would duplicate it. Revisit only if a View needs text-input focus that `Shell` cannot coordinate.
- **A `View` trait with `dyn` or `AnyView` erasure.** Still rejected, as in ADR-0016's #538 update. The concrete Entities held per destination make erasure unnecessary.
