# Research: GUI architecture patterns and where the Clients sit

**Question.** What are the common, industry-standard code architectures for desktop GUI apps, which one is each Personal Ledger Client closest to, what approach does the Zed team use with GPUI, how does it differ from the Elm Architecture, what does it buy, and what Rust-specific considerations follow for the Desktop?

This note builds on [`gpui-desktop-architecture.md`](gpui-desktop-architecture.md), which verified GPUI 0.2.2, Zed and `gpui-component` claims against primary sources; Zed and GPUI facts below are cited through that note's sections rather than re-cited. Repo claims are read from `concept` at `50e83be`. The descriptions of MVC, MVVM and the Elm Architecture in section 1 are a summary of widely documented patterns, not primary-source research; section 7 lists what wasn't verified. No code was changed.

The three reference diagrams are [`gui-architecture-mvc.html`](../diagrams/gui-architecture-mvc.html), [`gui-architecture-mvvm.html`](../diagrams/gui-architecture-mvvm.html) and [`gui-architecture-elm.html`](../diagrams/gui-architecture-elm.html). For the Ratatui-specific patterns (TEA, Component, Flux) see [`tui-application-architecture.md`](tui-application-architecture.md).

## 1. Three common GUI architectures

### 1.1 Model–View–Controller (MVC)

- **Shape.** The Controller turns raw input into Model updates; the Model holds state and domain rules and knows nothing of the UI; the View observes the Model (Observer pattern), reads its state and redraws. MVP is the common variant: a passive View, with a Presenter pushing data into it.
- **Where used.** Smalltalk-80 (origin), Apple Cocoa / AppKit, Java Swing; Qt Model/View merges view and controller into delegates; WinForms and GTK apps often use MVP.
- **Trade-offs.** Simple and well known, and the Model is testable without a UI. Controllers tend to grow large ("Massive View Controller"), View and Controller are tightly paired in practice, and observer webs get hard to trace as an app grows.

### 1.2 Model–View–ViewModel (MVVM)

- **Shape.** The View binds declaratively to ViewModel properties and commands; a framework binding engine copies values both ways and refreshes on change events (`PropertyChanged`); the ViewModel shapes Model data for one screen and holds UI state such as selection and validation, and never references the View.
- **Where used.** WPF, WinUI, .NET MAUI, Avalonia (origin: Microsoft, 2005); Android Jetpack `ViewModel`; SwiftUI with `ObservableObject`; Slint and Qt Quick property bindings.
- **Trade-offs.** Screen logic is unit-testable without rendering, and View and logic can be worked on apart. It needs a binding engine (magic strings or code generation), bindings can be hard to debug, and it is overkill for simple screens.

### 1.3 The Elm Architecture (unidirectional data flow)

- **Shape.** Data moves one way: Msg → `update` → State → `view` → User → Msg. `update` is the only place state changes; `view` is a pure function of state; async work is returned from `update` as a Command whose result comes back as another Msg.
- **Where used.** Elm (origin), Redux / Flux, Bubble Tea (Go), The Composable Architecture (Swift); in Rust, iced and relm4, and Ratatui apps by convention.
- **Trade-offs.** Predictable and easy to test, and replay, undo and time-travel debugging come cheap. Every interaction needs a Msg variant, one big state tree needs nesting (child Msg wrapped by the parent) to scale, and re-rendering everything relies on diffing or cheap redraws.

## 2. Which pattern each Client is closest to

### 2.1 Desktop: MVC-shaped, with an Elm-style write path

| Role | In the Desktop | Pattern |
|---|---|---|
| Model | `LedgerStore` holds the only `Ledger` and emits `LedgerChanged { what }` naming the domains a write touched (`crates/bins/bin-desktop/src/ledger_store.rs` L11–51) | MVC Model, observed by Views |
| Controller | `Shell` takes raw keys in one `on_key_down` and `navigation::key_router::route_key` dispatches to the active View (ADR-0032; research note §9.2) | MVC, one front controller |
| View | Each destination is an Entity that implements `Render`, owns its own View state and repaints through `notify_on` (ADR-0032; `ledger_store.rs` L53–65) | MVC View, with its own controller logic |
| Writes | The Dialog host's `Apply` runs a typed `LedgerCommand` through `LedgerStore::apply`; a refusal emits nothing, a landed command announces its domains (`ledger_store.rs` L30–45, ADR-0037) | Elm / Flux: typed command, one store, announce |

It is not MVVM: there is no binding engine, and ADR-0032 keeps a View's state "next to the rendering" in `view/<domain>/`. The read-only per-domain stores for Categories, Tags and Inventory (ADR-0038's halfway state) are the nearest thing to a ViewModel, and they are being removed. It is not Elm either: View state is spread across Entities rather than one tree, commands mutate the `Ledger` in place, Views are retained Entities rather than pure functions, and there is no Command/Task loop for I/O yet (`cx.spawn` is used only for timers; Ledger writes aren't persisted until #636).

Finishing ADR-0038 and wiring the `LedgerRepository` (#636) move it further towards the Elm / Flux side for data; keeping the hand-rolled key router (ADR-0032) keeps the input side a classic MVC front controller.

### 2.2 TUI: the Elm Architecture, component variant

| Elm role | In the TUI |
|---|---|
| Msg | `Action`, one enum (`crates/bins/bin-tui/src/view/mod.rs` L47) |
| Event → Msg | `Shell::map_event` translates a crossterm event to an `Action`: open popups first, then global keys and chords, then the active View's `handle_key` (`crates/bins/bin-tui/src/shell.rs` L291) |
| `update` | `Shell::update` → `apply(action)`, which matches on the `Action` and forwards unhandled ones to `view.update(&action)` (`shell.rs` L1107–1115) |
| `view` | `Shell::draw` redraws the whole frame from state after every `Action` (`shell.rs` L1786) |
| Command | `View::init` receives an `action_tx`, and background work reports back as an `Action` on `action_rx` (`view/mod.rs` L605–609; `shell.rs` L74–81) |
| Runtime | `run()` waits on `tokio::select!` for a terminal event or a background `Action`, then `update`, quit check, `draw` (`shell.rs` L231–260) |

Departures from textbook Elm: `update` mutates in place (`&mut self`), as iced and relm4 also do in Rust; state is split between `Shell` (popups, Toasts, command history) and a `Box<dyn View>` with its own `handle_key`/`update`/`view`, which is nested TEA, Ratatui's "Component" pattern; and some popups commit to their store inside key mapping (`popup.commit(store)`, `shell.rs` L1036 and L1094), so not every change passes through `update`.

The two Clients therefore share pure domain crates and `lib-ledger` but differ in presentation: the TUI is message-driven, the Desktop is an observed Entity graph. The TUI's `Action` enum is already the vocabulary of user intents that a Desktop command palette or gpui actions would need, and the Desktop's key router has no such value.

## 3. The approach Zed uses

GPUI is a retained graph of state Entities that observe each other, with each view's element tree rebuilt every frame. Its nearest named pattern is MVC with observing views, Zed's own variant.

- **One owner.** `App` owns every Entity; an `Entity<T>` is an inert, reference-counted handle, and `update` leases the state out of `App` to satisfy the borrow checker (research note §2).
- **State is distributed.** Models such as `Project` and `Terminal` are Entities that don't render; views such as `TerminalView` are Entities that implement `Render` and hold a strong handle to their model (§1, §3).
- **Two data channels.** `cx.notify()` + `cx.observe()` for "I changed, re-read me", with no payload; `cx.emit(Event)` + `cx.subscribe()` for typed events a parent must act on. Data flows down through handles and reads; intent flows up through events (§3).
- **Input is actions.** Action structs are bound in a keymap file to a key context and dispatched along the focus path (§5).
- **Async is tasks.** `cx.spawn` and `background_executor().spawn` return a `Task` that cancels when dropped (§4).
- **Configuration is Globals.** Settings and theme live in Globals (§6).
- **Crates split by layer.** Model crates such as `project` and `terminal` don't depend on the UI crates; view crates such as `project_panel` and `terminal_view` do; there are about 250 crates (§7).

## 4. How the Zed approach differs from Elm

| | Elm / TEA (the TUI) | Zed / GPUI (the Desktop) |
|---|---|---|
| State | One tree held by the runtime | Many Entities, each owning its part |
| Change | `update(state, msg)`: pure in Elm, `&mut` in Rust | Direct method calls: `entity.update(cx, \|e, cx\| e.do_thing(cx))` |
| Messages | One `Msg` enum per app or component, wrapped by parents | No global Msg: actions for input, typed events per emitter, otherwise method calls |
| Propagation | Implicit: the whole view re-runs | Explicit: `notify` / `emit`, and observers re-read |
| Rendering | `view(state)` rebuilds everything | `render(&mut self)` per view, rebuilt each frame from that Entity's state |
| Effects | Returned from `update` as Commands | Spawn a task anywhere; write back through a `WeakEntity` |
| Input routing | `update` matches; child messages wrapped by hand | The focus tree and key contexts choose the handler |
| Ordering | Messages processed one at a time | Effects queued and flushed at the end of the outermost update: run-to-completion (§2) |

What they share: both rebuild the element tree from state rather than binding widgets to it, and both queue effects for run-to-completion semantics. The real difference is where state lives and how a change is announced: one store plus messages in Elm, many owners plus observation in GPUI.

## 5. What the Zed approach buys

- **Scale without a giant Msg enum.** A deeply nested component, such as a text input in a dialog in a pane, owns its state and events; Elm needs parent-wraps-child Msg plumbing that grows with every level of nesting.
- **Encapsulation and reuse.** An `Editor` or `gpui-component`'s `InputState` (an Entity that emits `InputEvent`, §7) works anywhere, because it knows nothing of its parent and only emits.
- **Targeted updates.** Only Entities that `notify` cause their observers to repaint, rather than the whole UI on every message.
- **Actions as a package.** A rebindable keymap, the same key meaning different things in different contexts, a command palette that can list every action, and one handler shared by mouse and keyboard (§5).
- **Lifetime-bound async.** A task stored on an Entity is cancelled with it, so a closed view's load simply stops (§4).
- **Borrow-checker friendly.** Because `App` owns everything and handles are inert, there is no web of `Rc<RefCell<…>>` and no runtime borrow panics from it (§2).
- **Deterministic headless tests.** `TestAppContext` and `VisualTestContext` simulate keystrokes and clicks, and `run_until_parked` drains the executors (§8).

The costs are what Elm buys: data flow is harder to trace through observer webs, there is no free replay, undo or time-travel, an Entity can hold a stale copy of data another changed (ADR-0032's Consequences name this), and cycles and Subscriptions need deliberate management.

## 6. Rust considerations for the Desktop

1. **Strong down, weak up.** A parent holds `Entity<Child>`; back-references and spawned tasks hold `WeakEntity`. A strong cycle is a leak (§2). ADR-0032's "events, never `WeakEntity<Shell>`" is stricter still, and worth keeping.
2. **No reentrant updates.** Updating an Entity from inside its own update isn't supported (§2) and fails at run time. Emit an event, or defer the work, rather than calling back into the same Entity.
3. **Borrows don't cross context calls.** A `&T` from `entity.read(cx)` can't live while you mutate through `cx`; copy out the small value you need, or do the whole job inside one `update` closure.
4. **Own Subscriptions and Tasks.** Store them in a field, or `.detach()` them deliberately; dropping one silently unsubscribes or cancels (§3, §4).
5. **Mind `Send` boundaries.** Entities and `AsyncApp` aren't `Send`; background futures must be. For #636: snapshot plain data on the main thread, run the SQLx call on the background executor, hop back with `cx.spawn` and write through the `WeakEntity`. Treat a failed upgrade or an `Err` from `AsyncApp` as normal, since the window or Entity may be gone (§1, §4). Keeping `lib-ledger` plain, `Send` and I/O-free is what makes this easy.
6. **Keep `render` cheap.** It runs every frame. Cache derived data (filtered rows, totals) on the View Entity and recompute it only when a `LedgerChanged` names its domain, as `notify_on` already filters. Read through `LedgerStore` rather than cloning the `Ledger` into Views, which avoids stale copies.
7. **One typed event enum per emitter.** `LedgerChanged { what: BTreeSet<Domain> }` already does this; prefer it over one app-wide event type or strings.
8. **Closed enum over open trait, deliberately.** Zed uses `dyn Item` and `AnyView` because extensions and many crates add item types. The Desktop has a fixed set of destinations, so the exhaustive `ActiveViewEntity` match (ADR-0016, ADR-0037) is the better Rust fit: a new View fails to compile in exactly one place.
9. **Crate splits pay in compile time.** Any test in `bin-desktop` links all of `gpui` (research note §9.2). Moving `LedgerStore` and the per-domain stores into a gpui-dependent model crate with no views, Zed's `project`/`project_panel` split, would cut rebuilds and let model tests skip the view code.
10. **Keyboard is the main divergence from Zed.** The hand-rolled `key_router` gives a directly tested modal grammar; GPUI actions with key contexts give rebinding, a command palette and per-context keys for free. A middle path is for the router to emit typed intent values, much like the TUI's `Action`, and dispatch them as gpui actions, giving both Clients one vocabulary of intents.

In short, the Desktop already follows Zed's approach for ownership, events, ports and tests. The open choices are moving I/O onto background tasks (#636), whether to split the model layer into its own crate, and whether to adopt gpui actions.

## 7. Unverified

- **Section 1's pattern summaries.** MVC, MVVM and Elm descriptions, origins and framework lists are a summary of widely documented material, not checked against primary sources in this note.
- **Reentrancy failure mode.** That a reentrant update of the same Entity is unsupported comes from Sobo's post (research note §2); the exact run-time behaviour (panic text) and `cx.defer`'s signature weren't checked against `gpui 0.2.2`.
- **View caching.** Whether `gpui 0.2.2` skips re-rendering un-notified views, beyond observers repainting on `notify`, wasn't checked; section 5's "targeted updates" claims only the notify/observe behaviour verified in research note §3.
- **TUI state.** The TUI reading is from `shell.rs` and `view/mod.rs` at `50e83be`; most TUI Views render stub data, so the TUI's background `Action` path isn't yet exercised by real I/O.
