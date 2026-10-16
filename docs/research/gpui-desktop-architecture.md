# Research: The ideal architecture for a GPUI desktop app

**Question.** What does an "ideal" codebase architecture look like for a Rust desktop app built on GPUI (Zed's UI framework, crate `gpui`) plus `gpui-component`: how should it be layered, how should ownership and data flow work, where do async I/O and persistence go, how do actions, keymaps and focus fit, where do settings and theme live, how should crates be split, and where are the testing seams? And how does Personal Ledger's Desktop Client (`crates/bins/bin-desktop`) compare?

Research was done against primary sources only: GPUI's own source and docs at the `gpui-v0.2.2` tag (commit `69e2130`, the version Personal Ledger's `Cargo.lock` pins), Zed's workspace at `main` commit `28d0a35` (read through `gh api` and `raw.githubusercontent.com`), `gpui-component` at the `v0.5.1` tag (commit `0f0ab35`, also the pinned version), and Nathan Sobo's Zed blog post "Ownership and data flow in GPUI". The GPUI and `gpui-component` files cited were checked byte-for-byte (`git hash-object` against the tag's blob SHA) against the crates.io sources in `~/.cargo/registry`, so the line numbers below are the ones the Desktop actually compiles. Sections 1–8 are verified facts with the design reason; where the reason is an inference rather than a stated one, it says so. Section 9 is the comparison with this repo; section 10 is what wasn't verified. The companion diagram is [`docs/diagrams/base-architect-diagram.html`](../diagrams/base-architect-diagram.html).

Short URL prefixes used below:

- **GPUI** = `https://github.com/zed-industries/zed/blob/gpui-v0.2.2/crates/gpui/`
- **Zed** = `https://github.com/zed-industries/zed/blob/28d0a3547f16b9609b301b48ab363bfca649c864/crates/`
- **GC** = `https://github.com/longbridge/gpui-component/blob/v0.5.1/crates/ui/src/`

## 1. GPUI's three registers and its core types

- **Three registers.** GPUI's README says it offers state management through `Entity`s, high-level declarative UI through views, and low-level imperative UI through `Element`s, each with its own context type (GPUI `README.md` "The Big Picture", L42–52).
- **Views are Entities that render.** "Views are `Entity`'s which `impl Render` and drawn to the screen" (GPUI `src/element.rs` L129–134). `RenderOnce` is for "reusable `components` out of plain data", consumed by value when rendered (GPUI `src/element.rs` L142–147).
- **Contexts.** `App` owns all entities' data; `Context<T>` is `App` plus entity-level services such as `notify` and `emit`, and derefs to `App`; `AsyncApp`/`AsyncWindowContext` can be held across `await`, and calls through them "become fallible, because the context may outlive the window or even the app itself"; `TestAppContext` is the test equivalent (GPUI `docs/contexts.md` L1–21). `Window` is not a context: it owns the root view and is passed alongside `&mut App` (GPUI `docs/contexts.md` L27–29).

**Decision and reason.** Model state lives in plain Entities, presentation in Entities that also implement `Render`, and stateless widgets as `RenderOnce` components. The split exists because the three registers have different lifetimes: an Entity persists across frames and can be observed, while a `RenderOnce` component is rebuilt each frame from data and so carries no state worth observing.

## 2. Ownership: the App owns everything, handles are inert

- **Single owner.** "Every model or view in the application is actually owned by a single top-level object called the `App`" (GPUI `src/_ownership_and_data_flow.rs` L1). An `Entity<T>` is "an inert identifier plus a compile-time type tag" with `Rc`-like reference counting, and only gives access to state when an `App` is available (L16–18).
- **Leasing and run-to-completion.** Sobo's post explains that `update` temporarily leases the entity's state out of the `App` to satisfy the borrow checker, and that `notify` and `emit` push effects onto a queue flushed at the end of the outermost update, giving run-to-completion semantics without reentrancy bugs; reentrant updates of the same entity aren't supported ([Ownership and data flow in GPUI](https://zed.dev/blog/gpui-ownership), Nathan Sobo, 25 January 2024, updated 12 December 2025).
- **Strong and weak handles.** `Entity<T>` is the strong handle (GPUI `src/app/entity_map.rs` L377); `WeakEntity<T>` the weak one (L654); `Context::weak_entity()` hands out a weak handle to the current entity (GPUI `src/app/context.rs` L57).

**Decision and reason.** Treat `Entity<T>` like `Rc<RefCell<T>>` minus the `RefCell`: parents hold strong handles to children, and anything pointing back up (a child to its parent, a long-running task to its owner) holds a `WeakEntity`. Because handles are reference counted, a strong cycle is a leak; because updates are leased, a strong back-reference also invites reentrant updates. Zed's `Workspace` keeps a `weak_self: WeakEntity<Self>` and `WeakEntity<Pane>` maps for exactly these back-pointers (Zed `workspace/src/workspace.rs` L1580–1600).

## 3. Data flow: notify/observe versus emit/subscribe

- **notify/observe.** `cx.notify()` tells GPUI "this entity has changed and observers of it should be notified" (GPUI `src/app/context.rs` L228–229); `cx.observe(&entity, ..)` runs a callback on each notify (L60–63). It carries no payload: the observer re-reads the observed entity's state.
- **emit/subscribe.** An entity that implements `EventEmitter<E>` can `cx.emit(event)` (GPUI `src/app/context.rs` L722–723), and others `cx.subscribe(&entity, ..)` to receive the typed event (L98). The ownership doc introduces this as the way to "emit typed events" in addition to change notification (GPUI `src/_ownership_and_data_flow.rs` L90).
- **Subscriptions don't keep observers alive.** `observe` captures the observer as a `WeakEntity` and upgrades it on each call (GPUI `src/app/context.rs` L71–74). Each returns a `Subscription`; store it to cancel by dropping, or `detach()` it to keep it for as long as both entities exist (GPUI `src/_ownership_and_data_flow.rs` L54).
- **In Zed.** The model/view pairs talk this way: `Terminal` implements `EventEmitter<Event>` (Zed `terminal/src/terminal.rs` L3500), and `TerminalView` holds a strong `Entity<Terminal>` (Zed `terminal_view/src/terminal_view.rs` L131–132). Workspace items are required to be `EventEmitter<Self::Event>` (Zed `workspace/src/item.rs` L170).

**Decision and reason.** Use `notify` + `observe` for "my state changed, re-render or re-read" (cheap, payload-free, the default for views), and `emit` + `subscribe` for semantic events a parent must act on (a child requesting navigation, a store announcing which domains a write touched). Data flows down through strong handles and reads; intent flows up through events. This keeps children ignorant of their parents, so a view can be reused or tested without the shell above it.

## 4. Async work, I/O and persistence

- **Two executors.** `BackgroundExecutor::spawn` runs a `Send` future "to completion on a background thread" (GPUI `src/executor.rs` L143–145); `ForegroundExecutor::spawn` runs a non-`Send` future on the main thread (L469–471).
- **Tasks cancel on drop.** "If you drop a task it will be cancelled immediately. Calling `Task::detach` allows the task to continue running" (GPUI `src/executor.rs` L52–58).
- **Entity-scoped async.** `Context::spawn` gives the future "a weak handle to the entity owned by this context and a context that can be held across await points", and the returned task "must be held or detached" (GPUI `src/app/context.rs` L232–237).
- **Services behind traits, injected once.** Zed abstracts the filesystem as `pub trait Fs: Send + Sync` (Zed `fs/src/fs.rs` L104), with a `FakeFs` behind the `test-support` feature (L41–71, L1444). The real `Fs` is handed to the app through `AppState { fs: Arc<dyn fs::Fs>, .. }`, stored as the `GlobalAppState` Global (Zed `workspace/src/workspace.rs` L1332–1345). Zed's `db` crate wraps a SQLite connection "set as a GPUI global per-App" (Zed `db/src/db.rs` L39–40).

**Decision and reason.** Keep the domain synchronous and pure; put I/O behind a port trait in a non-UI crate, with a real and a fake adapter; call it from a model Entity via `cx.background_executor().spawn(..)` (or `cx.spawn` when the result must update the entity), store the `Task` on the entity so dropping the entity cancels the work, and write back through the `WeakEntity` the spawn hands you. The reason is that the main thread renders every frame, so blocking I/O there stalls the UI, and a trait seam lets tests swap the adapter for a fake without a real disk or database.

## 5. Actions, keymaps, focus and key contexts

- **Keyboard via actions in key contexts.** "To expose functionality to the keyboard, you bind an _action_ in a _key context_" (GPUI `docs/key_dispatch.md` L7). Actions are user-defined structs, declared with `#[gpui::action]` or `actions!`, handled with `.on_action(..)` on an element, and bound in a keymap that targets a `key_context` by fully qualified type name (L9–100).
- **Keymaps are data.** `App::bind_keys` installs bindings (GPUI `src/app.rs` L1677). Zed loads its default keymap from a JSON asset and then the user's keymap through `bind_keys` (Zed `zed/src/zed.rs` L2335, L2347–2354).
- **Focus decides dispatch.** `FocusHandle` tracks the focused element (GPUI `src/window.rs` L262–266), and a view exposes its handle through `Focusable` "using window.focus_view(cx, view)" (L438–440). Zed's `Item` and `Panel` traits both require `Focusable` (Zed `workspace/src/item.rs` L170; `workspace/src/dock.rs` L36).
- **Actions per crate.** Zed has a shared `zed_actions` crate and a `menu` crate for actions several crates use, both registered at start-up (Zed `zed/src/main.rs` L489–490), while feature crates declare their own with `actions!` (e.g. Zed `workspace/src/workspace.rs` L387, L607).

**Decision and reason.** Name every user intent as an action, handle it on the view that owns the behaviour, scope it with a key context, and keep keystrokes in a keymap file. Decoupling keystroke from behaviour makes bindings user-rebindable, lets a command palette list every action, lets the same key mean different things in different contexts, and keeps mouse and keyboard paths calling the same handler.

## 6. Settings and theme as Globals

- **Globals.** `Global` is a marker trait for app-wide singletons; its docs suggest wrapping a private Global in a newtype to restrict read/write access (GPUI `src/global.rs` L3–22). `App::set_global` and `App::observe_global` set and watch them (GPUI `src/app.rs` L1497, L1522).
- **Settings.** Zed's `SettingsStore` is a Global (Zed `settings/src/settings_store.rs` L254). Each feature defines a typed `Settings` value, registers it with `Settings::register(cx)` and reads it with `get_global(cx)` (L66–110).
- **Theme.** Zed exposes the active theme through an `ActiveTheme` trait implemented on `App`, backed by a `GlobalTheme` Global (Zed `theme/src/theme.rs` L147–157, L322–326). `gpui-component` follows the same shape: `ActiveTheme` on `App` and `impl Global for Theme` (GC `theme/mod.rs` L28–34, L98), changed through `Theme::change` (L150).

**Decision and reason.** Configuration and theme are read everywhere and change rarely, so they live in Globals rather than being threaded through every constructor; views that must react call `observe_global`. Keeping each feature's settings as its own typed struct registered into one store means a feature crate owns its own settings without a central struct that every crate must depend on.

## 7. Crate boundaries, composition root and the window root

- **Model crates without UI, view crates with it.** At `28d0a35`, `project`, `terminal` and `git` depend on `gpui` and `settings` but not on `ui`, `workspace` or `editor`; their view counterparts `project_panel`, `terminal_view` and `git_ui` depend on `ui` and `workspace` (each crate's `Cargo.toml` under Zed `crates/`). `Project` is documented as the semantics-aware entity for worktrees, LSP and collaboration, with no rendering (Zed `project/src/project.rs` L212–217). Foundation crates such as `text` use `gpui` only as a dev-dependency (Zed `text/Cargo.toml` L33–36), and `sum_tree` not at all.
- **Many crates.** Zed's `crates/` holds about 250 crates at this commit. Zed's sources read for this note don't state why; faster incremental and parallel builds and enforced dependency direction are the usual reasons, but that is an inference here, not a cited claim.
- **One `init(cx)` per crate, called from `main`.** Zed's `main` calls `settings::init`, `theme_settings::init`, `editor::init`, `workspace::init(app_state.clone(), cx)` and dozens more in order (Zed `zed/src/main.rs` L489–498, L670, L731–745). Each crate registers its own actions, settings and Globals there.
- **A root view per window.** `Workspace` is "a counterpart of a window", owns the panes, docks and status bar, and "serves as a default, 'global context', that can be used to register a global action" (Zed `workspace/src/workspace.rs` L1575–1580). In `gpui-component`, `Root` "is a view for the App window for as the top level view (Must be the first view in the window)" and manages Sheet, Dialog and Notification layers (GC `root.rs` L210–212); `gpui_component::init(cx)` registers its theme and components (GC `lib.rs` L97–116).
- **Widgets with state are Entities too.** `gpui-component`'s text input keeps its editing state in an `InputState` Entity that emits `InputEvent::{Change, PressEnter, Focus, Blur}` (GC `input/state.rs` L93–98, L260–261, L337).

**Decision and reason.** Layer the code so dependencies point one way: pure domain crates, then I/O ports and adapters, then gpui model Entities, then views, then the window root and the composition root (`main`). A model crate that never imports the UI layer can be tested headless and reused by a different frontend, and a change to a view doesn't recompile the models below it.

## 8. Testing seams

- **Headless app tests.** `#[gpui::test]` provides a `TestAppContext` (GPUI `src/app/test_context.rs` L18), and `VisualTestContext` is "the test-equivalent of a `Window` and `App`" (L667–669), with `simulate_keystrokes` (L715), `simulate_click` (L771) and `run_until_parked` (L696) to drain the executors deterministically. The README points at these for testing GPUI apps (GPUI `README.md` "Other Resources").
- **Fakes behind `test-support`.** Zed's crates expose fakes (`FakeFs`) and test helpers under a `test-support` Cargo feature that only dev-dependencies enable (Zed `fs/Cargo.toml` L65–70; `fs/src/fs.rs` L41–71).
- **Pure crates need no gpui.** Code that doesn't depend on `gpui` (Zed's `sum_tree`; `text` outside its tests) is tested with plain `#[test]`.

**Decision and reason.** Test three ways, from cheapest to dearest: pure rules with plain unit tests, model Entities with `TestAppContext` and fake adapters, and views through `VisualTestContext` driving real key and mouse dispatch with no window or GPU. Putting logic as low in the layers as it can go keeps most tests in the cheap tier.

## 9. Personal Ledger's Desktop compared

Read from `CLAUDE.md`'s workspace layout, `docs/diagrams/desktop-architecture.json` (pinned at `16f4b96`), ADR-0016, ADR-0030, ADR-0032, ADR-0037, ADR-0038, ADR-0039 and the `bin-desktop` source on `concept`. No code was changed.

### 9.1 Where it matches

- **Pure domain layer.** Domain crates are "pure, `gpui`-free, I/O-free" and never depend on each other, with the `Ledger` aggregate in `lib-ledger` on top (`CLAUDE.md` workspace layout; ADR-0038). This is section 7's bottom layer, and stricter than Zed's, where model crates do depend on `gpui`.
- **One model Entity with typed events.** `LedgerStore` holds the only `Ledger` and emits `LedgerChanged` naming the domains a write touched (`crates/bins/bin-desktop/src/ledger_store.rs` L1–40), which is section 3's emit/subscribe pattern for a model.
- **Views are Entities that own state and render themselves.** ADR-0032 and ADR-0037 make each destination an Entity that implements `Render` and handles its own input.
- **Events up, no back-references.** ADR-0032 says a View reports to `Shell` with `cx.emit` and "never holds a `WeakEntity<Shell>`". That is stricter than section 2 needs, and avoids cycles altogether: no file under `bin-desktop/src` names `WeakEntity`.
- **Persistence port with two adapters.** ADR-0039 defines one `LedgerRepository` port with a stub and a SQLite adapter, with `lib-ledger` never depending on `lib-database`: section 4's port-plus-fake, with commands that stay synchronous and record changes.
- **Theme as a Global.** `Colours` implements `Global` (`crates/bins/bin-desktop/src/theme/colours.rs` L28).
- **Headless UI tests.** ADR-0030 tests through `TestAppContext`/`VisualTestContext` with `simulate_keystrokes`/`simulate_click`, behind a `test-support` feature only a self dev-dependency enables (`crates/bins/bin-desktop/Cargo.toml` L26–29). `build_shell` is shared by the app and the tests (`crates/bins/bin-desktop/src/lib.rs` L74).

### 9.2 Where it diverges

- **Keyboard doesn't use GPUI actions, key contexts or keymaps.** `bin-desktop/src` has no `actions!`, `.key_context(` or `bind_keys`; `Shell` takes raw keys in one `on_key_down` (`crates/bins/bin-desktop/src/app/render.rs` L580) and routes them through `navigation::key_router::route_key`, a pure function chosen so the order of the routing tiers is unit-testable (`crates/bins/bin-desktop/src/navigation/key_router.rs` L1–15). Rebinding goes through `lib-config`'s `[keybindings]` (`crates/bins/bin-desktop/src/lib.rs` L125–134). The trade-off: a modal, chorded grammar (`docs/navigation-design.md`) gets a directly tested router, at the cost of GPUI's context-scoped dispatch, an action registry a command palette could list, and a data keymap.
- **Models and views share one bin crate.** `LedgerStore`, the per-domain stores and every View live in `bin-desktop`, so there is no model-crate/view-crate split as in section 7. The domain logic below is already out in lib crates, which gives most of the testing benefit; the cost is compile and link time: any test of a store or View links all of `gpui`, which is why the UI tests are kept to one binary (`crates/bins/bin-desktop/Cargo.toml` L19–20).
- **No Item-style trait; a closed enum instead.** ADR-0016 and ADR-0037 reject a `View` trait and `AnyView`, forwarding through one exhaustive `ActiveViewEntity` match in `view/active.rs`. Zed uses an open `Item` trait because extensions and many crates add item types; Personal Ledger has a fixed set of destinations, so the enum's "a new View fails to compile in one place" is a reasonable trade.
- **I/O isn't on the background executor yet.** Window and navigation state is a synchronous JSON load and save (`crates/bins/bin-desktop/src/persistence.rs` L120, L139), and Ledger writes are not persisted yet (#636). `cx.spawn` is used only for timers (`app/toasts.rs` L47, `app/log_feed.rs` L37, `app/rail.rs` L37, `view/budgets/state.rs` L954). The `LedgerRepository` wiring is the point where section 4's background-task pattern applies.
- **`gpui-component`'s `Root` isn't used.** The window root is `Shell`, with its own Chrome, Dialog host and Toasts (ADR-0016); `bin-desktop` uses `gpui-component` for charts, scrollbars and its `Theme` (`use gpui_component::…` across `src/`), not for `Root`, dialogs, notifications or `InputState`.
- **Settings are read once, not a settings Global.** `run` reads `lib_config::Config` once and passes keybindings and theme overrides into `Shell` and `Colours` (`crates/bins/bin-desktop/src/lib.rs` L122–145); the Locale is fixed for the run by ADR-0021. Only `Colours` is a Global. This is enough while preferences change only through Settings pages that `Shell` owns.
- **Halfway state.** Categories, Tags and Inventory still have read-only per-domain stores that re-emit `LedgerChanged` (`docs/diagrams/desktop-architecture.json`, card "Data"), so there are two model layers until those Views read `LedgerStore` directly.
- **GPUI version.** The Desktop pins `gpui 0.2.2`, whose entry point is `Application::new()` (`crates/bins/bin-desktop/src/lib.rs` L140); Zed `main` has moved to `gpui_platform::application()` (Zed `gpui/README.md`). Upgrading will touch the composition root.

## 10. Unverified

- **Why Zed splits so many crates.** No Zed-authored source read here states the reason; the compile-time explanation in section 7 is inference.
- **Zed's `main` vs `gpui-v0.2.2`.** Zed claims (workspace, settings, theme, project) are from `main` at `28d0a35`, not the `gpui-v0.2.2` tag; the patterns are stable but exact APIs may differ from what `gpui 0.2.2` offers.
- **`gpui-component` `main`.** At `main` (`1b6059c`), `gpui-component` has restructured into `gpui_kit` and other crates; this note uses `v0.5.1`, the pinned version.
