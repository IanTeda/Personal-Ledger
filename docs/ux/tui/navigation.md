# TUI Navigation Model

This is the living reference for how keyboard navigation actually works in the Personal Ledger TUI: the philosophy behind it, what a command is, what a keybinding is, and how the two relate. `README.md` in this directory is the original design handoff — low-to-mid fidelity, and now partly overtaken by real implementation decisions (see "Where this differs from the handoff" below). This document describes what's actually decided and, where marked, what's actually shipped, so it can be read on its own without cross-referencing every ticket that produced it.

The full decision trail behind this document lives on the closed [Command popup finishing](https://github.com/IanTeda/Personal-Ledger/issues/89) Wayfinder map and its children (issues #90–#98).

## Philosophy

The TUI has no menus, no breadcrumb trail, and no sidebar. At any moment there is exactly one thing on screen: a status line, one full-bleed view, an idle command line, and a dim keybind hint bar. Everything else — every action the app can take — is reached through a single floating command popup.

This is a deliberate bet, not a default. A persistent nav structure (tabs, a sidebar, a menu bar) has a fixed cost in screen space and a fixed ceiling on how many actions it can expose before it becomes its own navigation problem. A command popup has zero footprint at rest and scales to an arbitrarily large action surface — dozens of commands across many domains cost the same one line of chrome as three. As the app grows from a handful of screens to dozens of report variants, budget operations, and account actions, the popup doesn't get more crowded; only its own internal list does, and that list is searchable.

The trade-off this bet makes is discoverability: a menu bar shows you everything you can do just by looking; a command popup shows you nothing until you open it. Everything below — the registry, the grouping, the ranking, the argument preview — exists to pay down that trade-off deliberately, rather than accept it as a cost of "keyboard-only."

Three commitments follow from that bet:

- **Every action must be reachable by name.** A keybinding is a shortcut to a command, never the only path to it. If a chord is forgotten, mistyped, or simply not memorised yet, typing the command's name in the popup always works.
- **The popup never lies about what it can do.** A command that has no real behaviour behind it yet says so explicitly when run, rather than silently doing nothing or (worse) opening something half-finished with no explanation.
- **The interface always says what mode it's in.** Modal, vim-flavoured: `NORMAL` at rest, `COMMAND` while the popup is open, `INSERT` inside a form. The status line names the mode whenever it isn't `NORMAL`, so a keystroke's meaning is never ambiguous.

## Commands

A **command** is an entry in a domain-grouped registry (`crates/bins/bin-tui/src/popup/command/commands/`, one file per domain — Dashboard, Accounts, Balance Checks, Budgets, Categories, Help, Payees, Reports, Transactions, Units). Each command has:

- a **name** — noun-first, e.g. `budget new <category> <limit>`, `account list`, `unit edit <code>` — the canonical, typeable form of the command, placeholders included as literal text
- an optional **chord** — a keybinding shortcut, rendered as `—` when none exists yet
- a **description** — one line, shown alongside the command in the popup
- a set of **args** — for a command with placeholders, a fixed preview value per argument, shown as a confirmation row before the command runs (see "Argument preview" below)

Commands are grouped by the domain they belong to, not by whether they're built yet. An unbuilt command still appears in the list, still has a description, and still tells you what it would do — the registry describes the *intended* surface of the app, and the "not yet built" fallback (below) is what happens when you try to use a piece of that surface before its real behaviour exists.

The registry is a single source of truth: the popup's results list, its ranking, its keybinding column, and (eventually) a `:help` browse view are all generated from the same data rather than kept in sync by hand.

## Keybindings

Keybindings fall into three tiers, checked in this order — a key that means something at a higher tier always wins over a lower one:

1. **Hard quit** — `Ctrl+C` exits immediately, from anywhere, no matter what's open. This is the one keybinding with no graceful variant.
2. **Popup-owned keys** — while the command popup or a form popup (e.g. a Units form) is open, it owns every keystroke. Nothing below this tier is reachable until the popup closes.
3. **Global keys** — reachable only when no popup is open: opening the popup, jump chords, and the two navigation keys, `Esc` and `q` (see "Navigating between views" below).
4. **View-owned keys** — whatever's left falls to the active view's own key handling (e.g. a future list view's own `j`/`k` row selection).

### Global keys (no popup open)

| key | effect |
| --- | --- |
| `Ctrl+;` | opens the command popup |
| `Ctrl+U` | opens Units directly — a shortcut alongside the popup's own `unit`/`g u` route |
| `?` | opens Help |
| `g` then a letter | jumps directly to a domain view (`g a` accounts, `g b` budgets, `g c` categories, `g d` dashboard, `g k` balance checks, `g p` payees, `g r` reports, `g t` transactions, `g u` units) |
| `Esc` | pops one level back through the view-navigation stack |
| `q` | quits the app |

### Popup keys (command popup open)

| key | effect |
| --- | --- |
| typing | filters and ranks the command list live, updating the match count |
| `↑` / `↓` | move the selection |
| `Tab` | completes the input to the selected command's text up to its first placeholder — a narrow-to-one-match confirmation, not argument entry |
| `Ctrl+r` | walks backward through a history of previously run commands |
| `Enter` | runs the selected command, or shows the "not yet built" message |
| `Esc` | closes the popup, returns to `NORMAL` |

A chord is always optional. Every command above is equally reachable by opening the popup and typing its name — the chord column exists so frequent commands can be learned as shortcuts over time, not so infrequent ones have to be memorised up front.

## Using the popup: ranking, argument preview, and the "not yet built" message

Typing into the popup filters the full command list and ranks what's left in three tiers: a match at the very start of a command's name ranks above a match found elsewhere in its name or domain, which ranks above a match found only in its description. Ties keep the list's normal order. The part of each row that actually matched is highlighted, so it's visible *why* a result surfaced, not just *that* it did.

If the highlighted command takes arguments, a preview row appears above the popup's footer showing what it would act on — today resolved against fixed placeholder data (e.g. `<category> — dining · limit 300.00 · actual 412.00`), since there's no way yet to type a real argument value into the popup. This exists because a command-driven UI with no menus has no other confirmation step before an action runs; the preview is that check.

If a command has no real behaviour behind it yet, running it (`Enter`) replaces that same row with `:{command} — not yet built` instead of doing nothing or opening something empty. The popup stays open — nothing about a failed run closes it or changes what you're doing — and the message clears itself as soon as you type, arrow, or tab again.

## Navigating between views

`Shell` holds exactly one active view and a stack of the views you navigated away from to get there. Opening a new view pushes whatever was active onto that stack; `Esc` (with no popup open) pops it back. Two things sit outside that rule:

- **Dashboard is home.** Jumping to Dashboard from anywhere clears the stack rather than adding to it — it's the one view every navigation implicitly starts from, so returning to it always means starting fresh, never retracing a specific path back through it.
- **Popups are not stack levels.** The command popup and any form popup are transient overlays on top of whichever view is active; they never get pushed, and closing one (`Esc`) never pops the view underneath it. `Esc`'s two jobs — close a popup, or step back through the view stack — never collide, because a popup, while open, always wins.

`q` and `Esc` are deliberately two different gestures, not two names for the same idea. `Esc` retraces a step and is always safe to press — worst case, it does nothing (an empty stack, or the Dashboard you're already home at). `q` ends the session outright. Keeping them distinct means muscle memory for "back out of what I just did" never accidentally exits the app, and "I'm done" never gets confused with "take me back one screen."

`q` quits gracefully rather than reusing `Ctrl+C`'s hard-quit path, even though the two currently do the same thing: nothing in the app holds unsaved state yet, so there's nothing to confirm. Routing `q` through its own action now means that when a form eventually does hold dirty state, a confirm-before-quit check has one place to slot in — `q`'s own handler — without changing what key the user presses.

## Current implementation status

Two views have real (if wireframe-stage) content: **Dashboard**, the default home view, and **Units**. The other eight domain views (Accounts, Balance Checks, Budgets, Categories, Payees, Reports, Transactions, Help) exist as bare, empty bordered boxes — real screens with real navigation paths to them, but no content yet. Only five commands have real behaviour behind `Enter` in the popup today: `unit`, `unit new/edit/delete`, and `dashboard`. Every other command in the registry — every mutation, every specific report, and, deliberately, every one of the eight placeholder-view list commands — shows the "not yet built" message described above rather than opening something empty.

One known inconsistency: the `g`-jump keys (`g a`, `g b`, etc.) still open those eight placeholder views directly, silently, rather than showing the "not yet built" message — that message currently only exists inside the command popup, and a bare `g`-jump has no equivalent surface to show it on. This is tracked separately as [#96](https://github.com/IanTeda/Personal-Ledger/issues/96); closing it means designing a feedback surface outside the popup, not a quick fix.

## Where this differs from the handoff

`README.md` in this directory is the original design reference and is treated as an idea inventory, not a binding spec — real implementation decisions have superseded a few of its specifics:

- The popup opens on `Ctrl+;`, not a bare `:` — the handoff's own key.
- The command list is grounded in this app's actual screens (Units, Accounts, Categories, and so on), not the handoff's aspirational grammar, which includes nouns (`sync`, `price`) nothing yet backs.
- `Esc` and `q` are assigned as described above — the handoff's own state notes describe a "view stack for `q`," an earlier framing this document's key assignments deliberately replace.

Where the handoff and this document disagree, this document is current.
