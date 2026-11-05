# Personal Ledger - VIPER Architecture

This page describes the end state the Clients are moving to, not the code as it is today. [Migrating a domain](#migrating-a-domain) is the checklist a domain follows to get there, and [During the transition](#during-the-transition) lists what stands in for the parts not built yet. The order of the slices and what each found are on the transition map, [#747](https://github.com/IanTeda/Personal-Ledger/issues/747), and the slice tickets it links. The goal is a codebase that Claude can change one module at a time, that a person can review one layer at a time, and that grows by adding modules rather than by growing a central file.

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
    │   │   │   │   ├── state.rs       # The frame's state the Presenter doesn't hold
    │   │   │   │   ├── router.rs      # Root Router: builds the Shell, hosts Views and Dialogs
    │   │   │   │   └── router/        # impl Root blocks, one per concern
    │   │   │   └── <domain>/          # Or <workflow>/ for a cross-domain module
    │   │   │       ├── mod.rs
    │   │   │       ├── ports.rs       # The module's ports, built over the shared stores
    │   │   │       ├── view/<view>.rs # Draws the view model, handles mouse, sends events
    │   │   │       ├── view/dialog/<dialog>.rs  # One per Dialog Presenter
    │   │   │       └── router.rs      # impl <Domain>Router and the module's entry point
    │   │   └── tests/ui/              # Wiring and drawing tests only
    │   ├── bin-tui/                   # Same shape as bin-desktop, drawn with ratatui
    │   └── bin-sync-server/           # Not VIPER: no Views. Reuses Interactors and ports
    └── libs/
        ├── lib-core/                  # Shared value types: RowID, Money, periods
        ├── lib-dashboard/             # The Dashboard module: reads Bills and Budgets through its own ports
        ├── lib-database/              # Adapter: implements the modules' ports with sqlx
        ├── lib-shell/                 # The Shell module, Help's Dialog Presenter included (see below)
        ├── lib-viper/                 # Generic Presenter types: Keymap, form and Dialog views
        └── lib-<domain>/              # One VIPER module, toolkit-free; a cross-domain module is lib-<workflow>
            ├── Cargo.toml
            ├── i18n/                  # This module's Messages
            └── src/
                ├── lib.rs
                ├── entity.rs          # Or entity/ when large
                ├── error.rs           # The crate's typed refusals, unless the Entity raises them
                ├── interactor/<use_case>.rs  # One file per user intent
                ├── presenter/<view>.rs       # View model + event enum + Presenter
                ├── protocols.rs       # Ports (repository and read traits) and the <Domain>Router trait
                ├── keymap.rs          # Default keys: command name + chord -> event
                ├── msg.rs             # This module's msg:: accessors, generated from i18n/
                ├── contract.rs        # Port contract suite, behind the contract-tests feature
                └── fake.rs            # In-memory port impls and stub seed, for tests and UI-first work
```

## Rules

These are the rules Claude follows and a review checks against.

### Dependencies

- A feature crate never depends on a UI toolkit (gpui, gpui-component, ratatui, crossterm), on `lib-database`, or on another feature crate, dev-dependencies included. `deny.toml`'s bans and the `cargo metadata` test (`lib-ledger/tests/viper_dependencies.rs`) check this for each migrated crate.
- A feature crate may depend on `lib-core` and the shared toolkit-free libs: `lib-config` (for `[keybindings]`), `lib-locale`, `lib-list-cursor`, `lib-toast` (a Router sends the Shell `Notice`s) and `lib-viper`, and on `tracing` for its Interactors' events (ADR-0047).
- A feature crate does no I/O. It reaches the disk through a port of its own with an in-memory fake, implemented in the bin (`lib-documents`' `DocumentFiles`, [#960](https://github.com/IanTeda/Personal-Ledger/issues/960)).
- Value types shared across domains, like `RowID` and `Money`, live in `lib-core`, and so do the seed ids another domain's stub names (`InstitutionId::SEED_ANZ`) and a constant two domains read (the Financial Year start), rather than one feature crate depending on another.
- Generic types every module's Presenters and keymap use live in `lib-viper`, not in the first feature crate that needed them ([#760](https://github.com/IanTeda/Personal-Ledger/issues/760)): `keymap` holds `Keymap`, `Binding` and the flat command names modules share, `view` a form's `TextFieldView`, `SelectView`, `SelectList` and `Choice`, a Dialog's `DialogFrame` and the status line's `Hint`, and `dialog` a module Dialog's `DialogInput` and `DialogRouter`. It is not a feature crate and holds no domain; it keeps the feature crates' bans.
- Operations that write more than one domain live in a cross-domain module, one per user workflow (see [Persistence](#persistence)). It may depend on the feature crates it writes; feature crates never depend on it.
- `lib-database` depends on the feature crates and the cross-domain modules and implements their ports, a cross-domain module's atomic port included (`BillPaymentsAdapter` implements `lib-bill-payments`' `BillPayments`). Dependencies point inward, toward the Entities. Only bins depend on `lib-database`. `lib-database` may depend on `lib-shell` only to answer its read port, the one module port the Shell declares.
- A module's reads of other domains are answered by one stored reads struct per reading module in `lib-database` (`StoredPayeeReads`, [#992](https://github.com/IanTeda/Personal-Ledger/issues/992); `StoredBillReads`, `StoredImportReads` and `StoredInstitutionReads`, [#994](https://github.com/IanTeda/Personal-Ledger/issues/994); `StoredTransactionReads`, [#998](https://github.com/IanTeda/Personal-Ledger/issues/998); `StoredInventoryReads`, [#1014](https://github.com/IanTeda/Personal-Ledger/issues/1014)), holding the shared stores of the domains it reads. It is the one place the other domains' rows are mapped into the reading module's types, so neither `lib-ledger`, the bin nor a domain's adapter answers them: no adapter implements another feature crate's read port. Adding a reader edits another domain's store trait only when the count must be made in the file; a count answerable from the store's rows is computed in the reading module's struct (see [Stores](#stores)).
- The Sync Server follows this too: `lib-rpc` is the wire protocol only (generated types, clients and server traits), and `bin-sync-server` implements `SyncService`'s `Push`/`Pull` over `lib-database`'s `change_sets` itself, as it does `/authorize` and `/token` over `sync_users`. Ports would only add a seam with one implementation ([#759](https://github.com/IanTeda/Personal-Ledger/issues/759)).
- No `cfg` flag selects a UI toolkit inside a lib.

### Read ports

- An Interactor or Presenter that only reads another domain ("is this Payee used by a Split?", "which Accounts hold this Unit?") declares a read port in its own crate's `protocols.rs`, and `lib-database` answers it. It doesn't add a crate. `lib-units`' `UnitUsage` and `UnitHoldings` are the first.
- This holds whichever domain migrated first: a domain that a later one links to declares what it needs of the later one as its own ports, lent at open, never as a dependency (Inventory's `LocationUsage`, [#866](https://github.com/IanTeda/Personal-Ledger/issues/866)).
- A read port returns types its own crate declares, never another feature crate's Entity: `UnitHoldings` returns a `Holding`, not an `Account`, and `lib-bills` reads a `BillAccount`, a `BillCategory` and a `BillSplit` ([#792](https://github.com/IanTeda/Personal-Ledger/issues/792)). The type names only the fields its crate reads, so a change to the other domain's Entity recompiles nobody.
- Those fields are checked against the code, not the spec: a field nothing reads is dropped (`TransactionAccount` has no open flag), and one the crate needs but can't compute is added (`TransactionCategory::parent`, for "a parent matches its descendants", [#925](https://github.com/IanTeda/Personal-Ledger/issues/925)).
- A fact the reading crate's rules need, such as whether an Account takes Transactions or a Category is a leaf, is a field the answering side fills by calling the owning domain's rule, so each rule lives once, in its own domain, and the reading crate checks it against the field rather than receiving a list filtered for it.
- The exception is a domain the reading crate also writes: a cross-domain module's read of it answers that domain's own type, since the module's write carries that type and runs its rules anyway (`lib-transaction-import`'s `ImportPayees` answers `lib-payees`' `Payee`, [#949](https://github.com/IanTeda/Personal-Ledger/issues/949)).
- A read port that guards a name answers every row, inactive ones too, since an inactive row's name stays taken; offering only active rows is the select's job. A domain with no active flag answers by its own rule (`ImportAccount::is_active` is `lib_transactions::takes_transactions`, [#943](https://github.com/IanTeda/Personal-Ledger/issues/943)).
- A read port's methods carry the reading module's prefix (`transaction_accounts`, `transaction_unit_code`), so an adapter that answers two modules' ports never has two `unit_code`s in scope.
- Two crates may declare read types of the same shape, and that's deliberate: each says what it reads, and only the few lines mapping onto it repeat. When a third wants the same shape and it is a genuine value (an id and a display name, say), it moves to `lib-core` as a shared value type.
- A figure is counted from the rows it describes (`AccountUsage`), not stored on an Entity. A count that only feeds a figure is infallible, a value type plus a read port answering 0 on a failed read (`TagUse`, `TagUsage`); a count that feeds a refusal fails open or closed on purpose. `PayeeUsage` fails open, offering Delete, because the foreign key refuses the write and its Error Toast reloads.
- A usage read goes to the adapter on each call, so a page issues one per row per render; a grouped query or a cache waits until that shows ([#863](https://github.com/IanTeda/Personal-Ledger/issues/863)).
- A ports bundle that reads through a shared store's `RefCell` returns owned rows, since it can't lend a `&dyn` getter (`TransactionsPorts`, [#929](https://github.com/IanTeda/Personal-Ledger/issues/929)).

### Entity

- Plain structs and rules about themselves only. No I/O, no clock reads (take the date as a parameter), no knowledge of Interactors or Presenters.
- A rule about one Entity lives on it, and a rule over the set (unique names, the one default) on the set's type, which writes what changed (`lib-budgets`' `Budget`, and `Budgets::changed_since`, which writes the Budget losing the default before the one taking it).
- A refusal the Entity made for an empty value goes when the field becomes an id; the form reports the empty select itself (`LocationError::UnitMissing`, [#867](https://github.com/IanTeda/Personal-Ledger/issues/867)).

### Interactor

- One Interactor per user intent, in its own file.
- Reaches storage only through the ports in `protocols.rs`: it loads the rows, runs the rule and writes what changed. Refusals are variants of the crate's error type: typed, never strings. It lives in `error.rs`, or beside the Entity when the Entity's own rules raise it (`lib-payees`' `PayeeError` in `entity.rs`).
- Knows nothing about Views, Presenters, Messages or keys.
- Enforces every rule itself, even one a Presenter already keeps a person from reaching (the Delete unit Dialog always offers a new base Unit, so `BaseUnitNeeded` is reachable only through the Interactor). A refusal no Client can reach is tested at the Interactor.

### Presenter

- One Presenter per View, holding that View's state. State that is never written (Documents' Skip and one-level Undo) is the Presenter's, not the Entity's or the file's.
- Its contract is `handle(event, router)` and `view_model()`, where `router` is a `&mut dyn <Domain>Router`. The view model is plain data: formatted Money and dates, localised text from `msg::` accessors, selection, enabled flags, which Dialog is open. The View never formats or decides anything.
- It doesn't hold a reference to its View. Both Clients redraw from the view model, so nothing calls into the View.
- It asks for navigation only through its module's `<Domain>Router` trait, which it is lent for each `handle` call rather than holding: the Desktop can open a Dialog only through the gpui context that call runs in.
- A port the Desktop answers through that context is lent the same way, at open for reads and to `confirm` for writes (`DeletionReads`, the Categories form's `CategoryBudget`, [#952](https://github.com/IanTeda/Personal-Ledger/issues/952)).
- A Presenter may live only while its View shows: the Entity holds an `Option` of it, built when the View opens with its selects' rows read then, and dropped on close, so it needs no read copy between (`ImportPresenter`, [#943](https://github.com/IanTeda/Personal-Ledger/issues/943)).
- An enabled flag or an offered action comes from a read, not from a refused write. If a Unit in use can only be deactivated, the Presenter reads its usage before the person confirms a Delete, rather than offering Deactivate after the Interactor refuses ([#763](https://github.com/IanTeda/Personal-Ledger/issues/763)).
- What a View shows and where an outcome lands are separate settings. A Client that shows a detail pane is not thereby one that logs outcomes to ACTIVITY instead of raising Toasts ([#765](https://github.com/IanTeda/Personal-Ledger/issues/765)).
- A Dialog's Presenter holds its fields' values, focus, which fields show, validity and refusal text, and its selects' open lists and highlights (`lib_viper::view::SelectList`, which its view model carries beside each `SelectView`). It takes the Dialog keymap's commands and typed keys as `lib_viper::dialog::DialogInput`s and decides what each means on its focused field, so the View reads no key ([#988](https://github.com/IanTeda/Personal-Ledger/issues/988)). It raises its own Toast before it closes, so the page lands only the outcome, and it tells the page Presenter its outcome when it closes, confirmed or cancelled. What the page opens next is in that outcome (`BudgetsOutcome::Limit`, `Stop`, `Transactions`, `Show`), since a Dialog's Router opens nothing ([#955](https://github.com/IanTeda/Personal-Ledger/issues/955)).
- A nested Presenter (the Transactions filter's `FilterPresenter`, held by the page while the popover is open) has the page's events routed to it and tells its host how it closed through an outcome returned from `handle` (`FilterOutcome::Applied` or `Cancelled`), since its crate has no Router.
- A row's context menu is split: the Presenter holds the row the menu is on and its entries, and the View holds the pointer position.
- A command whose Dialog isn't built raises a "not yet built" status Notice through `Notify` rather than binding nothing.
- List movement goes through `lib-list-cursor`.

### View

- Lives in the bin. Draws the view model, handles the mouse for its own layout, and sends events to the Presenter.
- No business logic, no formatting, no hardcoded display text.
- Keeps a widget's own mechanics, which are the toolkit's and decide nothing: a text field's caret, a select's scroll window, a secret field's typed text before it is sent. A select's open list and highlight are the Presenter's `SelectList`; the View draws them and hit-tests a click on a row.
- On the Desktop, the bin wraps each page Presenter in a gpui `Entity` and calls `cx.notify()` after `handle`. A Dialog is an `Entity` too, holding its Presenter and the widget state that is gpui's (`shell/view/dialog_entity.rs`); the Shell's Dialog host holds the open one and closes it when the Dialog's Presenter does, and the Entity hands its outcome to the page (`DocumentsPage::dialog_landed`). The TUI holds every Presenter directly.
- A row's render can't read the page Entity it is drawn inside (gpui panics), so the page passes in the lookups it needs (`Names` in the Accounts rows, [#848](https://github.com/IanTeda/Personal-Ledger/issues/848)).
- On the Desktop, a click sends the page's Presenter event through `PageClicks` (`send`, `send_with`, and `row` and `menu` for a row's click and right-click, which also hand the page focus), from a `PresenterPage<E>` impl per Presenter; a page has no Props struct or callback alias (ADR-0037's 2026-11-04 amendment).
- A page with rows has a selected-row highlight and the shared flat commands (`j`/`k`, `n`), and a Desktop Settings page emits `PageRowClicked`, a supertrait of `Page` it doesn't declare separately.
- A Desktop Settings page overrides `Page::hints` with its Presenter's hints, which the default `page_status` and the `?` cheat sheet both read, so both follow `[keybindings]`, and overrides `status_note` for a right-hand text of its own rather than `page_status` ([#839](https://github.com/IanTeda/Personal-Ledger/issues/839)). Its ports reach `Root` through a field on `settings::pages::SettingsSetup`, not a parameter.
- A top-level View hands `?` a `HelpGroup` under its own heading Message through `ActiveViewEntity::help_group`, read from the hints its status line shows ([#845](https://github.com/IanTeda/Personal-Ledger/issues/845)).

### Router

- Each module declares a `<Domain>Router` trait in `protocols.rs` (`show_transaction(id)`, `open_pay_dialog(bill)`), and each bin implements it in `<domain>/router.rs`. Routing is a module concern; only the implementation is toolkit-specific.
- A Router builds its own module: View, Presenter, Interactor and ports. On the Desktop it is built for each `handle` call around that call's gpui context, and the module's entry point (`units::router::units_page`) builds the page.
- The bin's `router.rs` is the module's entry point for every other module (`transactions::router::show(cx, id)`, `show_filtered(cx, filters)`). It finds the page through a gpui global installed in `Root::new` and does nothing without a Shell ([#929](https://github.com/IanTeda/Personal-Ledger/issues/929)).
- To go to another module, a Router calls that module's entry point in the same bin. It never builds another module's View or Presenter itself. A hand-off back from a View (the Import's commit) is a method on the module's Router trait, which the bin implements by calling the destination's entry point (`transactions::router::first_row`).
- `open_dialog` returns whether the Dialog opened, so a Presenter whose row went away under it doesn't record a Dialog as open.
- A Dialog's Router opens nothing: it closes its Dialog and notifies. Every module Dialog closes through one Router, `lib_viper::dialog::DialogRouter<O>`, generic over the Dialog's outcome, with `close(outcome)` (`None` for a cancel) and `Notify` as a supertrait; the Desktop's Dialog host implements it once for every module (`shell::view::dialog_entity::HostRouter`), and its Dialog hands a confirmed outcome to the page. A cross-domain module's Dialog closes with the owning domain's outcome, so the page hears of it as of its own Dialogs (`lib-unit-deletion` with `UnitsOutcome`).
- The Router doesn't handle keys.
- Every module Router, the cross-domain and Dialog Routers included, declares `lib-toast`'s `Notify` as a supertrait, so a Presenter tells the Shell things by sending a `Notice`: a Toast (`ToastKind` and text) or a status-line message. Only these one-way notices are shared; navigating and opening a Dialog stay typed methods on each module's Router, since they need module types and an answer. A module only sets the status line: the Shell clears it on the next keypress ([ADR-0048](../adr/0048-modules-notify-the-shell-through-one-lib-toast-trait.md)).

### Input

- Each module declares its default keys in `keymap.rs` as `lib_viper::keymap::Binding`s, a command name and its default chord mapped to an event, for example `pay` / `"p"` → `BillsListEvent::Pay`. `[keybindings]` Configuration overrides a key by command name, and both Clients share one keymap.
- A command name shared across modules stays flat (`new`, `delete`, `move_down`), so one override applies in every module; its constant lives in `lib_viper::keymap`. A command only one module has is namespaced by the module, for example `units.set_base`, and so is one whose flat name's `lib-config` default would move the page's key (`tags.remove`, since `delete` defaults to `d`).
- The keymap is the command palette's table. Each `Binding` chooses its `Palette` (`lib_viper::keymap::Palette`): `Binding::listed` names the Command as typed, `<domain> <verb>` in kebab case (`units set-base`), a stable English id, with its description a Message in the module's `i18n/` under its prefix (`units-command-set-base`); `Binding::hidden` leaves it out.
- Hide list movement, `select` and `open` (`enter`, which means something different per row), expand and collapse, and a key that only makes sense on the row it acts on; every Dialog binding is hidden.
- The palette shows each listed Command's live key (`Keymap::chord`, `—` while unbound) and runs it by name through `Keymap::event_for`, opening its page first, so the event reaches the Presenter as the key's does and commits once. The global keymap's descriptions are `lib-locale`'s ([#846](https://github.com/IanTeda/Personal-Ledger/issues/846)).
- `lib-shell` owns the keys that work in every View, its global keymap (`lib_shell::keymap`), under flat names a `[keybindings]` override moves: switching View (the `g` jumps), the command palette, Search, Help, the rail toggle, moving focus between the Shell and the View, the Toast keys, and list movement. A sequence (`g d`) is one chord whose first key is a prefix: the Presenter waits for the next key, and the bin's one-second timer ends the wait with a `PrefixExpired` input, so the Presenter never reads time.
- Each bin has one `input.rs`. It turns a toolkit key into chord text and offers it, in one fixed order, to (1) an open Dialog, which takes every key; (2) a mode left in the bin (on the Desktop the palette, Search, a filter, the file explorer, or a mode the View entered); (3) the Shell's global keymap; (4) the focused module's keymap, which sends the event to that module's Presenter; (5) a View not yet a module, with its own key handler. It doesn't grow when a module is added.
- The Shell keeps most of its keys, taking them at step 3, but falls back on the rest: focus `tab`, `a` (Insert), list movement and `enter` reach the Shell only once the module and the View have declined them, so a list's own `tab` or `a` wins. The movement commands keep the flat names each module binds (`move_down`, and `arrow_down`, `home`, `end`, `page_down`, `page_up` for the arrow, Home/End and Page keys, each its own command since `[keybindings]` gives a command one chord), so one override moves a list and the rails alike. A key that finishes a sequence (`g g`) reaches the module whole.
- The keys the Shell keeps win, so a module's default chord must not be one in either Client (`units.set_base` is `B`, since `b` is the Desktop's rail toggle). A test per Client checks every module's defaults against its global keys: `no_module_default_chord_is_a_key_the_shell_keeps` in the Desktop's `input.rs`, reading `lib_shell::keymap::keeps`, and `no_module_default_chord_is_a_global_key` in the TUI's `shell.rs` ([#764](https://github.com/IanTeda/Personal-Ledger/issues/764)).
- Every module adds its bindings to both tests' `module_defaults`, even while that Client draws none of its Views, leaving out the shared `back`: the Shell keeps `esc` and hands it to the View (`ImportPage::escape`).
- A View with a Search reads the Shell's Search chord for its legend (`new(…, search_chord)`, `set_keymap`) rather than binding `/`, which the Shell keeps; the Shell hands the page its Search through `start_search`.
- A Dialog's keys are the Dialog keymap in `lib-shell` (`lib_shell::dialog`), under flat names: `select` (`enter`), `back` (`esc`), `next_field`, `previous_field`, `select_next`, `select_previous`, `submit` (`ctrl+enter`), and `list_next` and `list_previous` (`j` and `k`), so a `[keybindings]` override reaches every module's Dialogs. The list step is a letter, so the host sends it as a select step only to a Dialog whose `HostedDialog::steps_list` says it is stepping a list (its Presenter's `steps_list`), and types it into any other. Not `confirm` and `cancel`, which `[keybindings]` binds to the TUI's `y` and `x`.
- The Client hands a module Dialog's Presenter every Dialog keymap command but `back`, and every typed character and `Backspace`, as one `lib_viper::dialog::DialogInput` (`DialogCommand::input` turns a command into one). What each means is the Presenter's call: `select` opens a closed select, picks from an open one, and otherwise confirms only a valid form; `submit` confirms a valid form from any field; leaving a select commits its open list's highlight. On `back` the host closes an open select first, otherwise sends the Dialog's Presenter its `Cancel` and closes it. A key the keymap doesn't bind types into the focused field.
- A `DialogFrame` names the keys its Dialog takes as `DialogHint`s, each the Dialog keymap commands it presses and what it does: the Presenter chooses the commands, and the bin reads their chords from `lib-shell`'s Dialog keymap through `DialogFrame::legend`, since a feature crate can't depend on `lib-shell` ([#915](https://github.com/IanTeda/Personal-Ledger/issues/915)).
- `DialogFrame::legend` gives each as a `Hint`, a chord and what it does, the one type every Presenter's status-line legend is a list of. `lib-shell` re-exports it as `lib_shell::Hint`, the Shell's own hints are `Hint`s too, and the bins name it only by that path ([#937](https://github.com/IanTeda/Personal-Ledger/issues/937)).
- A Presenter receives events (`Pay`), never keys (`p`).

### Tracing

- An Interactor logs each intent that changes the Ledger at `info`, done or refused; reads never log. Presenters, Views and Routers never log. Adapters and the writer log at `debug`, a failed write at `error` (ADR-0047, `/tracing`).
- A log field holds ids, kinds, counts and dates only, never an amount, a name or user-entered text.

### Localisation

- New UI text is a Message. A module's Messages live in its crate's `i18n/` and are read by its Presenter. A bin's own catalogue holds only text that belongs to one toolkit, such as the Desktop menu bar.
- A module's Message ids carry its domain as a prefix (`units-`), generated by `lib-locale-build` with `id_prefix`. A count in a plural is `{ $count }`, never a typed digit.
- A new Messages layer joins `locale::init_for_tests` as well as `init`; a missing one shows only in a test that reads its text ([#867](https://github.com/IanTeda/Personal-Ledger/issues/867)).
- A time or date in a view model is formatted by `lib-locale`, never by `chrono` in the Presenter ([#762](https://github.com/IanTeda/Personal-Ledger/issues/762) adds the minutes-only time ACTIVITY needs).

### Tests

- Entities, Interactors and Presenters have unit tests in their crate, run against the in-memory ports in `fake.rs`. These carry the behaviour coverage: a Presenter test is "send events, assert on the view model". `fake.rs`'s `RecordingRouter` records what the Presenter asked for, its Notices in one variant.
- Each module's ports have a contract suite in `contract.rs`, run against both `fake.rs` and the `lib-database` adapter: write, reload, and get the same rows. It sits behind the crate's `contract-tests` feature, which `lib-database`'s dev-dependencies enable, so `cargo test -p lib_<domain>` alone doesn't run it. Its writes go through the Interactors.
- A `PortsFactory` hook makes the other domains' rows a contract check names (`name_payee`), and the adapter's factory writes real rows for it, with whatever their foreign keys need. The contract leaves out what the schema doesn't keep: it sorts a Payee's aliases, since `payee_aliases` has no order column.
- A read port's contract check is generic over the row type (`lib_transactions::contract::ReadsFactory<Row>`): it hands over rows in the reading module's types and the factory stores them as the owning domain's, one factory for every adapter's checks.
- An adapter test pins each refusal the file makes, a foreign key or a composite key, so a rollback the in-memory ports don't model is caught.
- Each View still has `<view>_keyboard.rs` and `<view>_mouse.rs` in the bin's `tests/ui/`, but they check wiring and drawing (the key reaches the Presenter, the view model is drawn), not behaviour the Presenter tests already cover. They run on the `fake.rs` ports, which the Desktop's `shell::router::build` takes as parameters, so they never open SQLite.
- The UI tests add a test for each hand-off into a page (a Document's Payee link) and each writer from another module reaching it, asserting one commit for the input. The keyboard test opens `?` and checks it lists the status line's keys under the page's heading (`Harness::assert_cheat_sheet_matches_status`).
- `Stores::fake` takes one `FakeStores` with a field per domain, and the UI harness's `stub_fakes()` fills every one, so a test overrides only the store it changes. `Harness::with_failing_<domain>` runs on a `Failing<Domain>` store, and a test of a file cascade runs on a Ledger file (`Harness::with_storage`).
- A test helper writes a store directly (`add_payee_for_test`) rather than through a command that's gone, and a restart test the adapter's contract already covers is dropped.
- An end-to-end smoke test per module runs the real app on a temporary SQLite file: start, change a row, restart, see it, its readers seeing it from the adapter. Where the Desktop can't add a row yet, it reads back one that exists and the adapter's test covers the add. It runs when a parent ticket closes and before a merge to `main`.

## The Shell module

The Shell is the always-present frame around the current View (see GLOSSARY.md). `lib-shell` is a VIPER module like any other:

- Its Presenter holds the Shell's state (the active View, focus and the input mode, the status line, the Toasts, the open Dialog) and builds the Shell view model, owned values with the Shell's own status-line hints keyed as the global keymap binds them after `[keybindings]`. A status message answers one key: the bin hands the Presenter every key before routing it (`keypress`), which clears the message, so one the key sets survives; clicks and ticks leave it. The Notices the Routers queue reach it when `send` drains them, each stamped with the local time for the Toast history.
- Its keymap holds the global keys, and it owns the `Noun` and `ActiveView` the bins navigate by (`ActiveView` generic over the bin's Settings page, whose names are Messages).
- It owns the Clock (`lib_core::Clock`), which each bin builds on the local date (`wall_clock`) and a test on a day it sets; every View and module is built with it rather than a date. The Presenter never reads time: the bin keeps every timer and delivers each tick: as an input, `DayTick` from the 60s day poll and `PrefixExpired` from the `g` timeout, and the Toast timer's time to `tick_toasts(now)` while a Toast is up, which writes nothing so it's no input, and answers whether the stack changed, the only time the bin redraws. In a new calendar month it calls `month_turned(today)` on its Router.
- It holds the rail's badge counts (`RailCounts`: the Bills Needs Attention, the default Budget's over-budget Categories this month, the Documents Inbox), read through its own read port, `ShellRailCounts`, on the Clock's day. `lib-database`'s `StoredShellReads` answers it over the Bills, Budgets and Documents stores by calling each domain's own rule, so `lib-shell` depends on no feature crate. The Desktop's `send` refreshes them once each input is committed, and the Ledger opening and closing do; with no Ledger open it holds none ([#1026](https://github.com/IanTeda/Personal-Ledger/issues/1026)). `StoredShellReads` lives in `lib-database`, not the bin as the #1026 spec first said: the rail counts are answered from the stores the way `StoredDashboardReads` answers the Dashboard's ports, and the two share their Needs Attention and default Budget figures there. So `lib-database` is the one crate besides the bins that may depend on `lib-shell`, which `deny.toml` and `viper_dependencies.rs` both enforce ([#1021](https://github.com/IanTeda/Personal-Ledger/issues/1021)).
- Its Router, implemented in each bin's `shell/router.rs`, is the root Router: `main` calls `shell::router::build`, and it hosts whichever module's View is active and the one open Dialog.

## Out of scope

- **The Sync Server** has no Views, so it isn't split into VIPER modules. It reuses Interactors and ports where it needs domain rules.
- **The transition** from today's code to this layout is planned in tickets, not here, on the transition map ([#747](https://github.com/IanTeda/Personal-Ledger/issues/747)). [During the transition](#during-the-transition) lists only what stands in meanwhile.

## Persistence

Each module owns its repository ports, replacing the single `LedgerRepository` ([ADR-0039](../adr/0039-ledger-persists-through-one-store-wide-repository.md), amended).

Ports are synchronous, and `sqlx` is async ([ADR-0046](../adr/0046-per-domain-adapters-hold-rows-in-memory-behind-one-shared-writer.md)). Each `lib-database` adapter holds its domain's rows in memory, loaded when the Client starts, and serves reads from there. A write changes memory at once and goes to one writer thread shared by every domain, which runs it in one database transaction. A failed write is reported on a channel the bin listens on, so it can reload, redraw and show an error Toast. `lib-database`'s `adapter/units.rs` and `writer.rs` are the worked example.

### Writes and commits

- **The bin ends each input.** A port write is one row (`upsert_unit`, `set_base_unit`), so a port can't tell where an Interactor call ends. Every adapter adds its writes to one pending batch on the writer, and the Desktop's `input.rs` `send` calls `commit()` exactly once after each Presenter `handle`, whether or not it wrote (an empty commit is a no-op), which sends the batch to the writer as one transaction ([#757](https://github.com/IanTeda/Personal-Ledger/issues/757), [#823](https://github.com/IanTeda/Personal-Ledger/issues/823)).
- **No port or store commits for itself.** A call from one module to another runs inside the current `send`, so its writes join that commit: deleting a Price Source (N Unit moves and the delete) lands whole, and so does a call that writes through two domains' adapters, or a month turn's Bills.
- **Nothing inside an input sends.** A View method the Shell calls from inside its own input (`/`, the Import commit's selection reset) calls the page's `handle` directly rather than `send`, and `on_enter` must not commit (`Entered` writes nothing), or a hand-off commits twice ([#929](https://github.com/IanTeda/Personal-Ledger/issues/929)).
- **A write into another domain can ride the same commit.** The Categories Monthly budget writes a Category and the Personal spending Budget through two stores and needs no cross-domain module: the Desktop answers `lib-categories`' `CategoryBudget` port with `lib-budgets`' `SetMonthlyLimit`, checked on a copy of the Budgets before the Category is written so a refused budget writes nothing, and both writes join `send`'s one commit ([#950](https://github.com/IanTeda/Personal-Ledger/issues/950)). The Categories handover is the same ([#1010](https://github.com/IanTeda/Personal-Ledger/issues/1010)): a leaf's Splits, Bill Plans and Payee defaults pass to its first child through `lib-categories`' `CategoryHandover` port, answered over the Transactions, Bills and Payees stores in `send`'s one commit, and is named as the exception to "a workflow writing two domains is a cross-domain module" as `CategoryBudget` is. A cross-domain module is for a workflow that can't rely on that, because it commits for itself or runs outside `send`.
- **Loads go through the writer**, queued behind the writes, so a reload sees every earlier write and one runtime owns the pool.
- **The bin reloads.** Ports know nothing about the failure channel, so on a `WriteFailure` the bin calls `reload()` on each adapter the failure names. On the Desktop a task polls the channel and sends each failure as an input (`ShellInput::WriteFailed`), so its Error Toast shows and the window redraws without a keypress. A failure also names the domains of the writes dropped behind it, since their adapters are stale too. If the reload fails, the rows in memory stand and the Toast says they aren't saved.
- **A cascade reloads the rows it changed** ([#964](https://github.com/IanTeda/Personal-Ledger/issues/964), ADR-0046's 2026-11-02 amendment). Deleting a Transaction, Account, Payee, Inventory Item or Location cascades in the file to the Document Links naming it, rows the Documents adapter holds in memory. Each such delete's `Change` says so (`cascades_to`), `commit()` returns the domains its batch cascades into, and the Desktop's `send` reloads each after the commit (`Root::reload_cascades`), through the writer, so the reload sees the cascade. No delete writes Documents itself, so Payee, Transaction and Account delete keep the stores they have.

### Stores

- **The bins build on one bundle.** `Stores`, beside `Storage` in `lib-database`, holds each module's store, the commit and the failure channel, built by `Stores::from(storage)`; `Stores::fake` builds it on in-memory stores with a `CountingCommit`, so a test asserts one commit per input. The Desktop's `shell::router::build` takes it as its one storage argument; the TUI's Shell starts with no Ledger open and builds the Open Ledger on the `Stores` it makes on `:open` or `:new` (#871).
- **One adapter per domain.** A domain's adapter is one file, `adapter/<domain>.rs` ([ADR-0041](../adr/0041-lib-database-one-file-per-entity.md)), with its `<Domain>Store` trait and the in-memory fake's impl in `adapter/store.rs`. A domain whose rows live in several tables is still one adapter and one in-memory aggregate: `InventoryAdapter` holds Locations, Areas and Items as one `Inventory`.
- **The store traits serve the in-memory reads.** A `<Domain>Store` trait carries a read for each port the Desktop's in-memory fakes must answer, so `Stores::fake` serves them ([#925](https://github.com/IanTeda/Personal-Ledger/issues/925)).
- **Another domain's rows are read through a stored reads struct** ([#992](https://github.com/IanTeda/Personal-Ledger/issues/992)). One struct per reading module (`StoredPayeeReads`, as `StoredDocumentReads` and `StoredBudgetReads` before it) holds the `Shared<Domain>Store`s of the domains it reads and implements the module's read ports, borrowing each store for the one read. Each store answers about its own rows in its own terms only when the count must be made in the file (`TransactionsStore::payee_splits`, `BillsStore::payee_bill_plans`): the in-memory store counts what it holds and the adapter counts in the file. A count answerable from the store's rows is not a store method: the reading module's struct computes it with a free function over the rows ([#998](https://github.com/IanTeda/Personal-Ledger/issues/998)), so the Accounts, Bill Plans, Institutions and Locations naming a Unit are counted in `StoredUnitReads`. The struct maps the answers into the reading module's types (`PayeeUse`), so the same code stands over `Stores::fake`'s in-memory stores and the adapters, and the UI tests read what the app reads. A `<Domain>Store` trait answers only its own domain ([#994](https://github.com/IanTeda/Personal-Ledger/issues/994)): no method returns another module's read type, and the mappings are the adapter modules' functions the reads structs call over the stores' rows. The bin builds it from the stores it was given and copies nothing from the `Ledger` for it; no store method hands out a `fake` type. A test that needs a count to fail or miss wraps the counting store (the UI harness' `MiscountedTransactions`), not the reading module's. A stored reads struct composes its domains' store methods; it never runs SQL across two domains' tables in one job. An atomic write across domains is a cross-domain adapter's job (`BillPaymentsAdapter` and the like), not a read's. `StoredPayeeReads::payee_usage` counts the Splits and the Bill Plans one after the other, not from one snapshot ([#1008](https://github.com/IanTeda/Personal-Ledger/issues/1008)).
- **A shared store is one value.** When several holders share a store (`SharedPayeesStore`), the module's ports own the boxed store and implement the store trait as well as the module's ports, so one `Rc<RefCell<…>>` coerces to every holder (`bin-desktop/src/payees/ports.rs`). A port a shared store answers for another crate (`lib-bills`' `BillPayees`) is answered by the reading module's stored reads struct over the shared stores, never by a method on the store trait ([#994](https://github.com/IanTeda/Personal-Ledger/issues/994)).

### Schema

- **Edit the table's file in place.** While the schema is in concept, a domain's change edits its `create_<table>_table.sql`, then `mise run db` and a commit of `.sqlx`. Rewriting the file changes its checksum, so a Ledger file made earlier can't migrate, and one made before a new table that sorts ahead of its referrer has none of its rows: the domain's docs say so, and the ticket decides whether a fresh Ledger is the answer or the seed must also run on an empty table.
- **A port's rules match the schema's.** A foreign key the in-memory ports don't model fails only in SQLite, as a rollback the person can't explain. Each such constraint is an Interactor rule and a contract check ([#761](https://github.com/IanTeda/Personal-Ledger/issues/761): deleting a Unit other Units quote).
- **A key with no cascade backs a refusal.** The tables that name a persisted domain have foreign keys (`splits.payee_id`), with no cascade where the Interactor's refusal is the person-facing rule. That rule is decided before the Dialog moves: Account delete reads the Bill Plans that pay from the Account when its Dialog opens, keeps confirm off and says how many, and the Interactor refuses too ([#854](https://github.com/IanTeda/Personal-Ledger/issues/854)). A column that names a domain not yet persisted has no key until it is.
- **A child keeps its order and its parent.** Writing a parent rewrites its children with a `position` column, so order needs no child-level write, and `position` can't be unique mid-write (a swap). A child that must not cross parents has a composite key (`areas UNIQUE (id, location_id)`, `items` referencing the pair, [#866](https://github.com/IanTeda/Personal-Ledger/issues/866)).
- **A row may name a row defined in code.** A Unit's Price Source can be Manual entry (never stored) or a built-in with no override row (ADR-0043), so `units.price_source_id` has no foreign key. The schema says why, and the Interactor guards the link.

### Seeds

- A domain's stub seed is its one seed, with ids that match the stub Ledger, written through `lib-database`'s `seed::stub` and the first-launch seed in `open_storage`, ahead of every domain whose rows reference it ([database.md](database.md#the-first-launch-seeds-order)).
- A stub names another domain's rows only by id: the sample purchases a Document names belong in the booked domain's own stub (`lib-transactions`), not the Documents seed ([#961](https://github.com/IanTeda/Personal-Ledger/issues/961)).
- A row a foreign key needs on every Ledger (the Personal Institution, `SEED_PERSONAL`) is written by `open_storage` on a new file, apart from the stub seed (#849, #875).
- The first-launch seed runs only on a new file. A domain whose existing Ledgers need a row seeds its empty table on open instead (an empty default Budget, the default Document Types).

### Cross-domain modules

An operation that writes more than one domain belongs to a cross-domain module, one crate per user workflow rather than per operation. It holds one Interactor per operation and one atomic port, which `lib-database` implements with one database transaction, so all of an operation's changes are saved or none are. The expected cross-domain modules are:

| Module | Operations | Domains written | Atomic port |
|---|---|---|---|
| `lib-bill-payments` | Pay a Bill Schedule entry directly | Bills, Transactions | `BillPayments` |
| `lib-transaction-import` | Commit an Import, refusing a plan whose new Payee names or aliases are taken | Transactions, Payees | `TransactionImport` |
| `lib-account-deletion` | Delete an Account with its Transactions, Unmatching the Bill Schedule entries they paid, refusing one a Bill Plan pays from | Accounts, Transactions, Bills | `AccountDeletion` |
| `lib-unit-deletion` | Delete a Unit, moving the base and reassigning the Institutions that name it as their Usual unit | Units, Institutions | `UnitDeletion` |
| `lib-tag-removal` | Remove a Tag, untagging its Splits; merge one Tag into another, retagging the source's Splits | Tags, Transactions | `TagRemoval` |
| `lib-category-deletion` | Delete a leaf Category, moving its Splits and Bill Plans to the Uncategorised of its type, clearing it as a Payee's Default category and dropping its Budget Limits | Categories, Transactions, Bills, Payees, Budgets | `CategoryDeletion` |

Units and Price Sources are one domain, `lib-units`, since they only change together, so deleting a Price Source and setting the base Unit are single-domain Interactors. The transition tickets confirm the list against the code.

- **One domain written is not cross-domain.** Match writes only the Bill Schedule entry and reads Transactions through a read port, so it is a `lib-bills` Interactor. Filing a Document writes only Documents, reading Transactions and Payees to match, so `lib-documents` holds its Interactors; a workflow that later files and writes a Transaction or a Payee in one operation gets a module then.
- **The atomic port spans shared stores.** Each `<Module>Adapter` (`AccountDeletionAdapter`) holds the stores it writes, each shared with its own adapter (`SharedAccountsStore`, `SharedTransactionsStore`, `SharedBillsStore`), so its writes join the one pending batch, in the foreign keys' order (the Bill Schedule entries, then the Transactions, then the Account). `Stores` holds boxed stores, so the Desktop builds each atomic adapter from its shared stores; over the in-memory stores the adapter is its own fake and runs the contract too.
- **The atomic port serves the reads its Interactors check**, as owned values, rather than an Interactor taking a feature module's port beside it: `BillPayments` answers `bill_plan` and `entry` as well as `record_payment`, since an Interactor given both would borrow the one shared Bills store twice ([#779](https://github.com/IanTeda/Personal-Ledger/issues/779)). When the port's store and a read are one `RefCell`, the Interactor takes the read's rows as a value read before the call (`CommitImport`'s Payees). Other domains' reads are their adapters' (the Accounts and Categories adapters answer `ImportAccounts` and `ImportCategories`).
- **A count only for wording takes no store.** A port that needs a count or a code only for a Dialog's wording (the Budgets count, the Unit code) doesn't take another store: the Presenter is lent a read port once, at open (`DeletionReads`), and the bin answers it.
- **The owning domain's rules are reused, not copied.** `lib-unit-deletion`'s Interactor runs `lib-units`' delete rules through `DeleteUnit::check`.
- **A cross-domain module may own the Dialog whose confirm calls its Interactor.** `lib-bill-payments` holds the Pay Dialog's Presenter, both panels, because Pay it directly is its Interactor; the Match panel calls `lib-bills`' Match Interactor, which the module may depend on. It has its own `i18n/`, and each bin gives it a folder like a domain's (`bill_payments/view/dialog/`, `bill_payments/router.rs`). The Bills Router opens it through that folder's entry point (`bill_payments::router::open_pay(cx, entry)`), as it would another module's View.

## Migrating a domain

A domain's tickets link here rather than restate these steps. The worked examples are Payees for a Settings page ([Payees (development)](payees.md), [#829](https://github.com/IanTeda/Personal-Ledger/issues/829)), the Transactions View for a destination ([#923](https://github.com/IanTeda/Personal-Ledger/issues/923)), the Import for a cross-domain module with a View ([#943](https://github.com/IanTeda/Personal-Ledger/issues/943)), Budgets for a View with Dialogs ([#950](https://github.com/IanTeda/Personal-Ledger/issues/950)) and Documents for a View that reads other domains and the disk ([#960](https://github.com/IanTeda/Personal-Ledger/issues/960)). A slice is usually a straight chain: a prefactor, persistence, the Interactors, the page, the Dialogs and the docs.

1. **Lay out the crate** as [Layout](#layout) shows, after `lib-units`, and grep the bins as well as the crates before calling a function dead.
2. **Move seeds first.** Move a stub row that belongs to another domain into that domain's stub, and a seed check or seed count that needs another domain's seed into `lib-ledger`'s seed (`STUB_TRANSACTION_COUNTS`).
3. **Replace borrowed rows with read ports** by [Read ports](#read-ports), answered in `lib-database`, else by an `impl` for `Ledger` the bin copies (a transition row), filled in every in-memory store.
4. **Keep `lib-ledger`'s callers running.** While it applies a domain rule to its own rows, write the rule over rows and a read port (`delete_payee(&mut Vec<Payee>, &dyn PayeeUsage, id)`), not the repository.
5. **Move, don't copy.** A rule or query engine that leaves `lib-ledger` is deleted there and its rule tests move with it.
6. **Add the crate to `FEATURE_CRATES`** in `lib-ledger/tests/viper_dependencies.rs`, so it keeps the dependency rule from its first commit; `deny.toml` needs no edit.
7. **Edit the schema** by [Schema](#schema): the table's file, the foreign keys the referencing tables gain, and the refusal each key without a cascade backs.
8. **Grep what writes the old schema:** every `::seed(` and adapter test factory that writes a referencing table, the older per-entity `sqlx::test`s, and `bin-sync-server/tests`.
9. **Write the adapter** after `adapter/units.rs`, with its `<Domain>Store` trait and fake, and its field in `Stores`, `Stores::fake` and `FakeStores::stub`.
10. **Wire write failures:** a `Failing<Domain>` store, `Harness::with_failing_<domain>`, an arm in `roll_back` (`shell/router/write_failures.rs`) and two `desktop-toast-write-failed-<domain>` Messages.
11. **Write the contract suite** by [Tests](#tests), with a `PortsFactory` hook for the other domains' rows it names.
12. **Seed it** through `seed::stub` and `open_storage`, ahead of the domains that reference it ([Seeds](#seeds)).
13. **Keep a read copy** for the Views that aren't modules (`LedgerStore::refresh_categories`), refreshed after each of the module's writes, and say which rows stay.
14. **Announce once nothing reads the copy:** the store announces the domain after each write and a rollback reloads and announces (`LedgerStore::announce_documents`), and the copy goes.
15. **Move the module's readers to the store** in the same slice: the adapter counts in the file and the in-memory store recounts from its rows. Say in the findings when a kept `Ledger` reader goes.
16. **Split a second View out first** when the destination hides one (the Import inside Transactions), and check which keys now reach it.
17. **Build beside the old copy** for a cross-domain module with a View: crate, adapter and Presenter first, then the wiring ticket deletes the bin's copy.
18. **Move the page** into `bin-desktop/src/<domain>/` (`ports.rs`, `router.rs`, `view/`), delete its old page code and `OpenDialog` arm, and wire it by [View](#view) and [Router](#router).
19. **Make `router.rs` the entry point**, point every other module's Router at it, and check each gesture a doc names has a caller.
20. **Move the Dialogs** onto the Entity host, and take the domain out of the legacy-Dialogs row. Build the Dialogs the page has, and leave the rest to the View's ticket.
21. **Answer a reassign-shaped delete** with a cross-domain module (`lib-category-deletion`, `lib-tag-removal`): the domain crate keeps only the Router's Dialog variant, and the page's Router opens the module's Dialog through its entry point ([#1003](https://github.com/IanTeda/Personal-Ledger/issues/1003), [#1005](https://github.com/IanTeda/Personal-Ledger/issues/1005)).
22. **Register the keys:** both clash tests' `module_defaults`, each binding's `Palette` and description, `module_entries` in `navigation/command.rs`, `module_command` through `event_for`, and drop the old `COMMANDS` rows.
23. **Compare the view model with the old View** before deleting it. A change the person sees (a new Toast, new wording) is new behaviour: name it in the findings and gate it for review.
24. **Write the UI tests** by [Tests](#tests): keyboard and mouse for each View on the Entity host, the hand-offs, the one-commit writers, the cheat sheet and the smoke test.
25. **Name the transition rows.** Each ticket's acceptance criteria name the row it adds, narrows or removes; the last ticket checks the domain's rows against the code.
26. **Close with the docs.** Each ticket logs findings on the parent; the last brings the domain's pages into line (`/end-user-docs`) and turns each finding into a rule here, an ADR or a ticket.
27. **Expect review gates:** tickets that touch `migrations/`, `.sqlx/`, `lib-database` or `lib-ledger` gate for review (`docs/agents/ticket-labels.md`).

## During the transition

Until every domain is a module and `lib-shell` is whole, these stand in, each removed when its condition is met.

| Stand-in | Why | Removed when |
|---|---|---|
| The `Ledger` read copy of the Units, the base Unit and the Price Sources (`LedgerStore::refresh_units`) | The Accounts and Institutions ports (`accounts/ports.rs`' `copy_from_ledger`, `LedgerStore::refresh_institutions`), the Inventory page, Settings › Ledger and the Account delete Dialog still read Units from `Ledger` (`units()`, `base_unit_row()`, `unit_code`); the Bills ports' in-memory Units (`bills::ports::in_memory_units`) are built from it for tests | The last of them reads Units through its own domain's read port |
| The first-launch seed: `open_storage` seeds a new Ledger file with each domain's stub ([database.md](database.md#the-first-launch-seeds-order)) | The stubs name each other's rows by foreign key | Every domain persists; a new database then starts through the new-Ledger workflow ([#171](https://github.com/IanTeda/Personal-Ledger/issues/171)) |
| The `Ledger` read copy of the Institutions (`LedgerStore::reload_institutions`) | The Accounts page and its Institution select still read Institutions from `Ledger` | The last of them reads Institutions through its own domain's read port |
| The `Ledger` read copy of the Accounts (`LedgerStore::reload_accounts`, [#853](https://github.com/IanTeda/Personal-Ledger/issues/853)) | No View reads it now: only tests read `ledger.accounts()` (the in-memory fixtures in `accounts/ports.rs`, `ledger_store.rs`' tests), and its refresh carries `Domain::Accounts`' announce | The last of them reads Accounts through its own domain's read port |
| The `Ledger` read copy of the Categories, in the store's tree order (`LedgerStore::reload_categories`, [#858](https://github.com/IanTeda/Personal-Ledger/issues/858)) | `categories/ports.rs` and `budgets/ports.rs` build their tests' in-memory stores from it, and the Categories page refreshes it after each write | The last of them reads Categories through its own domain's read port |
| The `Ledger` read copy of the Transactions, newest first (`Ledger::refresh_transactions`, kept fresh by `LedgerStore::refresh_transactions` after Bill Payments, Account delete, the Import and the Categories handover) | No command rewrites Splits on it since Category delete moved to `lib-category-deletion` ([#1005](https://github.com/IanTeda/Personal-Ledger/issues/1005)), and the Dashboard no longer reads it ([#1027](https://github.com/IanTeda/Personal-Ledger/issues/1027) re-checked); `ledger_store.rs`' tests and the Views not yet modules read `ledger.transactions()` | The last of them reads Transactions through its own domain's read port |
| The `Ledger` read copy of the Bill Plans and the Bill Schedule (`LedgerStore::refresh_bills`) | No command re-points Bill Plans on it since Category delete moved to `lib-category-deletion` ([#1005](https://github.com/IanTeda/Personal-Ledger/issues/1005)); neither the Dashboard ([#1024](https://github.com/IanTeda/Personal-Ledger/issues/1024)) nor the Bills rail badge ([#1026](https://github.com/IanTeda/Personal-Ledger/issues/1026)) reads it ([#1027](https://github.com/IanTeda/Personal-Ledger/issues/1027) re-checked); `ledger_store.rs`' tests read it | The last of them reads Bills through its own domain's read port |
| The `Ledger` read copy of the Budgets (`LedgerStore::reload_budgets`, `refresh_budgets`, [#952](https://github.com/IanTeda/Personal-Ledger/issues/952)) | No command drops Limits on it since Category delete moved to `lib-category-deletion` ([#1005](https://github.com/IanTeda/Personal-Ledger/issues/1005)); neither the Dashboard ([#1024](https://github.com/IanTeda/Personal-Ledger/issues/1024)) nor the Budgets rail badge ([#1026](https://github.com/IanTeda/Personal-Ledger/issues/1026)) reads it ([#1027](https://github.com/IanTeda/Personal-Ledger/issues/1027) re-checked); `budgets/ports.rs`' and `ledger_store.rs`' tests read it | The last of them reads Budgets through its own domain's read port |
| `bill_reads.rs` keeps the stub Bills seed's mappings, and `Ledger::bill_splits` the tests that check the stub's Bills seed | The stub Bills seed maps its own rows | Both go with the stub's own Bills seed |
| `lib-ledger`'s `budget_reads.rs` keeps the free `budget_accounts` and `budget_categories` mappings for the stub's own Budgets seed and the Category-tree callers; the Budgets page, the Dashboard and the rail badge read `StoredBudgetReads` (the badge through `StoredShellReads`) ([#993](https://github.com/IanTeda/Personal-Ledger/issues/993)) | The stub Ledger's Budgets seed maps its own rows | The mappings go with the stub's own Budgets seed |
| `lib-ledger`'s `document_reads.rs` keeps the mappings for the stub's own Documents seed; the Documents page reads `StoredDocumentReads` ([#963](https://github.com/IanTeda/Personal-Ledger/issues/963)) | The stub Ledger's Documents seed maps its own rows | The mappings go with the stub's own Documents seed |
| Settings keeps its clicks and keys in its Entities (ADR-0037); the root Router sends each Toast as `ShellInput::Notice` | Settings isn't a module | It migrates, with `LedgerStore` ([#816](https://github.com/IanTeda/Personal-Ledger/issues/816)) |
| `Reveal` stays the Shell's routing vocabulary for the destinations not yet modules; a module's page turns what reaches it into Presenter events | Those destinations have no entry point | The last destination is a module |
| `RootRouter` hands each global command back for `Root` to act on, `ShellState` keeps the palette, Dialog slots and rails, and the Views other than Bills, Budgets and Documents keep the day they were built on | The palette and the non-module Views are `Root`'s own state | Their own slices move them out of `Root`, and the Router acts on each command itself |
| The palette, Search, filter and file explorer stay in the Desktop behind `InputMode` (step 2 of `input.rs`'s key route), with their keys hardcoded in `shell/router/key_dispatch.rs` | Moving them into `lib-shell` is out of #820's scope | Their own slice moves them into `lib-shell` |
| Tracing and Toast History stay on the legacy `OpenDialog` arm (`shell/view/dialog_host.rs`), their typing, `Tab`, `Enter` and `Esc` hardcoded at step 1 of `input.rs`'s key route | Their Views aren't modules, so they have no Presenter to hold | Each moves to the Dialog host with its slice; the legacy arm goes with the last |
| The Toast History Dialog has no Presenter: the Shell draws it from `lib-shell`'s Toast stack and it takes no key but `Esc` | A Toast History Presenter is out of #820's scope | Its own slice gives it one |
| The Desktop's Notice queue is per thread (`notices.rs`), not `input.rs`'s, and the TUI's Router turns a status into an Info Toast | The legacy Dialog host has no gpui context; the TUI has no `lib-shell` Presenter | The last legacy Dialog moves to the Entity host; the TUI's redesign ([#805](https://github.com/IanTeda/Personal-Ledger/issues/805)) |
| The TUI's Units ports hold the commit and commit after each `handle` (`bin-tui/src/units/ports.rs`); it builds on `Stores` but has no `send` | The TUI's redesign and its own `input.rs` are their own ticket ([#805](https://github.com/IanTeda/Personal-Ledger/issues/805)) | The TUI's `input.rs` commits after each input |
| The TUI's keys reach the Units keymap through `shell.rs`, and its Units View maps the arrows, Home/End and Page keys to `Move` itself | The TUI has no `input.rs` yet | The TUI's redesign gives it one ([#805](https://github.com/IanTeda/Personal-Ledger/issues/805)) |
| `LedgerRepository` and `LedgerChange`, persisting nothing since Units left and recording nothing since slice 10's close-out ([#1006](https://github.com/IanTeda/Personal-Ledger/issues/1006)); `LedgerChange` keeps one variant so the repository stands | `Ledger` and `LedgerStore` still stand | Every View is a module ([#816](https://github.com/IanTeda/Personal-Ledger/issues/816)), with `Ledger` and `LedgerStore` |

The slices left are on the transition map ([#747](https://github.com/IanTeda/Personal-Ledger/issues/747)): the Dashboard and Help ([#814](https://github.com/IanTeda/Personal-Ledger/issues/814)), the Settings frame ([#815](https://github.com/IanTeda/Personal-Ledger/issues/815)), then removing `Ledger` and its stand-ins ([#816](https://github.com/IanTeda/Personal-Ledger/issues/816)), and the first-launch seed after the new-Ledger workflow ([#171](https://github.com/IanTeda/Personal-Ledger/issues/171)). Each slice removes the rows above whose condition it meets.

The TUI waits for its redesign ([#805](https://github.com/IanTeda/Personal-Ledger/issues/805)), after the Desktop concept works. Until then it only keeps building, with the minimal edits a change to a shared trait forces, and nothing new moves on it.
