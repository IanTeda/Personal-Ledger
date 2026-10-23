---
status: accepted
---

# Modules notify the Shell through one `lib-toast` trait

A module's Presenter needs to tell the person something through the Shell: a Toast, or a status-line message. Until now each module's Router had its own `raise_toast` and `set_status`, and the two had already drifted (Units took a `String`, Bills an `Option<String>`, Bill Payments had neither). On the Desktop, Dialog Routers dropped them silently. A feature crate can't depend on `lib-shell`, so the message type needs a crate that both sides may use. Decided in the [slice 3 grilling](https://github.com/IanTeda/Personal-Ledger/issues/806), with the options in `docs/research/shell-messaging-options.md`.

## Decision

- **`lib-toast` owns the message type and the trait.** It defines a `Notice` with two variants, a Toast (`ToastKind` and text) and a status-line message (text), and a `Notify` trait with one method, `notify(Notice)`. `lib-toast` stays a shared, toolkit-free and I/O-free crate rather than a feature crate, so every feature crate and `lib-shell` may depend on it. Its scope widens from the Toast model to everything the Shell tells the person.
- **Every module Router is a `Notify`.** Each `<Domain>Router` and each cross-domain Router, including the Dialog Routers, declares `Notify` as a supertrait. `raise_toast` and `set_status` leave each `protocols.rs`. A Presenter still reaches the Shell only through its Router.
- **Only one-way notices are shared.** Navigating and opening a Dialog need module-specific types and an answer ("did it open?"), so they stay typed methods on each module's Router.
- **The Shell clears the status-line message.** A module only ever sets it. It goes on the next keypress, so no module sends a clear.
- **The Desktop delivers notices through one queue.** The bin's Routers push onto a queue that `input.rs` owns, and `send` drains it into the Shell's Presenter after `handle` and `commit`. A write failure is not caused by a key, so the bin hands its Error Toast straight to the Shell, which then redraws. The TUI keeps its existing queue and still shows status text as an Info Toast.
- **The dependency test covers `lib-toast`.** `lib_toast` joins `MIGRATED` in `lib-ledger/tests/viper_dependencies.rs`, so it can never reach a feature crate, a toolkit or `lib-database`.

## Considered Options

- **Keep `raise_toast` and `set_status` on each Router.** Rejected: every module re-declares them, and they had already drifted.
- **Put `Notice` and `Notify` in `lib-viper`.** It is already every module's generic Presenter crate. Rejected so that what the Shell tells the person lives with the Toast model it feeds, and `lib-viper` stays about keymaps and form view models.
- **A shared enum that also carries Navigate and open-Dialog.** Rejected: those need per-module types and a return value, which one shared enum can't express.
- **`handle` returns the notices, or takes a separate sink.** Rejected: it changes every Presenter's signature, and a child Presenter would have to pass its notices up by hand.
- **gpui `emit` per notice, as before.** Rejected: `lib-shell` can't depend on gpui, and it can't be tested without a window.
- **Feature crates depend on `lib-shell`.** Rejected: it couples every module to the Shell, and the Shell's Router already depends on the modules.

## Consequences

- `lib-toast` now holds the status-line message as well as Toasts. Its crate doc and `docs/toasts-design.md` must say so, even though the Glossary keeps the two concepts apart.
- A Presenter test's `RecordingRouter` records notices in one variant, whatever the module.
