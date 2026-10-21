# Personal Ledger - VIPER Architecture

This page describes the end state the Clients are moving to. It does not describe the code as it is today: tickets take the codebase from here to there. The goal is a codebase that Claude can change one module at a time, that a person can review one layer at a time, and that grows by adding modules rather than by growing a central file.

## VIPER

VIPER splits each module (one area of concern, such as Bills) into five parts, each with one job. It comes from iOS apps, where controllers grew huge because they did everything. Each part talks only to its neighbours, and data flows one way through them.

- **View:** Draws what it's told and reports what the user does. It holds no logic and makes no decisions: it never formats a number or decides whether a button is enabled. It gets a ready-made view model, draws it, and passes the user's actions back to the Presenter as events ("Pay pressed on row 3").
- **Interactor:** The business logic, or use case: "pay this bill", "delete this payee if it has no transactions", "compute this month's budget actuals". It knows the rules and reaches storage through ports (traits), but it has no idea a View exists. One Interactor maps to one user intent.
- **Presenter:** The middle part, and the one most often skipped. It takes events from the View, decides which Interactor to call, and turns the results into something the View can draw: formatted Money, localised text, which rows are selected, whether a Dialog is open, error messages from refusals. It holds the View's state but no UI toolkit code, so it can be tested without a window.
- **Entity:** The plain data model: Account, Bill Plan, Transaction, Money. Structs plus rules about themselves only, like "is this Bill Schedule entry overdue on this date?". No I/O, and no knowledge of the other parts.
- **Router:** (also called the Wireframe) Navigation and assembly. It builds its module by wiring a View, Presenter and Interactor together, and opens the next View or Dialog. When the Presenter says "show Transaction 42", the Router makes it happen.

## How VIPER maps to this workspace

The [VIPER target architecture diagram](../diagrams/viper-architecture.html) shows one module end to end, from a key press to SQLite.

Personal Ledger has two Clients, the Desktop (gpui) and the TUI (ratatui), that show the same Ledger. So a module is split across two places:

- **A feature crate per domain**, `crates/libs/lib-<domain>`, holds everything that doesn't touch a UI toolkit: Entities, Interactors, Presenters, ports, the Router trait and the keymap. Both Clients share it.
- **A folder per domain in each bin**, `crates/bins/bin-<client>/src/<domain>/`, holds what does touch the toolkit: the Views and the Router implementation.

A module is a domain, not a View. `lib-bills` holds one Presenter per Bills View (list, form, detail), so one ticket usually touches one crate, and the crate count tracks the domain count.

### Why Views are not in the libs behind `cfg` flags

An earlier draft put each module's Views in its lib behind `#[cfg(desktop)]` and `#[cfg(tui)]`. We rejected that:

- Each crate would need a build, clippy run and test run per flag, and a lint failure behind the other flag stays hidden until CI.
- rust-analyzer, and Claude reading the code through it or through `cargo check`, see only one flag's code at a time, so half the module is greyed out or unchecked.
- A lib that depends on gpui loses the rule that libs are toolkit-free. That rule is what lets a Presenter be tested without a window and reviewed without knowing gpui.

## Layout

```text
Personal-Ledger/
├── Cargo.toml                         # Workspace root
└── crates/
    ├── bins/
    │   ├── bin-desktop/
    │   │   ├── src/
    │   │   │   ├── main.rs            # Configuration, tracing, then shell::router::build
    │   │   │   ├── lib.rs             # Everything else, so tests/ builds the app the same way
    │   │   │   ├── input.rs           # gpui key -> chord text -> Shell, then focused module
    │   │   │   ├── locale.rs          # Desktop Locale setup
    │   │   │   ├── shell/             # The Shell module's Views and Router
    │   │   │   │   ├── view/          # top bar, rails, status line, command palette, Toasts
    │   │   │   │   └── router.rs      # Root Router: builds the Shell, hosts Views and Dialogs
    │   │   │   └── <domain>/
    │   │   │       ├── mod.rs
    │   │   │       ├── view/<view>.rs # Draws the view model, handles mouse, sends events
    │   │   │       └── router.rs      # impl <Domain>Router: builds the module, opens what's next
    │   │   └── tests/ui/              # Wiring and drawing tests only
    │   ├── bin-tui/                   # Same shape as bin-desktop, drawn with ratatui
    │   └── bin-sync-server/           # Not VIPER: no Views. Reuses Interactors and ports
    └── libs/
        ├── lib-core/                  # Shared value types: RowID, Money, periods
        ├── lib-database/              # Adapter: implements the modules' ports with sqlx
        ├── lib-shell/                 # The Shell module (see below)
        └── lib-<domain>/              # One VIPER module, toolkit-free
            ├── Cargo.toml
            ├── i18n/                  # This module's Messages
            └── src/
                ├── lib.rs
                ├── entity.rs          # Or entity/ when large
                ├── interactor/<use_case>.rs  # One file per user intent
                ├── presenter/<view>.rs       # View model + event enum + Presenter
                ├── protocols.rs       # Ports (repository traits) and the <Domain>Router trait
                ├── keymap.rs          # Default keys: command name + chord -> event
                └── fake.rs            # In-memory port impls and stub seed, for tests and UI-first work
```

## Rules

These are the rules Claude follows and a review checks against.

### Dependencies

- A feature crate never depends on a UI toolkit (gpui, gpui-component, ratatui, crossterm), on `lib-database`, or on another feature crate.
- Value types shared across domains, like `RowID` and `Money`, live in `lib-core`.
- An Interactor that only reads another domain ("is this Payee used by a Split?") declares a read port in its own crate's `protocols.rs`, and `lib-database` answers it. It doesn't add a crate.
- Operations that write more than one domain live in a cross-domain module, one per user workflow (see Persistence). It may depend on the feature crates it writes; feature crates never depend on it.
- `lib-database` depends on the feature crates and implements their ports. Dependencies point inward, toward the Entities.
- No `cfg` flag selects a UI toolkit inside a lib.

### Entity

- Plain structs and rules about themselves only. No I/O, no clock reads (take the date as a parameter), no knowledge of Interactors or Presenters.

### Interactor

- One Interactor per user intent, in its own file.
- Reaches storage only through the ports in `protocols.rs`. Refusals are typed errors, not strings.
- Knows nothing about Views, Presenters, Messages or keys.

### Presenter

- One Presenter per View, holding that View's state.
- Its contract is `handle(event)` and `view_model()`. The view model is plain data: formatted Money and dates, localised text from `msg::` accessors, selection, enabled flags, which Dialog is open. The View never formats or decides anything.
- It doesn't hold a reference to its View. Both Clients redraw from the view model, so nothing calls into the View.
- It asks for navigation only through its module's `<Domain>Router` trait.
- List movement goes through `lib-list-cursor`.

### View

- Lives in the bin. Draws the view model, handles the mouse for its own layout, and sends events to the Presenter.
- No business logic, no formatting, no hardcoded display text.
- On the Desktop, the bin wraps each Presenter in a gpui `Entity` and calls `cx.notify()` after `handle`. The TUI holds the Presenter directly.

### Router

- Each module declares a `<Domain>Router` trait in `protocols.rs` (`show_transaction(id)`, `open_pay_dialog(bill)`), and each bin implements it in `<domain>/router.rs`. Routing is a module concern; only the implementation is toolkit-specific.
- A Router builds its own module: View, Presenter, Interactor and ports.
- To go to another module, a Router calls that module's public entry point in the same bin (for example `transactions::router::show(cx, id)`). It never builds another module's View or Presenter itself.
- The Router doesn't handle keys.

### Input

- Each module declares its default keys in `keymap.rs` as a command name and its default chord mapped to an event, for example `pay` / `"p"` → `BillsListEvent::Pay`. `[keybindings]` Configuration overrides a key by command name, and both Clients share one keymap.
- A command name shared across modules stays flat (`new`, `delete`, `move_down`), so one override applies in every module. A command only one module has is namespaced by the module, for example `units.set_base`.
- `lib-shell` owns the keys that work in every View: switching View, opening the command palette, moving focus between the Shell and the View.
- Each bin has one `input.rs`. It turns a toolkit key into chord text and offers it to an open Dialog first, which takes every key, then to the Shell, then to the focused module's keymap. It sends the resulting event to that module's Presenter. It doesn't grow when a module is added.
- A Presenter receives events (`Pay`), never keys (`p`).

### Localisation

- New UI text is a Message. A module's Messages live in its crate's `i18n/` and are read by its Presenter. A bin's own catalogue holds only text that belongs to one toolkit, such as the Desktop menu bar.

### Tests

- Entities, Interactors and Presenters have unit tests in their crate, run against the in-memory ports in `fake.rs`. These carry the behaviour coverage: a Presenter test is "send events, assert on the view model".
- Each View still has `<view>_keyboard.rs` and `<view>_mouse.rs` in the bin's `tests/ui/`, but they check wiring and drawing (the key reaches the Presenter, the view model is drawn), not behaviour the Presenter tests already cover. They run on the `fake.rs` ports, which `build_shell` takes as parameters, so they never open SQLite.
- An end-to-end smoke test per module runs the real app on a temporary SQLite file: start, change a row, restart, see it. It runs when a parent ticket closes and before a merge to `main`.

## The Shell module

The Shell is the always-present frame around the current View (see GLOSSARY.md). `lib-shell` is a VIPER module like any other:

- Its Presenter holds the Shell's state (the active View, focus, the status line, the Toasts, the open Dialog) and builds the Shell view model.
- Its keymap holds the global keys.
- Its Router, implemented in each bin's `shell/router.rs`, is the root Router: `main` calls `shell::router::build`, and it hosts whichever module's View is active and the one open Dialog.

## Out of scope

- **The Sync Server** has no Views, so it isn't split into VIPER modules. It reuses Interactors and ports where it needs domain rules.
- **The transition** from today's code to this layout is planned in tickets, not here: it starts with the slices decided in [#745](https://github.com/IanTeda/Personal-Ledger/issues/745).

## Persistence

Each module owns its repository ports, replacing the single `LedgerRepository` ([ADR-0039](../adr/0039-ledger-persists-through-one-store-wide-repository.md), amended).

Ports are synchronous, and `sqlx` is async ([ADR-0046](../adr/0046-per-domain-adapters-hold-rows-in-memory-behind-one-shared-writer.md)). Each `lib-database` adapter holds its domain's rows in memory, loaded when the Client starts, and serves reads from there. A write changes memory at once and goes to one writer thread shared by every domain, which runs it in one database transaction. A failed write makes the adapter reload from SQLite and is reported on a channel the bin listens on, so it can redraw and show an error Toast.

Each module's ports have a contract suite in its crate, behind a `contract-tests` feature, run against both `fake.rs` and the `lib-database` adapter: write, reload, and get the same rows.

An operation that writes more than one domain belongs to a cross-domain module, one crate per user workflow rather than per operation. It holds one Interactor per operation and one atomic port, which `lib-database` implements with one database transaction, so all of an operation's changes are saved or none are. The expected cross-domain modules are:

| Module | Operations | Domains written |
|---|---|---|
| `lib-bill-payments` | Pay and match a Bill Schedule entry | Bills, Transactions |
| `lib-document-filing` | Accept a Document, accept all | Documents, Transactions, Payees |
| `lib-transaction-import` | Commit an Import | Transactions, Payees |
| `lib-reassign` | Delete or merge a Category, Tag or Location, re-pointing the rows that use it | Categories, Tags, Inventory, Transactions, Bill Plans, Budgets, Documents |
`lib-reassign` may split per domain if it grows. Units and Price Sources are one domain, `lib-units`, since they only change together, so deleting a Price Source and setting the base Unit are single-domain Interactors. The transition tickets confirm the list against the code.
