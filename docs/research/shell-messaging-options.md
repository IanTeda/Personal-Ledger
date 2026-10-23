# Research: How VIPER modules send Toasts and status text to the Shell

**Question.** Feature crates (`lib-units`, `lib-bills`, the cross-domain `lib-bill-payments` and every later `lib-<domain>`) need to ask the Shell for a Toast, a status-line message and perhaps, later, a Navigate or open-Dialog. `lib-shell` is planned as a VIPER module in its own right. Given the workspace's dependency rules, what are the options for how a module's Presenter hands those requests to `lib-shell`, where the message type should live, and is a `lib-toast`-style shared crate the right home for it? Asked on [#806](https://github.com/IanTeda/Personal-Ledger/issues/806) (slice 3 of the VIPER transition, parent [#747](https://github.com/IanTeda/Personal-Ledger/issues/747)).

Research was done by reading `docs/development/viper-architecture.md`, ADR-0027, ADR-0032, ADR-0037, ADR-0045 and ADR-0046, `docs/toasts-design.md`, `GLOSSARY.md`, `deny.toml`, `crates/libs/lib-ledger/tests/viper_dependencies.rs`, every `Cargo.toml` under `crates/libs/`, the Router traits and their fakes in `lib-units`, `lib-bills` and `lib-bill-payments`, their Desktop and TUI implementations, and the gpui 0.2.2 source in the local Cargo registry. Sections 1 and 2 are verified facts; sections 3 to 6 are analysis and a recommendation built on them.

## 1. What the rules say (verified)

### 1.1 Dependencies

- A feature crate never depends on a UI toolkit, on `lib-database`, or on another feature crate (`docs/development/viper-architecture.md:84`).
- A feature crate may depend on `lib-core` and "the shared toolkit-free libs": `lib-config`, `lib-locale`, `lib-list-cursor`, `lib-toast` ("a Router asks for a Toast by `ToastKind`") and `lib-viper` (`docs/development/viper-architecture.md:85`). So `lib-toast` is already named as a shared lib, not a feature crate.
- `lib-viper` holds generic types every module's Presenters and keymap use; it "is not a feature crate and holds no domain; it keeps the feature crates' bans" (`docs/development/viper-architecture.md:86`).
- A cross-domain module may depend on the feature crates it writes; feature crates never depend on it (`docs/development/viper-architecture.md:90`).
- `deny.toml` restricts only the crates it lists: the four toolkits, `lib_database` and `lib_bill_payments` (`deny.toml:70-77`). `lib_toast` and `lib_viper` are not listed, so any crate may depend on them; the comment says a new shared lib is left out of the list on purpose (`deny.toml:62-69`).
- The `cargo metadata` test checks that a crate in `MIGRATED` depends on no feature crate, toolkit or `lib_database` (`crates/libs/lib-ledger/tests/viper_dependencies.rs:21`, `:54-67`). `MIGRATED` holds `lib_bills`, `lib_transactions`, `lib_units` and `lib_viper`; `lib_toast` is not in it, and neither is a `lib_shell` (`:21-36`). `FEATURE_CRATES` lists the eleven domains only (`:24-36`), so the test as written would not flag a feature crate depending on `lib_shell`.

### 1.2 Presenter, Router and the Shell module

- A Presenter's contract is `handle(event, router)` with `router: &mut dyn <Domain>Router`, lent for each call, not held (`docs/development/viper-architecture.md:108`, `:110`).
- Each module declares its `<Domain>Router` trait in `protocols.rs` and each bin implements it (`docs/development/viper-architecture.md:125`). To go to another module, a Router calls that module's public entry point in the same bin, never building another module's View or Presenter (`:128`).
- `RecordingRouter` in `fake.rs` records what the Presenter asked for, and Presenter tests assert on it (`docs/development/viper-architecture.md:154`).
- `lib-shell`'s Presenter holds the active View, focus, the status line, the Toasts and the open Dialog; its Router, in each bin's `shell/router.rs`, is the root Router (`docs/development/viper-architecture.md:158-164`).
- The "During the transition" table lists `UnitsRouter::raise_toast` and `set_status`, and `BillsRouter`'s, as stand-ins "removed when `lib-shell`'s Router takes them", because "the Toasts and the status line are `lib-shell`'s, which isn't built; the TUI, with no status slot, raises an Info Toast" (`docs/development/viper-architecture.md:221`). The per-module commits are also a stand-in until `input.rs` "commits after each event it sends" (`:224`).
- The TUI waits for its redesign; until then it gets only "the minimal edits a change to a shared trait forces" (`docs/development/viper-architecture.md:231`).

### 1.3 Toasts and the status-line message are two concepts

- GLOSSARY: a Toast reports "the outcome of an action or event"; it is "distinct from the status-line message, which answers the key or command just typed … and clears on the next keypress" (`GLOSSARY.md:91`). The Shell owns both (`GLOSSARY.md:107`).
- `docs/toasts-design.md:11` repeats the split and adds that the two coexist. On the status line, precedence is "command echo > status-line message > Toast echo > page legend" (`docs/toasts-design.md:46`), and the session history records Toasts only, never status-line messages (`docs/toasts-design.md:53`).
- ADR-0027 puts the Toast echo below "the command echo and the status-line message" (`docs/adr/0027-toasts-off-is-a-client-scoped-preference-and-errors-always-toast.md:7`).

### 1.4 Desktop event rules

- A View reports to the Shell by emitting typed events and never holds a handle to it (`docs/adr/0032-desktop-views-are-entities-that-own-their-view-state.md:11`).
- "`ViewEvent` is the only View→Shell channel … A new variant is added only for a request that is truly cross-cutting" (`docs/adr/0037-desktop-view-entities-render-and-handle-their-own-input.md:13`).
- ADR-0045 renames the frame to the Shell; `chrome/` is renamed when the Shell becomes `lib-shell` (`docs/adr/0045-the-frame-is-the-shell-not-the-chrome.md:7`, `:11`).
- A failed write is reported on a channel the bin listens on, and the bin shows an error Toast; ports know nothing about the channel (`docs/adr/0046-per-domain-adapters-hold-rows-in-memory-behind-one-shared-writer.md:11`).

## 2. What the code does today (verified)

### 2.1 `lib-toast`

- Package `lib_toast`, with no dependencies at all (`crates/libs/lib-toast/Cargo.toml`, empty `[dependencies]`).
- Its crate doc: "The Toast model shared by the Desktop and TUI Clients", owning Toast Kinds, lifetimes, the visible stack, `×N` merging, dismissal, the history "and choosing what the status-line echo shows when Toasts are off. Pure: no I/O, no drawing and no real clock" (`crates/libs/lib-toast/src/lib.rs:1-7`).
- It exports `ToastKind`, `Toast`, `Toasts<S>`, `Display`, `History` and friends (`crates/libs/lib-toast/src/lib.rs:13-15`). `ToastKind` carries lifetimes and glyphs (`crates/libs/lib-toast/src/kind.rs:5-42`). `Toasts::raise(kind, text, raised_at)` takes already-resolved text (`crates/libs/lib-toast/src/toasts.rs:114-117`).
- The design doc places it with the `lib-colour-theme` precedent: a pure crate both bins use (`docs/toasts-design.md:84`).
- Dependents: `bin_desktop`, `bin_tui`, `lib_bills` and `lib_units` (each `Cargo.toml`). `lib_bill_payments` does not depend on it.

So `lib-toast` is a shared-type and shared-model crate in the `lib-viper` / `lib-colour-theme` mould, not a feature crate. It models the Toast stack, which in the target is state held by `lib-shell`'s Presenter. It knows the status-line *echo* of a Toast, but nothing of the status-line *message*.

### 2.2 `lib-viper`

- Depends only on `lib_config` (`crates/libs/lib-viper/Cargo.toml`), holds `keymap.rs` and `view.rs`, and is in `MIGRATED` (`crates/libs/lib-ledger/tests/viper_dependencies.rs:21`).

### 2.3 The Router traits

- `UnitsRouter` (`crates/libs/lib-units/src/protocols.rs:190-206`): `open_dialog`, `close_dialog`, `raise_toast(&mut self, kind: ToastKind, text: String)` at `:202` and `set_status(&mut self, text: String)` at `:205`.
- `BillsRouter` (`crates/libs/lib-bills/src/protocols.rs:190-215`): five navigation methods, `raise_toast(kind, String)` at `:211` and `set_status(&mut self, text: Option<String>)` at `:214`. The two `set_status` signatures already differ.
- `BillPaymentsRouter` (`crates/libs/lib-bill-payments/src/protocols.rs:47-50`) has only `close_dialog`; the Pay Dialog cannot raise a Toast or set status at all. Today it doesn't need to: "A Pay raises no Toast, since the row turning Paid is the feedback" (`crates/bins/bin-desktop/src/bills/view/page.rs:131`).

Call sites in the Presenters:

- Units raises a Success Toast when an outcome lands and the detail pane is not docked (`crates/libs/lib-units/src/presenter/units_page.rs:627-641`) and sets a "not yet built" status (`:571-573`).
- Bills never raises a Toast; it sets status on a refused Skip (`crates/libs/lib-bills/src/presenter/bills_page.rs:205`) and a row that isn't actionable (`crates/libs/lib-bills/src/presenter/schedule.rs:379`), and clears it on a tab switch (`crates/libs/lib-bills/src/presenter/bills_page.rs:221-229`).

Fakes: `lib-units`' `RouterCall::RaiseToast` / `SetStatus` (`crates/libs/lib-units/src/fake.rs:169-207`) and `lib-bills`' `Routed::Toast` / `Status` (`crates/libs/lib-bills/src/fake.rs:357-417`). Ten references across four files use these variants.

### 2.4 The bin implementations

- Desktop page Routers emit gpui `ViewEvent::RaiseToast` / `SetStatus` (`crates/bins/bin-desktop/src/units/router.rs:34-40`, `crates/bins/bin-desktop/src/bills/router.rs:81-87`), and `Shell` handles them in one place (`crates/bins/bin-desktop/src/app/view_events.rs:39`, `:49`; enum at `crates/bins/bin-desktop/src/view/event.rs:16-46`).
- Desktop Dialog Routers implement the same traits with no-op `raise_toast` and `set_status` (`crates/bins/bin-desktop/src/units/view/dialog.rs:174-192`, `crates/bins/bin-desktop/src/bills/view/dialog.rs:143-165`). A Toast a Dialog Presenter asked for would be dropped silently. Nothing asks today, but it is a trap the shared-sink options below remove.
- The TUI Units Router already queues: it pushes `(ToastKind, String)` onto a `Vec` borrowed from the View and maps `set_status` to an Info Toast because the TUI has no status slot (`crates/bins/bin-tui/src/units/router.rs:11-49`). The View hands the queue over through `take_toasts` (`crates/bins/bin-tui/src/view/mod.rs:646-649`), and `Shell::update` drains it after every `Action` (`crates/bins/bin-tui/src/shell.rs:1156-1163`).
- Write failures: the Desktop polls the writer's channel and, per failed domain, reloads and calls `Shell::raise_toast(ToastKind::Error, text)` with bin Messages (`crates/bins/bin-desktop/src/app/write_failures.rs:23-93`). `WriteFailure { domains, error }` is `lib-database`'s (`crates/libs/lib-database/src/writer.rs:87-90`), so only a bin can see it.

### 2.5 gpui's `emit` is already a queue

`Context::emit` pushes an `Effect::Emit` onto `pending_effects` (`gpui-0.2.2/src/app/context.rs:723-733`), and effects are flushed only when the outermost `update` finishes (`gpui-0.2.2/src/app.rs:759-777`, `:1201`). A `ViewEvent` emitted inside a Presenter's `handle` therefore reaches the Shell's subscription after the handler, and anything else in the same update, has returned. On the Desktop the "event queue the Shell listens to" exists already; it is gpui's, not ours, and untestable without a window.

## 3. The three separate choices

The question mixes three choices that can be made independently:

1. **How a Presenter hands a request out:** per-module Router methods, a shared supertrait, a separate sink parameter, or a return value.
2. **Where the message type lives:** `lib-viper`, `lib-toast`, a new small crate, or `lib-shell`.
3. **How the bin delivers it to the Shell:** gpui `emit`, or an explicit queue that `input.rs`'s `send` drains into the `lib-shell` Presenter.

Section 4 lays out the options on each axis; section 5 assesses `lib-toast` as the home; section 6 recommends.

## 4. Options

### 4.1 Hand-off: how the Presenter asks

**(a) Status quo: each `<Domain>Router` declares `raise_toast` and `set_status`.**

- Dependencies: each feature crate on `lib-toast` for `ToastKind` (allowed, `viper-architecture.md:85`).
- Rules: respects both rules.
- Cost: every module re-declares two methods, and they drift (Units `String`, Bills `Option<String>`, Bill Payments none). Every bin Router, page and Dialog, re-implements them; Dialog Routers drop them (section 2.4). Every `RecordingRouter` re-records them. The transition table marks these as stand-ins to remove (`viper-architecture.md:221`), and #806 lists removing them.

**(b) A shared supertrait.** `lib-viper` declares, say, `trait ShellRouter { fn request(&mut self, request: ShellRequest); }`, possibly with provided helpers `toast(kind, text)` and `status(text)` that call `request`. Each module's trait becomes `trait UnitsRouter: ShellRouter { … }`.

- Dependencies: feature crates already depend on `lib-viper`; `lib-viper` gains `lib-toast` if `ShellRequest` carries `ToastKind` (both toolkit-free, neither a feature crate).
- Rules: respects both. `lib-viper` stays in `MIGRATED`, so the test still forbids it reaching a feature crate.
- Presenters keep `handle(event, router)`; `router.request(…)` works through `&mut dyn UnitsRouter` because supertrait methods are callable on the subtrait object, and `&mut dyn UnitsRouter` coerces to `&mut dyn ShellRouter` where a helper wants one (trait upcasting, stable since Rust 1.86, [release notes](https://blog.rust-lang.org/2025/04/03/Rust-1.86.0/); the workspace pins 1.98.0, `mise.toml:9`).
- Bins: one `impl ShellRouter` per Router type, each one line pushing onto a queue. A Dialog Router pushes too, so nothing is dropped.
- Tests: each `RecordingRouter` records `ShellRequest`s in one variant (`RouterCall::Shell(ShellRequest)`), so the ten references change shape, not meaning. A shared `lib_viper::fake::RecordingShell` could hold that half.
- TUI: its Router impls change from two methods to one; mapping Status to an Info Toast moves to where the TUI drains the queue. That is the "minimal edit a change to a shared trait forces" (`viper-architecture.md:231`).
- Cross-domain modules: `BillPaymentsRouter: ShellRouter` gives the Pay Dialog the same channel for free.

**(c) A separate sink parameter.** `handle(event, router, shell: &mut dyn ShellSink)` or a small context struct `Ctx { router, shell }`.

- Same dependencies and rules as (b).
- Keeps module Routers purely about navigation, matching "Router (also called the Wireframe) Navigation and assembly" (`viper-architecture.md:13`).
- Cost: changes every `handle` signature and every test call site, and conflicts with the stated contract `handle(event, router)` (`viper-architecture.md:108`), which would need amending. Gains little over (b), since the bin builds both from the same queue anyway.

**(d) Presenter outbox or return value.** `handle` returns `Vec<ShellRequest>`, or the Presenter keeps an outbox the bin takes after `handle` (the TUI's `take_toasts` pattern, `bin-tui/src/view/mod.rs:646-649`).

- Same dependencies and rules as (b).
- Tests become very plain: assert on the returned requests.
- Cost: a Presenter that calls a child Presenter (a page forwarding to its Schedule tab) must merge outboxes by hand; the Router already threads through those calls. Two output channels (Router for navigation, return value for notices) for one concept, "what the Presenter asked for". A Dialog Presenter closes through its Router, so its notices would arrive by a different path from its close.

### 4.2 Home of the message type

**(i) `lib-viper`.** Generic, domain-free, already a dependency of every module and in `MIGRATED`. Its stated remit is "generic types every module's Presenters and keymap use" (`viper-architecture.md:86`), which a Presenter-to-Shell request fits. Needs `lib_viper -> lib_toast` for `ToastKind`, or a re-export so feature crates can drop their direct `lib_toast` dependency.

**(ii) `lib-toast`.** See section 5. Fits Toasts; misfits the status-line message.

**(iii) A new crate** (`lib-shell-protocol`, `lib-notice`). Toolkit-free, depended on by feature crates and `lib-shell`. Clean in principle, but it would hold one enum and one trait, adds a workspace member, a `MIGRATED` entry and a line in `viper-architecture.md:85`, and duplicates `lib-viper`'s job. Worth it only if the protocol grows beyond notices.

**(iv) `lib-shell` itself, with feature crates depending on it.** The dependency test would not catch it, since `lib_shell` is not in `FEATURE_CRATES` (`viper_dependencies.rs:24-36`), but it breaks the rule's intent: `lib-shell` is "a VIPER module like any other" (`viper-architecture.md:160`), and modules don't depend on modules. Every module would recompile on any Shell Presenter, keymap or Message change, and any Shell need to name module types (a Dialog host holding module Presenters, a destination list) would become a cycle. Rejected; if the Shell's protocol needs its own crate, that is (iii).

### 4.3 Delivery in the bin

**(x) gpui `emit` per request, as today.** Already deferred to after the update (section 2.5). Keeps the Shell a gpui subscriber. But it is Desktop-only, and once the Shell is a toolkit-free `lib-shell` Presenter, its handling of a request is plain Rust that should be unit-tested without gpui.

**(y) An explicit queue drained by `send`.** The bin's Router impls push `ShellRequest`s onto a `Vec` (or `VecDeque`) that `input.rs`'s `send(cx, presenter, event)` owns; after `handle` and the one `commit`, `send` drains it into the `lib-shell` Presenter (for example `shell.handle(ShellEvent::Request(r))`) and notifies. This is what the TUI already does with `take_toasts` (`bin-tui/src/shell.rs:1156-1163`), so both Clients converge on one shape. Write failures, which arrive later on the writer's channel (`write_failures.rs:23-46`), are mapped to Message text in the bin (only the bin sees `WriteFailure`) and pushed onto the same queue, or handed straight to the Shell Presenter, since they arrive outside any `send`.

The commit ordering matters less than it looks: `commit()` only queues the batch to the writer thread, and a failure arrives asynchronously later (ADR-0046, `:11`), so a Success Toast always shows before any rollback Error Toast whichever side of `commit` the drain sits. Draining after commit is still the right default, so a future synchronous refusal from commit could suppress or replace the pending Toast.

### 4.4 What about Navigate and open-Dialog?

They don't fit a shared enum. A Router reaches another module through that module's entry point in the same bin (`viper-architecture.md:128`), and a Dialog is module-specific: `UnitsDialog`, `PayRequest` and the rest are each crate's own types, and `open_dialog` returns whether it opened (`viper-architecture.md:127`), which a fire-and-forget queued request cannot. A generic `Navigate(Destination)` would need the destination list in a shared crate, which every new View would edit. These stay as typed methods on each `<Domain>Router`, implemented in the bin by calling the target's entry point or the Shell's Dialog host. So the shared type is for one-way notices to the Shell, and its name should say so (`ShellRequest` invites growth; `ShellNotice` or `Notice` doesn't).

## 5. Is `lib-toast` the right home?

- **What it is.** A shared, pure, dependency-free crate already on the allowed list for feature crates (`viper-architecture.md:85`; `lib-toast/Cargo.toml`), not a feature crate. Moving a request type into it breaks no rule.
- **Why it fits partly.** `ToastKind` is already there, and the rule text says "a Router asks for a Toast by `ToastKind`" (`viper-architecture.md:85`).
- **Why it is the wrong home for the request.** The request carries status-line messages too, which GLOSSARY and the design doc define as a separate concept from Toasts, with different lifetime, precedence and history rules (`GLOSSARY.md:91`; `docs/toasts-design.md:11`, `:46`, `:53`). A `lib_toast::ShellRequest::Status` would put a non-Toast into the Toast crate and make its name misleading. It would also pull a Presenter-to-Shell protocol into what is today a self-contained model, so a change to the protocol recompiles the model's users, and vice versa.
- **Where `lib-toast` goes in the target.** It stays the Toast model: `lib-shell`'s Presenter holds a `Toasts<S>` and the bins draw it. `lib-shell` depends on `lib-toast`; so does whatever crate holds the request, for `ToastKind`.
- **A rename instead?** Renaming `lib-toast` to something like `lib-notice` to cover both would blur a crate whose scope is crisp and documented (`docs/toasts-design.md:84`), for the sake of one enum.

So: `lib-toast` keeps `ToastKind` and the model; the request type goes elsewhere.

## 6. Recommendation

1. **Hand-off: (b), a supertrait.** `lib_viper::shell` (or `lib_viper::router`) declares `pub enum ShellNotice { Toast(ToastKind, String), Status(String) }` and `pub trait ShellRouter { fn notify(&mut self, notice: ShellNotice); }`, with provided `toast` and `status` helpers if they read better at call sites. Every `<Domain>Router` and cross-domain Router declares `: ShellRouter`, and `raise_toast` / `set_status` leave each `protocols.rs`. Presenters keep `handle(event, router)`.
2. **Home: `lib-viper`.** It is already every module's generic Presenter crate and in `MIGRATED`. Add `lib_viper -> lib_toast` and re-export `ToastKind` so feature crates can drop their own `lib_toast` dependency; update `viper-architecture.md:85` and `:86` to match. Add `lib_toast` to `MIGRATED` while there, so neither shared crate can reach a feature crate.
3. **Delivery: (y), one queue drained by `send`.** Each bin's Router impls (page and Dialog alike) push onto a queue owned by `input.rs`; `send` runs `handle`, commits once, drains the queue into the Shell (today's `Shell::raise_toast` / `status_message`, later the `lib-shell` Presenter), then notifies. The bin maps each `WriteFailure` to an Error notice into the same Shell entry point. Desktop `ViewEvent::RaiseToast` / `SetStatus` stay for Views not yet modules and go when the last of them migrates.
4. **Keep navigation and Dialogs typed** on each module's Router (section 4.4); the shared enum carries notices only.
5. **Tests:** each `RecordingRouter` records `ShellNotice`s in one variant; consider a shared recording half in a `lib-viper` `fake` module so a new module doesn't re-write it.
6. **TUI:** implement `ShellRouter` on the TUI Units Router by pushing onto its existing queue, and map `Status` to an Info Toast at the drain. Nothing else moves (`viper-architecture.md:231`).

## 7. Open decisions for the user

- **Name.** `ShellRequest` (as in the working proposal) or `ShellNotice` / `Notice`. The recommendation prefers a name that rules out Navigate and Dialog requests, per section 4.4.
- **Status payload.** `Status(String)` with the Shell clearing the message on the next keypress itself, as GLOSSARY defines it (`GLOSSARY.md:91`), or `Status(Option<String>)` so a module can clear explicitly, as Bills does on a tab switch (`bills_page.rs:221-229`). With `input.rs` offering every key to the Shell first (`viper-architecture.md:136`), the Shell can clear on each key and `None` becomes unnecessary.
- **Supertrait or sink parameter.** (b) keeps the documented `handle(event, router)` contract; (c) keeps Routers purely navigational at the cost of every signature. Confirm (b).
- **Helpers or one method.** `notify(ShellNotice)` alone, or provided `toast(kind, text)` and `status(text)` helpers on the trait for readable call sites.
- **`lib-viper` re-exporting `ToastKind`.** Whether feature crates drop their direct `lib_toast` dependency, and whether `viper-architecture.md:85` stops listing `lib-toast` for feature crates.
- **Where write failures enter.** Pushed onto the `send` queue (drained on the next event) or handed straight to the Shell when the failure poll fires (shown at once). The latter matches today's `roll_back` (`write_failures.rs:51-93`) and avoids an Error Toast waiting for a keypress.
- **Drain before or after commit.** After is recommended (section 4.3); confirm, since it fixes where a future synchronous commit refusal would hook in.
