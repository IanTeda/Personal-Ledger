# Personal Ledger - VIPER Architecture

This page describes the end state the Clients are moving to. It does not describe the code as it is today: tickets take the codebase from here to there, and [During the transition](#during-the-transition) lists what stands in for the parts not built yet. Slice 1 (Units and Price Sources, [#746](https://github.com/IanTeda/Personal-Ledger/issues/746)) built one module to this page, and its findings are folded in. The goal is a codebase that Claude can change one module at a time, that a person can review one layer at a time, and that grows by adding modules rather than by growing a central file.

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
    │   │   │       ├── view/dialog/<dialog>.rs  # One per Dialog Presenter
    │   │   │       └── router.rs      # impl <Domain>Router: builds the module, opens what's next
    │   │   └── tests/ui/              # Wiring and drawing tests only
    │   ├── bin-tui/                   # Same shape as bin-desktop, drawn with ratatui
    │   └── bin-sync-server/           # Not VIPER: no Views. Reuses Interactors and ports
    └── libs/
        ├── lib-core/                  # Shared value types: RowID, Money, periods
        ├── lib-database/              # Adapter: implements the modules' ports with sqlx
        ├── lib-shell/                 # The Shell module (see below)
        ├── lib-viper/                 # Generic Presenter types: Keymap, form and Dialog views
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
                ├── msg.rs             # This module's msg:: accessors, generated from i18n/
                ├── contract.rs        # Port contract suite, behind the contract-tests feature
                └── fake.rs            # In-memory port impls and stub seed, for tests and UI-first work
```

## Rules

These are the rules Claude follows and a review checks against.

### Dependencies

- A feature crate never depends on a UI toolkit (gpui, gpui-component, ratatui, crossterm), on `lib-database`, or on another feature crate. `deny.toml`'s bans and the `cargo metadata` test (`lib-ledger/tests/viper_dependencies.rs`) check this for each migrated crate.
- A feature crate may depend on `lib-core` and the shared toolkit-free libs: `lib-config` (for `[keybindings]`), `lib-locale`, `lib-list-cursor`, `lib-toast` (a Router asks for a Toast by `ToastKind`) and `lib-viper`.
- Value types shared across domains, like `RowID` and `Money`, live in `lib-core`. Generic types every module's Presenters and keymap use live in `lib-viper`, not in the first feature crate that needed them ([#760](https://github.com/IanTeda/Personal-Ledger/issues/760)): `keymap` holds `Keymap`, `Binding` and the flat command names modules share, and `view` a form's `TextFieldView`, `SelectView` and `Choice` and a Dialog's `DialogFrame`. It is not a feature crate and holds no domain; it keeps the feature crates' bans.
- An Interactor or Presenter that only reads another domain ("is this Payee used by a Split?", "which Accounts hold this Unit?") declares a read port in its own crate's `protocols.rs`, and `lib-database` answers it. It doesn't add a crate. `lib-units`' `UnitUsage` and `UnitHoldings` are the first.
- Operations that write more than one domain live in a cross-domain module, one per user workflow (see Persistence). It may depend on the feature crates it writes; feature crates never depend on it.
- `lib-database` depends on the feature crates and implements their ports. Dependencies point inward, toward the Entities. Only bins and cross-domain modules depend on `lib-database`; `lib-rpc` is a recorded exception until the Sync Server's storage is decided ([#759](https://github.com/IanTeda/Personal-Ledger/issues/759)).
- No `cfg` flag selects a UI toolkit inside a lib.

### Entity

- Plain structs and rules about themselves only. No I/O, no clock reads (take the date as a parameter), no knowledge of Interactors or Presenters.

### Interactor

- One Interactor per user intent, in its own file.
- Reaches storage only through the ports in `protocols.rs`. Refusals are typed errors, not strings.
- Knows nothing about Views, Presenters, Messages or keys.
- Enforces every rule itself, even one a Presenter already keeps a person from reaching (the Delete unit Dialog always offers a new base Unit, so `BaseUnitNeeded` is reachable only through the Interactor). A refusal no Client can reach is tested at the Interactor.

### Presenter

- One Presenter per View, holding that View's state.
- Its contract is `handle(event, router)` and `view_model()`, where `router` is a `&mut dyn <Domain>Router`. The view model is plain data: formatted Money and dates, localised text from `msg::` accessors, selection, enabled flags, which Dialog is open. The View never formats or decides anything.
- It doesn't hold a reference to its View. Both Clients redraw from the view model, so nothing calls into the View.
- It asks for navigation only through its module's `<Domain>Router` trait, which it is lent for each `handle` call rather than holding: the Desktop can open a Dialog only through the gpui context that call runs in.
- An enabled flag or an offered action comes from a read, not from a refused write. If a Unit in use can only be deactivated, the Presenter reads its usage before the person confirms a Delete, rather than offering Deactivate after the Interactor refuses ([#763](https://github.com/IanTeda/Personal-Ledger/issues/763)).
- What a View shows and where an outcome lands are separate settings. A Client that shows a detail pane is not thereby one that logs outcomes to ACTIVITY instead of raising Toasts ([#765](https://github.com/IanTeda/Personal-Ledger/issues/765)).
- A Dialog's Presenter holds its fields' values, focus, which fields show, validity and refusal text, and it tells the page Presenter its outcome when it closes, confirmed or cancelled.
- List movement goes through `lib-list-cursor`.

### View

- Lives in the bin. Draws the view model, handles the mouse for its own layout, and sends events to the Presenter.
- No business logic, no formatting, no hardcoded display text.
- Keeps a widget's own mechanics, which are the toolkit's and decide nothing: a text field's caret, a select's open list and highlight, a secret field's typed text before it is sent.
- On the Desktop, the bin wraps each page Presenter in a gpui `Entity` and calls `cx.notify()` after `handle`. A Dialog's Presenter sits in the Dialog host (`Rc<RefCell<_>>`), since today's `Dialog` trait gets no gpui context to update an Entity through; `lib-shell`'s Dialog host can take one. The TUI holds every Presenter directly.

### Router

- Each module declares a `<Domain>Router` trait in `protocols.rs` (`show_transaction(id)`, `open_pay_dialog(bill)`), and each bin implements it in `<domain>/router.rs`. Routing is a module concern; only the implementation is toolkit-specific.
- A Router builds its own module: View, Presenter, Interactor and ports. On the Desktop it is built for each `handle` call around that call's gpui context, and the module's entry point (`units::router::units_page`) builds the page.
- `open_dialog` returns whether the Dialog opened, so a Presenter whose row went away under it doesn't record a Dialog as open.
- To go to another module, a Router calls that module's public entry point in the same bin (for example `transactions::router::show(cx, id)`). It never builds another module's View or Presenter itself.
- The Router doesn't handle keys.

### Input

- Each module declares its default keys in `keymap.rs` as `lib_viper::keymap::Binding`s, a command name and its default chord mapped to an event, for example `pay` / `"p"` → `BillsListEvent::Pay`. `[keybindings]` Configuration overrides a key by command name, and both Clients share one keymap.
- A command name shared across modules stays flat (`new`, `delete`, `move_down`), so one override applies in every module; its constant lives in `lib_viper::keymap`. A command only one module has is namespaced by the module, for example `units.set_base`.
- `lib-shell` owns the keys that work in every View: switching View, opening the command palette, moving focus between the Shell and the View.
- Each bin has one `input.rs`. It turns a toolkit key into chord text and offers it to an open Dialog first, which takes every key, then to the Shell, then to the focused module's keymap. It sends the resulting event to that module's Presenter. It doesn't grow when a module is added.
- The Shell's global keys win, so a module's default chord must not be a global key in either Client. Slice 1 found three clashes: `units.set_base`'s `b` is the Desktop's rail toggle, and `ctrl+u` and `g g` are TUI View jumps. A test per Client checks every module's defaults against its global keys ([#764](https://github.com/IanTeda/Personal-Ledger/issues/764)).
- A Dialog's keys (confirm, cancel, next field, select stepping) are the Dialog host's keymap in `lib-shell`, under the same flat names (`confirm`, `cancel`), so a `[keybindings]` override reaches every module's Dialogs. The host sends the Dialog's Presenter its `Cancel` event when `Esc` closes it.
- A Presenter receives events (`Pay`), never keys (`p`).

### Localisation

- New UI text is a Message. A module's Messages live in its crate's `i18n/` and are read by its Presenter. A bin's own catalogue holds only text that belongs to one toolkit, such as the Desktop menu bar.
- A module's Message ids carry its domain as a prefix (`units-`), generated by `lib-locale-build` with `id_prefix`. A count in a plural is `{ $count }`, never a typed digit.
- A time or date in a view model is formatted by `lib-locale`, never by `chrono` in the Presenter ([#762](https://github.com/IanTeda/Personal-Ledger/issues/762) adds the minutes-only time ACTIVITY needs).

### Tests

- Entities, Interactors and Presenters have unit tests in their crate, run against the in-memory ports in `fake.rs`. These carry the behaviour coverage: a Presenter test is "send events, assert on the view model". `fake.rs`'s `RecordingRouter` records what the Presenter asked for.
- Each View still has `<view>_keyboard.rs` and `<view>_mouse.rs` in the bin's `tests/ui/`, but they check wiring and drawing (the key reaches the Presenter, the view model is drawn), not behaviour the Presenter tests already cover. They run on the `fake.rs` ports, which `build_shell` takes as parameters, so they never open SQLite.
- An end-to-end smoke test per module runs the real app on a temporary SQLite file: start, change a row, restart, see it. It runs when a parent ticket closes and before a merge to `main`.

## The Shell module

The Shell is the always-present frame around the current View (see GLOSSARY.md). `lib-shell` is a VIPER module like any other:

- Its Presenter holds the Shell's state (the active View, focus, the status line, the Toasts, the open Dialog) and builds the Shell view model.
- Its keymap holds the global keys.
- Its Router, implemented in each bin's `shell/router.rs`, is the root Router: `main` calls `shell::router::build`, and it hosts whichever module's View is active and the one open Dialog.

## Out of scope

- **The Sync Server** has no Views, so it isn't split into VIPER modules. It reuses Interactors and ports where it needs domain rules.
- **The transition** from today's code to this layout is planned in tickets, not here: it starts with the slices decided in [#745](https://github.com/IanTeda/Personal-Ledger/issues/745). [During the transition](#during-the-transition) lists only what stands in meanwhile.

## Persistence

Each module owns its repository ports, replacing the single `LedgerRepository` ([ADR-0039](../adr/0039-ledger-persists-through-one-store-wide-repository.md), amended).

Ports are synchronous, and `sqlx` is async ([ADR-0046](../adr/0046-per-domain-adapters-hold-rows-in-memory-behind-one-shared-writer.md)). Each `lib-database` adapter holds its domain's rows in memory, loaded when the Client starts, and serves reads from there. A write changes memory at once and goes to one writer thread shared by every domain, which runs it in one database transaction. A failed write is reported on a channel the bin listens on, so it can reload, redraw and show an error Toast.

What that means for a module, as slice 1 built it (`lib-database`'s `adapter/units.rs` and `writer.rs`):

- **The bin ends each Interactor call.** A port write is one row (`upsert_unit`, `set_base_unit`), so a port can't tell where an Interactor call ends. Every adapter adds its writes to one pending batch on the writer, and the bin calls `commit()` after each Presenter `handle`, which sends the batch to the writer as one transaction. Deleting a Price Source (N Unit moves and the delete) and deleting the base Unit (the move and the delete) each land whole, and so does a call that writes through two domains' adapters. Slice 1 kept the batch per adapter; slice 2 shares it ([#757](https://github.com/IanTeda/Personal-Ledger/issues/757)).
- **The bin reloads.** Ports know nothing about the failure channel, so on a `WriteFailure` the bin calls `reload()` on each adapter the failure names. A failure also names the domains of the writes dropped behind it, since their adapters are stale too. If the reload fails, the rows in memory stand and the Toast says they aren't saved.
- **Loads go through the writer**, queued behind the writes, so a reload sees every earlier write and one runtime owns the pool.
- **A row may name a row defined in code.** A Unit's Price Source can be Manual entry (never stored) or a built-in with no override row (ADR-0043), so `units.price_source_id` has no foreign key. The schema says why, and the Interactor guards the link.
- **A port's rules match the schema's.** A foreign key the in-memory ports don't model fails only in SQLite, as a rollback the person can't explain. Each such constraint is an Interactor rule and a contract check ([#761](https://github.com/IanTeda/Personal-Ledger/issues/761): deleting a Unit other Units quote).

Each module's ports have a contract suite in its crate, behind a `contract-tests` feature, run against both `fake.rs` and the `lib-database` adapter: write, reload, and get the same rows.

An operation that writes more than one domain belongs to a cross-domain module, one crate per user workflow rather than per operation. It holds one Interactor per operation and one atomic port, which `lib-database` implements with one database transaction, so all of an operation's changes are saved or none are. The expected cross-domain modules are:

| Module | Operations | Domains written |
|---|---|---|
| `lib-bill-payments` | Pay a Bill Schedule entry directly | Bills, Transactions |
| `lib-document-filing` | Accept a Document, accept all | Documents, Transactions, Payees |
| `lib-transaction-import` | Commit an Import | Transactions, Payees |
| `lib-reassign` | Delete or merge a Category, Tag or Location, re-pointing the rows that use it | Categories, Tags, Inventory, Transactions, Bill Plans, Budgets, Documents |
`lib-reassign` may split per domain if it grows. An operation that writes one domain and only reads another is not cross-domain: Match writes only the Bill Schedule entry and reads Transactions through a read port, so it is a `lib-bills` Interactor.

A cross-domain module may own the Dialog whose confirm calls its Interactor. `lib-bill-payments` holds the Pay Dialog's Presenter, both panels, because Pay it directly is its Interactor; the Match panel calls `lib-bills`' Match Interactor, which the module may depend on. It has its own `i18n/`, and each bin gives it a folder like a domain's (`bill_payments/view/dialog/`, `bill_payments/router.rs`). The Bills Router opens it through that folder's entry point (`bill_payments::router::open_pay(cx, entry)`), as it would another module's View.

Units and Price Sources are one domain, `lib-units`, since they only change together, so deleting a Price Source and setting the base Unit are single-domain Interactors. The transition tickets confirm the list against the code.

## During the transition

Until every domain is a module and `lib-shell` exists, these stand in, each removed when its condition is met. Slice 1 added them for Units (`bin-desktop/src/units/`, `bin-tui/src/units/`); slice 2 ([#774](https://github.com/IanTeda/Personal-Ledger/issues/774)) adds the Transactions ones (`bin-desktop/src/transactions/ports.rs`, `ledger_store.rs`).

| Stand-in | Why | Removed when |
|---|---|---|
| The `Ledger` read copy of Units, the base Unit and the Price Sources, refreshed after each Units write and on rollback (`LedgerStore::refresh_units`) | Bills, Budgets, Transactions, the Dashboard and the Bill and Budget forms still read Units from `Ledger` | The last of them reads Units through its own domain's read port |
| `impl UnitUsage` and `impl UnitHoldings` for `Ledger` in `lib-ledger`, reached through `LedgerBackedPorts`, which copies them from the Ledger before each Units event | Accounts, Bill Plans and Budgets aren't persisted, and a port can't read the `LedgerStore` Entity without a gpui context | Each count moves to `lib-database` as its domain migrates |
| The first-launch seed: a Ledger with no base Unit is seeded with `InMemoryPorts::stub`'s Units, Price Sources and base Unit (`UnitsAdapter::seed`), and with the stub Ledger's Transactions dated from that first launch (`TransactionsAdapter::seed`) | The stub Accounts, Bill Plans and Budgets name the stub's Units, and the stub Bill Schedule and Documents name the stub's Transactions | Every domain persists; a new database then starts through the new-Ledger workflow |
| `lib-database` takes the stub Transactions from `lib-ledger`'s joined stub (`seed::stub_ledger_transactions`), not from `lib_transactions::fake` alone | The Bills and Documents stubs add the Transactions their Paid entries and Links name | Bills and Documents persist their own seeds, with the slice 1 seed |
| The `Ledger` read copy of the Transactions, refreshed from the Transactions port after each commit and on rollback, newest first (`LedgerStore::with_storage`, `Ledger::refresh_transactions`) | The Transactions View, Bills, Budgets, Documents, the Dashboard and the Settings pages that count Splits still read Transactions from `Ledger` | The last of them reads Transactions through its own domain's read port |
| The forwarding shim: `LedgerStore::apply` sends each command's `UpsertTransaction` and `DeleteTransaction` to the Transactions port (`transactions::ports::forward`) in the same commit as the command. Writers carried: Account delete, Import commit, Tag delete and merge, Category reassign, Bills' Pay | Every writer of Transactions is still a `LedgerCommand` | Each writer leaves it when it migrates (Bills' Pay with `lib-bill-payments`, [#779](https://github.com/IanTeda/Personal-Ledger/issues/779)); the two `LedgerChange` variants go with the last |
| No foreign keys from `transactions.account_id`, `splits.category_id`, `splits.payee_id` or `split_tags.tag_id` | Accounts, Categories, Payees and Tags aren't persisted, so no row would back them | Each is added when its domain persists |
| `UnitsRouter::raise_toast` and `set_status` | The Toasts and the status line are `lib-shell`'s, which isn't built; the TUI, with no status slot, raises an Info Toast | `lib-shell`'s Router takes them |
| Keys reach the Units keymap through today's dispatch (`route_key` on the Desktop, `shell.rs` in the TUI), and the View maps arrows, Home/End and Page keys to `Move` | No `input.rs` yet | Each bin has its one `input.rs` |
| `LedgerRepository` and `LedgerChange`, persisting nothing since Units left | Other domains still apply changes through them | Charted in [#758](https://github.com/IanTeda/Personal-Ledger/issues/758) |
