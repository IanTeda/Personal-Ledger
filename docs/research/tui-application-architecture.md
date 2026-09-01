# Research: Ratatui application architecture patterns

**Question.** ADR-0002 (`docs/adr/0002-ratatui-for-tui-charting.md`) already locked in `ratatui` + `crossterm` for the TUI client, based on charting-library research (`docs/research/tui-charting-libraries.md`). That ADR does not address how the TUI *application itself* should be structured — state management, the event loop, and how a growing set of screens (Personal Ledger's TUI will eventually need account, transaction, budget, and category screens, not just the chart/table feasibility demos) fit together. This note is primary-source research only, surveying the architecture patterns ratatui's own documentation describes, so that a later decision (a future ADR, not this document) has options and tradeoffs to choose from. It does not recommend a pattern.

All claims are cited to primary sources: the official documentation site at `ratatui.rs` (specifically the `/concepts/application-patterns/` section and the `/tutorials/counter-async-app/` and `/recipes/testing/` sections it links from), and the `ratatui/templates` and `ratatui/ratatui` GitHub repositories. Research was carried out on 2026-09-01 via ratatui.rs directly; content on the live site may change after this date.

Ratatui's own docs frame the scope explicitly: the top-level page (https://ratatui.rs/concepts/application-patterns/) states "This page covers several patterns one can use for their application and acts as a top-level page for the following articles where these patterns are explored more in-depth," and links to exactly three named patterns:

1. **The Elm Architecture** — https://ratatui.rs/concepts/application-patterns/the-elm-architecture/
2. **Component Architecture** — https://ratatui.rs/concepts/application-patterns/component-architecture/
3. **Flux Architecture** — https://ratatui.rs/concepts/application-patterns/flux-architecture/

Each is covered below, followed by ratatui's async/tokio and testing guidance (relevant since Personal Ledger's `server` crate already depends on `tokio`/`tonic` — see root `CLAUDE.md`), and a closing comparison.

## 1. The Elm Architecture (TEA)

**Source:** https://ratatui.rs/concepts/application-patterns/the-elm-architecture/

**Structure.** TEA organises the app into three pieces, per the docs: **Model** ("your application's state. It contains all the data your application works with"), **Update** (processes messages, taking the current model and input to "produce a new model"), and **View** ("responsible for displaying your model to the user... it'll produce terminal UI elements"). The flow is: user input → `update(model, message)` → `view(model)` renders → display.

State is a plain struct, e.g. the documented counter example:

```rust
#[derive(Debug, Default)]
struct Model {
    counter: i32,
    running_state: RunningState,
}
```

Actions are an enum, e.g.:

```rust
#[derive(PartialEq)]
enum Message {
    Increment,
    Decrement,
    Reset,
    Quit,
}
```

**Update signature — two documented variants.** The docs present both a "pure" immutable-TEA form, `fn update(model: &Model, msg: Message) -> Model`, and a "pragmatic Rust" mutable form, `fn update(model: &mut Model, msg: Message)`, and say explicitly: "while immutability is emphasized in TEA, Rust developers can choose the most suitable approach based on performance and their application's needs... it would be perfectly valid" to mutate in place.

**Message cascading.** The docs also show a form that returns `Option<Message>` so an update can trigger a further update (e.g. `Message::Increment` returning `Some(Message::Reset)` once a counter exceeds a bound), explicitly to "chain messages or have an update lead to another update."

**Rendering.** The view function is meant to be pure: "for a given state of the model, it should always produce the same UI representation." The docs flag a real constraint of immediate-mode rendering: "the `view` function is only aware of the area available to draw in at render time" — a "recognized constraint of immediate mode GUIs" — with two documented mitigations: store the drawable size from a prior frame (risks flicker on resize) or use ratatui's `Resize` event to force a redraw. The docs also concede that `StatefulWidget`s (https://docs.rs/ratatui/latest/ratatui/widgets/trait.StatefulWidget.html) and cursor-positioned text inputs (via `f.set_cursor(...)`) require a mutable `&mut Model` in `view`, i.e. "you may choose to forego the `view` immutability principle" in practice.

**Event handling and main loop.** Events map to `Option<Message>` via a `handle_event`/`handle_key` pair, and the documented `main()` loop is a `while` loop: draw the view, poll for one event, then drain any cascaded messages through `update` before looping back to draw again — a synchronous, single-threaded loop with `event::poll(Duration::from_millis(250))`.

**Framing.** The docs describe TEA apps as "a Finite State Machine": "an initial state and an event... lead to a subsequent state. This cascading approach ensures that the system remains in a consistent and predictable state," which lets developers "break down intricate state transitions into smaller, more manageable steps."

**What the docs say it's good for.** Clear separation of data (model) from logic (update); predictable, pure rendering; explicit, enumerable state transitions via the Message enum — useful for reasoning about complex state machines.

**What the docs don't cover here.** This page has no discussion of async/tokio, no testing guidance, and no multi-screen/multi-model scaling guidance — those are addressed (for async) on separate tutorial pages, covered in §4 below, and (for testing) on the separate recipes pages in §5.

**Ecosystem link.** The page links an external, ratatui-specific TEA framework crate: `tui-realm` (https://github.com/veeso/tui-realm/), and a text-input widget it uses in the cursor example, `tui-input` (https://github.com/sayanarijit/tui-input). It also links a hands-on walkthrough, the counter-app tutorial (https://ratatui.rs/tutorials/counter-app/).

## 2. Component Architecture

**Source:** https://ratatui.rs/concepts/application-patterns/component-architecture/

**Structure.** Instead of one global `Model`, each UI component "encapsulates its own state, event handlers, and rendering logic" behind a shared `Component` trait. The docs' example trait:

```rust
pub trait Component {
  fn init(&mut self) -> Result<()> {
    Ok(())
  }
  fn handle_events(&mut self, event: Option<Event>) -> Action {
    match event {
      Some(Event::Quit) => Action::Quit,
      Some(Event::Tick) => Action::Tick,
      Some(Event::Key(key_event)) => self.handle_key_events(key_event),
      Some(Event::Mouse(mouse_event)) => self.handle_mouse_events(mouse_event),
      Some(Event::Resize(x, y)) => Action::Resize(x, y),
      Some(_) => Action::Noop,
      None => Action::Noop,
    }
  }
  fn handle_key_events(&mut self, key: KeyEvent) -> Action {
    Action::Noop
  }
  fn handle_mouse_events(&mut self, mouse: MouseEvent) -> Action {
    Action::Noop
  }
  fn update(&mut self, action: Action) -> Action {
    Action::Noop
  }
  fn render(&mut self, f: &mut Frame, rect: Rect);
}
```

Four lifecycle operations are documented: `init` ("where a component can set up any initial state or resources it needs"), event handling (`handle_events` dispatching to `handle_key_events`/`handle_mouse_events`, described as giving "a finer-grained approach to event handling, with each component only dealing with the events it's interested in"), `update` (a component reacts to an `Action` and mutates its own private state), and `render` ("each component defines its own rendering logic. It knows how to draw itself, given a rendering context").

**Tradeoff the docs state.** "One advantage of this approach is that it incentivizes co-locating the `handle_events`, `update` and `render` functions on a component level" — i.e. the pitch is locality/cohesion per screen or widget, versus TEA's single centralised `update` function that has to know about every message in the whole app.

**What the docs don't cover here.** No explicit discussion of how parent/child components compose or how actions route between them, no async/tokio discussion, and no testing guidance on this page.

**Template and real-world examples.** Ratatui ships an official starter for this pattern: the **Component template** (https://github.com/ratatui/templates/tree/main/component, browsable via `cargo-generate` at https://ratatui.rs/templates/component/). The docs cite two real applications built this way: `gobang` (https://github.com/TaKO8Ki/gobang, a cross-platform TUI database client) and `edma` (https://github.com/nomadiz/edma).

## 3. Flux Architecture

**Source:** https://ratatui.rs/concepts/application-patterns/flux-architecture/

**Structure.** The docs describe Flux as "a design pattern introduced by Facebook to address the challenges of building large scale web applications," repurposed for terminal apps that handle "complex user interactions, multiple views, and dynamic data sources." Four pieces:

- **Dispatcher** — a central hub with, per the docs, "no logic of its own; it simply ensures that all registered callbacks receive the action data":

```rust
struct Dispatcher {
    store: Store,
}
impl Dispatcher {
    fn dispatch(&mut self, action: Action) {
        self.store.update(action);
    }
}
```

- **Store** — owns state and update logic, and "notifies any listening components" once it updates:

```rust
struct Store {
    counter: i32,
}
impl Store {
    fn new() -> Self { Self { counter: 0 } }
    fn update(&mut self, action: Action) {
        match action {
            Action::Increment => self.counter += 1,
            Action::Decrement => self.counter -= 1,
        }
    }
    fn get_state(&self) -> i32 { self.counter }
}
```

- **Actions** — plain enums describing "any change or event in your application" (e.g. `enum Action { Increment, Decrement }`).
- **Views/Widgets** — "don't hold or manage the application state, but they display it."

**Data flow.** Unidirectional: user input → Action → Dispatcher → Store → state update → widget re-render — structurally close to Flux's original web-app shape (and, by extension, close to TEA's Model/Update/View loop, but with an explicit Dispatcher as a named intermediary and Stores as an implied plural — the docs' own example only shows a single `Store`, so they don't demonstrate the "many independent stores" idea that distinguishes Flux from TEA in the original web-Flux design).

**Tradeoffs.** Ratatui's docs state none explicitly for this pattern — no pros/cons, no scalability discussion, no guidance on when not to use it. This is the thinnest of the three pages.

**Real-world example.** The docs link one implementation: a chat client's TUI, https://github.com/Yengas/rust-chat-server/tree/main/tui — no dedicated official template exists for Flux (unlike Component, which has the `component` template).

## 4. Async / tokio integration

Ratatui's docs treat async integration as an *extension* of the Elm/event-driven pattern rather than a fourth named architecture, worked through in the `/tutorials/counter-async-app/` tutorial series:

- **Full Async Events** — https://ratatui.rs/tutorials/counter-async-app/full-async-events/
- **Async Event Stream** — https://ratatui.rs/tutorials/counter-async-app/async-event-stream/
- **Full Async Actions** — https://ratatui.rs/tutorials/counter-async-app/full-async-actions/
- **Async Increment & Decrement** — https://ratatui.rs/tutorials/counter-async-app/async-increment-decrement/

**Structural approach (Full Async Events).** A `Tui` struct wraps both the terminal and event handling, using `tokio::sync::mpsc` (`UnboundedSender`/`UnboundedReceiver`) rather than `std::sync::mpsc`, so receiving is `.await`-based:

```rust
pub struct Tui {
  pub terminal: ratatui::Terminal<Backend<std::io::Stderr>>,
  pub task: JoinHandle<()>,
  pub event_rx: UnboundedReceiver<Event>,
  pub event_tx: UnboundedSender<Event>,
  pub frame_rate: f64,
  pub tick_rate: f64,
}
```

Events are widened beyond keyboard/mouse into an enum that also carries lifecycle and render-pacing variants: `Init, Quit, Error, Closed, Tick, Render, FocusGained, FocusLost, Paste(String), Key(KeyEvent), Mouse(MouseEvent), Resize(u16, u16)`. The core loop multiplexes three sources with `tokio::select!` — a tick interval, a render interval, and the crossterm event stream (`reader.next().fuse()`) — so a dedicated `Event::Render` variant, not every input event, is what triggers a redraw: `if let Event::Render = event.clone() { tui.draw(|f| { ui(f, &app); })?; }`. This decouples render cadence (the tutorial's example uses 30 FPS) from input/tick cadence. The tutorial explicitly notes this stage is not yet "truly async capable" — rendering itself runs on the async loop, but background *work* (e.g. a long-running action) needs the further step covered next. Full `Tui`-struct implementation detail is cross-referenced to https://ratatui.rs/recipes/apps/terminal-and-event-handler/.

**Structural approach (Full Async Actions).** Adds a second, independent `mpsc::unbounded_channel` for `Action`s (distinct from the `Event` channel), whose sender (`action_tx`) is cloned into the `App` struct so any part of the app can push actions. Long-running work is offloaded via `tokio::spawn`, sending a follow-up action back over the channel on completion:

```rust
Action::NetworkRequestAndThenIncrement => {
    let tx = app.action_tx.clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_secs(5)).await;
        tx.send(Action::Increment).unwrap();
    });
}
```

The main loop drains actions with `action_rx.try_recv()`. The tutorial's own example code `.unwrap()`s the channel send from inside the spawned task and does not discuss task cancellation if the app quits mid-flight — real caveats to carry over into any Personal Ledger implementation rather than gaps the docs flag themselves.

**Relevance to Personal Ledger.** Since `crates/server` already runs on `tokio`/`tonic` (per root `CLAUDE.md`), a TUI client talking to that server over gRPC would naturally want this async-events-plus-async-actions shape — non-blocking RPC calls dispatched as spawned tasks that report back into the same action/event channel driving redraws — regardless of which of the three named patterns (TEA/Component/Flux) owns the surrounding state shape. The docs present this async machinery as orthogonal to, and layerable onto, TEA specifically (the tutorial builds on the counter-app TEA example), and by extension nothing in the Component or Flux pages contradicts using the same `tokio::select!`-driven event loop underneath either of those instead.

## 5. Testing guidance

**Source:** https://ratatui.rs/recipes/testing/ (index), linking to:

- Snapshot testing — https://ratatui.rs/recipes/testing/snapshots/
- Debugging widget state — https://ratatui.rs/recipes/testing/debug-widget-state/

**Snapshot testing.** The docs recommend `insta` (https://crates.io/crates/insta) and `cargo-insta` (https://crates.io/crates/cargo-insta) together with ratatui's own `TestBackend`, which renders to an in-memory buffer instead of a real terminal. Documented pattern:

```rust
#[cfg(test)]
mod tests {
    use super::App;
    use insta::assert_snapshot;
    use ratatui::{backend::TestBackend, Terminal};

    #[test]
    fn test_render_app() {
        let app = App::default();
        let mut terminal = Terminal::new(TestBackend::new(80, 20)).unwrap();
        terminal
            .draw(|frame| frame.render_widget(&app, frame.area()))
            .unwrap();
        assert_snapshot!(terminal.backend());
    }
}
```

The docs advise fixed terminal dimensions (their example: 80x20) for reproducibility, reviewing intentional diffs with `cargo insta review`, and note colour assertions aren't currently supported by this approach. Further reading is pointed at https://insta.rs/docs/.

**Testability-by-construction.** The counter-app tutorial (linked from the TEA page, https://ratatui.rs/tutorials/counter-app/) demonstrates testing the UI by rendering to a buffer and asserting its content, and separately notes that splitting keyboard-event handling into its own function (`handle_key`, shown in §1 above returning `Option<Message>`) lets update logic be tested by constructing `Message`s directly, without emulating a real terminal or keyboard at all. This testability argument is specific to TEA's page in the docs, but the same principle — pure/isolated update logic decoupled from I/O — applies equally to a Component's `update` method or a Flux `Store::update`, since all three documented patterns isolate "what changes state" from "how it's rendered" in the same basic shape.

**Not covered.** None of the three pattern pages, nor the testing recipes, discuss integration-testing a full async event loop (e.g. asserting on `tokio::select!`-driven behaviour end-to-end) — only synchronous `update`/render testing is demonstrated.

## 6. Comparison summary

| Aspect | The Elm Architecture | Component Architecture | Flux Architecture |
|---|---|---|---|
| State shape | One centralised `Model` struct for the whole app | Private state owned per-component, no global model | State centralised in one or more `Store`s |
| Update flow | Single `update(model, message)` function/`match`, optionally cascading via `Option<Message>` | Each component's own `update(&mut self, action)`; no central dispatcher shown | `Dispatcher::dispatch` forwards `Action`s to `Store::update`; docs' own example uses exactly one store |
| Rendering trigger | Pure `view(model, frame)`, called once per loop iteration after updates settle | Each component's own `render(&mut self, frame, rect)` | Views/widgets read `Store::get_state()` and re-render after a store update; docs don't detail the notify/subscribe mechanism beyond "notifies any listening components" |
| Docs' own stated advantage | Predictable state machine; clear model/update/view separation; message cascading for chained transitions | Co-locates events/update/render "on a component level" — cohesion per widget/screen | Unidirectional flow suited to "complex user interactions, multiple views, and dynamic data sources" (asserted, not elaborated) |
| Docs' own stated caveat | View "only aware of the area available to draw in at render time" (immediate-mode constraint); immutability principle often broken in practice for `StatefulWidget`s/cursor input | None stated explicitly beyond the co-location advantage | None stated at all — thinnest of the three pages |
| Official template | None dedicated (counter-app tutorial serves this role); external `tui-realm` crate implements TEA specifically for ratatui | Official `component` template (`ratatui/templates`) | None |
| Real-world examples cited by ratatui docs | — (tutorial only) | `gobang`, `edma` | `rust-chat-server`'s `tui` |
| Async/tokio guidance on the pattern's own page | None (covered separately in the counter-async-app tutorial, which builds on top of the TEA counter example) | None | None |
| Testing guidance on the pattern's own page | None on the architecture page itself, but demonstrated in the companion counter-app tutorial | None | None |

Every pattern's page is silent on multi-screen/multi-view scaling specifically — none of the three describes how to route between, say, an accounts screen and a transactions screen, or how nested state should be organised once an app outgrows a single counter-sized example. The async-events/async-actions tutorial pages and the testing recipes pages are also pattern-agnostic in ratatui's own docs: they're demonstrated by extending the TEA counter example, but nothing on any of the three pages suggests the async event-loop shape or the `TestBackend`/`insta` testing approach couldn't be layered onto Component or Flux equally.

## Closing note

Ratatui's own documentation presents three named patterns — Elm/TEA, Component, and Flux — without ranking them, and this note deliberately does the same. TEA is the pattern most thoroughly documented on ratatui.rs (the only one with a full worked tutorial, and the only one whose async and testing extensions are shown end-to-end), Component is the only one with an official `cargo-generate` starter template and named real-world adopters, and Flux is documented in the least depth (no tradeoffs section, a single-store example that doesn't demonstrate the multi-store shape that distinguishes it from TEA, and no official template). Choosing among them — or blending them (e.g. TEA-style centralised state per screen, wrapped in Component-style per-screen trait boundaries) — and reconciling the choice with Personal Ledger's planned multi-screen TUI (accounts, transactions, budgets, categories) and its existing `tokio`/`tonic` server is left to a future ADR.
