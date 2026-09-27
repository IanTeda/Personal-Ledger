# Toasts design

The developer-facing design for Toasts in the Desktop and TUI Clients. This page records the decisions reached on the [Toasts for the Desktop and TUI](https://github.com/IanTeda/Personal-Ledger/issues/305) Wayfinder map; each decision's detail lives on its ticket. The terms Toast, Toast Kind, Message, Preference and Colour Role are defined in `CONTEXT.md`.

## Status

Built in both Clients ([#342](https://github.com/IanTeda/Personal-Ledger/issues/342)–[#350](https://github.com/IanTeda/Personal-Ledger/issues/350)). Lifetime and stacking ([#306](https://github.com/IanTeda/Personal-Ledger/issues/306)), placement and layout ([#307](https://github.com/IanTeda/Personal-Ledger/issues/307)), Toast Kind colours and glyphs ([#308](https://github.com/IanTeda/Personal-Ledger/issues/308), [ADR-0026](adr/0026-toast-kinds-colour-their-marks-from-colour-roles.md)), turning Toasts off and the status-line echo ([#309](https://github.com/IanTeda/Personal-Ledger/issues/309), [ADR-0027](adr/0027-toasts-off-is-a-client-scoped-preference-and-errors-always-toast.md)), the Desktop and TUI rendering approach ([#310](https://github.com/IanTeda/Personal-Ledger/issues/310), [#311](https://github.com/IanTeda/Personal-Ledger/issues/311)), the event inventory ([#312](https://github.com/IanTeda/Personal-Ledger/issues/312)), the session Toast history ([#339](https://github.com/IanTeda/Personal-Ledger/issues/339)) and the shared model ([#340](https://github.com/IanTeda/Personal-Ledger/issues/340)) were decided on the map. Accessibility ([#341](https://github.com/IanTeda/Personal-Ledger/issues/341)) is deferred. Shipped differently from the design: the Toasts Preference is held in memory only (not yet stored), and each Client raises only the Toasts its current screens can trigger: the Desktop raises account, category and unit deletes, ledger open/new and save failures; the TUI raises account (including the moved-transactions variant `toast-account-deleted-moved`), payee and tag deletes and save failures.

## Toasts and the status-line message

A **status-line message** is feedback on the key or command just typed (an unbound `g`-jump, "not yet built", an ambiguous name) and clears on the next keypress. A **Toast** reports the outcome of an action or event, especially one whose effect is not visible where the user is looking, and is non-blocking. When the UI change is itself the feedback (a dialog closing on the edited row, a row greying on deactivate, a repaint on a Colour Theme change), neither is raised. The two coexist. Toasts carry no actions (no Undo or View buttons).

## Toast Kinds

| Kind | Lifetime | Mark colour | Glyph | TUI fallback |
|---|---|---|---|---|
| Info | 4 s | `foreground` | `i` | default colour |
| Success | 4 s | `positive` | `✓` | green |
| Warning | 8 s | `accent` | `!` | red |
| Error | sticky until dismissed | `negative` | `✗` (U+2717) | red |

Colour sits on the non-text marks only: the leading bar and the glyph, each in a calculated **mark shade** (the role corrected to 3:1 against chrome). The border stays `muted` and the text `foreground` on chrome, for every Kind. The glyph is the same in both Clients, so no Kind relies on colour alone; `✗` is distinct from the Desktop's `✕` dismiss. Lifetimes are fixed constants, not settings, and resolve at the TUI's 250 ms tick. See ADR-0026 and the Toast rows in `docs/colour-themes-design.md`.

## Lifetime, stacking and dismissal

- **Pausing:** a Toast's timer pauses on hover (Desktop) and while a modal surface is open (command palette, dialog, help overlay, TUI popup, the Toast history). No pause on window or terminal unfocus.
- **Stacking:** at most 3 visible. Past the limit the oldest *timed* Toast is evicted early; when every visible Toast is sticky, the oldest Error is not dropped but held back out of view until a slot frees. A muted `+N more` line above the stack counts the held-back Errors.
- **Duplicates:** a Toast with the same Kind and text as a visible one merges into it, restarting its timer and showing a muted `×N` badge.
- **Dismissal:** the Desktop ✕ on each Toast; `:dismiss` (newest) and `:dismiss all` in both Clients; `Ctrl+L` dismisses all, in `Normal`/`NORMAL` mode only (inert while the palette, a popup or a dialog has focus), remappable as `[keybindings] dismiss_toasts`. Deliberately not `Esc`, which already pops the TUI view stack and leaves Desktop modes and overlays.

## Placement and layout

- **Position:** bottom-right of the view, stacking upwards from just above the Desktop status line / TUI footer rule, newest nearest it. Desktop inset 16 px right, 12 px above the status line, 8 px between Toasts. The stack covers the right-hand end of the bottom rows, including a selected row, and that is accepted.
- **Layering:** Toasts draw last, above dialogs, the command palette and their scrim, so an outcome is never hidden by the surface that caused it. They never cover the status line or footer.
- **Content:** one line per Toast: leading bar, glyph, Message, `×N` badge, then the Desktop ✕. No title.
- **Width:** at most 420 px (Desktop) or 48 columns (TUI), never wider than the view. Long text truncates with `…` and never wraps; the full text is in the history. The badge and ✕ sit outside the truncated text and are never cut.
- **TUI:** each Toast is a 3-row bordered box. Under a 40×8 view no Toast is drawn (the status-line echo carries it), and stacking stops at whatever fits.

The chosen layout is Variant A of the prototype on branch `prototype/307-toast-placement`; the implementation rewrites it rather than merging it.

## Turning Toasts off

Toasts on/off is the first **Client-scoped Preference**: default on, one switch, stored locally and never synced, held in memory until the Settings screens read and write Preferences (as the Colour Theme Preferences are). It never silences an Error. See ADR-0027.

- **Echo:** with Toasts off, Info, Success and Warning Toasts go to a **status-line echo** (the Kind glyph in its mark colour, then the Message) for the Toast's lifetime; an Error still toasts. The echo also carries Toasts in a TUI view under 40×8, where an Error echo stays until dismissed. It shows only then, so the same text never shows twice.
- **Precedence on the status line (left side):** command echo > status-line message > Toast echo > page legend / shell hint strip. The right-hand side is untouched.
- **Switching off** removes the showing Info, Success and Warning Toasts, and the newest moves to the echo for the rest of its lifetime.
- **Control:** a "Toasts" On/Off row in each Client's Settings Display group, above the Colour Theme group, with a note that Errors still show; plus `toasts on` / `toasts off` palette commands.

## Session Toast history

- **Open:** the `toasts` command (alias `messages`), and an unbound `[keybindings] toast_history`.
- **Contents:** Toasts only, whatever the Preference; status-line messages are not recorded. 100 entries in memory, oldest dropped, no clear; it empties on restart.
- **Entry:** Kind glyph in its mark colour, full untruncated Message, time last raised (Locale-formatted `HH:MM:SS`), `×N`. A merged duplicate is one entry. Dismissed or expired state is not shown.
- **Display:** a modal popup in both Clients, newest first, scrollable, `Esc` closes. Empty state is the Message `toast-history-empty`. The TUI follows existing popups' small-size behaviour.
- **Interaction:** showing Toasts are hidden while the popup is open and their timers resume on close. Viewing dismisses nothing, including sticky Errors.

## Which events raise a Toast

Nothing on the status line today moves to a Toast. New Messages are namespaced `toast-*` in the shared `lib-locale` Catalogue.

| Event | Client | Kind | Message |
|---|---|---|---|
| Delete account (with transactions moved or deleted) | both | Success | `toast-account-deleted` |
| Delete category (splits re-pointed to Uncategorised) | both | Success | `toast-category-deleted` |
| Delete payee / tag / unit | both | Success | `toast-payee-deleted`, `toast-tag-deleted`, `toast-unit-deleted` |
| Store refusal now swallowed by `let _ =` / `.is_ok()` | TUI (Desktop once persisted) | Error | `toast-save-failed` |
| Open ledger / new ledger | Desktop | Success | `toast-ledger-opened`, `toast-ledger-created` |
| Open or create ledger fails (once wired) | both | Error | `toast-ledger-open-failed` |
| Base-unit change committed | TUI, later Desktop | Success | `toast-base-unit-changed` |
| CSV import finished / partial / failed | both (future) | Success / Warning / Error | `toast-import-finished`, `toast-import-partial`, `toast-import-failed` |
| Backup or export finished / failed | Desktop (future) | Success / Error | `toast-backup-*`, `toast-export-*` |
| Sync finished with changes / up to date (manual `Sync now` only) / failed / conflict resolved | both (future) | Success / Info / Error / Warning | `toast-sync-finished`, `toast-sync-up-to-date`, `toast-sync-failed`, `toast-sync-conflict` |
| Configuration or Colour Theme file fell back at start-up | both (future) | Warning | `toast-config-fallback` |
| Balance check mismatch on commit | both (future) | Warning | `toast-balance-mismatch` |

Future rows are wired by the ticket that builds their feature, not by the Toasts build.

## Architecture

### `lib-toast`

A new pure, I/O-free crate, `crates/libs/lib-toast` (package `lib_toast`), used by both bins, following the `lib-colour-theme` precedent. It owns the Toast Kinds, per-Kind lifetimes, the visible queue (max 3, eviction, the `+N more` count), pause and resume, `×N` merging on Kind + text, dismissal (newest / all), the 100-entry history and choosing what the status-line echo shows given the Preference. The rules are unit-tested once so the Clients cannot drift.

- **Clock:** `advance(elapsed: Duration)`, called from the TUI's `Event::Tick` and a Desktop timer; pausing means not advancing. It is deterministic in tests.
- **Messages:** resolved to a `String` at raise time (the Locale is set once per run). The history's time raised is a wall-clock stamp the bin passes in.
- **Bins own:** drawing, input and key bindings, the hover and modal signals that pause the timer, and reading the Toasts Preference, which they pass to the model.

### Desktop

A custom Toast layer owned by `Shell`, drawn as the last absolute child in `Shell::render`, not gpui-component 0.5.1's `Notification` (which needs `gpui_component::Root` as the window root, hardcodes a 5 s timeout and top-right placement, colours glyphs from its own theme and has no hover pause or keyboard dismissal). Timers use `cx.spawn` + `Timer::after`, hover uses `on_hover`, keys go through `key_router`, colours read `crate::theme`.

### TUI

A hand-rolled overlay owned by `Shell`, drawn last in `Shell::draw` (`Clear` + bordered `Block`, before `paint_base`), advanced on the existing 250 ms `Event::Tick`. No ratatui crate fits: `ratatui-toaster` shows one Toast with fixed colours, `ratatui-notifications` lacks Success and is unmaintained, `hjkl-holler-tui` is app-specific, `ratatui-comfy-toaster` fails `deny.toml`.

## Not part of Toasts

- **Actions on a Toast** (Undo after a delete, View after an import): neither Client has a command or undo model yet.
- **Accessibility** ([#341](https://github.com/IanTeda/Personal-Ledger/issues/341)): deferred. `gpui` 0.2.2 has no accessibility tree, so Desktop screen-reader announcement waits on gpui; in the TUI, Toasts off plus the status-line echo is the natural fallback.
- **Operating-system notifications:** "notification" is reserved for them; a Toast is always drawn inside the Client.
