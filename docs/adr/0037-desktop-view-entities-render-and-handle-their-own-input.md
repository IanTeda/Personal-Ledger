# Desktop View Entities render and handle their own input

[ADR-0032](0032-desktop-views-are-entities-that-own-their-view-state.md) moved each Desktop View's state into a gpui Entity, but its behaviour stayed on `Shell`. No View Entity implements `Render`: one `Shell::render` of about 1790 lines builds every page from Props structs and `On*` callbacks, and `Shell` reaches into Views through 111 `edit_*_state` calls. Each destination also has its own branch in Esc handling, search, the keys claimed ahead of the router, the `NoOp` fallthrough, movement and page status, spread across `app/key_dispatch.rs`, `app/focus.rs`, `app/status.rs` and `app/render.rs`. Decided in #594 on the [Deepen the domain libs and finish the View Entities](https://github.com/IanTeda/Personal-Ledger/issues/588) map.

We're making each View Entity render itself and handle its own input. `Shell` keeps navigation, Chrome, the Dialog host, Toasts, and one forwarding match over the active View.

## Decision

- **One forwarding enum.** `Shell::active_view_entity(cx)` returns an `ActiveViewEntity` enum holding the active View's typed Entity, e.g. `Bills(Entity<BillsView>)`. Each of its methods holds the one exhaustive match and forwards to an inherent method of the same name on the Entity. Every per-destination match lives in that one file (`view/active.rs`), so a new View fails to compile there and nowhere else. There is still no `View` trait and no `dyn` or `AnyView` erasure (ADR-0016).
- **The interface.** `key_ahead(&Keystroke) -> bool` for keys a View claims before `route_key`. `handle_key(&Keystroke) -> bool` for the router's `NoOp` fallthrough. `apply_movement(Movement)`. `escape() -> bool`, which reports whether the View consumed the Esc. `start_search() -> bool` and `search_key(&Keystroke) -> bool`, where `false` means the View has no Search and `/` is inert. `on_enter()`, which resets the View's own scroll and internal focus when it becomes the destination. `reveal(Reveal)`. `page_status()`, plus the existing title and hints as data.
- **Shell owns the generic gates.** `Shell` forwards a key only when the View zone has focus and applies the modifier and pending-`g` checks. A View never checks `nav.noun()` or `nav.focus()`. Esc asks the Dialog host's open select first, then the View, then falls back to `Shell`'s close-and-exit-mode.
- **Each View implements `Render`.** `Shell::render` embeds the active View's Entity as a child element. Click handlers become `cx.listener` closures on the View, and the scroll handle moves into the View. A View reads the store Entities it needs, such as Categories and Accounts for a filter's choices. The Props structs and `On*` callback aliases go as each View moves.
- **`ViewEvent` is the only View→Shell channel.** A View builds the Dialog it wants and emits `ViewEvent::OpenDialog`, or `RaiseToast`, `Navigate`, `SetStatus`, or `Reveal`. The per-View intent enums (`BillsEvent`, `AccountsEvent`, …) and their handlers in `app/*_ui.rs` are deleted, and one generic subscription replaces the per-View ones. A new variant is added only for a request that is truly cross-cutting.
- **Cross-View jumps are `Reveal`.** `ViewEvent::Reveal(Reveal::Transaction(id))` makes `Shell` navigate to the owning destination and call that View's `reveal`. Bills' paid rows and Documents' links use it.
- **Dialog hints come from the Dialog.** While a Dialog is open, `Shell` takes the hints from the Dialog host (`OpenDialog::hints()`), so a View's `page_status` covers only the View's own state.
- **Tests read snapshots the View builds.** Each View Entity builds its own `#[doc(hidden)]` snapshot, and `Shell`'s accessor forwards to it, so the headless tests (ADR-0030) are unchanged. The `edit_*_state` hatches are used only inside `Shell` and are deleted.
- **Migration.** One View per PR: Bills first, then Budgets, Transactions with Import, Documents, the Settings Entity with its static pages, then each Settings page that has data, Dashboard with Help, and a final cleanup.

## Consequences

- `Shell` shrinks toward navigation, Chrome, the Dialog host, Toasts and the forwarding enum, and `app/*_ui.rs` shrinks to cross-cutting wiring.
- Confirming or refusing a Dialog still runs through `Shell` until the Dialog host's cycle is decided (#596). Refreshing stale copies when another View writes shared data waits on the Ledger aggregate (#592).
- A View now depends on the stores it reads, so its tests must set up those store Entities.

## Considered Options

- **Keep `Shell`'s `ActiveView` match at each call site, each arm one line.** Rejected: the per-destination matches would stay spread across four files.
- **A statically dispatched `ViewEntity` trait behind the enum.** Rejected, as in ADR-0016's #538 update: the enum already makes every View handle every method, and the trait would add nothing but uniform signatures.
- **Keep the per-View intent events and delete `ViewEvent`.** Rejected: `Shell` would keep a View-specific match that turns each intent into a Dialog. This reverses the reasoning in ADR-0016's #538 update that a View's handling needs `Shell`'s shared state. A View reads the stores itself and asks `Shell` only for what `Shell` owns.
